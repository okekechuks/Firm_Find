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
pub mod ipc;
mod strategies;
pub use strategies::AcquisitionStrategyRegistry;
#[derive(Default)]
pub struct Service(pub std::sync::Mutex<BTreeMap<String, AcquisitionCapabilityReport>>);

pub type IdentityEvidence = EvidenceRecord;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    NotChecked,
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
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscoveryRequest {
    pub device_id: String,
    pub endpoint: Option<String>,
    pub interface_id: Option<String>,
    pub authorized: bool,
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
    let signals = [
        ("e_link_oem", title.contains("e-link")),
        ("openwrt", title.contains("openwrt") || server == "uhttpd"),
        ("huawei_candidate", title.contains("huawei")),
        ("zte_candidate", title.contains("zte")),
        ("d_link_candidate", title.contains("d-link")),
        ("vida_oem", title.contains("vida")),
    ];
    signals.into_iter().filter(|(_, found)| *found).map(|(family, _)| VendorProtocolCandidate {
        family: family.into(), confidence: CapabilityState::Candidate,
        evidence: vec!["Allowlisted root-page title/server signal; unauthenticated observation".into()],
        limitations: "Candidate family only. No exact model, management API, credentials or firmware export established.".into(),
    }).collect()
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

pub fn discover(
    device: &Device,
    evidence: Vec<IdentityEvidence>,
    revision: u64,
    input: &DiscoveryRequest,
    token: &AtomicBool,
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
    };
    let mut protocols = vec![];
    if input.authorized {
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
        let (result, body) = connection::root_metadata(&target, interface, &device.networks, token);
        probe = ProbeSummary {
            attempted: true,
            code: result.code,
            http_status: result.status,
            metadata_truncated: body.as_ref().is_some_and(|b| b.len() == 16384),
        };
        if result.status.is_some_and(|s| (200..300).contains(&s)) {
            protocols = recognize(
                body.as_deref().unwrap_or_default(),
                result.server.as_deref(),
            );
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
            vec![]
        } else {
            evidence
        },
        now,
        fingerprint.endpoint.as_deref(),
        fingerprint.interface_id.as_deref(),
    );
    let candidates = AcquisitionStrategyRegistry::default().evaluate(&identity);
    let mut capabilities = BTreeMap::new();
    capabilities.insert("deviceDetected".into(), capability(CapabilityState::CompletedVerified, "Windows currently observes the device or selected network transport. This does not verify retail identity.", &[]));
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
                CapabilityState::Unknown,
                "Not independently qualified by this discovery. No operation is enabled.",
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
    serde_json::json!({"schemaVersion":1,"engine":"7.5A-C-sprint-a","evaluatedAt":report.evaluated_at,"usbVid":report.fingerprint.usb_vid,"usbPid":report.fingerprint.usb_pid,"transports":report.fingerprint.transports,"identityConfidence":report.identity.confidence,"evidenceCounts":report.identity.evidence.len(),"protocolCandidates":report.protocols.iter().map(|p| &p.family).collect::<Vec<_>>(),"probe":report.probe,"strategies":report.candidates,"canObtain":false})
}

#[cfg(test)]
mod tests;
