use crate::{
    discovery::{DiscoveryService, DiscoverySnapshot},
    domain::{Settings, Snapshot},
    storage::Repository,
};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub struct AppState {
    pub repository: Arc<Mutex<Repository>>,
    pub directory: PathBuf,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: &'static str,
    pub message: &'static str,
}

fn database_error(error: impl std::fmt::Display) -> CommandError {
    tracing::error!(error = %error, "Workspace database request failed");
    CommandError {
        code: "DATABASE_UNAVAILABLE",
        message: "Workspace database is unavailable. Restart OBTAINER; check available disk space and folder permissions if this continues.",
    }
}

#[tauri::command]
pub async fn get_snapshot(state: tauri::State<'_, AppState>) -> Result<Snapshot, CommandError> {
    let repository = Arc::clone(&state.repository);
    let directory = state.directory.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let repository = repository.lock().map_err(database_error)?;
        repository.snapshot(&directory).map_err(database_error)
    })
    .await
    .map_err(database_error)?
}

#[tauri::command]
pub async fn save_settings(
    settings: Settings,
    state: tauri::State<'_, AppState>,
) -> Result<Settings, CommandError> {
    let repository = Arc::clone(&state.repository);
    tauri::async_runtime::spawn_blocking(move || {
        let mut repository = repository.lock().map_err(database_error)?;
        repository
            .save_settings(&settings)
            .map_err(database_error)?;
        Ok(settings)
    })
    .await
    .map_err(database_error)?
}

fn discovery_error(message: &'static str) -> CommandError {
    CommandError {
        code: "DISCOVERY_UNAVAILABLE",
        message,
    }
}

#[tauri::command]
pub fn get_discovery(
    state: tauri::State<'_, DiscoveryService>,
) -> Result<DiscoverySnapshot, CommandError> {
    state.snapshot().map_err(discovery_error)
}

#[tauri::command]
pub async fn refresh_discovery(
    state: tauri::State<'_, DiscoveryService>,
) -> Result<DiscoverySnapshot, CommandError> {
    let reply = state.refresh_request().map_err(discovery_error)?;
    tauri::async_runtime::spawn_blocking(move || {
        reply
            .recv_timeout(std::time::Duration::from_secs(20))
            .map_err(|_| {
                discovery_error("Inventory refresh timed out. Wait a moment and try again.")
            })
    })
    .await
    .map_err(|_| discovery_error("Discovery worker is unavailable. Restart OBTAINER."))?
}

use crate::connection::{ConnectionService, EndpointResult, ManualInput, SavedRouter};

fn endpoint_error(code: &str, message: &str) -> Box<EndpointResult> {
    EndpointResult::failure(code, message)
}

#[tauri::command]
pub async fn get_manual_routers(
    state: tauri::State<'_, AppState>,
    connections: tauri::State<'_, ConnectionService>,
    discovery: tauri::State<'_, DiscoveryService>,
) -> Result<Vec<SavedRouter>, CommandError> {
    let repository = Arc::clone(&state.repository);
    let service = connections.inner().clone();
    let snapshot = discovery.snapshot().map_err(discovery_error)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut routers = repository
            .lock()
            .map_err(database_error)?
            .routers()
            .map_err(database_error)?;
        service.annotate(&mut routers, &snapshot);
        Ok(routers)
    })
    .await
    .map_err(database_error)?
}

#[tauri::command]
pub async fn add_manual_router(
    input: ManualInput,
    state: tauri::State<'_, AppState>,
    discovery: tauri::State<'_, DiscoveryService>,
) -> Result<Vec<SavedRouter>, Box<EndpointResult>> {
    let snapshot = discovery
        .snapshot()
        .map_err(|m| endpoint_error("discovery_unavailable", m))?;
    let saved = crate::connection::prepare_saved(input, &snapshot)?;
    let repository = Arc::clone(&state.repository);
    tauri::async_runtime::spawn_blocking(move || {
        let mut repository = repository
            .lock()
            .map_err(|_| endpoint_error("database_unavailable", "Database unavailable."))?;
        repository.save_router(&saved).map_err(|_| {
            endpoint_error(
                "save_failed",
                "Could not save router. Check storage and the 100-entry limit.",
            )
        })?;
        repository
            .routers()
            .map_err(|_| endpoint_error("database_unavailable", "Could not read saved routers."))
    })
    .await
    .map_err(|_| endpoint_error("worker_failed", "Save worker unavailable."))?
}

