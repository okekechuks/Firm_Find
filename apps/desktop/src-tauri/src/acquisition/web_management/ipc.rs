//! Native local-file selection only. No caller-supplied path, HAR body or script
//! text crosses IPC, and no capture is ever stored in the firmware Library.
use super::*;
use crate::discovery::DiscoveryService;
use std::{
    io::Read,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::Manager;

pub(crate) fn read_local_file(path: &Path, max: usize) -> Result<Vec<u8>, WebEvidenceError> {
    crate::library::files::safe_path(path).map_err(|_| {
        sanitize::fail(
            "UNSAFE_SOURCE",
            "Select an absolute local file without traversal, network paths or alternate streams.",
        )
    })?;
    let parent = path.parent().ok_or_else(|| {
        sanitize::fail("UNSAFE_SOURCE", "Selected file has no normal local parent.")
    })?;
    let _directories = crate::library::files::guard_directories(parent).map_err(|_| {
        sanitize::fail(
            "UNSAFE_SOURCE",
            "Linked, reparse-point or unavailable parent folders are not supported.",
        )
    })?;
    let file = crate::library::files::read_file(path).map_err(|_| {
        sanitize::fail(
            "UNSAFE_SOURCE",
            "Select a regular local file, not a symlink, device or reparse point.",
        )
    })?;
    let size = file
        .metadata()
        .map_err(|_| sanitize::fail("READ_FAILED", "Cannot inspect the selected file."))?
        .len();
    if size > max as u64 {
        return Err(sanitize::fail(
            "INPUT_TOO_LARGE",
            "Selected capture exceeds the bounded import limit.",
        ));
    }
    let mut bytes = Vec::with_capacity((size as usize).min(max));
    file.take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| sanitize::fail("READ_FAILED", "Cannot read the selected file."))?;
    if bytes.len() > max {
        bytes.fill(0);
        return Err(sanitize::fail(
            "INPUT_TOO_LARGE",
            "Selected file grew beyond the import limit.",
        ));
    }
    Ok(bytes)
}

fn current_device(discovery: &DiscoveryService, id: &str) -> Result<(), WebEvidenceError> {
    let snapshot = discovery.snapshot().map_err(|_| {
        sanitize::fail(
            "DISCOVERY_UNAVAILABLE",
            "Refresh discovery before importing evidence.",
        )
    })?;
    if !snapshot.devices.iter().any(|d| d.id == id && d.connected) {
        return Err(sanitize::fail(
            "DEVICE_NOT_FOUND",
            "Select a currently connected device.",
        ));
    }
    Ok(())
}

fn owner(app: &tauri::AppHandle) -> isize {
    #[cfg(windows)]
    {
        app.get_webview_window("main")
            .and_then(|w| w.hwnd().ok())
            .map(|h| h.0 as isize)
            .unwrap_or(0)
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        0
    }
}

struct DialogGuard<'a>(&'a AtomicBool);
impl Drop for DialogGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[tauri::command]
pub fn get_web_ui_evidence(
    device_id: String,
    discovery: tauri::State<'_, DiscoveryService>,
    evidence: tauri::State<'_, WebEvidenceStore>,
) -> Result<WebEvidenceReport, WebEvidenceError> {
    current_device(&discovery, &device_id)?;
    evidence.get_or_empty(&device_id)
}

#[tauri::command]
pub async fn import_web_ui_capture(
    device_id: String,
    app: tauri::AppHandle,
    discovery: tauri::State<'_, DiscoveryService>,
    evidence: tauri::State<'_, WebEvidenceStore>,
) -> Result<Option<WebEvidenceReport>, WebEvidenceError> {
    current_device(&discovery, &device_id)?;
    if evidence
        .dialog_open
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(sanitize::fail(
            "BUSY",
            "Another evidence file dialog is open.",
        ));
    }
    let _guard = DialogGuard(&evidence.dialog_open);
    let owner = owner(&app);
    let input = tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = choose_file(owner, true)? else {
            return Ok(None);
        };
        read_local_file(&path, MAX_HAR_BYTES).map(Some)
    })
    .await
    .map_err(|_| sanitize::fail("WORKER_FAILED", "Capture import worker failed."))??;
    let Some(mut bytes) = input else {
        return Ok(None);
    };
    // Parse in bounded worker; the chosen path and input content are never retained.
    let id = device_id.clone();
    let report = tauri::async_runtime::spawn_blocking(move || {
        let report = parse_har(&id, &bytes);
        bytes.fill(0);
        report
    })
    .await
    .map_err(|_| sanitize::fail("WORKER_FAILED", "Capture parser worker failed."))??;
    current_device(&discovery, &device_id)?;
    evidence.merge(report).map(Some)
}

