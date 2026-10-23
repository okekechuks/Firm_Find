//! Backend receipts follow the Phase 5 bound, expiring, single-use grant pattern.
use super::strategy_api::{StrategyError, StrategyId};
use super::*;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Binding {
    device: String,
    revision: u64,
    interface: String,
    endpoint: String,
    advanced: bool,
}
pub(super) fn binding(
    device: &Device,
    revision: u64,
    input: &DiscoveryRequest,
) -> Result<Binding, DiscoveryError> {
    if !device.connected || device.id != input.device_id {
        return Err(error("TARGET_MISMATCH", "Select a connected device."));
    }
    let target = connection::parse_address(input.endpoint.as_deref().unwrap_or(""))
        .map_err(|_| error("INVALID_ENDPOINT", "Select a private root address."))?;
    let network = device
        .networks
        .iter()
        .find(|n| {
            Some(n.id.as_str()) == input.interface_id.as_deref()
                && n.link_up
                && !n.virtual_interface
        })
        .ok_or_else(|| error("INTERFACE_REQUIRED", "Select an active physical interface."))?;
    let ip = target.host.parse::<std::net::IpAddr>().map_err(|_| {
        error(
            "INVALID_ENDPOINT",
            "Use the numeric management address for an approved check.",
        )
    })?;
    if !connection::on_link(ip, network)
        || (!input.advanced_endpoint
            && !network
                .gateways
                .iter()
                .any(|g| g.parse::<std::net::IpAddr>().ok() == Some(ip)))
    {
        return Err(error(
            "ENDPOINT_NOT_GATEWAY",
            "Select this interface's gateway, or explicitly approve an advanced local endpoint.",
        ));
    }
    if device.interfaces.iter().any(|i| {
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
        .any(|c| i.class_name.eq_ignore_ascii_case(c))
    }) {
        return Err(error("PROTECTED_DEVICE", "Protected hardware is excluded."));
    }
    Ok(Binding {
        device: device.id.clone(),
        revision,
        interface: network.id.clone(),
        endpoint: target.url,
        advanced: input.advanced_endpoint,
    })
}
#[derive(Default)]
pub(super) struct Approvals(Mutex<BTreeMap<String, (Binding, ApprovalScope, Instant)>>);
impl Approvals {
    pub(super) fn issue_challenge(
        &self,
        binding: Binding,
        approved: bool,
    ) -> Result<String, DiscoveryError> {
        if !approved {
            return Err(error("CANCELLED", "Authentication challenge not approved."));
        }
        self.issue_scoped(binding, challenge_scope())
    }
    pub(super) fn claim_challenge(
        &self,
        id: &str,
        binding: &Binding,
    ) -> Result<AuthChallengeApproval, DiscoveryError> {
        self.claim_scoped(id, binding, challenge_scope())
            .map(|proof| AuthChallengeApproval(proof, AtomicBool::new(false)))
    }
    pub(super) fn issue(&self, binding: Binding) -> Result<String, DiscoveryError> {
        self.issue_scoped(binding, ApprovalScope::EndpointMetadata)
    }
    pub(super) fn issue_menu(
        &self,
        binding: Binding,
        approved: bool,
    ) -> Result<String, DiscoveryError> {
        if !approved {
            return Err(error("CANCELLED", "Static menu check not approved."));
        }
        self.issue_scoped(binding, menu_scope())
    }
    fn issue_scoped(
        &self,
        binding: Binding,
        scope: ApprovalScope,
    ) -> Result<String, DiscoveryError> {
        let id = crate::library::files::nonce()
            .map_err(|_| error("INTERNAL", "Approval unavailable."))?;
        let mut receipts = self
            .0
            .lock()
            .map_err(|_| error("INTERNAL", "Approval unavailable."))?;
        receipts.retain(|_, (_, _, deadline)| *deadline > Instant::now());
        if receipts.len() >= 64 {
            return Err(error("BUSY", "Too many pending approvals."));
        }
        receipts.insert(
            id.clone(),
            (binding, scope, Instant::now() + Duration::from_secs(30)),
        );
        Ok(id)
    }
    pub(super) fn claim(
        &self,
        id: &str,
        binding: &Binding,
    ) -> Result<ClaimedApproval, DiscoveryError> {
        self.claim_scoped(id, binding, ApprovalScope::EndpointMetadata)
    }
    pub(super) fn claim_menu(
        &self,
        id: &str,
        binding: &Binding,
    ) -> Result<StaticMenuApproval, DiscoveryError> {
        self.claim_scoped(id, binding, menu_scope())
            .map(|proof| StaticMenuApproval(proof, AtomicBool::new(false)))
    }
    fn claim_scoped(
        &self,
        id: &str,
        binding: &Binding,
        required: ApprovalScope,
    ) -> Result<ClaimedApproval, DiscoveryError> {
        let (expected, scope, deadline) = self
            .0
            .lock()
            .map_err(|_| error("INTERNAL", "Approval unavailable."))?
            .remove(id)
            .ok_or_else(|| error("APPROVAL_REQUIRED", "Approve this check again."))?;
        if expected != *binding || scope != required || Instant::now() >= deadline {
            return Err(error(
                "APPROVAL_EXPIRED",
                "Approval expired or target changed. Approve again.",
            ));
        }
        Ok(ClaimedApproval {
            binding: expected,
            scope,
            deadline,
        })
    }
}