#[tauri::command]
pub async fn remove_manual_router(
    id: String,
    state: tauri::State<'_, AppState>,
    connections: tauri::State<'_, ConnectionService>,
) -> Result<Vec<SavedRouter>, CommandError> {
    let repository = Arc::clone(&state.repository);
    let service = connections.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        service.remove(&id, &repository).map_err(database_error)?;
        repository
            .lock()
            .map_err(database_error)?
            .routers()
            .map_err(database_error)
    })
    .await
    .map_err(database_error)?
}

#[tauri::command]
pub async fn probe_management_endpoint(
    id: String,
    authorized: bool,
    state: tauri::State<'_, AppState>,
    discovery: tauri::State<'_, DiscoveryService>,
    connections: tauri::State<'_, ConnectionService>,
) -> Result<EndpointResult, Box<EndpointResult>> {
    if !authorized {
        return Err(endpoint_error(
            "authorization_required",
            "Confirm you own or administer this target before checking.",
        ));
    }
    let snapshot = discovery
        .snapshot()
        .map_err(|m| endpoint_error("discovery_unavailable", m))?;
    let live_discovery = discovery.shared_snapshot();
    let service = connections.inner().clone();
    let lease = service.begin(&id)?;
    let repository = Arc::clone(&state.repository);
    tauri::async_runtime::spawn_blocking(move || {
        let saved = repository
            .lock()
            .map_err(|_| endpoint_error("database_unavailable", "Database unavailable."))?
            .router(&id)
            .map_err(|_| endpoint_error("database_unavailable", "Database unavailable."))?
            .ok_or_else(|| endpoint_error("not_found", "Saved router no longer exists."))?;
        let result = service.check(&saved, &snapshot, &lease);
        let before: std::collections::BTreeMap<_, _> = snapshot
            .devices
            .iter()
            .filter(|d| d.connected)
            .flat_map(|d| &d.networks)
            .map(|n| (&n.id, n))
            .collect();
        let current = live_discovery
            .lock()
            .map_err(|_| endpoint_error("discovery_unavailable", "Discovery unavailable."))?;
        let after: std::collections::BTreeMap<_, _> = current
            .devices
            .iter()
            .filter(|d| d.connected)
            .flat_map(|d| &d.networks)
            .map(|n| (&n.id, n))
            .collect();
        if before != after {
            lease
                .cancelled
                .store(true, std::sync::atomic::Ordering::Release);
        }
        // Hold the snapshot guard through publish, so disconnect observations cannot race it.

        if !service
            .publish(&lease, &result, &repository)
            .map_err(|_| endpoint_error("save_failed", "Could not save connection observation."))?
        {
            return Err(endpoint_error(
                "cancelled",
                "Check cancelled or superseded. Previous observation retained.",
            ));
        }
        Ok(result)
    })
    .await
    .map_err(|_| endpoint_error("worker_failed", "Connection worker unavailable."))?
}

#[tauri::command]
pub fn cancel_endpoint_probe(id: String, state: tauri::State<'_, ConnectionService>) {
    state.cancel(&id);
}

#[tauri::command]
pub async fn cancel_discovery(
    state: tauri::State<'_, DiscoveryService>,
) -> Result<(), CommandError> {
    state.cancel();
    Ok(())
}

#[tauri::command]
pub fn get_profile_registry(
    registry: tauri::State<'_, crate::profiles::Registry>,
) -> crate::profiles::Registry {
    registry.inner().clone()
}

