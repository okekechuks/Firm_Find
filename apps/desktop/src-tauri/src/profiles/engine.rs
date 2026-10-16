use super::*;
use crate::{connection::SavedRouter, discovery::Device};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceOrigin {
    WindowsPnp,
    PhysicalRelationship,
    NetworkTopology,
    AuthenticatedManagement,
    SupportedModem,
    UserSupplied,
    EndpointResponse,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reliability {
    Strong,
    Observed,
    Hint,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verification {
    Verified,
    Observed,
    UserReported,
    Unverified,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceRecord {
    pub id: String,
    pub subject_id: String,
    pub attribute: Attribute,
    pub value: String,
    pub source: EvidenceOrigin,
    pub collection_method: String,
    pub collected_at: u64,
    pub reliability: Reliability,
    pub verification: Verification,
    pub interface_id: Option<String>,
    pub endpoint: Option<String>,
    pub reference: Option<String>,
    pub hardware_applicability: Option<String>,
    pub firmware_applicability: Option<String>,
    pub conflicts: Vec<String>,
    pub stale: bool,
}
impl EvidenceRecord {
    pub fn user_reported(
        subject_id: String,
        attribute: Attribute,
        value: String,
        now: u64,
    ) -> Result<Self, String> {
        if !matches!(
            attribute,
            Attribute::ReportedIdentifier
                | Attribute::Model
                | Attribute::HardwareRevision
                | Attribute::FirmwareFamily
                | Attribute::FirmwareVersion
        ) {
            return Err("Only reported identity fields may be saved.".into());
        }
        let value = value.trim().to_string();
        if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
            return Err(
                "Reported value must contain 1–256 bytes without control characters.".into(),
            );
        }
        Ok(Self {
            id: digest(format!("{subject_id}|{attribute:?}|{value}").as_bytes()),
            subject_id,
            attribute,
            value,
            source: EvidenceOrigin::UserSupplied,
            collection_method: "User-reported label or admin-page text; not independently verified"
                .into(),
            collected_at: now,
            reliability: Reliability::Hint,
            verification: Verification::UserReported,
            interface_id: None,
            endpoint: None,
            reference: None,
            hardware_applicability: None,
            firmware_applicability: None,
            conflicts: vec![],
            stale: false,
        })
    }
    fn strong(&self) -> bool {
        !self.stale
            && self.verification == Verification::Verified
            && self.reliability == Reliability::Strong
            && matches!(
                self.source,
                EvidenceOrigin::AuthenticatedManagement | EvidenceOrigin::SupportedModem
            )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateMatch {
    pub profile_id: String,
    pub profile_version: String,
    pub name: String,
    pub origin: String,
    pub confidence: MatchConfidence,
    pub authoritative: bool,
    pub matched_fields: Vec<Attribute>,
    pub missing_fields: Vec<Attribute>,
    pub conflicting_fields: Vec<Attribute>,
    pub reasons: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubDecision {
    pub assessment: Assessment,
    pub reason: String,
    pub evidence_sources: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureDecision {
    pub assessment: Assessment,
    pub readiness: Vec<Readiness>,
    pub quality: String,
    pub reason: String,
    pub next_action: String,
    pub evidence_sources: Vec<String>,
    pub sub_capabilities: BTreeMap<String, SubDecision>,
    pub verified_range: Option<Range>,
    pub adapter_implemented: bool,
    pub can_apply: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionChange {
    pub previous_evaluated_at: u64,
    pub previous_profile: Option<String>,
    pub previous_registry_digest: String,
    pub reasons: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityReport {
    pub device_id: String,
    pub evaluated_at: u64,
    pub expires_at: u64,
    pub engine_version: String,
    pub registry_digest: String,
    pub input_digest: String,
    pub discovery_revision: u64,
    pub confidence: MatchConfidence,
    pub selected_profile: Option<String>,
    pub profile_version: Option<String>,
    pub profile_content_hash: Option<String>,
    pub candidates: Vec<CandidateMatch>,
    pub evidence: Vec<EvidenceRecord>,
    pub conflicts: Vec<Attribute>,
    pub features: BTreeMap<String, FeatureDecision>,
    pub warnings: Vec<String>,
    pub change: Option<DecisionChange>,
}
fn norm(attribute: Attribute, value: &str) -> String {
    let v = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if matches!(attribute, Attribute::UsbVid | Attribute::UsbPid) {
        v.trim_start_matches("0x").to_ascii_uppercase()
    } else {
        v.to_lowercase()
    }
}
pub fn gather_evidence(device: &Device, saved: &[SavedRouter]) -> Vec<EvidenceRecord> {
    let mut records = vec![];
    let mut push =
        |attribute, value: String, source, at, endpoint: Option<String>, interface_id| {
            let id = digest(
                format!(
                    "{}|{attribute:?}|{source:?}|{value}|{at}|{endpoint:?}|{interface_id:?}",
                    device.id
                )
                .as_bytes(),
            );
            records.push(EvidenceRecord {
                id,
                subject_id: device.id.clone(),
                attribute,
                value,
                source,
                collection_method: match source {
                    EvidenceOrigin::WindowsPnp => "Windows device property",
                    EvidenceOrigin::PhysicalRelationship => {
                        "Windows parent/container and adapter association"
                    }
                    EvidenceOrigin::NetworkTopology => "Windows IP Helper",
                    EvidenceOrigin::UserSupplied => "Saved user label",
                    _ => "Unauthenticated HTTP HEAD observation",
                }
                .into(),
                collected_at: at,
                reliability: if matches!(
                    source,
                    EvidenceOrigin::WindowsPnp | EvidenceOrigin::PhysicalRelationship
                ) {
                    Reliability::Observed
                } else {
                    Reliability::Hint
                },
                verification: if source == EvidenceOrigin::UserSupplied {
                    Verification::UserReported
                } else {
                    Verification::Observed
                },
                interface_id,
                endpoint,
                reference: None,
                hardware_applicability: None,
                firmware_applicability: None,
                conflicts: vec![],
                stale: false,
            });
        };
    if let Some(v) = &device.vendor_id {
        push(
            Attribute::UsbVid,
            v.clone(),
            EvidenceOrigin::WindowsPnp,
            device.last_seen_at,
            None,
            None,
        );
    }
    if let Some(v) = &device.product_id {
        push(
            Attribute::UsbPid,
            v.clone(),
            EvidenceOrigin::WindowsPnp,
            device.last_seen_at,
            None,
            None,
        );
    }
    push(
        Attribute::UsbProduct,
        device.display_name.clone(),
        EvidenceOrigin::WindowsPnp,
        device.last_seen_at,
        None,
        None,
    );
    push(
        Attribute::PhysicalIdentity,
        device.id.clone(),
        EvidenceOrigin::PhysicalRelationship,
        device.last_seen_at,
        None,
        None,
    );
    for observation in device.identity.observations.iter().take(96) {
        let attribute = if observation.kind == "windows_manufacturer" {
            Attribute::WindowsManufacturer
        } else if observation.kind == "gateway" {
            Attribute::Gateway
        } else {
            continue;
        };
        push(
            attribute,
            observation.value.clone(),
            if attribute == Attribute::Gateway {
                EvidenceOrigin::NetworkTopology
            } else {
                EvidenceOrigin::WindowsPnp
            },
            observation.observed_at,
            None,
            None,
        );
    }
    for row in saved
        .iter()
        .filter(|r| r.device_id.as_deref() == Some(&device.id))
        .take(32)
    {
        if !row.label.is_empty() {
            push(
                Attribute::UserLabel,
                row.label.clone(),
                EvidenceOrigin::UserSupplied,
                row.created_at,
                Some(row.address.clone()),
                row.interface_id.clone(),
            );
        }
        if let Some(result) = &row.last_result {
            if let Some(status) = result.status {
                push(
                    Attribute::HttpStatus,
                    status.to_string(),
                    EvidenceOrigin::EndpointResponse,
                    result.checked_at,
                    Some(row.address.clone()),
                    row.interface_id.clone(),
                );
            }
        }
    }
    records.sort_by(|a, b| a.id.cmp(&b.id));
    records.dedup_by(|a, b| a.id == b.id);
    records
}
fn match_profile(
    entry: &RegistryEntry,
    evidence: &[EvidenceRecord],
    conflicts: &[Attribute],
) -> CandidateMatch {
    let p = &entry.profile;
    let mut rules = vec![
        (
            Attribute::Manufacturer,
            vec![p.manufacturer.clone()],
            Purpose::Identity,
        ),
        (
            Attribute::Model,
            std::iter::once(p.model.clone())
                .chain(p.model_aliases.clone())
                .collect(),
            Purpose::Identity,
        ),
        (
            Attribute::HardwareRevision,
            p.hardware_revisions.clone(),
            Purpose::Identity,
        ),
        (
            Attribute::FirmwareFamily,
            p.firmware_families.clone(),
            Purpose::Identity,
        ),
    ];
    rules.extend(
        p.match_rules
            .iter()
            .map(|r| (r.attribute, r.values.clone(), r.purpose)),
    );
    let mut matched = BTreeSet::new();
    let mut missing = BTreeSet::new();
    let mut mismatched = BTreeSet::new();
    let mut strong_all = true;
    let mut endpoints = BTreeSet::new();
    let mut reasons = vec![];
    for (attribute, expected, purpose) in rules {
        let observations: Vec<_> = evidence
            .iter()
            .filter(|e| !e.stale && e.attribute == attribute)
            .collect();
        if expected.is_empty() || observations.is_empty() {
            if purpose == Purpose::Identity {
                missing.insert(attribute);
                strong_all = false;
            }
            continue;
        }
        let values: BTreeSet<_> = expected.iter().map(|s| norm(attribute, s)).collect();
        if purpose == Purpose::Identity
            && observations
                .iter()
                .any(|e| e.strong() && !values.contains(&norm(attribute, &e.value)))
        {
            mismatched.insert(attribute);
        }
        let hits: Vec<_> = observations
            .iter()
            .filter(|e| values.contains(&norm(attribute, &e.value)))
            .collect();
        if hits.is_empty() {
            if purpose == Purpose::Identity && observations.iter().any(|e| e.strong()) {
                mismatched.insert(attribute);
            } else if purpose == Purpose::Identity {
                missing.insert(attribute);
            }
            strong_all = false;
        } else {
            matched.insert(attribute);
            if purpose == Purpose::Identity {
                let strong: Vec<_> = hits.iter().filter(|e| e.strong()).collect();
                if strong.is_empty() {
                    strong_all = false;
                }
                for e in strong {
                    if let Some(endpoint) = &e.endpoint {
                        endpoints.insert(endpoint.clone());
                    }
                }
            }
        }
    }
    let firmware: Vec<_> = evidence
        .iter()
        .filter(|e| !e.stale && e.attribute == Attribute::FirmwareVersion)
        .collect();
    if firmware.is_empty() {
        missing.insert(Attribute::FirmwareVersion);
        strong_all = false;
    }
    for e in firmware {
        match version_allowed(&e.value, &p.firmware_version_constraints) {
            Some(true) => {
                matched.insert(Attribute::FirmwareVersion);
                if !e.strong() {
                    strong_all = false;
                } else if let Some(endpoint) = &e.endpoint {
                    endpoints.insert(endpoint.clone());
                }
            }
            Some(false) if e.strong() => {
                mismatched.insert(Attribute::FirmwareVersion);
            }
            _ => {
                missing.insert(Attribute::FirmwareVersion);
                strong_all = false;
            }
        }
    }
    if endpoints.len() != 1 {
        strong_all = false;
        reasons.push("Exact identity must be anchored to one trusted management endpoint; unrelated responses cannot be combined.".into());
    }
    if !missing.is_empty() {
        reasons.push(format!(
            "Missing or unverifiable identity fields: {missing:?}."
        ));
    }
    if !mismatched.is_empty() {
        reasons.push(format!(
            "Excluded by exact hardware/firmware/identity constraints: {mismatched:?}."
        ));
    }
    let conflicts: Vec<_> = conflicts
        .iter()
        .copied()
        .filter(|a| {
            [
                Attribute::Manufacturer,
                Attribute::Model,
                Attribute::HardwareRevision,
                Attribute::FirmwareFamily,
                Attribute::FirmwareVersion,
                Attribute::ManagementIdentity,
                Attribute::ModemManufacturer,
                Attribute::ModemModel,
            ]
            .contains(a)
        })
        .collect();
    let confidence = if !conflicts.is_empty() {
        MatchConfidence::Conflicting
    } else if !mismatched.is_empty() || matched.is_empty() {
        MatchConfidence::NoMatch
    } else if missing.is_empty() && strong_all && entry.authoritative {
        MatchConfidence::Verified
    } else if matched.contains(&Attribute::Model)
        && evidence
            .iter()
            .any(|e| e.attribute == Attribute::Model && e.strong())
    {
        MatchConfidence::Probable
    } else {
        MatchConfidence::Candidate
    };
    if !entry.authoritative {
        reasons.push("Observation, experimental and imported profiles cannot grant authoritative capability support.".into());
    }
    if confidence == MatchConfidence::Verified {
        reasons.push("Fresh verified identity, exact hardware revision and constrained firmware match the curated profile.".into());
    }
    CandidateMatch {
        profile_id: p.profile_id.clone(),
        profile_version: p.profile_version.clone(),
        name: format!("{} {}", p.manufacturer, p.model),
        origin: entry.origin.clone(),
        confidence,
        authoritative: entry.authoritative,
        matched_fields: matched.into_iter().collect(),
        missing_fields: missing.into_iter().collect(),
        conflicting_fields: mismatched.into_iter().chain(conflicts).collect(),
        reasons,
    }
}
pub fn evaluate(
    registry: &Registry,
    device: &Device,
    mut evidence: Vec<EvidenceRecord>,
    now: u64,
    revision: u64,
) -> CompatibilityReport {
    let mut warnings = registry.diagnostics.clone();
    let invalid = evidence.len() > 256
        || evidence.iter().any(|e| {
            e.subject_id != device.id || e.value.len() > 1024 || e.collection_method.len() > 1024
        });
    if invalid {
        warnings.push("Evidence exceeds bounds or belongs to another device; no capability authority is granted.".into());
        evidence.retain(|e| e.subject_id == device.id && e.value.len() <= 1024);
        evidence.truncate(256);
    }
    for e in &mut evidence {
        e.stale = !device.connected
            || e.collected_at.saturating_add(EVIDENCE_TTL_MS) < now
            || e.collected_at > now.saturating_add(30_000);
        e.conflicts.clear();
    }
    evidence.sort_by(|a, b| a.id.cmp(&b.id));
    let mut conflicts = BTreeSet::new();
    for attribute in [
        Attribute::Manufacturer,
        Attribute::Model,
        Attribute::HardwareRevision,
        Attribute::FirmwareFamily,
        Attribute::FirmwareVersion,
        Attribute::ModemManufacturer,
        Attribute::ModemModel,
        Attribute::ManagementIdentity,
    ] {
        let values: BTreeSet<_> = evidence
            .iter()
            .filter(|e| e.attribute == attribute && e.strong())
            .map(|e| norm(attribute, &e.value))
            .collect();
        if values.len() > 1 {
            conflicts.insert(attribute);
            let ids: Vec<_> = evidence
                .iter()
                .filter(|e| e.attribute == attribute && e.strong())
                .map(|e| e.id.clone())
                .collect();
            for e in evidence.iter_mut().filter(|e| e.attribute == attribute) {
                e.conflicts = ids.iter().filter(|id| *id != &e.id).cloned().collect();
            }
        }
    }
    let conflicts: Vec<_> = conflicts.into_iter().collect();
    let candidates: Vec<_> = registry
        .profiles
        .iter()
        .map(|entry| match_profile(entry, &evidence, &conflicts))
        .collect();
    let viable: Vec<_> = candidates
        .iter()
        .filter(|c| {
            ![MatchConfidence::NoMatch, MatchConfidence::Conflicting].contains(&c.confidence)
        })
        .collect();
    let confidence = if invalid {
        MatchConfidence::NoMatch
    } else if !conflicts.is_empty() {
        MatchConfidence::Conflicting
    } else if viable.len() > 1 {
        MatchConfidence::Ambiguous
    } else {
        viable
            .first()
            .map_or(MatchConfidence::NoMatch, |c| c.confidence)
    };
    let selected = if confidence == MatchConfidence::Verified {
        viable
            .first()
            .and_then(|c| registry.by_id(&c.profile_id, Some(&c.profile_version)))
    } else {
        None
    };
    let mut features = BTreeMap::new();
    for key in ["dns", "wifiUserLimit", "apn", "ussd"] {
        let specification = selected.and_then(|p| {
            p.profile
                .capabilities
                .entries()
                .into_iter()
                .find(|(k, _)| *k == key)
                .map(|(_, f)| f)
        });
        let assessment = specification.map_or(Assessment::Unknown, |s| s.assessment);
        let mut readiness = specification.map_or_else(
            || vec![Readiness::RequiresManualVerification],
            |s| s.requirements.clone(),
        );
        // This phase implements no device adapter; profile declarations cannot override it.
        readiness.push(Readiness::NotImplemented);
        if !device.connected || assessment == Assessment::Unsupported {
            readiness.push(Readiness::Unavailable);
        }
        readiness.dedup();
        let reason = specification.map_or_else(|| format!("No unique, fresh, verified profile match ({confidence:?}). USB IDs, labels and HTTP success cannot establish support."), |s| s.reason.clone());
        let subs = sub_keys(key).iter().map(|name| {
            let s = specification.and_then(|f| f.sub_capabilities.get(*name));
            ((*name).into(),SubDecision {assessment: s.map_or(Assessment::Unknown, |s| s.assessment), reason: s.map_or_else(|| "Exact hardware/firmware and a trustworthy feature-specific source are required.".into(), |s| s.reason.clone()), evidence_sources: s.map_or_else(Vec::new, |s| s.evidence_sources.clone())})
        }).collect();
        features.insert(key.into(), FeatureDecision { assessment, readiness, quality: if confidence == MatchConfidence::Conflicting { "conflicting" } else if selected.is_some() { "documented" } else { "insufficient" }.into(), reason,
            next_action: if selected.is_some() { "Inspect feature prerequisites. Configuration adapters and safe transactions are later phases; no Apply action is available." } else { "Obtain trusted model, exact hardware revision and firmware identity through a supported read-only adapter. Do not select a profile by nickname." }.into(),
            evidence_sources: specification.map_or_else(Vec::new, |s| s.evidence_sources.clone()), sub_capabilities: subs, verified_range: specification.and_then(|s| s.verified_range.clone()), adapter_implemented: false, can_apply: false });
    }
    let input_digest = digest(
        &serde_json::to_vec(&(
            ENGINE_VERSION,
            &registry.digest,
            device.connected,
            &evidence,
            confidence,
        ))
        .expect("Evidence serialization"),
    );
    let expires_at = evidence
        .iter()
        .filter(|e| e.strong())
        .map(|e| e.collected_at.saturating_add(EVIDENCE_TTL_MS))
        .min()
        .unwrap_or(now.saturating_add(EVIDENCE_TTL_MS));
    CompatibilityReport {
        device_id: device.id.clone(),
        evaluated_at: now,
        expires_at,
        engine_version: ENGINE_VERSION.into(),
        registry_digest: registry.digest.clone(),
        input_digest,
        discovery_revision: revision,
        confidence,
        selected_profile: selected.map(|p| p.profile.profile_id.clone()),
        profile_version: selected.map(|p| p.profile.profile_version.clone()),
        profile_content_hash: selected.map(|p| p.content_hash.clone()),
        candidates,
        evidence,
        conflicts,
        features,
        warnings,
        change: None,
    }
}
pub fn record_change(current: &mut CompatibilityReport, previous: &CompatibilityReport) {
    if current.input_digest == previous.input_digest {
        current.change = previous.change.clone();
        return;
    }
    let mut reasons = vec![];
    if current.registry_digest != previous.registry_digest {
        reasons.push(
            "Profile registry content/version changed; the previous result was re-evaluated."
                .into(),
        );
    }
    if current.engine_version != previous.engine_version {
        reasons.push("Capability engine version changed.".into());
    }
    if current.confidence != previous.confidence {
        reasons.push(format!(
            "Match changed from {:?} to {:?}.",
            previous.confidence, current.confidence
        ));
    }
    reasons.push("Observed identity, connection state or evidence freshness changed; cached support is not reused as authority.".into());
    current.change = Some(DecisionChange {
        previous_evaluated_at: previous.evaluated_at,
        previous_profile: previous.selected_profile.clone(),
        previous_registry_digest: previous.registry_digest.clone(),
        reasons,
    });
}
