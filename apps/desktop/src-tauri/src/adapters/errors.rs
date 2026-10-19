use super::types::AdapterOperation;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdapterErrorCode {
    DeviceNotFound,
    DeviceDisconnected,
    ProtectedDevice,
    AdapterNotFound,
    AdapterNotCompatible,
    IdentityUnverified,
    IdentityConflict,
    AuthorizationRequired,
    AuthenticationRequired,
    AuthenticationFailed,
    EndpointUntrusted,
    CapabilityUnknown,
    OperationUnsupported,
    OperationNotImplemented,
    OperationNotAuthorized,
    DependencyMissing,
    ConnectionTimeout,
    OperationTimeout,
    OperationCancelled,
    TransportError,
    SessionExpired,
    InternalAdapterError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterError {
    pub code: AdapterErrorCode,
    pub message: String,
    pub operation: Option<AdapterOperation>,
    pub retryable: bool,
    pub suggested_action: String,
}

impl AdapterError {
    pub fn new(
        code: AdapterErrorCode,
        message: impl Into<String>,
        operation: Option<AdapterOperation>,
        retryable: bool,
        suggested_action: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            operation,
            retryable,
            suggested_action: suggested_action.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        let _ = message.into();
        Self::new(
            AdapterErrorCode::InternalAdapterError,
            "Adapter evaluation could not be completed.",
            None,
            true,
            "Refresh discovery and try again.",
        )
    }
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AdapterError {}