/// Authority stays operation-specific: only endpoint metadata and static menu scopes are issued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalScope {
    AuthChallenge {
        request: RequestId,
    },
    EndpointMetadata,
    StaticMenuMetadata {
        request: RequestId,
    },
    StrategyProbe {
        strategy: StrategyId,
        requests: Vec<super::request_registry::RequestId>,
    },
}

fn menu_scope() -> ApprovalScope {
    ApprovalScope::StaticMenuMetadata {
        request: RequestId::StaticMenu,
    }
}
fn challenge_scope() -> ApprovalScope {
    ApprovalScope::AuthChallenge {
        request: RequestId::Challenge,
    }
}

/// One credential-free GET only; cannot be cloned, serialized or upgraded into a session.
pub(crate) struct AuthChallengeApproval(ClaimedApproval, AtomicBool);
impl AuthChallengeApproval {
    #[cfg(test)]
    pub(super) fn for_test(device: &Device, revision: u64, input: &DiscoveryRequest) -> Self {
        let b = binding(device, revision, input).unwrap();
        let a = Approvals::default();
        let receipt = a.issue_challenge(b.clone(), true).unwrap();
        a.claim_challenge(&receipt, &b).unwrap()
    }
    pub(super) fn validate(&self, binding: &Binding) -> Result<(), DiscoveryError> {
        if self.0.scope != challenge_scope()
            || self.0.binding != *binding
            || Instant::now() >= self.0.deadline
        {
            return Err(error(
                "APPROVAL_EXPIRED",
                "Challenge approval expired or target changed.",
            ));
        }
        Ok(())
    }
    pub(crate) fn take_target(&self, target: &connection::Target, interface: &str) -> bool {
        self.0.scope == challenge_scope()
            && self.0.binding.endpoint == target.url
            && self.0.binding.interface == interface
            && Instant::now() < self.0.deadline
            && self
                .1
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
    }
    pub(crate) fn remaining(&self) -> Duration {
        self.0.deadline.saturating_duration_since(Instant::now())
    }
}
pub(super) fn challenge_confirmation(device: &Device, binding: &Binding) -> String {
    format!("Approve one credential-free authentication challenge check?\n\nDevice: {}\nInterface: {}\nEndpoint: {}\nRequest: GET /login.cgi, one attempt, maximum 5 seconds.\n\nNo username, password, Authorization header or cookies will be sent. Only a bounded realm and qop observation may be reported; the nonce and response body will not be kept. Plain HTTP is not cryptographically verified. This does not authorize login, identity reads, logout or firmware access. Continue only if you own or administer this endpoint.",confirmation_value(&device.display_name),confirmation_value(&binding.interface),confirmation_value(&binding.endpoint))
}

