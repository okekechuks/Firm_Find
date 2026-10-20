use super::{model::Router, types::*, Authority};
use crate::adapters::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

#[derive(Clone)]
pub(super) struct SimulatorAdapter {
    pub state: Arc<Mutex<Router>>,
    pub authority: Authority,
    pub resources: Arc<AtomicUsize>,
}
fn adapter_error(code: AdapterErrorCode, message: &str) -> AdapterError {
    AdapterError::new(
        code,
        message,
        None,
        true,
        "Review the virtual scenario and reconnect if required.",
    )
}
impl SimulatorAdapter {
    fn gate(
        &self,
        stage: FaultStage,
        control: &OperationControl,
    ) -> std::result::Result<(), AdapterError> {
        control.checkpoint()?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| AdapterError::internal("simulator lock"))?;
        if !self.authority.enabled.load(Ordering::Acquire) || !state.available {
            return Err(adapter_error(
                AdapterErrorCode::DeviceDisconnected,
                "Virtual router is offline.",
            ));
        }
        let delay = state.fault(stage);
        drop(state);
        if delay {
            loop {
                control.checkpoint()?;
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        Ok(())
    }
}
impl DeviceAdapter for SimulatorAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        AdapterDescriptor {
            id: "obtainer-simulator".into(),
            name: "SIMULATION: Virtual Router Lab".into(),
            version: "1.0.0".into(),
            implementation_status: AdapterImplementationStatus::Simulator,
            transports: vec![Transport::Simulation],
            authentication: AuthenticationMethod::Password,
            operations: vec![
                AdapterOperation::AuthorizedIdentityProbe,
                AdapterOperation::InspectCapabilities,
                AdapterOperation::ReadDns,
                AdapterOperation::ReadWifiLimit,
                AdapterOperation::ReadApn,
                AdapterOperation::ReadUssdCapability,
            ],
            dependencies: vec![],
            limitations: vec![
                "Virtual state only. No physical transport or write implementation.".into(),
            ],
            profile_ids: vec![PROFILE_ID.into()],
            firmware_families: vec!["Simulated OpenWrt-like".into()],
        }
    }
    fn connector(&self) -> Option<&dyn ReadOnlyConnectionAdapter> {
        Some(self)
    }
}
impl ReadOnlyConnectionAdapter for SimulatorAdapter {
    fn connect(
        &self,
        b: &SessionBinding,
        secret: &Secret,
        c: &OperationControl,
    ) -> std::result::Result<Box<dyn AdapterConnection>, AdapterError> {
        self.gate(FaultStage::Connect, c)?;
        if !b.simulated
            || b.endpoint != self.authority.endpoint
            || b.endpoint_identity != self.authority.pin
            || b.interface_id != "sim-interface-001"
            || b.discovery_revision != self.authority.epoch.load(Ordering::Acquire)
            || self
                .state
                .lock()
                .map_err(|_| AdapterError::internal("simulator lock"))?
                .scenario
                == Scenario::EndpointSubstitution
        {
            return Err(adapter_error(
                AdapterErrorCode::EndpointUntrusted,
                "Virtual endpoint/interface/pin differs from the approved runtime.",
            ));
        }
        self.gate(FaultStage::Authenticate, &c.stage(Stage::Authentication))?;
        if secret.expose() != b"lab-only"
            || self
                .state
                .lock()
                .map_err(|_| AdapterError::internal("simulator lock"))?
                .scenario
                == Scenario::AuthenticationFailure
        {
            return Err(adapter_error(
                AdapterErrorCode::AuthenticationFailed,
                "Virtual authentication failed; credentials omitted.",
            ));
        }
        self.resources.fetch_add(1, Ordering::AcqRel);
        Ok(Box::new(Connection {
            adapter: self.clone(),
            resource: Some(Resource(self.resources.clone())),
            binding: b.clone(),
        }))
    }
}
struct Resource(Arc<AtomicUsize>);
impl Drop for Resource {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
struct Connection {
    adapter: SimulatorAdapter,
    resource: Option<Resource>,
    binding: SessionBinding,
}
impl Connection {
    fn read(
        &self,
        c: &OperationControl,
    ) -> std::result::Result<std::sync::MutexGuard<'_, Router>, AdapterError> {
        self.adapter.gate(FaultStage::Read, c)?;
        if self.binding.discovery_revision != self.adapter.authority.epoch.load(Ordering::Acquire) {
            return Err(adapter_error(
                AdapterErrorCode::SessionExpired,
                "Virtual session generation expired.",
            ));
        }
        self.adapter
            .state
            .lock()
            .map_err(|_| AdapterError::internal("simulator lock"))
    }
}
impl AdapterConnection for Connection {
    fn endpoint_trust(&self) -> Option<EndpointTrust> {
        Some(EndpointTrust {
            endpoint: self.binding.endpoint.clone(),
            interface_id: self.binding.interface_id.clone(),
            endpoint_identity: self.binding.endpoint_identity.clone(),
            simulated: true,
        })
    }
    fn read_identity(
        &mut self,
        c: &OperationControl,
    ) -> std::result::Result<AdapterIdentity, AdapterError> {
        let state = self.read(c)?;
        Ok(state.configuration.identity.clone())
    }
    fn probe_capabilities(
        &mut self,
        c: &OperationControl,
    ) -> std::result::Result<Vec<AdapterCapability>, AdapterError> {
        drop(self.read(c)?);
        Ok(self
            .adapter
            .descriptor()
            .operations
            .into_iter()
            .map(|operation| AdapterCapability {
                operation,
                exists: true,
                can_read: true,
                can_plan: false,
                can_apply: false,
                can_verify: false,
                can_restore: false,
                authentication_required: true,
                reason:
                    "SIMULATION: state-backed read; mutations use a separate virtual-only boundary."
                        .into(),
            })
            .collect())
    }
    fn close(&mut self, c: &OperationControl) -> std::result::Result<(), AdapterError> {
        c.checkpoint()?;
        self.resource.take();
        Ok(())
    }
    fn dns(&mut self) -> Option<&mut dyn DnsReader> {
        Some(self)
    }
    fn wifi(&mut self) -> Option<&mut dyn WifiClientLimitReader> {
        Some(self)
    }
    fn apn(&mut self) -> Option<&mut dyn ApnReader> {
        Some(self)
    }
    fn ussd(&mut self) -> Option<&mut dyn UssdReader> {
        Some(self)
    }
}
impl DnsReader for Connection {
    fn read_dns(
        &mut self,
        c: &OperationControl,
    ) -> std::result::Result<DnsConfiguration, AdapterError> {
        Ok(self.read(c)?.configuration.dns.clone())
    }
}
impl WifiClientLimitReader for Connection {
    fn read_wifi_client_limit(
        &mut self,
        c: &OperationControl,
    ) -> std::result::Result<WifiClientLimitConfiguration, AdapterError> {
        Ok(self.read(c)?.configuration.wifi[0].clone())
    }
    fn read_wifi_configurations(
        &mut self,
        c: &OperationControl,
    ) -> std::result::Result<Vec<WifiClientLimitConfiguration>, AdapterError> {
        Ok(self.read(c)?.configuration.wifi.clone())
    }
}
impl ApnReader for Connection {
    fn read_apn(
        &mut self,
        c: &OperationControl,
    ) -> std::result::Result<ApnConfiguration, AdapterError> {
        let state = self.read(c)?;
        if let Some(error) = state.modem_error() {
            return Err(adapter_error(
                AdapterErrorCode::OperationUnsupported,
                &error.message,
            ));
        }
        Ok(state.configuration.apn.clone())
    }
}
impl UssdReader for Connection {
    fn read_ussd_capability(
        &mut self,
        c: &OperationControl,
    ) -> std::result::Result<UssdCapability, AdapterError> {
        let state = self.read(c)?;
        let supported =
            state.scenario != Scenario::UnsupportedModem && state.configuration.modem.present;
        Ok(UssdCapability {modem_interface:if supported {Some("sim-modem-001".into())}else{None},
            assessment:if supported {crate::profiles::Assessment::Supported}else{crate::profiles::Assessment::Unsupported},
            authentication_required:true,interactive_session:supported,
            restrictions:vec!["SIMULATION ONLY: scripted fictional responses; no AT commands, SIM or mobile network.".into()]})
    }
}
