use super::*;
use crate::{commands::AppState, connection::ConnectionService, discovery::DiscoveryService};
use std::sync::Arc;

pub(super) fn reserve_probe(
    connections: &ConnectionService,
    service: &Service,
    device: &Device,
    revision: u64,
    input: &DiscoveryRequest,
) -> Result<(connection::ProbeLease, Option<ClaimedApproval>), DiscoveryError> {
    let lease = connections
        .begin(&format!("acquisition:{}", device.id))
        .map_err(|_| error("BUSY", "Another route check is finishing. Try again."))?;
    let proof = if let Some(receipt) = input.approval_token.as_deref() {
        let binding = approval::binding(device, revision, input)?;
        Some(service.1.claim(receipt, &binding)?)
    } else {
        None
    };
    Ok((lease, proof))
}

#[tauri::command]
pub async fn discover_firmware_routes(
    input: DiscoveryRequest,
    state: tauri::State<'_, AppState>,
    discovery: tauri::State<'_, DiscoveryService>,
    connections: tauri::State<'_, ConnectionService>,
    service: tauri::State<'_, Service>,
) -> Result<AcquisitionCapabilityReport, DiscoveryError> {
    let snapshot = discovery
        .snapshot()
        .map_err(|_| error("DISCOVERY_UNAVAILABLE", "Refresh discovery first."))?;
    let device = snapshot
        .devices
        .iter()
        .find(|d| d.id == input.device_id)
        .cloned()
        .ok_or_else(|| error("DEVICE_NOT_FOUND", "Select a current device."))?;
    let (lease, approval) =
        reserve_probe(&connections, &service, &device, snapshot.revision, &input)?;
    service
        .0
        .lock()
        .map_err(|_| error("INTERNAL", "Discovery state unavailable."))?
        .remove(&input.device_id);
    let repo = Arc::clone(&state.repository);
    let live = discovery.shared_snapshot();
    let (report, lease) = tauri::async_runtime::spawn_blocking(move || {
        let repo = repo
            .lock()
            .map_err(|_| error("DATABASE_UNAVAILABLE", "Identity evidence unavailable."))?;
        let saved = repo
            .routers()
            .map_err(|_| error("DATABASE_UNAVAILABLE", "Saved evidence unavailable."))?;
        let mut evidence = profiles::gather_evidence(&device, &saved);
        evidence.extend(
            repo.identity_observations(&device.id)
                .map_err(|_| error("DATABASE_UNAVAILABLE", "Identity evidence unavailable."))?,
        );
        drop(repo);
        let all_networks: Vec<_> = snapshot
            .devices
            .iter()
            .flat_map(|d| d.networks.clone())
            .map(|n| (n.id.clone(), n))
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect();
        let report = discover_on_networks(
            &device,
            evidence,
            snapshot.revision,
            &input,
            &lease.cancelled,
            &all_networks,
            approval,
        )?;
        if lease.cancelled.load(Ordering::Acquire) {
            return Err(error("CANCELLED", "Route discovery cancelled."));
        }
        Ok((report, lease))
    })
    .await
    .map_err(|_| error("WORKER_FAILED", "Route discovery worker unavailable."))??;
    let current = live
        .lock()
        .map_err(|_| error("DISCOVERY_UNAVAILABLE", "Inventory unavailable."))?;
    if lease.cancelled.load(Ordering::Acquire) {
        return Err(error("CANCELLED", "Route discovery cancelled."));
    }
    if current.revision != report.discovery_revision
        || !current
            .devices
            .iter()
            .any(|d| d.id == report.device_id && d.connected)
    {
        return Err(error(
            "STALE_RESULT",
            "Device changed during route discovery. Check again.",
        ));
    }
    lease
        .if_current(|| {
            let mut reports = service
                .0
                .lock()
                .map_err(|_| error("INTERNAL", "Route state unavailable."))?;
            reports.retain(|_, r| r.expires_at > connection::now());
            if reports.len() >= 64 {
                reports.clear();
            }
            reports.insert(report.device_id.clone(), report.clone());
            Ok(())
        })
        .ok_or_else(|| error("CANCELLED", "Route check was cancelled or superseded."))??;
    Ok(report)
}