#[cfg(test)]
mod challenge_scope_tests {
    use super::*;
    #[test]
    fn challenge_scope_replay_cross_scope_binding_expiry_and_no() {
        let mut d = super::super::tests::device();
        d.networks[0].gateways = vec!["192.168.100.1".into()];
        let input = DiscoveryRequest {
            device_id: d.id.clone(),
            endpoint: Some("http://192.168.100.1/".into()),
            interface_id: Some(d.networks[0].id.clone()),
            approval_token: None,
            advanced_endpoint: false,
        };
        let b = binding(&d, 4, &input).unwrap();
        let a = Approvals::default();
        assert!(a.issue_challenge(b.clone(), false).is_err());
        assert!(a.0.lock().unwrap().is_empty());
        for scope in [
            ApprovalScope::EndpointMetadata,
            menu_scope(),
            ApprovalScope::StrategyProbe {
                strategy: StrategyId::OfficialImage,
                requests: vec![RequestId::Root],
            },
        ] {
            let receipt = a.issue_scoped(b.clone(), scope).unwrap();
            assert!(a.claim_challenge(&receipt, &b).is_err());
        }
        let receipt = a.issue_challenge(b.clone(), true).unwrap();
        assert!(a.claim(&receipt, &b).is_err());
        let receipt = a.issue_challenge(b.clone(), true).unwrap();
        assert!(a.claim_menu(&receipt, &b).is_err());
        let receipt = a.issue_challenge(b.clone(), true).unwrap();
        a.0.lock().unwrap().get_mut(&receipt).unwrap().1 = ApprovalScope::AuthChallenge {
            request: RequestId::Login,
        };
        assert!(a.claim_challenge(&receipt, &b).is_err());
        for field in 0..3 {
            let receipt = a.issue_challenge(b.clone(), true).unwrap();
            let mut changed = b.clone();
            match field {
                0 => changed.endpoint = "http://192.168.100.2/".into(),
                1 => changed.interface = "other".into(),
                _ => changed.revision += 1,
            };
            assert!(a.claim_challenge(&receipt, &changed).is_err());
            assert!(a.claim_challenge(&receipt, &b).is_err());
        }
        let receipt = a.issue_challenge(b.clone(), true).unwrap();
        a.0.lock().unwrap().get_mut(&receipt).unwrap().2 = Instant::now();
        assert!(a.claim_challenge(&receipt, &b).is_err());
        let receipt = a.issue_challenge(b.clone(), true).unwrap();
        let proof = a.claim_challenge(&receipt, &b).unwrap();
        assert!(a.claim_challenge(&receipt, &b).is_err());
        let target = connection::parse_address(&b.endpoint).unwrap();
        assert!(!proof.take_target(&target, "other"));
        assert!(proof.take_target(&target, &b.interface));
        assert!(!proof.take_target(&target, &b.interface));
        assert!(proof.0.validate(&b).is_err());
        assert!(proof.0.strategy_probe(StrategyId::OfficialImage).is_err());
    }
    #[test]
    fn challenge_confirmation_matches_design_and_ipc_acl_is_closed() {
        let mut d = super::super::tests::device();
        d.networks[0].gateways = vec!["192.168.100.1".into()];
        d.display_name = format!("private\n\u{202e}{}", "x".repeat(100));
        let input = DiscoveryRequest {
            device_id: d.id.clone(),
            endpoint: Some("http://192.168.100.1/".into()),
            interface_id: Some(d.networks[0].id.clone()),
            approval_token: None,
            advanced_endpoint: false,
        };
        let b = binding(&d, 4, &input).unwrap();
        let design = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../docs/PHASE_7_5_STAGE_2_AUTH_DESIGN.md"
        ))
        .replace("\r\n", "\n");
        let block = design
            .split_once("**AuthChallenge title:**")
            .unwrap()
            .1
            .split_once("```text\n")
            .unwrap()
            .1
            .split_once("\n```")
            .unwrap()
            .0;
        let expected = block
            .replace("{device_name}", &confirmation_value(&d.display_name))
            .replace("{interface_id}", &confirmation_value(&b.interface))
            .replace("{normalized_endpoint}", &confirmation_value(&b.endpoint));
        assert_eq!(challenge_confirmation(&d, &b), expected);
        assert_eq!(confirmation_value(&d.display_name).chars().count(), 80);
        assert!(!challenge_confirmation(&d, &b).contains('\u{202e}'));
        let acl = include_str!("../../capabilities/main.json");
        let ipc = include_str!("ipc.rs");
        let build = include_str!("../../build.rs");
        let handlers = include_str!("../lib.rs");
        for (command, permission) in [
            ("approve_auth_challenge", "allow-approve-auth-challenge"),
            ("read_auth_challenge", "allow-read-auth-challenge"),
        ] {
            assert!(
                acl.contains(permission)
                    && ipc.contains(command)
                    && build.contains(command)
                    && handlers.contains(command)
            );
        }
        assert!(ipc.contains("OBTAINER - approve authentication challenge"));
    }
}