#[tauri::command]
pub async fn validate_device_profile(
    document: String,
) -> Result<crate::profiles::RegistryEntry, crate::profiles::ProfileError> {
    tauri::async_runtime::spawn_blocking(move || {
        let profile = crate::profiles::validate(&document)?;
        Ok(crate::profiles::RegistryEntry {
            content_hash: crate::profiles::digest(
                &serde_json::to_vec(&profile).expect("Profile serialization"),
            ),
            profile,
            origin: "local_unverified".into(),
            authoritative: false,
        })
    })
    .await
    .map_err(|_| crate::profiles::ProfileError {
        code: "PROFILE_WORKER_FAILED",
        message: "Profile validation worker unavailable.".into(),
    })?
}

#[tauri::command]
pub async fn get_device_compatibility(
    device_id: String,
    state: tauri::State<'_, AppState>,
    discovery: tauri::State<'_, DiscoveryService>,
    registry: tauri::State<'_, crate::profiles::Registry>,
) -> Result<crate::profiles::CompatibilityReport, CommandError> {
    let snapshot = discovery.snapshot().map_err(discovery_error)?;
    let device = snapshot
        .devices
        .iter()
        .find(|d| d.id == device_id)
        .cloned()
        .ok_or(CommandError {
            code: "DEVICE_NOT_FOUND",
            message: "Device is not in the current inventory. Refresh discovery.",
        })?;
    let registry = registry.inner().clone();
    let live = discovery.shared_snapshot();
    let repository = Arc::clone(&state.repository);
    tauri::async_runtime::spawn_blocking(move || {
        let saved = repository
            .lock()
            .map_err(database_error)?
            .routers()
            .map_err(database_error)?;
        let mut evidence = crate::profiles::gather_evidence(&device, &saved);
        evidence.extend(
            repository
                .lock()
                .map_err(database_error)?
                .identity_observations(&device_id)
                .map_err(database_error)?,
        );
        let mut report = crate::profiles::evaluate(
            &registry,
            &device,
            evidence,
            crate::connection::now(),
            snapshot.revision,
        );
        let current = live.lock().map_err(database_error)?;
        if current.revision != snapshot.revision {
            return Err(CommandError {
                code: "COMPATIBILITY_STALE",
                message: "Discovery changed during evaluation. Evaluate the current device again.",
            });
        }
        let mut repository = repository.lock().map_err(database_error)?;
        if let Some(previous) = repository
            .prior_compatibility(&device_id)
            .map_err(database_error)?
        {
            crate::profiles::record_change(&mut report, &previous);
        }
        repository
            .save_compatibility(&report)
            .map_err(database_error)?;
        Ok(report)
    })
    .await
    .map_err(database_error)?
}

#[tauri::command]
pub async fn record_device_observation(
    device_id: String,
    attribute: crate::profiles::Attribute,
    value: String,
    state: tauri::State<'_, AppState>,
    discovery: tauri::State<'_, DiscoveryService>,
) -> Result<(), CommandError> {
    if !discovery
        .snapshot()
        .map_err(discovery_error)?
        .devices
        .iter()
        .any(|d| d.id == device_id)
    {
        return Err(CommandError {
            code: "DEVICE_NOT_FOUND",
            message: "Select an existing Windows device candidate.",
        });
    }
    let record = crate::profiles::EvidenceRecord::user_reported(
        device_id,
        attribute,
        value,
        crate::connection::now(),
    )
    .map_err(|_| CommandError {
        code: "INVALID_OBSERVATION",
        message: "Only bounded, user-reported identity fields may be saved.",
    })?;
    let repository = Arc::clone(&state.repository);
    tauri::async_runtime::spawn_blocking(move || {
        if !repository.lock().map_err(database_error)?.save_identity_observation(&record).map_err(database_error)? { return Err(CommandError {code:"OBSERVATION_LIMIT",message:"This device already has 32 reported observations. No additional observation was saved."}); }
        Ok(())
    }).await.map_err(database_error)?
}
