use crate::adapters::*;
use serde::{Deserialize, Serialize};

pub const DEVICE_ID: &str = "sim-router-001";
pub const PROFILE_ID: &str = "obtainer.virtual-router-lab";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scenario {
    Healthy,
    AuthenticationFailure,
    ConnectionTimeout,
    SlowAuthentication,
    SlowRead,
    WrongIdentity,
    EndpointSubstitution,
    RouterDisconnected,
    DnsApplyFailure,
    DnsVerificationMismatch,
    DnsServiceFailure,
    ResolutionFailure,
    RollbackFailure,
    ExternalRevision,
    WifiReloadInterruption,
    WifiCapacity,
    SimMissing,
    SimPinRequired,
    UnknownOperator,
    MissingApn,
    ModemAbsent,
    RegistrationFailure,
    DataConnectionFailure,
    CellularRestart,
    Ipv4Only,
    UnsupportedModem,
    UssdTimeout,
    UssdNetworkFailure,
}
pub const SCENARIOS: &[Scenario] = &[
    Scenario::Healthy,
    Scenario::AuthenticationFailure,
    Scenario::ConnectionTimeout,
    Scenario::SlowAuthentication,
    Scenario::SlowRead,
    Scenario::WrongIdentity,
    Scenario::EndpointSubstitution,
    Scenario::RouterDisconnected,
    Scenario::DnsApplyFailure,
    Scenario::DnsVerificationMismatch,
    Scenario::DnsServiceFailure,
    Scenario::ResolutionFailure,
    Scenario::RollbackFailure,
    Scenario::ExternalRevision,
    Scenario::WifiReloadInterruption,
    Scenario::WifiCapacity,
    Scenario::SimMissing,
    Scenario::SimPinRequired,
    Scenario::UnknownOperator,
    Scenario::MissingApn,
    Scenario::ModemAbsent,
    Scenario::RegistrationFailure,
    Scenario::DataConnectionFailure,
    Scenario::CellularRestart,
    Scenario::Ipv4Only,
    Scenario::UnsupportedModem,
    Scenario::UssdTimeout,
    Scenario::UssdNetworkFailure,
];
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultStage {
    Connect,
    Authenticate,
    Read,
    Apply,
    Verify,
    Resolve,
    Restore,
    Wifi,
    Ussd,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FaultPlan {
    pub stage: FaultStage,
    pub remaining: u8,
    pub repeat: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    Disabled,
    NotFound,
    Unauthorized,
    AuthenticationFailed,
    EndpointUntrusted,
    IdentityConflict,
    Expired,
    Stale,
    Busy,
    Cancelled,
    Timeout,
    Offline,
    Unsupported,
    InvalidInput,
    ApplyFailed,
    VerificationFailed,
    RestoreFailed,
    RecoveryRequired,
    SimMissing,
    SimPinRequired,
    ModemAbsent,
    UnknownOperator,
    MissingApn,
    RegistrationFailed,
    DataConnectionFailed,
    Internal,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimError {
    pub code: ErrorCode,
    pub message: String,
    pub retryable: bool,
}
pub type Result<T> = std::result::Result<T, SimError>;
pub fn fail(code: ErrorCode, message: &str) -> SimError {
    SimError {
        code,
        message: message.into(),
        retryable: true,
    }
}
impl From<AdapterError> for SimError {
    fn from(e: AdapterError) -> Self {
        let code = match e.code {
            AdapterErrorCode::AuthenticationFailed => ErrorCode::AuthenticationFailed,
            AdapterErrorCode::EndpointUntrusted => ErrorCode::EndpointUntrusted,
            AdapterErrorCode::IdentityConflict | AdapterErrorCode::IdentityUnverified => {
                ErrorCode::IdentityConflict
            }
            AdapterErrorCode::SessionExpired => ErrorCode::Expired,
            AdapterErrorCode::OperationCancelled => ErrorCode::Cancelled,
            AdapterErrorCode::ConnectionTimeout | AdapterErrorCode::OperationTimeout => {
                ErrorCode::Timeout
            }
            AdapterErrorCode::DeviceDisconnected | AdapterErrorCode::TransportError => {
                ErrorCode::Offline
            }
            AdapterErrorCode::OperationUnsupported => ErrorCode::Unsupported,
            _ => ErrorCode::Unauthorized,
        };
        fail(code, &e.message)
    }
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VirtualDevice {
    pub id: String,
    pub manufacturer: String,
    pub model: String,
    pub hardware_revision: String,
    pub firmware_family: String,
    pub firmware_version: String,
    pub operating_system: String,
    pub endpoint: String,
    pub interface_id: String,
    pub peer_pin: String,
    pub simulated: bool,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Modem {
    pub present: bool,
    pub sim_present: bool,
    pub pin_required: bool,
    pub registered: bool,
    pub operator: Option<String>,
    pub data_connected: bool,
    pub ipv6_supported: bool,
    pub service: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureRead<T> {
    pub value: Option<T>,
    pub error: Option<SimError>,
}
impl<T> FeatureRead<T> {
    pub fn from_result(r: Result<T>) -> Self {
        match r {
            Ok(v) => Self {
                value: Some(v),
                error: None,
            },
            Err(e) => Self {
                value: None,
                error: Some(e),
            },
        }
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Features {
    pub dns_service: String,
    pub wifi_service: String,
    pub dns: FeatureRead<DnsConfiguration>,
    pub wifi: FeatureRead<Vec<WifiClientLimitConfiguration>>,
    pub apn: FeatureRead<ApnConfiguration>,
    pub ussd: FeatureRead<UssdCapability>,
    pub modem: Modem,
    pub observed_revision: u64,
    pub observed_at: u64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Difference {
    pub field: String,
    pub before: Vec<String>,
    pub after: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsPreview {
    pub id: String,
    pub target: String,
    pub epoch: u64,
    pub base_revision: u64,
    pub previous: DnsConfiguration,
    pub proposed: DnsConfiguration,
    pub changes: Vec<Difference>,
    pub affected_services: Vec<String>,
    pub snapshot_required: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Rollback {
    NotRequested,
    Succeeded,
    Failed,
    Cancelled,
    Unavailable,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsOutcome {
    pub operation: String,
    pub accepted: bool,
    pub applied: bool,
    pub configuration_matches: bool,
    pub service_healthy: bool,
    pub resolution_successful: bool,
    pub verified: bool,
    pub rollback: Rollback,
    pub requires_reset: bool,
    pub message: String,
    pub snapshot_id: Option<String>,
    pub revision: u64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotInfo {
    pub scenario: Scenario,
    pub id: String,
    pub target: String,
    pub firmware_version: String,
    pub revision: u64,
    pub virtual_time: u64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UssdSession {
    pub id: String,
    pub state: String,
    pub text: String,
    pub expires_at: u64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub time: u64,
    pub message: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub active_connections: usize,
    pub available: bool,
    pub active: bool,
    pub device: Option<VirtualDevice>,
    pub epoch: u64,
    pub revision: u64,
    pub virtual_time: u64,
    pub scenario: Scenario,
    pub scenarios: Vec<Scenario>,
    pub faults: Vec<FaultPlan>,
    pub connection: ConnectionState,
    pub authentication: AuthenticationState,
    pub approval_id: Option<String>,
    pub read_authorized: bool,
    pub features: Option<Features>,
    pub preview: Option<DnsPreview>,
    pub outcome: Option<DnsOutcome>,
    pub snapshots: Vec<SnapshotInfo>,
    pub ussd: Option<UssdSession>,
    pub events: Vec<Event>,
    pub busy: bool,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Control {
    Start,
    Stop,
    Reset,
    Cancel,
    Scenario { scenario: Scenario, repeat: bool },
    Advance { milliseconds: u64 },
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub endpoint: String,
    pub target: String,
    pub epoch: u64,
    pub revision: u64,
    pub action: Action,
}
#[derive(Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Action {
    ApproveHandshake,
    Connect {
        approval_id: String,
        credential: String,
    },
    ApproveReads,
    ReadFeatures,
    Disconnect,
    PreviewDns {
        configuration: DnsConfiguration,
    },
    CancelDraft,
    ApplyDns {
        preview_id: String,
    },
    VerifyDns,
    Restore {
        snapshot_id: String,
    },
    SetWifi {
        radio: String,
        limit: u32,
    },
    SetApn {
        source: ConfigurationSource,
        apn: String,
        authentication: ApnAuthentication,
        ip_family: IpFamily,
    },
    UssdStart {
        code: String,
    },
    UssdReply {
        session_id: String,
        text: String,
    },
    UssdCancel {
        session_id: String,
    },
}
