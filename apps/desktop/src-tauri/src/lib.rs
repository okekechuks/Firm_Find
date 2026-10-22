pub mod acquisition;
pub mod adapters;
mod commands;
pub mod connection;
pub mod discovery;
pub mod domain;
pub mod library;
mod native_test;
pub mod openwrt;
pub mod profiles;
pub mod simulator;
pub mod startup;
pub mod storage;

use commands::AppState;
use std::sync::{Arc, Mutex};
use tauri::Manager;

pub fn run() -> tauri::Result<()> {
    native_test::reject_overrides(false)?;
    if std::env::args_os()
        .skip(1)
        .any(|a| a == "--simulator" || a == "--openwrt-lab")
    {
        return Err(std::io::Error::other(
            "Simulation requires the dedicated isolated native-test executable.",
        )
        .into());
    }
    run_profile(None)
}

struct NativeProfile {
    root: std::path::PathBuf,
    id: String,
    simulator: bool,
    openwrt_lab: bool,
    _owner: Box<dyn std::any::Any + Send + Sync>,
}

#[cfg(feature = "native-test-profile")]
pub fn run_native_test() -> tauri::Result<()> {
    let id = native_test::reject_overrides(true)?.expect("validated test ID");
    let local = native_test::local_data()?;
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--cleanup-native-test-run"] {
        native_test::cleanup(&local, &id)?;
        println!("{{\"removed\":true,\"runId\":\"{id}\"}}");
        return Ok(());
    }
    let simulator = args.iter().any(|arg| arg == "--simulator");
    let openwrt_lab = args.iter().any(|arg| arg == "--openwrt-lab");
    if args
        .iter()
        .any(|arg| arg != "--simulator" && arg != "--openwrt-lab")
        || args.iter().collect::<std::collections::BTreeSet<_>>().len() != args.len()
    {
        return Err(std::io::Error::other(
            "Unsupported native-test arguments; paths are never accepted.",
        )
        .into());
    }
    let run = native_test::TestRun::open(&local, &id)?;
    run_profile(Some(NativeProfile {
        root: run.root.clone(),
        id: run.id.clone(),
        simulator,
        openwrt_lab,
        _owner: Box::new(run),
    }))
}

