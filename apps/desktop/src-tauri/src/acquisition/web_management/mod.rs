//! Local, non-executable management evidence. Captures never authorize requests.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
mod har;
pub mod ipc;
mod sanitize;
mod static_analysis;
mod store;
pub mod strategy;
use store::finish;
pub use store::WebEvidenceStore;

pub const MAX_HAR_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_BUNDLE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    IdentityRead,
    StatusRead,
    Authentication,
    ConfigRead,
    ConfigWrite,
    ConfigBackup,
    ConfigRestore,
    FirmwareMetadata,
    FirmwareUpdateCheck,
    FirmwareUpload,
    FirmwareDownload,
    FileExport,
    Reboot,
    FactoryReset,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    HarObserved,
    StaticCandidate,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointEvidence {
    pub id: String,
    pub origin: String,
    pub path: String,
    pub method: String,
    pub query_names: Vec<String>,
    pub classification: Classification,
    pub evidence: EvidenceSource,
    pub read_only_qualified: bool,
    pub status: Option<u16>,
    pub mime_type: Option<String>,
    pub request_content_type: Option<String>,
    pub request_body_names: Vec<String>,
    pub selected_headers: BTreeMap<String, String>,
    pub context: Option<String>,
    pub response_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceEvidence {
    pub origin: String,
    pub path: String,
    pub kind: String,
    pub referenced: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceActions {
    pub import_har: bool,
    pub analyze_local_bundle: bool,
    pub collect_resources: bool,
    pub authenticate: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebEvidenceReport {
    pub schema_version: u32,
    pub device_id: String,
    pub entries: Vec<EndpointEvidence>,
    pub auth_candidates: Vec<String>,
    pub resources: Vec<ResourceEvidence>,
    pub counts: BTreeMap<String, usize>,
    pub warnings: Vec<String>,
    pub can_acquire: bool,
    pub strategy: strategy::WebStrategyAssessment,
    pub retention: String,
    pub actions: EvidenceActions,
}

#[derive(Debug, Clone, Serialize, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("{message}")]
pub struct WebEvidenceError {
    pub code: &'static str,
    pub message: &'static str,
}

fn empty_report(device_id: &str) -> Result<WebEvidenceReport, WebEvidenceError> {
    if device_id.is_empty()
        || device_id.len() > 128
        || !device_id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
    {
        return Err(sanitize::fail(
            "INVALID_DEVICE",
            "Select a valid discovered device identifier.",
        ));
    }
    Ok(WebEvidenceReport {
        schema_version: 1, device_id: device_id.into(), entries: vec![], auth_candidates: vec![], resources: vec![], counts: BTreeMap::new(),
        warnings: vec!["Query values, raw headers, bodies, identifiers and local paths are discarded. Unknown path/field/DNS names use opaque redacted labels; evidence descriptions cannot be replayed.".into()],
        can_acquire: false, strategy: strategy::WebManagementAcquisition::empty(), retention: "memory_only".into(),
        actions: EvidenceActions { import_har: true, analyze_local_bundle: true, collect_resources: false, authenticate: false,
            reason: "No independently qualified read-only identity/status route is registered for this device family.".into() },
    })
}

fn evidence_id(origin: &str, path: &str, method: &str, classification: &Classification) -> String {
    sanitize::digest(&format!("{origin}\n{path}\n{method}\n{classification:?}"))
}

pub fn parse_har(device_id: &str, bytes: &[u8]) -> Result<WebEvidenceReport, WebEvidenceError> {
    har::parse(device_id, bytes)
}

pub fn analyze_bundle(
    device_id: &str,
    origin: &str,
    resource_name: &str,
    bytes: &[u8],
) -> Result<WebEvidenceReport, WebEvidenceError> {
    static_analysis::parse(device_id, origin, resource_name, bytes)
}

#[cfg(test)]
mod tests;
