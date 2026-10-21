//! Declarative profiles cannot authorize operations or supply executable code.
mod engine;
mod json;
pub use engine::{
    evaluate, gather_evidence, record_change, CompatibilityReport, EvidenceOrigin, EvidenceRecord,
    Reliability, Verification,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_PROFILE_BYTES: usize = 65536;
pub const ENGINE_VERSION: &str = "1.1.0";
pub const EVIDENCE_TTL_MS: u64 = 300_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Attribute {
    Manufacturer,
    Model,
    HardwareRevision,
    FirmwareFamily,
    FirmwareVersion,
    UsbVid,
    UsbPid,
    UsbProduct,
    PhysicalIdentity,
    ManagementIdentity,
    ModemManufacturer,
    ModemModel,
    WindowsManufacturer,
    Gateway,
    UserLabel,
    HttpStatus,
    WirelessDriver,
    ReportedIdentifier,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Assessment {
    Supported,
    Unsupported,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Readiness {
    ReadyForFutureAdapter,
    RequiresAuthentication,
    RequiresPackage,
    RequiresFirmwareBuild,
    RequiresManualVerification,
    NotImplemented,
    Unavailable,
    Experimental,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MatchConfidence {
    Verified,
    Probable,
    Candidate,
    Ambiguous,
    NoMatch,
    Conflicting,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    Identity,
    Corroboration,
    CandidateHint,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceCategory {
    ManufacturerDocumentation,
    UpstreamDocumentation,
    HardwareObservation,
    HardwareTested,
    Experimental,
    LocalUnverified,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Http,
    Https,
    Ssh,
    Ubus,
    ModemManager,
    DocumentedModem,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatchRule {
    pub attribute: Attribute,
    pub values: Vec<String>,
    pub purpose: Purpose,
    pub explanation: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionConstraint {
    // Omitted in legacy schema-1 profiles. Do not change their canonical content hashes.
    #[serde(default, skip_serializing_if = "FirmwareVersionScheme::is_legacy")]
    pub scheme: FirmwareVersionScheme,
    pub exact: Vec<String>,
    pub min_inclusive: Option<String>,
    pub max_exclusive: Option<String>,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirmwareVersionScheme {
    #[default]
    NumericTriplet,
    OpaqueExact,
}
impl FirmwareVersionScheme {
    fn is_legacy(&self) -> bool {
        *self == Self::NumericTriplet
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceSource {
    pub id: String,
    pub category: SourceCategory,
    pub reference: String,
    pub summary: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Provenance {
    pub author: String,
    pub category: SourceCategory,
    pub notes: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManagementInterface {
    pub protocol: Protocol,
    pub authentication_required: bool,
    pub firmware_families: Vec<String>,
    pub operations: Vec<String>,
    pub security: String,
    pub limitations: String,
    pub evidence_sources: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModemInformation {
    pub integrated_modem: Option<bool>,
    pub manufacturers: Vec<String>,
    pub models: Vec<String>,
    pub interfaces: Vec<Protocol>,
    pub notes: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Range {
    pub min: u32,
    pub max: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubCapability {
    pub assessment: Assessment,
    pub reason: String,
    pub evidence_sources: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FeatureSpec {
    pub assessment: Assessment,
    pub reason: String,
    pub requirements: Vec<Readiness>,
    pub evidence_sources: Vec<String>,
    pub sub_capabilities: BTreeMap<String, SubCapability>,
    pub verified_range: Option<Range>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Features {
    pub dns: FeatureSpec,
    pub wifi_user_limit: FeatureSpec,
    pub apn: FeatureSpec,
    pub ussd: FeatureSpec,
}
impl Features {
    pub fn entries(&self) -> [(&'static str, &FeatureSpec); 4] {
        [
            ("dns", &self.dns),
            ("wifiUserLimit", &self.wifi_user_limit),
            ("apn", &self.apn),
            ("ussd", &self.ussd),
        ]
    }
}
pub fn sub_keys(feature: &str) -> &'static [&'static str] {
    match feature {
        "dns" => &[
            "upstream",
            "lanIpv4",
            "lanIpv6",
            "forwarding",
            "persistence",
        ],
        "wifiUserLimit" => &[
            "accessPoint",
            "managedBss",
            "clientLimit",
            "validRange",
            "persistence",
            "reload",
            "managementRisk",
        ],
        "apn" => &[
            "modemPresent",
            "managementInterface",
            "readCurrent",
            "manual",
            "automatic",
            "operatorIdentity",
            "reconnect",
        ],
        "ussd" => &[
            "modemPresent",
            "modemIdentity",
            "interface",
            "oneShot",
            "interactive",
            "responseHandling",
            "cancellation",
            "routerService",
            "routerWebUi",
        ],
        _ => &[],
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Profile {
    pub schema_version: u32,
    /// Schema 2 is narrowly reserved for authenticated OS inspection, never hardware support.
    #[serde(default, skip_serializing_if = "is_false")]
    pub inspection_only: bool,
    pub profile_id: String,
    pub profile_version: String,
    pub manufacturer: String,
    pub model: String,
    pub model_aliases: Vec<String>,
    pub device_category: Vec<String>,
    pub hardware_revisions: Vec<String>,
    pub firmware_families: Vec<String>,
    pub firmware_version_constraints: VersionConstraint,
    pub match_rules: Vec<MatchRule>,
    pub management_interfaces: Vec<ManagementInterface>,
    pub modem_information: ModemInformation,
    pub capabilities: Features,
    pub known_limitations: Vec<String>,
    pub evidence_sources: Vec<EvidenceSource>,
    pub last_reviewed: String,
    pub provenance: Provenance,
}
fn is_false(value: &bool) -> bool {
    !*value
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryEntry {
    pub profile: Profile,
    pub origin: String,
    pub authoritative: bool,
    pub content_hash: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
    pub profiles: Vec<RegistryEntry>,
    pub diagnostics: Vec<String>,
    pub digest: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileError {
    pub code: &'static str,
    pub message: String,
}
fn invalid(message: impl Into<String>) -> ProfileError {
    ProfileError {
        code: "INVALID_PROFILE",
        message: message.into(),
    }
}
pub fn digest(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}
pub fn version(value: &str) -> Option<[u32; 3]> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|p| p.is_empty() || p.len() > 5 || !p.bytes().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    Some([
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ])
}
pub fn version_allowed(value: &str, c: &VersionConstraint) -> Option<bool> {
    validate_firmware_constraint(c).ok()?;
    if c.scheme == FirmwareVersionScheme::OpaqueExact {
        // Opaque identifiers are case-sensitive literals, even if they look numeric or dated.
        if !valid_opaque_version(value) {
            return None;
        }
        return Some(c.exact.iter().any(|expected| expected == value));
    }
    let v = version(value)?;
    if !c.exact.is_empty() {
        return Some(c.exact.iter().filter_map(|v| version(v)).any(|x| x == v));
    }
    match (&c.min_inclusive, &c.max_exclusive) {
        (Some(a), Some(b)) => Some(v >= version(a)? && v < version(b)?),
        _ => None,
    }
}
fn valid_opaque_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1024
        && value.trim() == value
        && !value.chars().any(char::is_control)
}
fn validate_firmware_constraint(c: &VersionConstraint) -> Result<(), ProfileError> {
    if c.exact.len() > 64 {
        return Err(invalid("Too many exact firmware versions."));
    }
    if c.scheme == FirmwareVersionScheme::OpaqueExact {
        if c.exact.is_empty()
            || c.exact.iter().any(|v| !valid_opaque_version(v))
            || c.min_inclusive.is_some()
            || c.max_exclusive.is_some()
        {
            return Err(invalid(
                "opaque_exact requires nonempty literal versions and forbids range bounds.",
            ));
        }
        return Ok(());
    }
    if (!c.exact.is_empty() && (c.min_inclusive.is_some() || c.max_exclusive.is_some()))
        || c.exact.iter().any(|v| version(v).is_none())
        || c.min_inclusive
            .as_ref()
            .is_some_and(|v| version(v).is_none())
        || c.max_exclusive
            .as_ref()
            .is_some_and(|v| version(v).is_none())
        || c.min_inclusive.is_some() != c.max_exclusive.is_some()
    {
        return Err(invalid("numeric_triplet requires exact three-part numeric releases or a bounded numeric range."));
    }
    if let (Some(a), Some(b)) = (&c.min_inclusive, &c.max_exclusive) {
        if version(a) >= version(b) {
            return Err(invalid("Firmware range is reversed or empty."));
        }
    }
    Ok(())
}
fn bounded(value: &serde_json::Value, depth: usize, nodes: &mut usize) -> bool {
    *nodes += 1;
    if depth > 16 || *nodes > 4096 {
        return false;
    }
    match value {
        serde_json::Value::String(s) => s.len() <= 1024 && !s.chars().any(char::is_control),
        serde_json::Value::Array(a) => {
            a.len() <= 64 && a.iter().all(|v| bounded(v, depth + 1, nodes))
        }
        serde_json::Value::Object(o) => {
            o.len() <= 64
                && o.iter()
                    .all(|(k, v)| k.len() <= 64 && bounded(v, depth + 1, nodes))
        }
        _ => true,
    }
}
pub fn validate(document: &str) -> Result<Profile, ProfileError> {
    if document.len() > MAX_PROFILE_BYTES {
        return Err(invalid("Profile exceeds 64 KiB."));
    }
    let value = serde_json::from_str::<json::UniqueValue>(document)
        .map_err(|_| invalid("Profile must be valid JSON without duplicate fields."))?
        .0;
    if !bounded(&value, 0, &mut 0) {
        return Err(invalid(
            "Profile exceeds depth, string or collection limits.",
        ));
    }
    // Deserialize original bytes as well: serde rejects duplicate struct fields.
    let p: Profile = serde_json::from_str(document)
        .map_err(|e| invalid(format!("Profile schema rejected: {e}")))?;
    if !matches!(
        (p.schema_version, p.inspection_only),
        (1, false) | (2, true)
    ) {
        return Err(invalid(
            "Schema 1 requires hardware identity; schema 2 requires explicit inspectionOnly scope.",
        ));
    }
    if p.profile_id.is_empty()
        || p.profile_id.len() > 96
        || !p
            .profile_id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b".-_".contains(&b))
        || version(&p.profile_version).is_none()
    {
        return Err(invalid("Profile ID or content version is invalid."));
    }
    if (!p.inspection_only && p.manufacturer.trim().is_empty())
        || p.model.trim().is_empty()
        || p.device_category.is_empty()
        || p.match_rules.is_empty()
    {
        return Err(invalid(
            "Manufacturer, model, category and match rules are required.",
        ));
    }
    let categories = [
        "cellular_mifi",
        "usb_cellular_modem",
        "cellular_router",
        "ethernet_router",
        "wifi_router",
        "openwrt_router",
        "gateway",
        "unknown_network_device",
    ];
    if p.device_category
        .iter()
        .any(|v| !categories.contains(&v.as_str()))
    {
        return Err(invalid("Unknown device category."));
    }
    if p.last_reviewed.len() != 10
        || !p.last_reviewed.bytes().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
    {
        return Err(invalid("lastReviewed must use YYYY-MM-DD."));
    }
    let year: u32 = p.last_reviewed[0..4].parse().unwrap_or(0);
    let month: u32 = p.last_reviewed[5..7].parse().unwrap_or(0);
    let day: u32 = p.last_reviewed[8..10].parse().unwrap_or(0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        _ => 0,
    };
    if year < 2000 || day == 0 || day > days {
        return Err(invalid("lastReviewed must be a valid calendar date."));
    }
    validate_firmware_constraint(&p.firmware_version_constraints)?;
    if p.inspection_only
        && (!p.manufacturer.is_empty()
            || !p.hardware_revisions.is_empty()
            || p.firmware_families != ["OpenWrt"]
            || p.firmware_version_constraints.exact.len() != 1
            || p.firmware_version_constraints.min_inclusive.is_some()
            || p.firmware_version_constraints.max_exclusive.is_some()
            || !p.match_rules.iter().any(|r| {
                r.attribute == Attribute::ManagementIdentity
                    && r.purpose == Purpose::Identity
                    && r.values.len() == 1
                    && {
                        let fields: Vec<_> = r.values[0].split('|').collect();
                        fields.len() == 3 && fields.iter().all(|v| !v.trim().is_empty())
                    }
            })
            || p.capabilities.entries().iter().any(|(_, f)| {
                f.assessment != Assessment::Unknown
                    || f.verified_range.is_some()
                    || f.sub_capabilities
                        .values()
                        .any(|s| s.assessment != Assessment::Unknown)
            }))
    {
        return Err(invalid("Inspection-only profiles require an exact OS/build/board contract and UNKNOWN hardware features; they cannot assert manufacturer or hardware revision."));
    }
    let mut source_ids = BTreeSet::new();
    for source in &p.evidence_sources {
        if source.id.is_empty() || !source_ids.insert(source.id.clone()) {
            return Err(invalid("Evidence source IDs must be unique."));
        }
        if source.reference.starts_with("local:docs/")
            && !source.reference.contains("..")
            && !source.reference.contains(['\\', '?', '#'])
        {
            continue;
        }
        let url = url::Url::parse(&source.reference)
            .map_err(|_| invalid("Evidence references must be HTTPS URLs."))?;
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return Err(invalid(
                "Evidence references must be HTTPS without credentials.",
            ));
        }
    }
    let references_valid = |refs: &[String]| refs.iter().all(|r| source_ids.contains(r));
    for rule in &p.match_rules {
        if rule.values.is_empty()
            || rule.values.iter().any(|v| v.trim().is_empty())
            || rule.explanation.trim().is_empty()
        {
            return Err(invalid(
                "Match rules require values and an evidentiary explanation.",
            ));
        }
        if [
            Attribute::Gateway,
            Attribute::UserLabel,
            Attribute::HttpStatus,
            Attribute::WindowsManufacturer,
        ]
        .contains(&rule.attribute)
            && rule.purpose != Purpose::CandidateHint
        {
            return Err(invalid(
                "Gateway, HTTP status, labels and Windows driver manufacturers can only be hints.",
            ));
        }
    }
    for (key, feature) in p.capabilities.entries() {
        if feature
            .sub_capabilities
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            != sub_keys(key).iter().copied().collect()
        {
            return Err(invalid(format!(
                "{key} must declare each supported sub-capability key exactly once."
            )));
        }
        if feature.reason.trim().is_empty()
            || !references_valid(&feature.evidence_sources)
            || (feature.assessment != Assessment::Unknown && feature.evidence_sources.is_empty())
        {
            return Err(invalid("Feature assessments need reasons and known assessments need valid evidence references."));
        }
        for sub in feature.sub_capabilities.values() {
            if sub.reason.trim().is_empty()
                || !references_valid(&sub.evidence_sources)
                || (sub.assessment != Assessment::Unknown && sub.evidence_sources.is_empty())
            {
                return Err(invalid(
                    "Sub-capabilities need reasons and known assessments need evidence references.",
                ));
            }
        }
        if feature
            .requirements
            .contains(&Readiness::ReadyForFutureAdapter)
            || feature.requirements.contains(&Readiness::NotImplemented)
        {
            return Err(invalid(
                "Adapter implementation is owned by OBTAINER, not profile data.",
            ));
        }
        if feature.verified_range.as_ref().is_some_and(|r| {
            key != "wifiUserLimit"
                || r.min == 0
                || r.min > r.max
                || feature.sub_capabilities["validRange"].assessment != Assessment::Supported
        }) {
            return Err(invalid(
                "Client-limit range needs supported range evidence and valid bounds.",
            ));
        }
    }
    if p.management_interfaces.iter().any(|m| {
        !references_valid(&m.evidence_sources)
            || m.operations.iter().any(|op| {
                !["identity", "read", "dns", "wifi_user_limit", "apn", "ussd"]
                    .contains(&op.as_str())
            })
    }) {
        return Err(invalid(
            "Invalid management evidence or declarative operation name.",
        ));
    }
    Ok(p)
}
impl Registry {
    /// The active registry is sorted by stable ID; archived versions live in SQLite.
    pub fn by_id(&self, id: &str, version: Option<&str>) -> Option<&RegistryEntry> {
        let index = self
            .profiles
            .binary_search_by(|entry| entry.profile.profile_id.as_str().cmp(id))
            .ok()?;
        let entry = &self.profiles[index];
        if version.is_some_and(|v| v != entry.profile.profile_version) {
            None
        } else {
            Some(entry)
        }
    }
    pub fn load(documents: Vec<(&str, &str, bool)>) -> Self {
        let mut registry = Self::default();
        let mut entries = BTreeMap::<String, Vec<RegistryEntry>>::new();
        for (document, origin, authoritative) in documents {
            match validate(document) {
                Ok(profile) => {
                    let authoritative = authoritative
                        && origin == "bundled"
                        && matches!(
                            profile.provenance.category,
                            SourceCategory::ManufacturerDocumentation
                                | SourceCategory::UpstreamDocumentation
                                | SourceCategory::HardwareTested
                        );
                    let content_hash =
                        digest(&serde_json::to_vec(&profile).expect("Profile serialization"));
                    entries
                        .entry(profile.profile_id.clone())
                        .or_default()
                        .push(RegistryEntry {
                            profile,
                            origin: origin.into(),
                            authoritative,
                            content_hash,
                        });
                }
                Err(error) => registry.diagnostics.push(error.message),
            }
        }
        for (id, mut versions) in entries {
            if versions.len() != 1 {
                registry
                    .diagnostics
                    .push(format!("Duplicate profile ID quarantined: {id}"));
            } else {
                registry.profiles.push(versions.remove(0));
            }
        }
        registry.digest =
            digest(&serde_json::to_vec(&registry.profiles).expect("Registry serialization"));
        registry
    }
    pub fn bundled() -> Self {
        Self::load(vec![
            (
                include_str!("../../../../../device-profiles/observed-comfi-mg11.json"),
                "bundled_observation",
                false,
            ),
            (
                include_str!("../../../../../device-profiles/tplink-archer-c7-v5-openwrt.json"),
                "bundled",
                true,
            ),
        ])
    }
}

#[cfg(test)]
mod tests;
