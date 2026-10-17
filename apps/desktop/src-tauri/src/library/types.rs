use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("{message}")]
pub struct LibraryError {
    pub code: String,
    pub message: String,
}
impl LibraryError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
impl From<rusqlite::Error> for LibraryError {
    fn from(_: rusqlite::Error) -> Self {
        Self::new("DATABASE_UNAVAILABLE","The operation could not be saved. Check disk space and retry; published bytes are retained for recovery.")
    }
}
impl From<std::io::Error> for LibraryError {
    fn from(e: std::io::Error) -> Self {
        let (code, message) = match e.kind() {
            _ if matches!(e.raw_os_error(), Some(1 | 17 | 50)) => ("ATOMIC_PUBLICATION_UNAVAILABLE", "This filesystem cannot provide the required publication operation. Use a local volume supporting hard links; no existing file was overwritten."),
            std::io::ErrorKind::NotFound => (
                "FILE_MISSING",
                "The selected file or vault blob is missing. Select the source again.",
            ),
            std::io::ErrorKind::PermissionDenied => (
                "ACCESS_DENIED",
                "Windows denied access or the file is in use. Close other writers and retry.",
            ),
            std::io::ErrorKind::AlreadyExists => (
                "DESTINATION_EXISTS",
                "Destination exists. Choose a new name; existing files are never overwritten.",
            ),
            _ if matches!(e.raw_os_error(), Some(112 | 39 | 28)) => {
                ("DISK_FULL", "Storage is full. Free space before retrying.")
            }
            _ => (
                "IO_FAILED",
                "File operation failed. Check storage availability and permissions.",
            ),
        };
        Self::new(code, message)
    }
}
pub type Result<T> = std::result::Result<T, LibraryError>;
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestExportResult {
    pub saved: bool,
    pub activity_recorded: bool,
    pub warning: Option<String>,
}
#[derive(Default)]
pub(crate) struct JobInput<'a> {
    pub filename: Option<&'a str>,
    pub description: Option<&'a str>,
    pub device: Option<&'a str>,
    pub hash: Option<&'a str>,
}
pub fn valid_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}
impl JobStatus {
    pub fn text(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
    pub fn terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
    pub fn allows(self, next: Self) -> bool {
        matches!(
            (self, next),
            (
                Self::Pending,
                Self::Running | Self::Cancelled | Self::Failed
            ) | (
                Self::Running,
                Self::Completed | Self::Failed | Self::Cancelled
            )
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobRecord {
    pub id: String,
    pub kind: String,
    pub status: JobStatus,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub updated_at: u64,
    pub progress_bytes: u64,
    pub total_bytes: u64,
    pub stage: String,
    pub error_code: Option<String>,
    pub message: String,
    pub related_artifact: Option<String>,
    pub import_id: Option<String>,
    pub cancel_requested: bool,
    pub duplicate: bool,
    pub recovery_info: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareArtifact {
    pub sha256: String,
    pub size_bytes: u64,
    pub storage_status: String,
    pub integrity_status: String,
    pub first_imported_at: u64,
    pub last_verified_at: Option<u64>,
    pub format: String,
    pub source_count: u64,
    pub original_filename: String,
    pub source_category: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareImport {
    pub import_id: String,
    pub artifact_sha256: String,
    pub original_filename: String,
    pub imported_at: u64,
    pub source_category: String,
    pub provenance_status: String,
    pub description: Option<String>,
    pub associated_device_id: Option<String>,
    pub job_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityResult {
    pub artifact_sha256: String,
    pub expected_size: u64,
    pub actual_size: Option<u64>,
    pub actual_sha256: Option<String>,
    pub status: String,
    pub verified_at: u64,
    pub error_code: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactDetails {
    pub artifact: FirmwareArtifact,
    pub imports: Vec<FirmwareImport>,
    pub checks: Vec<IntegrityResult>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPage {
    pub artifacts: Vec<FirmwareArtifact>,
    pub total: u64,
    pub unique_bytes: u64,
    pub page: u32,
    pub page_size: u32,
    pub max_file_bytes: u64,
}
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryQuery {
    pub search: Option<String>,
    pub integrity: Option<String>,
    pub source: Option<String>,
    pub from_date: Option<u64>,
    pub to_date: Option<u64>,
    pub page: Option<u32>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSelection {
    pub ticket: String,
    pub filename: String,
    pub size_bytes: u64,
    pub expires_at: u64,
    pub source_category: String,
    pub max_file_bytes: u64,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportRequest {
    pub ticket: String,
    pub description: Option<String>,
    pub associated_device_id: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEvent {
    pub id: i64,
    pub kind: String,
    pub message: String,
    pub created_at: String,
    pub job_id: Option<String>,
    pub artifact_sha256: Option<String>,
    pub severity: String,
}