/// Non-Clone, non-serializable proof, constructed only by a successful claim.
pub(crate) struct ClaimedApproval {
    binding: Binding,
    scope: ApprovalScope,
    deadline: Instant,
}
impl ClaimedApproval {
    pub(crate) fn metadata_target(&self, target: &connection::Target, interface: &str) -> bool {
        self.scope == ApprovalScope::EndpointMetadata
            && self.binding.endpoint == target.url
            && self.binding.interface == interface
            && Instant::now() < self.deadline
    }

    pub(super) fn validate(&self, binding: &Binding) -> Result<(), DiscoveryError> {
        if self.scope != ApprovalScope::EndpointMetadata
            || self.binding != *binding
            || Instant::now() >= self.deadline
        {
            return Err(error(
                "APPROVAL_EXPIRED",
                "Approval expired or target changed.",
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn scope(&self) -> ApprovalScope {
        self.scope.clone()
    }

    #[allow(dead_code)] // Strategy-probe receipts are intentionally not issued in this phase.
    pub(crate) fn strategy_probe(
        &self,
        strategy: StrategyId,
    ) -> Result<StrategyProbeApproval, StrategyError> {
        match &self.scope {
            ApprovalScope::StrategyProbe {
                strategy: bound,
                requests,
            } if *bound == strategy && !requests.is_empty() && Instant::now() < self.deadline => {
                Ok(StrategyProbeApproval {
                    strategy,
                    deadline: self.deadline,
                })
            }
            _ => Err(StrategyError::ApprovalScopeMismatch),
        }
    }
}

/// Proof can only be minted by a claimed receipt inside this module.
/// ```compile_fail,E0451
/// use obtainer_lib::acquisition::{StrategyId, StrategyProbeApproval};
/// let _ = StrategyProbeApproval { strategy: StrategyId::OfficialImage, deadline: std::time::Instant::now() };
/// ```
pub struct StrategyProbeApproval {
    strategy: StrategyId,
    deadline: Instant,
}
impl StrategyProbeApproval {
    pub(crate) fn authorizes(&self, strategy: StrategyId) -> bool {
        self.strategy == strategy && Instant::now() < self.deadline
    }
    #[cfg(test)]
    pub(crate) fn for_test(strategy: StrategyId) -> Self {
        let claimed = ClaimedApproval {
            binding: Binding {
                device: "test".into(),
                revision: 1,
                interface: "test".into(),
                endpoint: "http://127.0.0.1/".into(),
                advanced: false,
            },
            scope: ApprovalScope::StrategyProbe {
                strategy,
                requests: vec![super::RequestId::Root],
            },
            deadline: Instant::now() + Duration::from_secs(30),
        };
        claimed.strategy_probe(strategy).unwrap()
    }
}

/// Separate, non-serializable scope proof; its request is always StaticMenu.
pub(crate) struct StaticMenuApproval(ClaimedApproval, AtomicBool);
impl StaticMenuApproval {
    #[cfg(test)]
    pub(super) fn for_test(device: &Device, revision: u64, input: &DiscoveryRequest) -> Self {
        let b = binding(device, revision, input).unwrap();
        let approvals = Approvals::default();
        let receipt = approvals.issue_menu(b.clone(), true).unwrap();
        approvals.claim_menu(&receipt, &b).unwrap()
    }

    pub(super) fn validate(&self, binding: &Binding) -> Result<(), DiscoveryError> {
        if self.0.scope != menu_scope()
            || self.0.binding != *binding
            || Instant::now() >= self.0.deadline
        {
            return Err(error(
                "APPROVAL_EXPIRED",
                "Menu approval expired or target changed.",
            ));
        }
        Ok(())
    }
    pub(crate) fn take_target(&self, target: &connection::Target, interface: &str) -> bool {
        self.0.scope == menu_scope()
            && self.0.binding.endpoint == target.url
            && self.0.binding.interface == interface
            && Instant::now() < self.0.deadline
            && self
                .1
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
    }
    pub(crate) fn remaining(&self) -> Duration {
        self.0.deadline.saturating_duration_since(Instant::now())
    }
}
pub(super) fn menu_confirmation(device: &Device, binding: &Binding) -> String {
    format!("Approve one read-only static menu check?\n\nDevice: {}\nInterface: {}\nEndpoint: {}\nRequest: GET /xml/ui_mifi.xml (maximum 16 KiB, one attempt)\n\nThis reads the dashboard menu mapping only. No login, settings change, firmware transfer or other request is authorized. Plain HTTP is not cryptographically verified. Proceed only if you own or administer this device.", confirmation_value(&device.display_name), confirmation_value(&binding.interface), confirmation_value(&binding.endpoint))
}

pub(super) fn confirmation_value(value: &str) -> String {
    value
        .chars()
        .filter(|c| {
            !c.is_control() && !matches!(*c,'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}')
        })
        .take(80)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a1_gateway_scope_replay_interface_revision_endpoint_expiry() {
        let mut d = super::super::tests::device();
        d.networks[0].gateways = vec!["192.168.100.1".into()];
        let n = &d.networks[0];
        let mut request = DiscoveryRequest {
            device_id: d.id.clone(),
            endpoint: Some("http://192.168.100.77/".into()),
            interface_id: Some(n.id.clone()),
            approval_token: None,
            advanced_endpoint: false,
        };
        assert!(binding(&d, 1, &request).is_err());
        request.endpoint = Some(format!("http://{}/", n.gateways[0]));
        let b = binding(&d, 1, &request).unwrap();
        let grants = Approvals::default();
        let id = grants.issue(b.clone()).unwrap();
        grants.claim(&id, &b).unwrap();
        assert!(grants.claim(&id, &b).is_err());
        for field in 0..3 {
            let id = grants.issue(b.clone()).unwrap();
            let mut other = b.clone();
            match field {
                0 => other.interface = "other".into(),
                1 => other.revision += 1,
                _ => other.endpoint = "http://192.168.100.2/".into(),
            };
            assert!(grants.claim(&id, &other).is_err());
            assert!(grants.claim(&id, &b).is_err());
        }
        let id = grants.issue(b.clone()).unwrap();
        grants.0.lock().unwrap().get_mut(&id).unwrap().2 = Instant::now();
        assert!(grants.claim(&id, &b).is_err());
    }
    #[test]
    fn menu_approval_replay_cross_scope_binding_expiry_and_no() {
        let mut d = super::super::tests::device();
        d.networks[0].gateways = vec!["192.168.100.1".into()];
        let input = DiscoveryRequest {
            device_id: d.id.clone(),
            endpoint: Some("http://192.168.100.1/".into()),
            interface_id: Some(d.networks[0].id.clone()),
            approval_token: None,
            advanced_endpoint: false,
        };
        let b = binding(&d, 4, &input).unwrap();
        let a = Approvals::default();
        assert!(a.issue_menu(b.clone(), false).is_err());
        assert!(a.0.lock().unwrap().is_empty());
        let token = a.issue_menu(b.clone(), true).unwrap();
        assert!(a.claim(&token, &b).is_err());
        let token = a.issue(b.clone()).unwrap();
        assert!(a.claim_menu(&token, &b).is_err());
        let wrong_request = a.issue(b.clone()).unwrap();
        a.0.lock().unwrap().get_mut(&wrong_request).unwrap().1 =
            ApprovalScope::StaticMenuMetadata {
                request: RequestId::Root,
            };
        assert!(a.claim_menu(&wrong_request, &b).is_err());
        let token = a.issue_menu(b.clone(), true).unwrap();
        let mut proof = a.claim_menu(&token, &b).unwrap();
        assert!(a.claim_menu(&token, &b).is_err());
        assert!(proof.validate(&b).is_ok());
        let target = connection::parse_address(&b.endpoint).unwrap();
        assert!(proof.take_target(&target, &b.interface));
        assert!(!proof.take_target(&target, &b.interface));
        assert!(proof.0.validate(&b).is_err());
        assert!(proof.0.strategy_probe(StrategyId::OfficialImage).is_err());
        let token = a.issue(b.clone()).unwrap();
        a.0.lock().unwrap().get_mut(&token).unwrap().1 = ApprovalScope::StrategyProbe {
            strategy: StrategyId::OfficialImage,
            requests: vec![RequestId::Root],
        };
        assert!(a.claim_menu(&token, &b).is_err());
        assert!(a.claim_menu(&token, &b).is_err());
        for field in 0..3 {
            let token = a.issue_menu(b.clone(), true).unwrap();
            let mut other = b.clone();
            match field {
                0 => other.endpoint = "http://192.168.100.2/".into(),
                1 => other.interface = "other".into(),
                _ => other.revision += 1,
            };
            assert!(a.claim_menu(&token, &other).is_err());
            assert!(a.claim_menu(&token, &b).is_err());
        }
        let token = a.issue_menu(b.clone(), true).unwrap();
        a.0.lock().unwrap().get_mut(&token).unwrap().2 = Instant::now();
        assert!(a.claim_menu(&token, &b).is_err());
        proof.0.deadline = Instant::now();
        assert!(proof.validate(&b).is_err());
    }
    #[test]
    fn menu_confirmation_caps_cleans_and_names_only_fixed_request() {
        let mut d = super::super::tests::device();
        d.networks[0].gateways = vec!["192.168.100.1".into()];
        d.display_name = format!("{}\n\u{202e}", "X".repeat(200));
        let input = DiscoveryRequest {
            device_id: d.id.clone(),
            endpoint: Some("http://192.168.100.1/".into()),
            interface_id: Some(d.networks[0].id.clone()),
            approval_token: None,
            advanced_endpoint: false,
        };
        let b = binding(&d, 4, &input).unwrap();
        let text = menu_confirmation(&d, &b);
        assert!(text.contains(&format!("Device: {}\n", "X".repeat(80))));
        assert!(!text.contains('\u{202e}'));
        assert!(text.contains("GET /xml/ui_mifi.xml (maximum 16 KiB, one attempt)"));
        assert!(!text.contains("login.cgi"));
    }
}
