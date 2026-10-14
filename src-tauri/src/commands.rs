use crate::{
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
