use super::*;
use tauri::Manager;
#[tauri::command]
pub async fn list_official_sources() -> Result<Vec<official::OfficialSource>> {
    official::catalogue()
}
#[tauri::command]
pub async fn start_official_download(
    source_id: String,
    confirm_exact_target: bool,
    service: tauri::State<'_, LibraryService>,
) -> Result<JobRecord> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        service.acquire_official(&source_id, confirm_exact_target)
    })
    .await
    .map_err(worker_error)?
}
#[tauri::command]
pub async fn get_official_provenance(
    sha256: String,
    service: tauri::State<'_, LibraryService>,
) -> Result<Vec<official::OfficialSource>> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.official_provenance(&sha256))
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn start_firmware_analysis(
    sha256: String,
    service: tauri::State<'_, LibraryService>,
) -> Result<JobRecord> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.analyze(&sha256))
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn get_firmware_analysis(
    sha256: String,
    service: tauri::State<'_, LibraryService>,
) -> Result<Option<analysis::AnalysisReport>> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.analysis_report(&sha256))
        .await
        .map_err(worker_error)?
}
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
pub async fn list_firmware_import_history(
    service: tauri::State<'_, LibraryService>,
) -> Result<Vec<FirmwareImport>> {
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.import_history())
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
) -> Result<ManifestExportResult> {
    let id = owner(&app);
    let service = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.export(&sha256, id))
        .await
        .map_err(worker_error)?
}

#[tauri::command]
pub async fn get_desktop_state(
    service: tauri::State<'_, LibraryService>,
) -> Result<desktop::DesktopState> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || s.desktop_state())
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn save_desktop_preferences(
    input: desktop::PreferenceInput,
    state: tauri::State<'_, crate::commands::AppState>,
) -> Result<desktop::DesktopPreferences> {
    let repo = Arc::clone(&state.repository);
    tauri::async_runtime::spawn_blocking(move || lock(&repo)?.save_desktop_preferences(input))
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn list_firmware_backups(
    service: tauri::State<'_, LibraryService>,
) -> Result<Vec<desktop::Backup>> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || s.backups())
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn create_firmware_backup(
    sha256: String,
    service: tauri::State<'_, LibraryService>,
) -> Result<JobRecord> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || s.create_backup(&sha256))
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn restore_firmware_backup(
    id: String,
    confirmed: bool,
    service: tauri::State<'_, LibraryService>,
) -> Result<JobRecord> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || s.restore_backup(&id, confirmed))
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn remove_firmware_item(
    id: String,
    backup: bool,
    confirmed: bool,
    service: tauri::State<'_, LibraryService>,
) -> Result<()> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || s.remove_item(&id, backup, confirmed))
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn open_managed_folder(
    target: String,
    id: Option<String>,
    service: tauri::State<'_, LibraryService>,
) -> Result<()> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || s.open_folder(&target, id.as_deref()))
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn select_library_directory(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::commands::AppState>,
    service: tauri::State<'_, LibraryService>,
) -> Result<Option<desktop::StoragePlan>> {
    let hwnd = owner(&app);
    let scope = state
        .test_profile
        .as_ref()
        .and_then(|_| state.directory.parent().map(Path::to_path_buf));
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || s.choose_directory(hwnd, scope.as_deref()))
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn recover_default_library(
    confirmed: bool,
    service: tauri::State<'_, LibraryService>,
) -> Result<()> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || s.recover_default_library(confirmed))
        .await
        .map_err(worker_error)?
}
#[tauri::command]
pub async fn relocate_firmware_library(
    ticket: String,
    confirmed: bool,
    service: tauri::State<'_, LibraryService>,
) -> Result<JobRecord> {
    let s = service.inner().clone();
    tauri::async_runtime::spawn_blocking(move || s.relocate(&ticket, confirmed))
        .await
        .map_err(worker_error)?
}