#[tauri::command]
pub fn cancel_firmware_route_discovery(
    device_id: String,
    connections: tauri::State<'_, ConnectionService>,
) {
    if device_id.len() == 64 {
        connections.cancel(&format!("acquisition:{device_id}"));
    }
}

#[tauri::command]
pub async fn export_acquisition_diagnostics(
    device_id: String,
    app: tauri::AppHandle,
    service: tauri::State<'_, Service>,
    discovery: tauri::State<'_, DiscoveryService>,
    library: tauri::State<'_, crate::library::LibraryService>,
) -> Result<bool, DiscoveryError> {
    let revision = discovery
        .snapshot()
        .map_err(|_| error("DISCOVERY_UNAVAILABLE", "Inventory unavailable."))?
        .revision;
    let value = {
        let reports = service
            .0
            .lock()
            .map_err(|_| error("INTERNAL", "Route state unavailable."))?;
        let report = reports
            .get(&device_id)
            .filter(|r| r.discovery_revision == revision && r.expires_at > connection::now())
            .ok_or_else(|| {
                error(
                    "STALE_RESULT",
                    "Run Check Compatibility again before exporting.",
                )
            })?;
        diagnostics(report)
    };
    use tauri::Manager;
    let owner = app
        .get_webview_window("main")
        .and_then(|w| w.hwnd().ok())
        .map(|h| h.0 as isize)
        .unwrap_or(0);
    let library = library.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        library.export_redacted_report(&value, owner).map_err(|_| {
            error(
                "EXPORT_FAILED",
                "Choose a new report filename outside application storage.",
            )
        })
    })
    .await
    .map_err(|_| error("WORKER_FAILED", "Export worker unavailable."))?
}

/// Native confirmation cannot be fabricated by a caller-controlled webview boolean.
#[tauri::command]
pub async fn approve_firmware_route_probe(
    input: DiscoveryRequest,
    app: tauri::AppHandle,
    discovery: tauri::State<'_, DiscoveryService>,
    service: tauri::State<'_, Service>,
) -> Result<String, DiscoveryError> {
    let snapshot = discovery
        .snapshot()
        .map_err(|_| error("DISCOVERY_UNAVAILABLE", "Refresh discovery."))?;
    let device = snapshot
        .devices
        .iter()
        .find(|d| d.id == input.device_id)
        .ok_or_else(|| error("DEVICE_NOT_FOUND", "Select a current device."))?;
    let binding = approval::binding(device, snapshot.revision, &input)?;
    use tauri::Manager;
    let owner = app
        .get_webview_window("main")
        .and_then(|w| w.hwnd().ok())
        .map(|h| h.0 as isize)
        .unwrap_or(0);
    let message=format!("Approve one bounded, read-only check?\n\nDevice: {}\nInterface: {}\nEndpoint: {}\nAdvanced endpoint: {}\n\nGET / and, only for a recognized ASR family, GET /version_num.txt. No login or device changes.",approval::confirmation_value(&device.display_name),approval::confirmation_value(input.interface_id.as_deref().unwrap_or("")),approval::confirmation_value(input.endpoint.as_deref().unwrap_or("")),input.advanced_endpoint);
    let approved = tauri::async_runtime::spawn_blocking(move || {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::*;
            let text: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
            let title: Vec<u16> = "OBTAINER - approve read-only check"
                .encode_utf16()
                .chain(Some(0))
                .collect();
            unsafe {
                MessageBoxW(
                    owner as _,
                    text.as_ptr(),
                    title.as_ptr(),
                    MB_YESNO | MB_DEFBUTTON2 | MB_ICONQUESTION,
                ) == IDYES
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (owner, message);
            false
        }
    })
    .await
    .map_err(|_| error("WORKER_FAILED", "Approval unavailable."))?;
    if !approved {
        return Err(error("CANCELLED", "Endpoint check not approved."));
    }
    let current = discovery
        .snapshot()
        .map_err(|_| error("DISCOVERY_UNAVAILABLE", "Refresh discovery."))?;
    if current.revision != snapshot.revision {
        return Err(error("STALE_RESULT", "Device changed. Approve again."));
    }
    service.1.issue(binding)
}

