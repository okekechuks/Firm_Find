use super::*;
use crate::{discovery::Device, profiles::CompatibilityReport};

pub struct DeviceContext<'a> {
    pub device: &'a Device,
    pub profile: &'a CompatibilityReport,
    pub profiles: &'a crate::profiles::Registry,
    pub revision: u64,
    pub now: u64,
}

/// Immutable registration data. Registry independently checks profile evidence.
pub trait DeviceAdapter: Send + Sync {
    fn descriptor(&self) -> AdapterDescriptor;
    fn connector(&self) -> Option<&dyn ReadOnlyConnectionAdapter> {
        None
    }
}

/// No production transport implements this in Phase 5. Implementations must validate endpoint
/// trust (including exact interface and peer pin) BEFORE releasing credentials, refuse redirects
/// or interface fallback, authenticate, and check control between bounded IO calls. A bootstrap
/// binding's profile fields are expectations, never proof of identity. This method permits only
/// transport establishment/authentication; identity is the sole subsequent bootstrap request.
/// Credentials never enter command arguments.
pub trait ReadOnlyConnectionAdapter: Send + Sync {
    fn connect(
        &self,
        binding: &SessionBinding,
        secret: &Secret,
        control: &OperationControl,
    ) -> Result<Box<dyn AdapterConnection>, AdapterError>;
}

pub trait AdapterConnection: Send {
    fn inspect_openwrt(
        &mut self,
        _control: &OperationControl,
    ) -> Result<crate::openwrt::adapter::Inspection, AdapterError> {
        Err(unsupported(AdapterOperation::ReadConfiguration))
    }
    /// No attestation means bootstrap cannot establish a trusted session.
    fn endpoint_trust(&self) -> Option<EndpointTrust> {
        None
    }
    fn read_identity(
        &mut self,
        control: &OperationControl,
    ) -> Result<AdapterIdentity, AdapterError>;
    fn probe_capabilities(
        &mut self,
        control: &OperationControl,
    ) -> Result<Vec<AdapterCapability>, AdapterError>;
    fn close(&mut self, control: &OperationControl) -> Result<(), AdapterError>;
    fn dns(&mut self) -> Option<&mut dyn DnsReader> {
        None
    }
    fn apn(&mut self) -> Option<&mut dyn ApnReader> {
        None
    }
    fn wifi(&mut self) -> Option<&mut dyn WifiClientLimitReader> {
        None
    }
    fn ussd(&mut self) -> Option<&mut dyn UssdReader> {
        None
    }
}

pub trait ApnReader {
    fn read_apn(&mut self, control: &OperationControl) -> Result<ApnConfiguration, AdapterError>;
}
pub trait WifiClientLimitReader {
    fn read_wifi_configurations(
        &mut self,
        control: &OperationControl,
    ) -> Result<Vec<WifiClientLimitConfiguration>, AdapterError> {
        self.read_wifi_client_limit(control).map(|v| vec![v])
    }
    fn read_wifi_client_limit(
        &mut self,
        control: &OperationControl,
    ) -> Result<WifiClientLimitConfiguration, AdapterError>;
}
pub trait DnsReader {
    fn read_dns(&mut self, control: &OperationControl) -> Result<DnsConfiguration, AdapterError>;
}
pub trait UssdReader {
    fn read_ussd_capability(
        &mut self,
        control: &OperationControl,
    ) -> Result<UssdCapability, AdapterError>;
}

/// Optional future interfaces. No placeholder returns successful application or recovery.
pub trait ChangePlanner: DeviceAdapter {
    fn validate_and_plan(&self, plan: ChangePlan) -> Result<ChangePlan, AdapterError>;
}
pub trait ConfigurationBackup: DeviceAdapter {
    fn backup(&self, binding: &SessionBinding) -> Result<BackupReference, AdapterError>;
}
pub trait PlannedConfigurationAdapter: DeviceAdapter {
    fn apply_change(&self, plan: &ChangePlan, backup: &BackupReference)
        -> Result<(), AdapterError>;
    fn verify_change(&self, plan: &ChangePlan) -> Result<(), AdapterError>;
    fn restore_change(
        &self,
        plan: &ChangePlan,
        backup: &BackupReference,
    ) -> Result<(), AdapterError>;
}
pub struct BackupReference {
    pub id: String,
    pub binding: SessionBinding,
    pub verified: bool,
}
pub trait UssdSessionAdapter: DeviceAdapter {
    fn start(&self, binding: &SessionBinding, request: UssdRequest)
        -> Result<String, AdapterError>;
    fn reply(&self, session: &str, response: &str) -> Result<(), AdapterError>;
    fn cancel(&self, session: &str) -> Result<(), AdapterError>;
}
pub struct UssdRequest {
    pub code: String,
    pub interactive: bool,
}
