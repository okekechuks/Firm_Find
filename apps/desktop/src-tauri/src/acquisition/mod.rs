//! Read-only route discovery. Candidate recognition never grants acquisition authority.
use crate::{
    connection,
    discovery::Device,
    profiles::{self, Attribute, EvidenceOrigin, EvidenceRecord, Reliability, Verification},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
};
mod approval;
pub mod ipc;
mod strategies;
use approval::Approvals;
pub use strategies::AcquisitionStrategyRegistry;
#[derive(Default)]
pub struct Service(
    pub std::sync::Mutex<BTreeMap<String, AcquisitionCapabilityReport>>,
    Approvals,
);

pub type IdentityEvidence = EvidenceRecord;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    NotChecked,
    Observed,
    NotImplemented,
    Unknown,
    Candidate,
    RequiresAuthentication,
    RequiresApproval,
    RequiresExternalTool,
    AvailableVerified,
    Unsupported,
    Failed,
    CompletedVerified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capability {
    pub state: CapabilityState,
    pub reason: String,
    pub evidence: Vec<String>,
    pub missing_requirements: Vec<String>,
    pub executable: bool,
}
fn capability(state: CapabilityState, reason: &str, missing: &[&str]) -> Capability {
    Capability {
        state,
        reason: reason.into(),
        evidence: vec![],
        missing_requirements: missing.iter().map(|s| (*s).into()).collect(),
        executable: false,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceFingerprint {
    pub device_id: String,
    pub windows_name: String,
    pub transports: Vec<String>,
    pub usb_vid: Option<String>,
    pub usb_pid: Option<String>,
    pub interface_classes: Vec<String>,
    pub endpoint: Option<String>,
    pub interface_id: Option<String>,
    pub target_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceIdentityAssessment {
    pub confidence: String,
    pub verified_manufacturer: Option<String>,
    pub verified_firmware_family: Option<String>,
    pub verified_model: Option<String>,
    pub verified_hardware_revision: Option<String>,
    pub verified_firmware: Option<String>,
    pub reported_model: Option<String>,
    pub evidence: Vec<IdentityEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorProtocolCandidate {
    pub family: String,
    pub confidence: CapabilityState,
    pub evidence: Vec<String>,
    pub limitations: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FirmwareAcquisitionStrategy {
    OfficialImage,
    DocumentedExport,
    AuthorizedStorageRead,
    ManufacturerPackage,
    ExternalLaboratory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareAcquisitionCandidate {
    pub strategy: FirmwareAcquisitionStrategy,
    pub name: String,
    pub capability: Capability,
    pub provenance_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcquisitionCapabilityReport {
    pub schema_version: u32,
    pub device_id: String,
    pub discovery_revision: u64,
    pub evaluated_at: u64,
    pub expires_at: u64,
    pub fingerprint: DeviceFingerprint,
    pub identity: DeviceIdentityAssessment,
    pub protocols: Vec<VendorProtocolCandidate>,
    pub candidates: Vec<FirmwareAcquisitionCandidate>,
    pub capabilities: BTreeMap<String, Capability>,
    pub probe: ProbeSummary,
    pub can_obtain: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeSummary {
    pub attempted: bool,
    pub code: String,
    pub http_status: Option<u16>,
    pub metadata_truncated: bool,
    #[serde(default)]
    pub advanced_endpoint: bool,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscoveryRequest {
    pub device_id: String,
    pub endpoint: Option<String>,
    pub interface_id: Option<String>,
    pub approval_token: Option<String>,
    #[serde(default)]
    pub advanced_endpoint: bool,
}

#[derive(Debug, Serialize)]
pub struct DiscoveryError {
    pub code: &'static str,
    pub message: &'static str,
}
pub fn error(code: &'static str, message: &'static str) -> DiscoveryError {
    DiscoveryError { code, message }
}

/// No response text is retained. Only allowlisted family signals leave this parser.
pub fn recognize(body: &[u8], server: Option<&str>) -> Vec<VendorProtocolCandidate> {
    let page = String::from_utf8_lossy(&body[..body.len().min(16384)]).to_ascii_lowercase();
    let title = page
        .split_once("<title>")
        .and_then(|(_, rest)| rest.split_once("</title>"))
        .map(|(title, _)| title.trim())
        .unwrap_or("");
    let server = server.unwrap_or("").to_ascii_lowercase();
    let asr = [
        "js/base/xml_helper.js",
        "js/base/ajax_calls.js",
        "images/asr_logo.png",
    ];
    let asr_signals: Vec<_> = asr.into_iter().filter(|v| page.contains(v)).collect();
    let openwrt = title.contains("openwrt") && server == "uhttpd";
    let mut candidates = vec![];
    if asr_signals.len() >= 2 {
        candidates.push(VendorProtocolCandidate {
            family: "asr_duster_xml".into(),
            confidence: CapabilityState::Candidate,
            evidence: asr_signals
                .iter()
                .map(|s| format!("Root static reference: {s}"))
                .chain(
                    title
                        .contains("e-link")
                        .then(|| "Brand hint: e_link_oem".into()),
                )
                .collect(),
            limitations:
                "Unauthenticated candidate only; no verified device identity or API/export support."
                    .into(),
        });
    }
    if openwrt {
        candidates.push(VendorProtocolCandidate {
            family: "openwrt".into(),
            confidence: CapabilityState::Candidate,
            evidence: vec!["OpenWrt title".into(), "uhttpd server header".into()],
            limitations: "Candidate only; no exact identity or export support.".into(),
        });
    }
    candidates
}

fn identity(
    evidence: Vec<IdentityEvidence>,
    now: u64,
    endpoint: Option<&str>,
    interface: Option<&str>,
) -> DeviceIdentityAssessment {
    let fresh = |e: &&EvidenceRecord| {
        !e.stale
            && e.collected_at <= now
            && now.saturating_sub(e.collected_at) < profiles::EVIDENCE_TTL_MS
    };
    let authoritative = |e: &&EvidenceRecord| {
        fresh(e)
            && e.verification == Verification::Verified
            && e.reliability == Reliability::Strong
            && matches!(
                e.source,
                EvidenceOrigin::AuthenticatedManagement | EvidenceOrigin::SupportedModem
            )
    };
    let trusted = |e: &&EvidenceRecord| {
        authoritative(e)
            && endpoint.is_none_or(|v| e.endpoint.as_deref() == Some(v))
            && interface.is_none_or(|v| e.interface_id.as_deref() == Some(v))
    };
    let attributes = [
        Attribute::Model,
        Attribute::HardwareRevision,
        Attribute::FirmwareVersion,
        Attribute::Manufacturer,
        Attribute::FirmwareFamily,
        Attribute::ManagementIdentity,
    ];
    let conflict = attributes.iter().any(|attribute| {
        evidence
            .iter()
            .filter(authoritative)
            .filter(|e| e.attribute == *attribute)
            .map(|e| e.value.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            > 1
    });
    let peers: std::collections::BTreeSet<_> = evidence
        .iter()
        .filter(trusted)
        .map(|e| (&e.endpoint, &e.interface_id))
        .collect();
    let bound = peers.len() == 1
        && peers
            .iter()
            .all(|(endpoint, interface)| endpoint.is_some() && interface.is_some());
    let value = |attribute| {
        if conflict || !bound {
            None
        } else {
            evidence
                .iter()
                .filter(trusted)
                .find(|e| e.attribute == attribute)
                .map(|e| e.value.clone())
        }
    };
    let model = value(Attribute::Model);
    DeviceIdentityAssessment {
        confidence: if conflict {
            "conflicting"
        } else if model.is_some() {
            "verified"
        } else {
            "unverified"
        }
        .into(),
        verified_hardware_revision: value(Attribute::HardwareRevision),
        verified_manufacturer: value(Attribute::Manufacturer),
        verified_firmware_family: value(Attribute::FirmwareFamily),
        verified_firmware: value(Attribute::FirmwareVersion),
        reported_model: evidence
            .iter()
            .filter(fresh)
            .find(|e| {
                e.attribute == Attribute::Model && e.verification == Verification::UserReported
            })
            .map(|e| e.value.clone()),
        verified_model: model,
        evidence,
    }
}

#[cfg(test)]
fn discover(
    device: &Device,
    evidence: Vec<IdentityEvidence>,
    revision: u64,
    input: &DiscoveryRequest,
    token: &AtomicBool,
) -> Result<AcquisitionCapabilityReport, DiscoveryError> {
    discover_on_networks(device, evidence, revision, input, token, &device.networks)
}

pub(super) fn discover_on_networks(
    device: &Device,
    mut evidence: Vec<IdentityEvidence>,
    revision: u64,
    input: &DiscoveryRequest,
    token: &AtomicBool,
    all_networks: &[crate::discovery::NetworkAdapter],
) -> Result<AcquisitionCapabilityReport, DiscoveryError> {
    if !device.connected {
        return Err(error(
            "DEVICE_DISCONNECTED",
            "Reconnect the selected device.",
        ));
    }
    if input.device_id != device.id {
        return Err(error("TARGET_MISMATCH", "Select the current device again."));
    }
    let host_gateway = device.category
        == crate::discovery::identity::Category::GenericNetworkInterface
        && !device.networks.is_empty();
    let peripheral = device.interfaces.iter().any(|i| {
        [
            "diskdrive",
            "hdc",
            "scsiadapter",
            "hidclass",
            "keyboard",
            "mouse",
            "display",
            "processor",
            "system",
            "computer",
            "battery",
        ]
        .iter()
        .any(|class| i.class_name.eq_ignore_ascii_case(class))
    });
    // Host NICs may supply a route to a separately selected gateway. Protected peripherals cannot.
    if peripheral || (crate::adapters::protected(device) && !host_gateway) {
        return Err(error(
            "PROTECTED_DEVICE",
            "Protected host hardware cannot be used as a router target.",
        ));
    }
    if evidence.len() > 256
        || evidence
            .iter()
            .any(|e| e.subject_id != device.id || e.value.len() > 1024)
    {
        return Err(error(
            "INVALID_EVIDENCE",
            "Identity evidence is invalid or exceeds bounds.",
        ));
    }
    let mut fingerprint = DeviceFingerprint {
        device_id: device.id.clone(),
        windows_name: device.display_name.clone(),
        transports: device.transports.clone(),
        usb_vid: device.vendor_id.clone(),
        usb_pid: device.product_id.clone(),
        interface_classes: device
            .interfaces
            .iter()
            .map(|i| i.class_name.clone())
            .collect(),
        endpoint: None,
        interface_id: None,
        target_kind: if host_gateway {
            "local_endpoint"
        } else {
            "usb_or_network_device"
        }
        .into(),
    };
    let mut probe = ProbeSummary {
        attempted: false,
        code: "not_approved".into(),
        http_status: None,
        metadata_truncated: false,
        advanced_endpoint: false,
        message: String::new(),
    };
    let mut protocols = vec![];
    if input.approval_token.is_some() {
        let endpoint = input
            .endpoint
            .as_deref()
            .ok_or_else(|| error("ENDPOINT_REQUIRED", "Select a local management endpoint."))?;
        let interface = input.interface_id.as_deref().ok_or_else(|| {
            error(
                "INTERFACE_REQUIRED",
                "Select the device's active network interface.",
            )
        })?;
        let target = connection::parse_address(endpoint).map_err(|_| {
            error(
                "INVALID_ENDPOINT",
                "Use a private local HTTP/HTTPS root address without paths or credentials.",
            )
        })?;
        let network = device
            .networks
            .iter()
            .find(|n| n.id == interface && n.link_up && !n.virtual_interface)
            .ok_or_else(|| {
                error(
                    "INTERFACE_UNAVAILABLE",
                    "Selected interface is no longer active.",
                )
            })?;
        fingerprint.endpoint = Some(target.url.clone());
        fingerprint.interface_id = Some(network.id.clone());
        let (result, body) = connection::root_metadata(&target, interface, all_networks, token);
        probe = ProbeSummary {
            attempted: true,
            code: result.code,
            http_status: result.status,
            metadata_truncated: body.as_ref().is_some_and(|b| b.len() == 16384),
            advanced_endpoint: input.advanced_endpoint,
            message: result.message.clone(),
        };
        if result.status.is_some_and(|s| (200..300).contains(&s)) {
            protocols = recognize(
                body.as_deref().unwrap_or_default(),
                result.server.as_deref(),
            );
        }
        if protocols.iter().any(|p| p.family == "asr_duster_xml") && !token.load(Ordering::Acquire)
        {
            let (version_result, version_body) =
                connection::version_metadata(&target, interface, all_networks, token);
            if version_result.status == Some(200) {
                if let Some(e) = version_body
                    .as_deref()
                    .and_then(|b| version_evidence(device, &target.url, interface, b))
                {
                    evidence.push(e);
                }
            }
        }
    }
    if token.load(Ordering::Acquire) {
        return Err(error("CANCELLED", "Route discovery cancelled."));
    }
    Ok(assess(
        device,
        evidence,
        revision,
        fingerprint,
        protocols,
        probe,
        connection::now(),
    ))
}

fn assess(
    device: &Device,
    evidence: Vec<IdentityEvidence>,
    revision: u64,
    fingerprint: DeviceFingerprint,
    protocols: Vec<VendorProtocolCandidate>,
    probe: ProbeSummary,
    now: u64,
) -> AcquisitionCapabilityReport {
    // A host interface's saved identity is never the identity of its remote gateway.
    let identity = identity(
        if fingerprint.target_kind == "local_endpoint" {
            evidence
                .into_iter()
                .filter(|e| {
                    e.source == EvidenceOrigin::EndpointResponse
                        && e.endpoint == fingerprint.endpoint
                        && e.interface_id == fingerprint.interface_id
                })
                .collect()
        } else {
            evidence
        },
        now,
        fingerprint.endpoint.as_deref(),
        fingerprint.interface_id.as_deref(),
    );
    let candidates = AcquisitionStrategyRegistry::default().evaluate(&identity);
    let mut capabilities = BTreeMap::new();
    capabilities.insert("deviceDetected".into(), capability(CapabilityState::Observed, "Windows currently observes the device or selected network transport. This does not verify retail identity.", &[]));
    capabilities.insert(
        "exactIdentity".into(),
        capability(
            if identity.verified_manufacturer.is_some()
                && identity.verified_firmware_family.is_some()
                && identity.verified_model.is_some()
                && identity.verified_firmware.is_some()
                && identity.verified_hardware_revision.is_some()
            {
                CapabilityState::AvailableVerified
            } else {
                CapabilityState::Unknown
            },
            "Exact identity requires fresh, consistent, endpoint-bound trusted evidence.",
            &["Verified model, hardware revision and firmware"],
        ),
    );
    capabilities.insert(
        "managementApi".into(),
        capability(
            if probe.http_status.is_some_and(|s| s == 401 || s == 403) {
                CapabilityState::RequiresAuthentication
            } else if !protocols.is_empty() {
                CapabilityState::Candidate
            } else {
                CapabilityState::Unknown
            },
            "A reachable root page or family signal does not establish a supported management API.",
            &["Qualified read-only management adapter"],
        ),
    );
    for key in [
        "officialImage",
        "deviceExport",
        "physicalRead",
        "firmwareAcquired",
        "imageFormat",
        "filesystemExtracted",
        "dnsCustomization",
        "wifiCustomization",
        "apnCustomization",
        "ussdCustomization",
        "rebuilding",
        "installation",
        "recovery",
    ] {
        capabilities.insert(
            key.into(),
            capability(
                CapabilityState::NotImplemented,
                "No implementation is available for this operation. No operation is enabled.",
                &["Reviewed implementation and operation-specific evidence"],
            ),
        );
    }
    AcquisitionCapabilityReport {
        schema_version: 1,
        device_id: device.id.clone(),
        discovery_revision: revision,
        evaluated_at: now,
        expires_at: now.saturating_add(60_000),
        fingerprint,
        identity,
        protocols,
        candidates,
        capabilities,
        probe,
        can_obtain: false,
    }
}

/// Deliberately omit all raw evidence, device IDs, endpoint/interface addresses and peer text.
pub fn diagnostics(report: &AcquisitionCapabilityReport) -> serde_json::Value {
    serde_json::json!({"schemaVersion":1,"engine":"7.5A-C-sprint-a","evaluatedAt":report.evaluated_at,"usbVid":report.fingerprint.usb_vid,"usbPid":report.fingerprint.usb_pid,"transports":report.fingerprint.transports,"identityConfidence":report.identity.confidence,"evidenceCounts":report.identity.evidence.len(),"protocolCandidates":report.protocols.iter().map(|p| &p.family).collect::<Vec<_>>(),"probe":{"attempted":report.probe.attempted,"code":report.probe.code,"httpStatus":report.probe.http_status,"metadataTruncated":report.probe.metadata_truncated,"advancedEndpoint":report.probe.advanced_endpoint},"strategies":report.candidates,"canObtain":false})
}

#[cfg(test)]
mod tests;

fn version_evidence(
    device: &Device,
    endpoint: &str,
    interface: &str,
    body: &[u8],
) -> Option<EvidenceRecord> {
    let text = std::str::from_utf8(body).ok()?.trim();
    let value = text
        .strip_prefix("<firmware_version>")?
        .strip_suffix("</firmware_version>")?;
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-/".contains(&b))
    {
        return None;
    }
    let mut e = EvidenceRecord::user_reported(
        device.id.clone(),
        Attribute::FirmwareVersion,
        value.into(),
        connection::now(),
    )
    .ok()?;
    e.source = EvidenceOrigin::EndpointResponse;
    e.verification = Verification::Observed;
    e.reliability = Reliability::Observed;
    e.collection_method = "GET /version_num.txt (static unauthenticated file)".into();
    e.endpoint = Some(endpoint.into());
    e.interface_id = Some(interface.into());
    e.reference = Some("/version_num.txt".into());
    Some(e)
}