fn cached_menu_report(
    service: &Service,
    device: &Device,
    revision: u64,
    input: &DiscoveryRequest,
) -> Result<AcquisitionCapabilityReport, DiscoveryError> {
    let target = connection::parse_address(input.endpoint.as_deref().unwrap_or(""))
        .map_err(|_| error("INVALID_ENDPOINT", "Select the checked endpoint."))?;
    approval::binding(device, revision, input)?;
    let report = service
        .0
        .lock()
        .map_err(|_| error("INTERNAL", "Menu state unavailable."))?
        .get(&device.id)
        .cloned()
        .ok_or_else(|| error("STALE_RESULT", "Run the root compatibility check first."))?;
    if report.discovery_revision != revision
        || report.expires_at <= connection::now()
        || report.fingerprint.endpoint.as_deref() != Some(&target.url)
        || report.fingerprint.interface_id != input.interface_id
    {
        return Err(error(
            "STALE_RESULT",
            "Run the root check again for this endpoint.",
        ));
    }
    if !report
        .protocols
        .iter()
        .any(|p| p.family == "asr_duster_xml")
    {
        return Err(error(
            "NO_RECOGNIZED_FAMILY",
            "No recognized family; no static menu request sent.",
        ));
    }
    Ok(report)
}

#[tauri::command]
pub async fn approve_static_menu(
    input: DiscoveryRequest,
    app: tauri::AppHandle,
    discovery: tauri::State<'_, DiscoveryService>,
    service: tauri::State<'_, Service>,
) -> Result<String, DiscoveryError> {
    let snapshot = discovery
        .snapshot()
        .map_err(|_| error("DISCOVERY_UNAVAILABLE", "Refresh discovery."))?;
    let device = snapshot
        .devices
        .iter()
        .find(|d| d.id == input.device_id)
        .ok_or_else(|| error("DEVICE_NOT_FOUND", "Select a current device."))?;
    cached_menu_report(&service, device, snapshot.revision, &input)?;
    let binding = approval::binding(device, snapshot.revision, &input)?;
    let message = approval::menu_confirmation(device, &binding);
    use tauri::Manager;
    let owner = app
        .get_webview_window("main")
        .and_then(|w| w.hwnd().ok())
        .map(|h| h.0 as isize)
        .unwrap_or(0);
    let approved = tauri::async_runtime::spawn_blocking(move || {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::*;
            let text: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
            let title: Vec<u16> = "OBTAINER - approve static menu check"
                .encode_utf16()
                .chain(Some(0))
                .collect();
            unsafe {
                MessageBoxW(
                    owner as _,
                    text.as_ptr(),
                    title.as_ptr(),
                    MB_YESNO | MB_DEFBUTTON2 | MB_ICONQUESTION,
                ) == IDYES
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (owner, message);
            false
        }
    })
    .await
    .map_err(|_| error("WORKER_FAILED", "Menu approval unavailable."))?;
    if !approved {
        return Err(error("CANCELLED", "Static menu check not approved."));
    }
    let current = discovery
        .snapshot()
        .map_err(|_| error("DISCOVERY_UNAVAILABLE", "Refresh discovery."))?;
    let device = current
        .devices
        .iter()
        .find(|d| d.id == input.device_id)
        .ok_or_else(|| error("DEVICE_NOT_FOUND", "Select a current device."))?;
    cached_menu_report(&service, device, current.revision, &input)?;
    if approval::binding(device, current.revision, &input)? != binding {
        return Err(error("STALE_RESULT", "Device changed. Approve again."));
    }
    service.1.issue_menu(binding, approved)
}

