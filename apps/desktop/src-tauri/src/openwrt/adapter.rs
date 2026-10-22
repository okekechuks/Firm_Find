//! The existing OpenWrt contract now uses a single-use backend-prepared, pinned SSH target.
use super::{
    feasibility,
    query::{self, Board, Network, ReadQuery, Wireless},
    transport::SshTarget,
};
use crate::adapters::*;
use serde::Serialize;
use std::sync::Mutex;

#[derive(Default)]
pub struct OpenWrtAdapter {
    prepared: Mutex<Option<(SessionBinding, SshTarget)>>,
}
impl OpenWrtAdapter {
    pub(crate) fn prepared(binding: SessionBinding, target: SshTarget) -> Self {
        Self {
            prepared: Mutex::new(Some((binding, target))),
        }
    }
}
impl DeviceAdapter for OpenWrtAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        AdapterDescriptor {
            id: "openwrt-readonly-contract".into(), name: "OpenWrt pinned SSH inspection".into(), version: "0.2.0".into(),
            implementation_status: AdapterImplementationStatus::BuiltIn,
            transports: vec![Transport::Network], authentication: AuthenticationMethod::SshKey,
            operations: vec![AdapterOperation::AuthorizedIdentityProbe, AdapterOperation::InspectCapabilities, AdapterOperation::ReadConfiguration],
            dependencies: vec!["Qualified Git for Windows OpenSSH; existing unencrypted SSH key access".into()],
            limitations: vec!["Identity-only bootstrap until a unique exact profile matches; separate approval required for reads.".into(), "No writes, extraction, flashing, package installation or live USSD.".into()],
            profile_ids: vec![], firmware_families: vec!["OpenWrt".into()],
        }
    }
    fn connector(&self) -> Option<&dyn ReadOnlyConnectionAdapter> {
        Some(self)
    }
}
impl ReadOnlyConnectionAdapter for OpenWrtAdapter {
    fn connect(
        &self,
        binding: &SessionBinding,
        _: &Secret,
        control: &OperationControl,
    ) -> Result<Box<dyn AdapterConnection>, AdapterError> {
        let (expected, target) = self
            .prepared
            .lock()
            .map_err(|_| AdapterError::internal("prepared key"))?
            .take()
            .ok_or_else(|| denied("Approve a new selected endpoint and key before connecting."))?;
        if expected != *binding || binding.simulated {
            return Err(denied("Prepared endpoint authorization does not match."));
        }
        let observed = target.observation();
        let actual_endpoint = match observed.address() {
            std::net::IpAddr::V4(address) => format!("ssh://{address}:{}", observed.port()),
            std::net::IpAddr::V6(address) => format!("ssh://[{address}]:{}", observed.port()),
        };
        let actual_peer_scope = format!(
            "{}|source={}",
            observed.fingerprint(),
            target.source_address()
        );
        if actual_endpoint != binding.endpoint || actual_peer_scope != binding.endpoint_identity {
            return Err(denied(
                "Prepared SSH peer differs from the approved endpoint or pin.",
            ));
        }
        control.checkpoint()?;
        // This fixed query simultaneously authenticates and retrieves the sole bootstrap data.
        let board =
            Board::parse(&target.query(ReadQuery::Board, &control.stage(Stage::Authentication))?)?;
        Ok(Box::new(Connection {
            target,
            binding: binding.clone(),
            board,
            closed: false,
        }))
    }
}
pub(crate) fn denied(message: &str) -> AdapterError {
    AdapterError::new(
        AdapterErrorCode::OperationNotAuthorized,
        message,
        None,
        false,
        "Select the current target and explicitly approve its next read-only step.",
    )
}
#[derive(Debug, Clone, Serialize)]
pub struct UssdAssessment {
    pub assessment: &'static str,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    pub networks: Vec<Network>,
    pub wireless: Vec<Wireless>,
    pub runtime_dns: Option<Vec<query::RuntimeDns>>,
    pub dhcp_dns: Option<Vec<query::DhcpDns>>,
    pub radios: Option<Vec<query::Radio>>,
    pub wireless_runtime: Option<Vec<query::WirelessRuntime>>,
    pub limitations: Vec<String>,
    pub ussd: UssdAssessment,
    pub firmware: feasibility::FirmwareFeasibilityReport,
}
struct Connection {
    target: SshTarget,
    binding: SessionBinding,
    board: Board,
    closed: bool,
}
impl AdapterConnection for Connection {
    fn endpoint_trust(&self) -> Option<EndpointTrust> {
        (!self.closed).then(|| EndpointTrust {
            endpoint: self.binding.endpoint.clone(),
            interface_id: self.binding.interface_id.clone(),
            endpoint_identity: self.binding.endpoint_identity.clone(),
            simulated: false,
        })
    }
    fn read_identity(
        &mut self,
        control: &OperationControl,
    ) -> Result<AdapterIdentity, AdapterError> {
        control.checkpoint()?;
        if self.closed {
            return Err(denied("The SSH inspection session is closed."));
        }
        Ok(AdapterIdentity {
            manufacturer: String::new(),
            hardware_revision: String::new(),
            model: self.board.model.clone(),
            firmware_version: self.board.release.version.clone(),
            firmware_family: self.board.release.distribution.clone(),
            management_identity: management_identity(&self.board),
            operating_system: "OpenWrt".into(),
            modem_identity: None,
            simulated: false,
        })
    }
    fn probe_capabilities(
        &mut self,
        control: &OperationControl,
    ) -> Result<Vec<AdapterCapability>, AdapterError> {
        control.checkpoint()?;
        Ok(vec![AdapterCapability {
            operation: AdapterOperation::AuthorizedIdentityProbe, exists: !self.closed, can_read: !self.closed,
            can_plan: false, can_apply: false, can_verify: false, can_restore: false, authentication_required: true,
            reason: "Pinned SSH authentication returned an OpenWrt board response. Configuration queries need their own exact profile and approval.".into(),
        }, AdapterCapability {
            operation: AdapterOperation::ReadConfiguration, exists: false, can_read: false,
            can_plan: false, can_apply: false, can_verify: false, can_restore: false, authentication_required: true,
            reason: "UCI, DNS, radio and modem query availability is established only by the separately authorized inspection result.".into(),
        }])
    }
    fn inspect_openwrt(&mut self, control: &OperationControl) -> Result<Inspection, AdapterError> {
        if self.closed {
            return Err(denied("The SSH inspection session is closed."));
        }
        // One pinned SSH login, fixed framed queries, identity checked before and after.
        let mut replies = self.target.inspection(control)?.into_iter();
        let mut next = || replies.next().ok_or_else(query::invalid)?;
        let before = Board::parse(&next()?)?;
        if before != self.board {
            return Err(AdapterError::new(
                AdapterErrorCode::IdentityConflict,
                "Authenticated board/build identity changed; reconnect.",
                None,
                false,
                "Reconfirm endpoint trust and current device identity.",
            ));
        }
        let networks = query::networks(&next()?)?;
        let wireless = query::wireless(&next()?)?;
        let mut limitations = vec![];
        let runtime_dns = optional_query(
            next(),
            ReadQuery::RuntimeDns,
            query::runtime_dns,
            &mut limitations,
        )?;
        let dhcp_dns = optional_query(
            next(),
            ReadQuery::DhcpDns,
            query::dhcp_dns,
            &mut limitations,
        )?;
        let radios = optional_query(
            next(),
            ReadQuery::WirelessRadios,
            query::radios,
            &mut limitations,
        )?;
        let wireless_runtime = optional_query(
            next(),
            ReadQuery::WirelessRuntime,
            query::wireless_runtime,
            &mut limitations,
        )?;
        let partitions = optional_query(
            next(),
            ReadQuery::Partitions,
            query::partitions,
            &mut limitations,
        )?;
        let after = Board::parse(&next()?)?;
        if after != before {
            return Err(AdapterError::new(
                AdapterErrorCode::IdentityConflict,
                "Device identity changed during inspection.",
                None,
                false,
                "Reconnect and repeat the review.",
            ));
        }
        Ok(Inspection { networks, wireless, runtime_dns, dhcp_dns, radios, wireless_runtime, limitations, ussd: UssdAssessment { assessment: "UNKNOWN", reason: "No qualified modem interface or USSD capability evidence. No live request was sent.".into() }, firmware: feasibility::evaluate(before, partitions, crate::connection::now()) })
    }
    fn close(&mut self, _: &OperationControl) -> Result<(), AdapterError> {
        self.closed = true;
        Ok(())
    }
}
fn optional_query<T>(
    reply: Result<Vec<u8>, AdapterError>,
    query: ReadQuery,
    parse: impl FnOnce(&[u8]) -> Result<T, AdapterError>,
    limitations: &mut Vec<String>,
) -> Result<Option<T>, AdapterError> {
    match reply.and_then(|bytes| parse(&bytes)) {
        Ok(value) => Ok(Some(value)),
        Err(error)
            if matches!(
                error.code,
                AdapterErrorCode::TransportError | AdapterErrorCode::IdentityUnverified
            ) =>
        {
            limitations.push(format!(
                "{query:?} metadata unavailable; capability remains unknown."
            ));
            Ok(None)
        }
        Err(error) => Err(error),
    }
}
pub fn management_identity(board: &Board) -> String {
    format!(
        "{}|{}|{}",
        board.board_name, board.release.target, board.release.revision
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn optional_parse_failures_stay_local_but_trust_failures_propagate() {
        let mut limitations = vec![];
        let value = optional_query(Ok(br#"[{"name":"lan","interface":"lan","ipv4_options":[],"ipv6_dns":[],"ra":"future-mode","dhcpv6":true}]"#.to_vec()), ReadQuery::DhcpDns, query::dhcp_dns, &mut limitations).unwrap();
        assert!(value.is_none());
        assert_eq!(limitations.len(), 1);
        let value = optional_query(
            Ok(b"[]".to_vec()),
            ReadQuery::WirelessRadios,
            query::radios,
            &mut limitations,
        )
        .unwrap();
        assert_eq!(value, Some(vec![]));
        for code in [
            AdapterErrorCode::AuthenticationFailed,
            AdapterErrorCode::EndpointUntrusted,
            AdapterErrorCode::IdentityConflict,
            AdapterErrorCode::OperationCancelled,
            AdapterErrorCode::OperationTimeout,
        ] {
            assert_eq!(
                optional_query::<()>(
                    Err(AdapterError::new(code, "test", None, false, "retry")),
                    ReadQuery::DhcpDns,
                    |_| Ok(()),
                    &mut limitations
                )
                .unwrap_err()
                .code,
                code
            );
        }
        assert!(Board::parse(b"malformed identity").is_err());
    }
    #[test]
    fn production_descriptor_extends_one_contract_and_has_no_write_scope() {
        let registry = Registry::production().unwrap();
        let descriptors = registry.descriptors();
        assert_eq!(descriptors.len(), 1);
        assert_eq!(descriptors[0].id, "openwrt-readonly-contract");
        assert_eq!(
            descriptors[0].implementation_status,
            AdapterImplementationStatus::BuiltIn
        );
        assert!(descriptors[0]
            .operations
            .iter()
            .all(|op| !op.is_physical_write()));
        assert!(OpenWrtAdapter::default().prepared.lock().unwrap().is_none());
    }
}