fn run_profile(profile: Option<NativeProfile>) -> tauri::Result<()> {
    let _instance =
        startup::InstanceLock::acquire(profile.as_ref().map_or("production", |p| p.id.as_str()))?;
    let mut context = tauri::generate_context!();
    let expected = if profile.is_some() {
        "com.obtainer.desktop.native-test"
    } else {
        "com.obtainer.desktop"
    };
    if context.config().identifier != expected {
        return Err(std::io::Error::other(
            "Executable/configuration identity mismatch; startup refused.",
        )
        .into());
    }
    if let Some(test) = &profile {
        for window in &mut context.config_mut().app.windows {
            window.data_directory = Some(test.root.join("webview"));
            window.title = format!("OBTAINER — ISOLATED NATIVE TEST {}", test.id);
        }
    }
    tauri::Builder::default()
        .setup(move |app| {
            let simulation_enabled = profile.as_ref().is_some_and(|p| p.simulator);
            let openwrt_lab_enabled = profile.as_ref().is_some_and(|p| p.openwrt_lab);
            let simulation_owner = profile.as_ref().map(|p| p.id.clone()).unwrap_or_default();
            let directory = profile
                .as_ref()
                .map(|p| p.root.join("data"))
                .unwrap_or(app.path().local_data_dir()?.join("OBTAINER"));
            let test_profile = profile.as_ref().map(|p| domain::NativeTestInfo {
                run_id: p.id.clone(),
                discovery_source: "windows_observed".into(),
                simulated_adapters: false,
                simulation_available: if simulation_enabled { Some(true) } else { None },
            });
            if let Some(profile) = profile {
                app.manage(profile);
            }
            for child in ["logs", "vault", "workspaces", "reports"] {
                std::fs::create_dir_all(directory.join(child))?;
            }
            let writer = tracing_appender::rolling::daily(directory.join("logs"), "obtainer.jsonl");
            let (writer, guard) = tracing_appender::non_blocking(writer);
            tracing_subscriber::fmt()
                .json()
                .with_ansi(false)
                .with_writer(writer)
                .try_init()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            app.manage(guard);
            let mut repository = storage::Repository::open(&directory.join("obtainer.db"))?;
            let mut profiles = profiles::Registry::bundled();
            repository.install_registry(&mut profiles)?;
            #[cfg(feature = "native-test-profile")]
            if openwrt_lab_enabled {
                profiles =
                    openwrt::runtime::lab_profiles(profiles).map_err(std::io::Error::other)?;
            }
            let _ = openwrt_lab_enabled;
            app.manage(profiles);
            #[cfg(feature = "native-test-profile")]
            let openwrt_service = if openwrt_lab_enabled {
                openwrt::runtime::Service::native_lab()
            } else {
                openwrt::runtime::Service::default()
            };
            #[cfg(not(feature = "native-test-profile"))]
            let openwrt_service = openwrt::runtime::Service::default();
            app.manage(Arc::new(openwrt_service));
            app.manage(adapters::Registry::production().map_err(std::io::Error::other)?);
            app.manage(adapters::SessionManager::default());
            app.manage(acquisition::Service::default());
            app.manage(Arc::new(simulator::SimulatorService::new(
                simulation_enabled,
                simulation_owner,
            )));
            let repository = Arc::new(Mutex::new(repository));
            let events = app.handle().clone();
            let max = std::env::var("OBTAINER_IMPORT_MAX_BYTES")
                .ok()
                .map(|v| v.parse::<u64>())
                .transpose()?
                .unwrap_or(library::DEFAULT_MAX);
            app.manage(library::LibraryService::start(
                Arc::clone(&repository),
                directory.clone(),
                max,
                Arc::new(move |job| {
                    use tauri::Emitter;
                    let _ = events.emit("library-job-changed", job);
                }),
            )?);
            repository
                .lock()
                .map_err(|_| std::io::Error::other("Workspace lock unavailable"))?
                .record_startup()?;
            let discovery =
                discovery::DiscoveryService::start(app.handle().clone(), Arc::clone(&repository))?;
            app.manage(discovery);
            app.manage(connection::ConnectionService::default());
            app.manage(AppState {
                repository,
                directory,
                test_profile,
            });
            tracing::info!(version = env!("CARGO_PKG_VERSION"), "OBTAINER initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            acquisition::ipc::discover_firmware_routes,
            acquisition::ipc::cancel_firmware_route_discovery,
            acquisition::ipc::export_acquisition_diagnostics,
            openwrt::ipc::openwrt_status,
            openwrt::ipc::openwrt_observe,
            openwrt::ipc::openwrt_connect,
            openwrt::ipc::openwrt_approve_reads,
            openwrt::ipc::openwrt_inspect,
            openwrt::ipc::openwrt_cancel,
            openwrt::ipc::openwrt_disconnect,
            simulator::ipc::simulation_status,
            simulator::ipc::simulation_control,
            simulator::ipc::simulation_request,
            library::ipc::get_desktop_state,
            library::ipc::save_desktop_preferences,
            library::ipc::list_firmware_backups,
            library::ipc::create_firmware_backup,
            library::ipc::restore_firmware_backup,
            library::ipc::remove_firmware_item,
            library::ipc::open_managed_folder,
            library::ipc::select_library_directory,
            library::ipc::relocate_firmware_library,
            library::ipc::recover_default_library,
            library::ipc::select_firmware_file,
            library::ipc::discard_firmware_selection,
            library::ipc::start_firmware_import,
            library::ipc::list_firmware_artifacts,
            library::ipc::get_firmware_artifact,
            library::ipc::list_firmware_import_history,
            library::ipc::verify_firmware_integrity,
            library::ipc::cancel_library_job,
            library::ipc::list_library_jobs,
            library::ipc::get_library_job,
            library::ipc::get_library_activity,
            library::ipc::export_artifact_manifest,
            commands::get_snapshot,
            commands::save_settings,
            commands::get_discovery,
            commands::refresh_discovery,
            commands::get_manual_routers,
            commands::add_manual_router,
            commands::remove_manual_router,
            commands::probe_management_endpoint,
            commands::cancel_endpoint_probe,
            commands::cancel_discovery,
            commands::get_profile_registry,
            commands::list_available_adapters,
            commands::get_device_adapter_matches,
            commands::record_device_observation,
            commands::get_device_compatibility,
            commands::validate_device_profile
        ])
        .build(context)?
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app.state::<Arc<openwrt::runtime::Service>>().shutdown();
                app.state::<Arc<simulator::SimulatorService>>().shutdown();
                app.state::<adapters::SessionManager>().shutdown();
                app.state::<library::LibraryService>().stop();
                app.state::<discovery::DiscoveryService>().shutdown();
            }
        });
    Ok(())
}