#[tauri::command]
pub async fn read_static_menu(
    input: DiscoveryRequest,
    discovery: tauri::State<'_, DiscoveryService>,
    connections: tauri::State<'_, ConnectionService>,
    service: tauri::State<'_, Service>,
) -> Result<AcquisitionCapabilityReport, DiscoveryError> {
    let snapshot = discovery
        .snapshot()
        .map_err(|_| error("DISCOVERY_UNAVAILABLE", "Refresh discovery."))?;
    let device = snapshot
        .devices
        .iter()
        .find(|d| d.id == input.device_id)
        .cloned()
        .ok_or_else(|| error("DEVICE_NOT_FOUND", "Select a current device."))?;
    let base = cached_menu_report(&service, &device, snapshot.revision, &input)?;
    let lease = connections
        .begin(&format!("acquisition:{}", device.id))
        .map_err(|_| error("BUSY", "Another check is finishing."))?;
    let binding = approval::binding(&device, snapshot.revision, &input)?;
    let proof = service.1.claim_menu(
        input
            .approval_token
            .as_deref()
            .ok_or_else(|| error("APPROVAL_REQUIRED", "Approve static menu check."))?,
        &binding,
    )?;
    let live = discovery.shared_snapshot();
    let (report, lease) = tauri::async_runtime::spawn_blocking(move || {
        let networks = snapshot
            .devices
            .iter()
            .flat_map(|d| d.networks.clone())
            .map(|n| (n.id.clone(), n))
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect::<Vec<_>>();
        let report = super::static_menu::collect(
            &base,
            &device,
            snapshot.revision,
            &input,
            &proof,
            &lease.cancelled,
            |target, interface, proof| {
                connection::menu_metadata(proof, target, interface, &networks, &lease.cancelled)
            },
        )?;
        Ok((report, lease))
    })
    .await
    .map_err(|_| error("WORKER_FAILED", "Static menu worker unavailable."))??;
    let current = live
        .lock()
        .map_err(|_| error("DISCOVERY_UNAVAILABLE", "Refresh discovery."))?;
    if current.revision != report.discovery_revision
        || !current
            .devices
            .iter()
            .any(|d| d.id == report.device_id && d.connected)
    {
        return Err(error("STALE_RESULT", "Device changed during menu check."));
    }
    lease
        .if_current(|| {
            service
                .0
                .lock()
                .map_err(|_| error("INTERNAL", "Menu state unavailable."))?
                .insert(report.device_id.clone(), report.clone());
            Ok(())
        })
        .ok_or_else(|| error("CANCELLED", "Static menu check superseded or cancelled."))??;
    Ok(report)
}

fn cached_challenge_report(
    service: &Service,
    device: &Device,
    revision: u64,
    input: &DiscoveryRequest,
) -> Result<AcquisitionCapabilityReport, DiscoveryError> {
    let report=cached_menu_report(service,device,revision,input).map_err(|_|error("no_recognized_family","A fresh matched root observation with a recognized family is required; no challenge request sent."))?;
    if !super::challenge::root_matches(&report, revision, input) {
        return Err(error("no_recognized_family","A fresh matched root observation with a recognized family is required; no challenge request sent."));
    }
    Ok(report)
}
#[tauri::command]
pub async fn approve_auth_challenge(
    input: DiscoveryRequest,
    app: tauri::AppHandle,
    discovery: tauri::State<'_, DiscoveryService>,
    service: tauri::State<'_, Service>,
) -> Result<String, DiscoveryError> {
    let snapshot = discovery
        .snapshot()
        .map_err(|_| error("DISCOVERY_UNAVAILABLE", "Refresh discovery."))?;
    let device = snapshot
        .devices
        .iter()
        .find(|d| d.id == input.device_id)
        .ok_or_else(|| error("DEVICE_NOT_FOUND", "Select a current device."))?;
    cached_challenge_report(&service, device, snapshot.revision, &input)?;
    let binding = approval::binding(device, snapshot.revision, &input)?;
    let message = approval::challenge_confirmation(device, &binding);
    use tauri::Manager;
    let owner = app
        .get_webview_window("main")
        .and_then(|w| w.hwnd().ok())
        .map(|h| h.0 as isize)
        .unwrap_or(0);
    let approved = tauri::async_runtime::spawn_blocking(move || {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::*;
            let text: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
            let title: Vec<u16> = "OBTAINER - approve authentication challenge"
                .encode_utf16()
                .chain(Some(0))
                .collect();
            unsafe {
                MessageBoxW(
                    owner as _,
                    text.as_ptr(),
                    title.as_ptr(),
                    MB_YESNO | MB_DEFBUTTON2 | MB_ICONQUESTION,
                ) == IDYES
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (owner, message);
            false
        }
    })
    .await
    .map_err(|_| error("WORKER_FAILED", "Challenge approval unavailable."))?;
    if !approved {
        return Err(error(
            "CANCELLED",
            "Authentication challenge check not approved.",
        ));
    }
    let current = discovery
        .snapshot()
        .map_err(|_| error("DISCOVERY_UNAVAILABLE", "Refresh discovery."))?;
    let device = current
        .devices
        .iter()
        .find(|d| d.id == input.device_id)
        .ok_or_else(|| error("DEVICE_NOT_FOUND", "Select a current device."))?;
    cached_challenge_report(&service, device, current.revision, &input)?;
    if approval::binding(device, current.revision, &input)? != binding {
        return Err(error("STALE_RESULT", "Device changed. Approve again."));
    }
    service.1.issue_challenge(binding, approved)
}

