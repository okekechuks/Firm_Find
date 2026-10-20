//! Deterministic contract fixture. Never compiled into either native executable.
use super::registry::failure;
use super::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Barrier,
};
use std::time::Duration;

#[derive(Clone, Copy)]
pub enum Fault {
    None,
    ConnectWait,
    AuthWait,
    IdentityWait,
    CapabilitiesWait,
    CloseWait,
    Transport,
    IdentityMismatch,
    EndpointSubstitution,
    ManagementIdentityMismatch,
    MissingTrust,
}
#[derive(Clone)]
pub struct MockAdapter {
    pub descriptor: AdapterDescriptor,
    pub fault: Fault,
    pub gate: Option<Arc<Barrier>>,
    pub resources: Arc<AtomicUsize>,
}
impl MockAdapter {
    pub fn new(id: &str) -> Self {
        Self {
            descriptor: AdapterDescriptor {
                id: id.into(),
                name: "SIMULATED Phase 5 contract fixture".into(),
                version: "1.0.0".into(),
                implementation_status: AdapterImplementationStatus::TestOnly,
                transports: vec![Transport::Network],
                authentication: AuthenticationMethod::Password,
                operations: vec![
                    AdapterOperation::AuthorizedIdentityProbe,
                    AdapterOperation::InspectCapabilities,
                ],
                dependencies: vec![],
                limitations: vec!["Test fixture only; no physical IO.".into()],
                profile_ids: vec![],
                firmware_families: vec!["OpenWrt".into()],
            },
            fault: Fault::None,
            gate: None,
            resources: Arc::new(AtomicUsize::new(0)),
        }
    }
    fn wait(&self, control: &OperationControl) -> Result<(), AdapterError> {
        if let Some(gate) = &self.gate {
            gate.wait();
        }
        loop {
            control.checkpoint()?;
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}
impl DeviceAdapter for MockAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        self.descriptor.clone()
    }
    fn connector(&self) -> Option<&dyn ReadOnlyConnectionAdapter> {
        Some(self)
    }
}
struct Resource(Arc<AtomicUsize>);
impl Drop for Resource {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
struct MockConnection {
    fixture: MockAdapter,
    resource: Option<Resource>,
    trust: EndpointTrust,
}
impl ReadOnlyConnectionAdapter for MockAdapter {
    fn connect(
        &self,
        binding: &SessionBinding,
        secret: &Secret,
        control: &OperationControl,
    ) -> Result<Box<dyn AdapterConnection>, AdapterError> {
        self.resources.fetch_add(1, Ordering::AcqRel);
        let resource = Resource(Arc::clone(&self.resources));
        control.checkpoint()?;
        if matches!(self.fault, Fault::ConnectWait) {
            self.wait(control)?;
        }
        if !binding.simulated
            || binding.endpoint_identity != "FIXTURE-PIN"
            || binding.endpoint != "fixture://trusted-endpoint"
        {
            return Err(failure(
                AdapterErrorCode::EndpointUntrusted,
                "Fixture endpoint fingerprint mismatch.",
            ));
        }
        let auth = control.stage(Stage::Authentication);
        auth.checkpoint()?;
        if matches!(self.fault, Fault::AuthWait) {
            self.wait(&auth)?;
        }
        if secret.expose() != b"test-only-credential" {
            return Err(failure(
                AdapterErrorCode::AuthenticationFailed,
                "Authentication failed; credential omitted.",
            ));
        }
        Ok(Box::new(MockConnection {
            fixture: self.clone(),
            resource: Some(resource),
            trust: EndpointTrust {
                endpoint: if matches!(self.fault, Fault::EndpointSubstitution) {
                    "fixture://substituted-peer".into()
                } else {
                    binding.endpoint.clone()
                },
                interface_id: binding.interface_id.clone(),
                endpoint_identity: binding.endpoint_identity.clone(),
                simulated: true,
            },
        }))
    }
}
impl AdapterConnection for MockConnection {
    fn endpoint_trust(&self) -> Option<EndpointTrust> {
        if matches!(self.fixture.fault, Fault::MissingTrust) {
            None
        } else {
            Some(self.trust.clone())
        }
    }
    fn read_identity(&mut self, c: &OperationControl) -> Result<AdapterIdentity, AdapterError> {
        c.checkpoint()?;
        if matches!(self.fixture.fault, Fault::IdentityWait) {
            self.fixture.wait(c)?;
        }
        if matches!(self.fixture.fault, Fault::Transport) {
            return Err(failure(
                AdapterErrorCode::TransportError,
                "Fixture transport closed.",
            ));
        }
        Ok(AdapterIdentity {
            manufacturer: "TP-Link".into(),
            model: "Archer C7".into(),
            hardware_revision: "v5".into(),
            firmware_version: "23.05.5".into(),
            firmware_family: if matches!(self.fixture.fault, Fault::IdentityMismatch) {
                "Other firmware"
            } else {
                "OpenWrt"
            }
            .into(),
            operating_system: "SIMULATED OpenWrt".into(),
            management_identity: if matches!(self.fixture.fault, Fault::ManagementIdentityMismatch)
            {
                "different,board".into()
            } else {
                "tplink,archer-c7-v5".into()
            },
            modem_identity: None,
            simulated: true,
        })
    }
    fn probe_capabilities(
        &mut self,
        c: &OperationControl,
    ) -> Result<Vec<AdapterCapability>, AdapterError> {
        c.checkpoint()?;
        if matches!(self.fixture.fault, Fault::CapabilitiesWait) {
            self.fixture.wait(c)?;
        }
        Ok(vec![AdapterCapability {
            operation: AdapterOperation::AuthorizedIdentityProbe,
            exists: true,
            can_read: true,
            can_plan: false,
            can_apply: false,
            can_verify: false,
            can_restore: false,
            authentication_required: true,
            reason: "Authenticated simulated identity read only.".into(),
        }])
    }
    fn close(&mut self, c: &OperationControl) -> Result<(), AdapterError> {
        if matches!(self.fixture.fault, Fault::CloseWait) {
            self.fixture.wait(c)?;
        }
        c.checkpoint()?;
        self.resource.take();
        Ok(())
    }
}