#[tauri::command]
pub async fn analyze_web_ui_bundle(
    device_id: String,
    origin: String,
    app: tauri::AppHandle,
    discovery: tauri::State<'_, DiscoveryService>,
    evidence: tauri::State<'_, WebEvidenceStore>,
) -> Result<Option<WebEvidenceReport>, WebEvidenceError> {
    current_device(&discovery, &device_id)?;
    sanitize::origin(&origin)?;
    if evidence
        .dialog_open
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(sanitize::fail(
            "BUSY",
            "Another evidence file dialog is open.",
        ));
    }
    let _guard = DialogGuard(&evidence.dialog_open);
    let owner = owner(&app);
    let input = tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = choose_file(owner, false)? else {
            return Ok(None);
        };
        let resource = path
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or_else(|| sanitize::fail("INVALID_SOURCE", "Select a UTF-8 resource filename."))?
            .to_string();
        read_local_file(&path, MAX_BUNDLE_BYTES).map(|bytes| Some((resource, bytes)))
    })
    .await
    .map_err(|_| sanitize::fail("WORKER_FAILED", "Resource selection worker failed."))??;
    let Some((resource, mut bytes)) = input else {
        return Ok(None);
    };
    let id = device_id.clone();
    let report = tauri::async_runtime::spawn_blocking(move || {
        let report = analyze_bundle(&id, &origin, &resource, &bytes);
        bytes.fill(0);
        report
    })
    .await
    .map_err(|_| sanitize::fail("WORKER_FAILED", "Static analysis worker failed."))??;
    current_device(&discovery, &device_id)?;
    evidence.merge(report).map(Some)
}

/// Calling IPC directly cannot bypass the missing qualification/approval boundary.
#[tauri::command]
pub fn collect_management_ui_resources(
    device_id: String,
    discovery: tauri::State<'_, DiscoveryService>,
) -> Result<WebEvidenceReport, WebEvidenceError> {
    current_device(&discovery, &device_id)?;
    Err(sanitize::fail("IDENTITY_UNQUALIFIED", "No independently qualified read-only identity/status route is registered for this device family. Imported evidence cannot authorize live collection."))
}

#[cfg(windows)]
fn choose_file(owner: isize, har: bool) -> Result<Option<std::path::PathBuf>, WebEvidenceError> {
    use windows_sys::Win32::UI::Controls::Dialogs::*;
    let title: Vec<u16> = (if har {
        "Import Web UI Capture (local evidence only)"
    } else {
        "Analyze local management UI resource (no requests)"
    })
    .encode_utf16()
    .chain(Some(0))
    .collect();
    let filter: Vec<u16> = (if har {
        "HAR JSON capture\0*.har;*.json\0\0"
    } else {
        "Management UI resources\0*.html;*.htm;*.js;*.mjs;*.json;*.css\0\0"
    })
    .encode_utf16()
    .collect();
    let mut buffer = vec![0u16; 32768];
    let mut info: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    info.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    info.hwndOwner = owner as _;
    info.lpstrTitle = title.as_ptr();
    info.lpstrFilter = filter.as_ptr();
    info.lpstrFile = buffer.as_mut_ptr();
    info.nMaxFile = buffer.len() as u32;
    info.Flags = OFN_EXPLORER
        | OFN_FILEMUSTEXIST
        | OFN_PATHMUSTEXIST
        | OFN_NOCHANGEDIR
        | OFN_DONTADDTORECENT;
    if unsafe { GetOpenFileNameW(&mut info) } == 0 {
        return if unsafe { CommDlgExtendedError() } == 0 {
            Ok(None)
        } else {
            Err(sanitize::fail(
                "DIALOG_FAILED",
                "Windows file selection failed.",
            ))
        };
    }
    let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
    let path = String::from_utf16(&buffer[..end])
        .map_err(|_| sanitize::fail("INVALID_SOURCE", "Selected path is not valid Unicode."))?;
    Ok(Some(path.into()))
}

#[cfg(not(windows))]
fn choose_file(_: isize, _: bool) -> Result<Option<std::path::PathBuf>, WebEvidenceError> {
    Err(sanitize::fail(
        "WINDOWS_REQUIRED",
        "Native evidence selection requires Windows.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_native_reader_uses_regular_local_files_and_absolute_bounds() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("synthetic-capture.har");
        std::fs::write(&file, b"abc").unwrap();
        assert_eq!(read_local_file(&file, 3).unwrap(), b"abc");
        assert_eq!(
            read_local_file(&file, 2).unwrap_err().code,
            "INPUT_TOO_LARGE"
        );
        assert_eq!(
            read_local_file(folder.path(), 10).unwrap_err().code,
            "UNSAFE_SOURCE"
        );
        assert_eq!(
            read_local_file(Path::new("relative.har"), 10)
                .unwrap_err()
                .code,
            "UNSAFE_SOURCE"
        );
    }

    #[test]
    fn synthetic_dialog_guard_releases_only_its_scoped_ownership() {
        let flag = AtomicBool::new(true);
        {
            let _guard = DialogGuard(&flag);
            assert!(flag.load(Ordering::Acquire));
        }
        assert!(!flag.load(Ordering::Acquire));
    }
}
