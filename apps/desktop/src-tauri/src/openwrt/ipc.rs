//! No IPC accepts commands, credentials, key paths, trust flags or caller-owned sessions.
use super::runtime::*;
use crate::{adapters::AdapterError, commands::AppState, discovery::DiscoveryService, profiles};
use std::sync::Arc;
use tauri::Manager;

fn selected(app: &tauri::AppHandle, service: &Service, id: &str) -> Result<Selected, AdapterError> {
    let state = app.state::<AppState>();
    let profiles = app.state::<profiles::Registry>().inner().clone();
    #[cfg(feature = "native-test-profile")]
    if service.lab_enabled() && id == LAB_TARGET {
        return super::runtime::lab_selected(profiles);
    }
    let _ = service;
    let snapshot = app
        .state::<DiscoveryService>()
        .snapshot()
        .map_err(AdapterError::internal)?;
    let repository = state
        .repository
        .lock()
        .map_err(|_| AdapterError::internal("workspace"))?;
    let saved = repository
        .routers()
        .map_err(|_| AdapterError::internal("saved targets"))?
        .into_iter()
        .find(|r| r.id == id)
        .ok_or_else(|| super::adapter::denied("Saved target no longer exists."))?;
    let observations = repository
        .identity_observations(&saved.id)
        .map_err(|_| AdapterError::internal("target evidence"))?;
    super::runtime::select(&saved, &snapshot, profiles, observations)
}
fn with_choices(
    app: &tauri::AppHandle,
    service: &Service,
    mut status: Status,
) -> Result<Status, AdapterError> {
    let state = app.state::<AppState>();
    let saved = state
        .repository
        .lock()
        .map_err(|_| AdapterError::internal("workspace"))?
        .routers()
        .map_err(|_| AdapterError::internal("saved targets"))?;
    status.targets = service.choices(&saved);
    Ok(status)
}
fn completed(
    app: &tauri::AppHandle,
    service: &Service,
    result: Result<Status, AdapterError>,
) -> Result<Status, AdapterError> {
    match result {
        Ok(status) => {
            if status.target_id.is_some() {
                let checked = selected(
                    app,
                    service,
                    status.target_id.as_deref().unwrap_or_default(),
                )
                .and_then(|current| service.validate_current(&current));
                if let Err(error) = checked {
                    service.fail_generation(status.generation, error.clone());
                    return Err(error);
                }
            }
            with_choices(app, service, status)
        }
        Err(error) => Err(error),
    }
}
#[tauri::command]
pub fn openwrt_status(
    app: tauri::AppHandle,
    service: tauri::State<'_, Arc<Service>>,
) -> Result<Status, AdapterError> {
    with_choices(&app, &service, service.status()?)
}
#[tauri::command]
pub async fn openwrt_observe(
    app: tauri::AppHandle,
    service: tauri::State<'_, Arc<Service>>,
    request: Observe,
) -> Result<Status, AdapterError> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let target = selected(&app, &service, &request.target_id)?;
        let result = service.observe(target, request);
        completed(&app, &service, result)
    })
    .await
    .map_err(|_| AdapterError::internal("OpenWrt worker"))?
}
#[tauri::command]
pub async fn openwrt_connect(
    app: tauri::AppHandle,
    service: tauri::State<'_, Arc<Service>>,
    request: Connect,
) -> Result<Status, AdapterError> {
    let service = service.inner().clone();
    #[cfg(windows)]
    let owner = app
        .get_webview_window("main")
        .and_then(|w| w.hwnd().ok())
        .map_or(0, |h| h.0 as isize);
    #[cfg(not(windows))]
    let owner = 0;
    tauri::async_runtime::spawn_blocking(move || {
        if !request.approved {
            return Err(super::adapter::denied(
                "Approve the independent host fingerprint before choosing a key.",
            ));
        }
        let id = service.target_id()?;
        let target = selected(&app, &service, &id)?;
        let result =
            service.connect_select_key(target, request, owner, || selected(&app, &service, &id));
        completed(&app, &service, result)
    })
    .await
    .map_err(|_| AdapterError::internal("OpenWrt worker"))?
}
#[tauri::command]
pub async fn openwrt_approve_reads(
    app: tauri::AppHandle,
    service: tauri::State<'_, Arc<Service>>,
    request: Approval,
) -> Result<Status, AdapterError> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let target = selected(&app, &service, &service.target_id()?)?;
        let result = service.approve_reads(target, request);
        completed(&app, &service, result)
    })
    .await
    .map_err(|_| AdapterError::internal("OpenWrt worker"))?
}
#[tauri::command]
pub async fn openwrt_inspect(
    app: tauri::AppHandle,
    service: tauri::State<'_, Arc<Service>>,
) -> Result<Status, AdapterError> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let target = selected(&app, &service, &service.target_id()?)?;
        let result = service.inspect(target);
        completed(&app, &service, result)
    })
    .await
    .map_err(|_| AdapterError::internal("OpenWrt worker"))?
}
#[tauri::command]
pub async fn openwrt_cancel(
    app: tauri::AppHandle,
    service: tauri::State<'_, Arc<Service>>,
) -> Result<Status, AdapterError> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = service.cancel();
        completed(&app, &service, result)
    })
    .await
    .map_err(|_| AdapterError::internal("OpenWrt worker"))?
}
#[tauri::command]
pub async fn openwrt_disconnect(
    app: tauri::AppHandle,
    service: tauri::State<'_, Arc<Service>>,
) -> Result<Status, AdapterError> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = service.cancel();
        completed(&app, &service, result)
    })
    .await
    .map_err(|_| AdapterError::internal("OpenWrt worker"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ipc_rejects_command_key_path_and_session_injection() {
        assert!(serde_json::from_value::<Connect>(serde_json::json!({"fingerprint":"x","username":"root","approved":true,"keyPath":"secret"})).is_err());
        assert!(serde_json::from_value::<Observe>(
            serde_json::json!({"targetId":LAB_TARGET,"port":22,"approved":true,"command":"reboot"})
        )
        .is_err());
        assert!(serde_json::from_value::<Approval>(
            serde_json::json!({"approved":true,"session":"other"})
        )
        .is_err());
    }
}
