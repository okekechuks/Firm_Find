use super::*;
use tauri::Manager;
fn owner(app: &tauri::AppHandle) -> isize {
    #[cfg(windows)]
    {
        app.get_webview_window("main")
            .and_then(|w| w.hwnd().ok())
            .map_or(0, |h| h.0 as isize)
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        0
    }
}
fn worker_error(_: impl std::fmt::Display) -> LibraryError {
    LibraryError::new(
        "WORKER_FAILED",
        "Library worker unavailable. Restart the app and check Activity.",
    )
}
#[tauri::command]
pub async fn select_firmware_file(
    app: tauri::AppHandle,
    service: tauri::State<'_, LibraryService>,
) -> Result<Option<FileSelection>> {
    let id = owner(&app);
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.select(id))
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn discard_firmware_selection(
    ticket: String,
    service: tauri::State<'_, LibraryService>,
) -> Result<()> {
    service.discard(&ticket)
}
#[tauri::command]
pub async fn start_firmware_import(
    input: ImportRequest,
    service: tauri::State<'_, LibraryService>,
) -> Result<JobRecord> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.import(input))
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn list_firmware_artifacts(
    query: LibraryQuery,
    service: tauri::State<'_, LibraryService>,
) -> Result<LibraryPage> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.list(&query))
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn get_firmware_artifact(
    sha256: String,
    service: tauri::State<'_, LibraryService>,
) -> Result<ArtifactDetails> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.details(&sha256))
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn verify_firmware_integrity(
    sha256: String,
    service: tauri::State<'_, LibraryService>,
) -> Result<JobRecord> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.verify(&sha256))
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn cancel_library_job(
    id: String,
    service: tauri::State<'_, LibraryService>,
) -> Result<JobRecord> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.cancel(&id))
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn list_library_jobs(
    service: tauri::State<'_, LibraryService>,
) -> Result<Vec<JobRecord>> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.jobs())
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn get_library_job(
    id: String,
    service: tauri::State<'_, LibraryService>,
) -> Result<JobRecord> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.job(&id))
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn get_library_activity(
    service: tauri::State<'_, LibraryService>,
) -> Result<Vec<ActivityEvent>> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.activity())
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn export_artifact_manifest(
    sha256: String,
    app: tauri::AppHandle,
    service: tauri::State<'_, LibraryService>,
) -> Result<bool> {
    let id = owner(&app);
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.export(&sha256, id))
        .await
        .map_err(worker_error)?
}