#[tauri::command]
pub async fn read_auth_challenge(
    input: DiscoveryRequest,
    discovery: tauri::State<'_, DiscoveryService>,
    connections: tauri::State<'_, ConnectionService>,
    service: tauri::State<'_, Service>,
) -> Result<AcquisitionCapabilityReport, DiscoveryError> {
    let snapshot = discovery
        .snapshot()
        .map_err(|_| error("DISCOVERY_UNAVAILABLE", "Refresh discovery."))?;
    let device = snapshot
        .devices
        .iter()
        .find(|d| d.id == input.device_id)
        .cloned()
        .ok_or_else(|| error("DEVICE_NOT_FOUND", "Select a current device."))?;
    let base = cached_challenge_report(&service, &device, snapshot.revision, &input)?;
    let lease = connections
        .begin(&format!("acquisition:{}", device.id))
        .map_err(|_| error("BUSY", "Another check is finishing."))?;
    let binding = approval::binding(&device, snapshot.revision, &input)?;
    let proof = service.1.claim_challenge(
        input.approval_token.as_deref().ok_or_else(|| {
            error(
                "APPROVAL_REQUIRED",
                "Approve authentication challenge check.",
            )
        })?,
        &binding,
    )?;
    let live = discovery.shared_snapshot();
    let (report, lease) = tauri::async_runtime::spawn_blocking(move || {
        let networks = snapshot
            .devices
            .iter()
            .flat_map(|d| d.networks.clone())
            .map(|n| (n.id.clone(), n))
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect::<Vec<_>>();
        let report = super::challenge::collect(
            &base,
            &device,
            snapshot.revision,
            &input,
            &proof,
            &lease.cancelled,
            |target, interface, proof| {
                connection::challenge_metadata(
                    proof,
                    target,
                    interface,
                    &networks,
                    &lease.cancelled,
                )
            },
        )?;
        Ok((report, lease))
    })
    .await
    .map_err(|_| {
        error(
            "WORKER_FAILED",
            "Authentication challenge worker unavailable.",
        )
    })??;
    let current = live
        .lock()
        .map_err(|_| error("DISCOVERY_UNAVAILABLE", "Refresh discovery."))?;
    if current.revision != report.discovery_revision
        || !current
            .devices
            .iter()
            .any(|d| d.id == report.device_id && d.connected)
    {
        return Err(error(
            "STALE_RESULT",
            "Device changed during challenge check.",
        ));
    }
    lease
        .if_current(|| {
            service
                .0
                .lock()
                .map_err(|_| error("INTERNAL", "Challenge state unavailable."))?
                .insert(report.device_id.clone(), report.clone());
            Ok(())
        })
        .ok_or_else(|| {
            error(
                "CANCELLED",
                "Authentication challenge check superseded or cancelled.",
            )
        })??;
    Ok(report)
}
