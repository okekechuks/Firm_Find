//! Evidence-driven source assessment, separate from executable adapter authority.
//! No source becomes qualified by appearing in a HAR or downloaded script.
use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WebSourceKind {
    DocumentedDeviceExport,
    ObservedFirmwareDownload,
    ObservedFotaPackage,
    VerifiedVendorDownload,
    ConfigExport,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WebSourceAssessment {
    pub evidence_id: String,
    pub source_type: WebSourceKind,
    pub state: String,
    pub read_only_qualified: bool,
    pub firmware_compatible: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WebStrategyAssessment {
    pub strategy: String,
    pub can_acquire: bool,
    pub sources: Vec<WebSourceAssessment>,
    pub missing_requirements: Vec<String>,
}

/// Generic family-adapter strategy; it deliberately exposes no request/replay API.
/// The separately reviewed authenticated reader can establish identity, while an
/// independently reviewed source descriptor must still qualify actual firmware.
pub struct WebManagementAcquisition;

impl WebManagementAcquisition {
    pub fn assess(report: &WebEvidenceReport) -> WebStrategyAssessment {
        Self::assess_entries(&report.entries)
    }
    pub fn empty() -> WebStrategyAssessment {
        Self::assess_entries(&[])
    }
    fn assess_entries(entries: &[EndpointEvidence]) -> WebStrategyAssessment {
        let sources = entries.iter().filter_map(|entry| {
            let (source_type, state, reason) = match entry.classification {
                Classification::ConfigBackup => (
                    WebSourceKind::ConfigExport, "not_firmware",
                    "A configuration backup/export is not firmware."),
                Classification::FileExport => (
                    WebSourceKind::Unknown, "unqualified_file_export",
                    "A generic file export does not establish configuration or firmware bytes."),
                Classification::FirmwareUpload => (
                    WebSourceKind::Unknown, "rejected",
                    "Firmware upload, upgrade or flash is not firmware acquisition and cannot be invoked."),
                Classification::FirmwareDownload if entry.evidence != EvidenceSource::StaticCandidate => (
                    WebSourceKind::ObservedFirmwareDownload, "requires_qualification",
                    "An observed browser request is evidence only; its route, output type and exact hardware selectors require independent qualification."),
                Classification::FirmwareUpdateCheck if entry.evidence != EvidenceSource::StaticCandidate => (
                    WebSourceKind::Unknown, "metadata_only",
                    "An update check does not prove a firmware package exists or matches this hardware. It is not replayed."),
                Classification::FirmwareMetadata if entry.evidence != EvidenceSource::StaticCandidate => (
                    WebSourceKind::Unknown, "metadata_only",
                    "Observed metadata does not establish a downloadable firmware package, its authenticity or hardware compatibility."),
                Classification::FirmwareMetadata | Classification::FirmwareDownload | Classification::FirmwareUpdateCheck => (
                    WebSourceKind::Unknown, "static_candidate",
                    "A firmware-related string does not establish a read-only endpoint or a compatible firmware source."),
                _ => return None,
            };
            Some(WebSourceAssessment { evidence_id: entry.id.clone(), source_type,
                state: state.into(), read_only_qualified: false, firmware_compatible: false, reason: reason.into() })
        }).collect();
        WebStrategyAssessment {
            strategy: "web_management_acquisition".into(),
            can_acquire: false,
            sources,
            missing_requirements: vec![
                "Current independently qualified device identity".into(),
                "Live selected-device source/route binding before and after the operation".into(),
                "Explicit operation-scoped owner approval and an authorized authenticated session"
                    .into(),
                "Independently qualified read-only firmware source descriptor".into(),
                "Exact model and hardware-revision compatibility, checksum/signature validation"
                    .into(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn synthetic_capture_does_not_grant_source_authority() {
        let capture = serde_json::to_vec(&json!({"log":{"entries":[
            {"request":{"method":"GET","url":"http://192.0.2.1/config/export"},"response":{"status":200}},
            {"request":{"method":"POST","url":"http://192.0.2.1/firmware/upload"},"response":{"status":200}},
            {"request":{"method":"GET","url":"http://192.0.2.1/firmware/download"},"response":{"status":200}}
        ]}})).unwrap();
        let report = parse_har("synthetic", &capture).unwrap();
        let assessment = WebManagementAcquisition::assess(&report);
        assert!(!assessment.can_acquire);
        assert_eq!(assessment.sources.len(), 3);
        assert!(assessment
            .sources
            .iter()
            .all(|s| !s.read_only_qualified && !s.firmware_compatible));
        assert!(assessment
            .sources
            .iter()
            .any(|s| s.source_type == WebSourceKind::ConfigExport && s.state == "not_firmware"));
        assert!(assessment.sources.iter().any(|s| s.state == "rejected"));
        assert!(assessment
            .sources
            .iter()
            .any(|s| s.source_type == WebSourceKind::ObservedFirmwareDownload));
    }

    #[test]
    fn synthetic_static_source_is_not_observed_or_executable() {
        let report = analyze_bundle(
            "synthetic",
            "http://192.0.2.1",
            "app.js",
            br#"fetch('/firmware/download')"#,
        )
        .unwrap();
        let assessment = WebManagementAcquisition::assess(&report);
        assert!(!assessment.can_acquire);
        assert_eq!(assessment.sources[0].source_type, WebSourceKind::Unknown);
        assert_eq!(assessment.sources[0].state, "static_candidate");
    }
}
