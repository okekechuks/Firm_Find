use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterTargetKind {
    #[default]
    Discovered,
    SavedEndpoint,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdapterQuery {
    pub device_id: String,
    #[serde(default)]
    pub target_kind: AdapterTargetKind,
}
impl AdapterQuery {
    pub fn validate(&self) -> Result<(), super::AdapterError> {
        if self.device_id.len() != 64
            || !self
                .device_id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(super::AdapterError::new(
                super::AdapterErrorCode::DeviceNotFound,
                "Device reference is invalid.",
                None,
                false,
                "Refresh discovery and select a current device.",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterImplementationStatus {
    BuiltIn,
    Declared,
    Unavailable,
    TestOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    Usb,
    Serial,
    Network,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthenticationMethod {
    None,
    Password,
    SshKey,
    ChallengeResponse,
    SessionToken,
    DocumentedVendor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthenticationState {
    NotAttempted,
    Required,
    Authenticating,
    Authenticated,
    Failed,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterOperation {
    Discovery,
    CapabilityEvaluation,
    AuthorizedIdentityProbe,
    ReadConfiguration,
    BackupConfiguration,
    ModifyConfiguration,
    FirmwareAcquisition,
    FirmwareBuild,
    FirmwareInstallation,
    UssdSession,
    InspectCapabilities,
    ReadDns,
    ReadApn,
    ReadWifiLimit,
    ReadUssdCapability,
    VerifyConfiguration,
    RestoreConfiguration,
}

impl AdapterOperation {
    pub fn is_physical_write(self) -> bool {
        matches!(
            self,
            Self::ModifyConfiguration
                | Self::FirmwareAcquisition
                | Self::FirmwareInstallation
                | Self::RestoreConfiguration
                | Self::UssdSession
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterMatchStatus {
    Exact,
    RequiresVerification,
    NotApplicable,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterSelectionStatus {
    Candidate,
    NoMatch,
    Ambiguous,
    Protected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Discovered,
    AdapterCandidate,
    IdentityVerificationRequired,
    AuthorizationRequired,
    AuthenticationRequired,
    Connecting,
    ConnectedReadOnly,
    Disconnected,
    Stale,
    Failed,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterDescriptor {
    pub id: String,
    pub name: String,
    pub version: String,
    pub implementation_status: AdapterImplementationStatus,
    pub transports: Vec<Transport>,
    pub authentication: AuthenticationMethod,
    pub operations: Vec<AdapterOperation>,
    pub dependencies: Vec<String>,
    pub limitations: Vec<String>,
    pub profile_ids: Vec<String>,
    pub firmware_families: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterCapability {
    pub operation: AdapterOperation,
    pub exists: bool,
    pub can_read: bool,
    pub can_plan: bool,
    pub can_apply: bool,
    pub can_verify: bool,
    pub can_restore: bool,
    pub authentication_required: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterMatch {
    pub adapter: AdapterDescriptor,
    pub status: AdapterMatchStatus,
    pub confidence: u8,
    pub reason: String,
    pub matched_evidence: Vec<String>,
    pub missing_evidence: Vec<String>,
    pub capabilities: Vec<AdapterCapability>,
    pub features: std::collections::BTreeMap<String, AdapterFeatureCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterEvaluation {
    pub device_id: String,
    pub discovery_revision: u64,
    pub protected_device: bool,
    pub selection: AdapterSelectionStatus,
    pub connection_state: ConnectionState,
    pub reason: String,
    pub matches: Vec<AdapterMatch>,
    pub selected_adapter: Option<String>,
    pub evaluated_at: u64,
    pub expires_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialReference {
    pub method: AuthenticationMethod,
    pub reference_id: Option<String>,
}

/// Internal only. Never deserialize authorization, trust or authentication from React.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionBinding {
    pub device_id: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub profile_id: String,
    pub profile_version: String,
    pub profile_hash: String,
    pub evidence_digest: String,
    pub discovery_revision: u64,
    pub endpoint: String,
    pub interface_id: String,
    pub endpoint_identity: String,
    pub expires_at: u64,
    pub simulated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    pub id: String,
    pub state: ConnectionState,
    pub simulated: bool,
    pub expires_at: u64,
    pub authentication: AuthenticationState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationSource {
    Automatic,
    Manual,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApnConfiguration {
    pub source: ConfigurationSource,
    pub apn: Option<String>,
    pub operator: Option<String>,
    pub authentication_required: bool,
    pub ip_family: IpFamily,
    pub authentication_method: ApnAuthentication,
    pub observed_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WifiClientLimitConfiguration {
    pub radio: String,
    pub ssid: Option<String>,
    pub bss_id: Option<String>,
    pub station_limit: Option<u32>,
    pub verified_minimum: Option<u32>,
    pub verified_maximum: Option<u32>,
    pub connected_clients: Option<u32>,
    pub observed_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsConfiguration {
    pub upstream_ipv4: Vec<String>,
    pub upstream_ipv6: Vec<String>,
    pub lan_ipv4: Vec<String>,
    pub lan_ipv6: Vec<String>,
    pub upstream_source_ipv4: ConfigurationSource,
    pub upstream_source_ipv6: ConfigurationSource,
    pub lan_source_ipv4: ConfigurationSource,
    pub lan_source_ipv6: ConfigurationSource,
    pub observed_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UssdCapability {
    pub modem_interface: Option<String>,
    pub assessment: crate::profiles::Assessment,
    pub authentication_required: bool,
    pub interactive_session: bool,
    pub restrictions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePlan {
    pub device_id: String,
    pub adapter_id: String,
    pub evidence_revision: u64,
    pub requested_change: RequestedChange,
    pub backup_required: bool,
    pub verification_available: bool,
    pub rollback_available: bool,
    pub profile_hash: String,
    pub firmware_identity: String,
    pub affected_services: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterFeatureCapability {
    pub assessment: crate::profiles::Assessment,
    pub readiness: Vec<crate::profiles::Readiness>,
    pub can_read: bool,
    pub can_write: bool,
    pub can_verify: bool,
    pub can_rollback: bool,
    pub evidence_sources: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterIdentity {
    pub manufacturer: String,
    pub model: String,
    pub hardware_revision: String,
    pub firmware_version: String,
    pub firmware_family: String,
    pub management_identity: String,
    pub operating_system: String,
    pub modem_identity: Option<String>,
    pub simulated: bool,
}

/// Connector attestation after transport trust verification, before identity can become evidence.
/// Internal only; a frontend address or successful HTTP response cannot create this value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointTrust {
    pub endpoint: String,
    pub interface_id: String,
    pub endpoint_identity: String,
    pub simulated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IpFamily {
    Ipv4,
    Ipv6,
    DualStack,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApnAuthentication {
    None,
    Pap,
    Chap,
    PapOrChap,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "feature", content = "configuration", rename_all = "snake_case")]
pub enum RequestedChange {
    Dns(DnsConfiguration),
    Apn(ApnConfiguration),
    WifiLimit(WifiClientLimitConfiguration),
}
