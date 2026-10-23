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
mod challenge;
pub(crate) use approval::AuthChallengeApproval;
pub(crate) use challenge::{parse_headers as parse_challenge_headers, ChallengeObservation};
#[cfg(test)]
mod challenge_tests;
mod static_menu;
pub(crate) use approval::StaticMenuApproval;
pub mod ipc;
pub mod request_registry;
#[cfg(test)]
mod static_menu_tests;
mod strategies;
mod strategy_api;
use approval::Approvals;
pub(crate) use approval::ClaimedApproval;
pub use approval::{ApprovalScope, StrategyProbeApproval};
pub use request_registry::{
    ParserId, ProtocolFamilyId, RequestEnvelope, RequestId, RequestRegistry, RequestSpec,
    RequestValidationError, ResponseType,
};
pub use strategies::AcquisitionStrategyRegistry;
pub use strategy_api::{
    AcquisitionPlan, AcquisitionStrategy, Applicability, EvaluationContext, LimitationCode,
    ProbeContext, ProbeReport, Requirement, StrategyError, StrategyId, StrategyKind,
};
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
    pub stages: Vec<CompatibilityStage>,
    pub capabilities: BTreeMap<String, Capability>,
    pub probe: ProbeSummary,
    #[serde(default)]
    pub static_menu: Option<static_menu::StaticMenuObservation>,
    #[serde(default)]
    pub auth_challenge: Option<ChallengeObservation>,
    pub can_obtain: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityStageId {
    Detected,
    ExactIdentity,
    ManagementApi,
    OfficialImage,
    FirmwareExport,
    PhysicalRead,
    FirmwareAcquired,
    FilesystemExtracted,
    Customization,
    Rebuild,
    Installation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityStageState {
    Verified,
    Observed,
    Unknown,
    NotChecked,
    NotSupported,
    NotImplemented,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityStage {
    pub id: CompatibilityStageId,
    pub state: CompatibilityStageState,
    pub reason: String,
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

/// Bounded lexical HTML scan: comments and raw script/style text are never tags.
/// Only complete src/href attributes on actual opening tags are resource evidence.
fn resource_attributes(page: &str) -> Vec<&str> {
    let bytes = page.as_bytes();
    let mut at = 0;
    let mut values = vec![];
    while at < bytes.len() {
        let Some(next) = page[at..].find('<') else {
            break;
        };
        at += next;
        // A comparison or a truncated text fragment is not an opening tag.
        // Resume at the next '<', rather than swallowing a following real tag.
        if !bytes
            .get(at + 1)
            .is_some_and(|b| b.is_ascii_alphabetic() || matches!(b, b'/' | b'!'))
        {
            at += 1;
            continue;
        }
        if page[at..].starts_with("<!--") {
            let Some(end) = page[at + 4..].find("-->") else {
                break;
            };
            at += 4 + end + 3;
            continue;
        }
        let start = at + 1;
        let mut end = start;
        let mut quote = 0;
        while end < bytes.len() {
            let b = bytes[end];
            if quote != 0 {
                if b == quote {
                    quote = 0;
                }
            } else if b == b'\'' || b == b'"' {
                quote = b;
            } else if b == b'>' {
                break;
            }
            end += 1;
        }
        if end == bytes.len() {
            break;
        }
        at = end + 1;
        let mut cursor = start;
        if !bytes[cursor].is_ascii_alphabetic() {
            continue;
        }
        while cursor < end && bytes[cursor].is_ascii_alphanumeric() {
            cursor += 1;
        }
        let tag = &page[start..cursor];
        while cursor < end {
            while cursor < end && (bytes[cursor].is_ascii_whitespace() || bytes[cursor] == b'/') {
                cursor += 1;
            }
            let key = cursor;
            while cursor < end
                && !bytes[cursor].is_ascii_whitespace()
                && !b"=/>".contains(&bytes[cursor])
            {
                cursor += 1;
            }
            if key == cursor {
                cursor += 1;
                continue;
            }
            let name = &page[key..cursor];
            while cursor < end && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor == end || bytes[cursor] != b'=' {
                continue;
            }
            cursor += 1;
            while cursor < end && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if cursor == end {
                break;
            }
            let delimiter = if bytes[cursor] == b'\'' || bytes[cursor] == b'"' {
                let q = bytes[cursor];
                cursor += 1;
                Some(q)
            } else {
                None
            };
            let value = cursor;
            while cursor < end
                && !delimiter.map_or_else(
                    || bytes[cursor].is_ascii_whitespace() || bytes[cursor] == b'>',
                    |q| bytes[cursor] == q,
                )
            {
                cursor += 1;
            }
            if name == "src" || name == "href" {
                values.push(&page[value..cursor]);
            }
            if delimiter.is_some() && cursor < end {
                cursor += 1;
            }
        }
        if tag == "script" || tag == "style" {
            let close = format!("</{tag}");
            let Some(end) = page[at..].find(&close) else {
                break;
            };
            at += end;
        }
    }
    values
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
    let resources = resource_attributes(&page);
    let asr_signals: Vec<_> = asr
        .into_iter()
        .filter(|v| {
            resources
                .iter()
                .any(|r| r.trim_start_matches("./") == *v || r.trim_start_matches('/') == *v)
        })
        .collect();
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
                        // Evidence text only; never a model or manufacturer selection branch.
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
    discover_on_networks(
        device,
        evidence,
        revision,
        input,
        token,
        &device.networks,
        None,
    )
}

pub(super) fn discover_on_networks(
    device: &Device,
    mut evidence: Vec<IdentityEvidence>,
    revision: u64,
    input: &DiscoveryRequest,
    token: &AtomicBool,
    all_networks: &[crate::discovery::NetworkAdapter],
    approval: Option<ClaimedApproval>,
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
    if let Some(approval) = approval {
        approval.validate(&approval::binding(device, revision, input)?)?;
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
        let (result, body) =
            connection::root_metadata(&approval, &target, interface, all_networks, token);
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
                connection::version_metadata(&approval, &target, interface, all_networks, token);
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
    let stages = compatibility_stages(&identity, &probe, &capabilities, &protocols);
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
        stages,
        capabilities,
        probe,
        static_menu: None,
        auth_challenge: None,
        can_obtain: false,
    }
}

fn compatibility_stages(
    identity: &DeviceIdentityAssessment,
    probe: &ProbeSummary,
    capabilities: &BTreeMap<String, Capability>,
    protocols: &[VendorProtocolCandidate],
) -> Vec<CompatibilityStage> {
    let state_for = |key: &str| {
        capabilities
            .get(key)
            .map(|capability| stage_state(capability.state))
            .unwrap_or(CompatibilityStageState::Unknown)
    };
    let reason_for = |key: &str| {
        capabilities
            .get(key)
            .map(|capability| capability.reason.clone())
            .unwrap_or_else(|| "No current evidence.".into())
    };
    let exact_identity = identity.verified_manufacturer.is_some()
        && identity.verified_firmware_family.is_some()
        && identity.verified_model.is_some()
        && identity.verified_hardware_revision.is_some()
        && identity.verified_firmware.is_some();
    let mut stages = vec![
        CompatibilityStage {
            id: CompatibilityStageId::Detected,
            state: CompatibilityStageState::Observed,
            reason: "Windows observes a connected device or selected network interface.".into(),
        },
        CompatibilityStage {
            id: CompatibilityStageId::ExactIdentity,
            state: if exact_identity {
                CompatibilityStageState::Verified
            } else {
                CompatibilityStageState::Unknown
            },
            reason: "Exact identity requires fresh, consistent, endpoint-bound trusted evidence."
                .into(),
        },
        CompatibilityStage {
            id: CompatibilityStageId::ManagementApi,
            state: if probe.http_status.is_some() && !protocols.is_empty() {
                CompatibilityStageState::Observed
            } else if probe.attempted {
                CompatibilityStageState::Unknown
            } else {
                CompatibilityStageState::NotChecked
            },
            reason: if probe.http_status.is_some() && protocols.is_empty() {
                "A web server answered; no management family recognized"
            } else if probe.http_status.is_some() {
                "A recognized management family is only a candidate; support remains unverified."
            } else {
                "No management family response observed."
            }
            .into(),
        },
    ];
    for (id, key) in [
        (CompatibilityStageId::OfficialImage, "officialImage"),
        (CompatibilityStageId::FirmwareExport, "deviceExport"),
        (CompatibilityStageId::PhysicalRead, "physicalRead"),
        (CompatibilityStageId::FirmwareAcquired, "firmwareAcquired"),
        (
            CompatibilityStageId::FilesystemExtracted,
            "filesystemExtracted",
        ),
    ] {
        stages.push(CompatibilityStage {
            id,
            state: state_for(key),
            reason: reason_for(key),
        });
    }
    let customization_states = [
        state_for("dnsCustomization"),
        state_for("wifiCustomization"),
        state_for("apnCustomization"),
        state_for("ussdCustomization"),
    ];
    let customization_state = if customization_states
        .iter()
        .all(|state| *state == customization_states[0])
    {
        customization_states[0]
    } else {
        CompatibilityStageState::Unknown
    };
    stages.push(CompatibilityStage {
        id: CompatibilityStageId::Customization,
        state: customization_state,
        reason: "Each feature requires its own exact hardware support; this stage grants no operation authority.".into(),
    });
    stages.push(CompatibilityStage {
        id: CompatibilityStageId::Rebuild,
        state: state_for("rebuilding"),
        reason: reason_for("rebuilding"),
    });
    stages.push(CompatibilityStage {
        id: CompatibilityStageId::Installation,
        state: state_for("installation"),
        reason: reason_for("installation"),
    });
    stages
}

fn stage_state(state: CapabilityState) -> CompatibilityStageState {
    match state {
        CapabilityState::AvailableVerified | CapabilityState::CompletedVerified => {
            CompatibilityStageState::Verified
        }
        CapabilityState::Observed => CompatibilityStageState::Observed,
        CapabilityState::NotChecked => CompatibilityStageState::NotChecked,
        CapabilityState::NotImplemented => CompatibilityStageState::NotImplemented,
        CapabilityState::Unsupported => CompatibilityStageState::NotSupported,
        CapabilityState::Unknown
        | CapabilityState::Candidate
        | CapabilityState::RequiresAuthentication
        | CapabilityState::RequiresApproval
        | CapabilityState::RequiresExternalTool
        | CapabilityState::Failed => CompatibilityStageState::Unknown,
    }
}

/// Deliberately omit all raw evidence, device IDs, endpoint/interface addresses and peer text.
pub fn diagnostics(report: &AcquisitionCapabilityReport) -> serde_json::Value {
    let mut exported = serde_json::json!({"schemaVersion":1,"engine":"7.5A-C-sprint-a","evaluatedAt":report.evaluated_at,"usbVid":report.fingerprint.usb_vid,"usbPid":report.fingerprint.usb_pid,"transports":report.fingerprint.transports,"identityConfidence":report.identity.confidence,"evidenceCounts":report.identity.evidence.len(),"protocolCandidates":report.protocols.iter().map(|p| &p.family).collect::<Vec<_>>(),"probe":{"attempted":report.probe.attempted,"code":report.probe.code,"httpStatus":report.probe.http_status,"metadataTruncated":report.probe.metadata_truncated,"advancedEndpoint":report.probe.advanced_endpoint},"strategies":report.candidates,"canObtain":false});
    if let Some(challenge) = &report.auth_challenge {
        exported["authChallenge"] =
            serde_json::to_value(challenge.sanitized()).expect("bounded challenge serialization");
    }
    if let Some(menu) = report
        .static_menu
        .as_ref()
        .filter(|m| m.code == "wrong_content_type")
    {
        exported["staticMenu"] = serde_json::json!({"code":"wrong_content_type", "observedMediaType":connection::menu_media_type(menu.observed_media_type.as_deref())});
    }
    exported
}

#[cfg(test)]
mod tests;

fn version_evidence(
    device: &Device,
    endpoint: &str,
    interface: &str,
    body: &[u8],
) -> Option<EvidenceRecord> {
    let text = std::str::from_utf8(body)
        .ok()?
        .trim_start_matches('\u{feff}')
        .trim();
    let text = if text.starts_with("<?xml ") {
        let (declaration, rest) = text.split_once("?>")?;
        if declaration.len() > 128
            || !declaration.is_ascii()
            || declaration[5..].contains(['<', '>', '&'])
        {
            return None;
        }
        rest.trim()
    } else {
        text
    };
    let text = if let Some(inner) = text.strip_prefix("<RGW>") {
        inner.strip_suffix("</RGW>")?.trim()
    } else {
        text
    };
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
