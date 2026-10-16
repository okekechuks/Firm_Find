mod commands;
pub mod connection;
pub mod discovery;
pub mod domain;
pub mod profiles;
pub mod storage;

use commands::AppState;
use std::sync::{Arc, Mutex};
use tauri::Manager;

pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .setup(|app| {
            let directory = app.path().local_data_dir()?.join("OBTAINER");
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
            repository.record_startup()?;
            let mut profiles = profiles::Registry::bundled();
            repository.install_registry(&mut profiles)?;
            app.manage(profiles);
            let repository = Arc::new(Mutex::new(repository));
            let discovery =
                discovery::DiscoveryService::start(app.handle().clone(), Arc::clone(&repository))?;
            app.manage(discovery);
            app.manage(connection::ConnectionService::default());
            app.manage(AppState {
                repository,
                directory,
            });
            tracing::info!(version = env!("CARGO_PKG_VERSION"), "OBTAINER initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
            commands::record_device_observation,
            commands::get_device_compatibility,
            commands::validate_device_profile
        ])
        .build(tauri::generate_context!())?
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app.state::<discovery::DiscoveryService>().shutdown();
            }
        });
    Ok(())
}
