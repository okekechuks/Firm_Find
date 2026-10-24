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
    auth: Option<crate::endpoint_binding::BoundEndpoint>,
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
        auth: None,
    })
}

pub(super) fn challenge_binding(
    service: &Service,
    snapshot: &crate::discovery::DiscoverySnapshot,
    input: &DiscoveryRequest,
) -> Result<Binding, DiscoveryError> {
    challenge_binding_with(
        service,
        snapshot,
        input,
        &crate::endpoint_binding::SystemRoute,
    )
}
fn challenge_binding_with(
    service: &Service,
    snapshot: &crate::discovery::DiscoverySnapshot,
    input: &DiscoveryRequest,
    provider: &impl crate::endpoint_binding::RouteProvider,
) -> Result<Binding, DiscoveryError> {
    let device = snapshot
        .devices
        .iter()
        .find(|d| d.id == input.device_id)
        .ok_or_else(|| error("DEVICE_NOT_FOUND", "Select a current device."))?;
    let mut out = binding(device, snapshot.revision, input)?;
    let selection = crate::endpoint_binding::Selection {
        device: out.device.clone(),
        interface: out.interface.clone(),
        endpoint: out.endpoint.clone(),
    };
    out.auth = Some(
        service
            .2
            .bind(snapshot, &selection, provider, connection::now())
            .map_err(|_| {
                error(
                    "BINDING_CHANGED",
                    "Endpoint consistency unavailable or changed. Refresh and approve again.",
                )
            })?,
    );
    out.revision = 0; // New auth scopes use selected material facts, never global scan revision.
    Ok(out)
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
        let scope = challenge_scope(&binding)?;
        self.issue_scoped(binding, scope)
    }
    pub(super) fn claim_challenge(
        &self,
        id: &str,
        binding: &Binding,
    ) -> Result<AuthChallengeApproval, DiscoveryError> {
        let scope = match challenge_scope(binding) {
            Ok(scope) => scope,
            Err(e) => {
                self.0
                    .lock()
                    .map_err(|_| error("INTERNAL", "Approval unavailable."))?
                    .remove(id);
                return Err(e);
            }
        };
        self.claim_scoped(id, binding, scope)
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
        binding: crate::endpoint_binding::Stamp,
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
fn challenge_scope(binding: &Binding) -> Result<ApprovalScope, DiscoveryError> {
    let bound = binding
        .auth
        .as_ref()
        .filter(|b| b.current())
        .ok_or_else(|| {
            error(
                "BINDING_CHANGED",
                "Fresh endpoint consistency binding required.",
            )
        })?;
    Ok(ApprovalScope::AuthChallenge {
        request: RequestId::Challenge,
        binding: bound.stamp(),
    })
}

/// One credential-free GET only; cannot be cloned, serialized or upgraded into a session.
pub(crate) struct AuthChallengeApproval(ClaimedApproval, AtomicBool);
impl AuthChallengeApproval {
    #[cfg(test)]
    pub(super) fn for_test(device: &Device, revision: u64, input: &DiscoveryRequest) -> Self {
        let b = test_challenge_binding(binding(device, revision, input).unwrap());
        let a = Approvals::default();
        let receipt = a.issue_challenge(b.clone(), true).unwrap();
        a.claim_challenge(&receipt, &b).unwrap()
    }
    pub(super) fn validate(&self, binding: &Binding) -> Result<(), DiscoveryError> {
        let expected = &self.0.binding;
        // The collector receives its immutable device selection. IPC additionally compares
        // a fresh material snapshot before dispatch and publication. Shared liveness covers
        // removals/cancellation observed while the transport is running.
        if challenge_scope(expected).ok().as_ref() != Some(&self.0.scope)
            || expected.device != binding.device
            || expected.interface != binding.interface
            || expected.endpoint != binding.endpoint
            || expected.advanced != binding.advanced
            || binding
                .auth
                .as_ref()
                .is_some_and(|b| Some(b) != expected.auth.as_ref() || !b.current())
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
        challenge_scope(&self.0.binding).ok().as_ref() == Some(&self.0.scope)
            && self.0.binding.endpoint == target.url
            && self.0.binding.interface == interface
            && Instant::now() < self.0.deadline
            && self
                .1
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
    }
    pub(crate) fn remaining(&self) -> Duration {
        self.0
            .deadline
            .saturating_duration_since(Instant::now())
            .min(
                self.0
                    .binding
                    .auth
                    .as_ref()
                    .map(|b| b.remaining())
                    .unwrap_or(Duration::ZERO),
            )
    }
}
#[cfg(test)]
fn test_challenge_binding(mut binding: Binding) -> Binding {
    binding.revision = 0;
    binding.auth = Some(crate::endpoint_binding::BoundEndpoint::synthetic());
    binding
}
pub(super) fn challenge_confirmation(device: &Device, binding: &Binding) -> String {
    format!("Approve one credential-free authentication challenge check?\n\nDevice: {}\nInterface: {}\nEndpoint: {}\nRequest: GET /login.cgi, one attempt, maximum 5 seconds.\n\nNo username, password, Authorization header or cookies will be sent. Only a bounded realm and qop observation may be reported; the nonce and response body will not be kept. Plain HTTP is not cryptographically verified. This does not authorize login, identity reads, logout or firmware access. Continue only if you own or administer this endpoint.",confirmation_value(&device.display_name),confirmation_value(&binding.interface),confirmation_value(&binding.endpoint))
}

#[cfg(test)]
mod challenge_scope_tests {
    use super::*;
    struct FakeRoute;
    impl crate::endpoint_binding::RouteProvider for FakeRoute {
        fn route(
            &self,
            _: &connection::Target,
            _: &crate::discovery::NetworkAdapter,
        ) -> Result<crate::endpoint_binding::Route, crate::endpoint_binding::BindingError> {
            Ok(crate::endpoint_binding::Route {
                source: "192.168.100.2".into(),
                gateway: None,
                luid: 1234,
            })
        }
    }
    fn bound_fixture() -> (
        crate::discovery::DiscoverySnapshot,
        DiscoveryRequest,
        AcquisitionCapabilityReport,
    ) {
        let (mut device, mut input, mut base) = super::super::challenge_tests::context();
        let network = &mut device.networks[0];
        network.id = "{00112233-4455-6677-8899-aabbccddeeff}".into();
        network.luid = "1234".into();
        network.addresses = vec!["192.168.100.2".into()];
        network.prefixes = vec![crate::discovery::AddressPrefix {
            address: "192.168.100.2".into(),
            length: 24,
        }];
        input.interface_id = Some(network.id.clone());
        base.fingerprint.interface_id = input.interface_id.clone();
        (
            crate::discovery::DiscoverySnapshot {
                revision: 4,
                scanned_at: Some(connection::now()),
                devices: vec![device],
                status: crate::discovery::DiscoveryStatus::Ready,
                ..Default::default()
            },
            input,
            base,
        )
    }
    #[test]
    fn stage4_new_auth_scope_requires_material_binding_and_keeps_legacy_scopes() {
        let (mut snapshot, input, _) = bound_fixture();
        let service = Service::default();
        let legacy = binding(&snapshot.devices[0], 4, &input).unwrap();
        assert!(service.1.issue_challenge(legacy.clone(), true).is_err());
        let bound = challenge_binding_with(&service, &snapshot, &input, &FakeRoute).unwrap();
        let receipt = service.1.issue(legacy.clone()).unwrap();
        assert!(service.1.claim_challenge(&receipt, &bound).is_err());
        let receipt = service.1.issue_menu(legacy.clone(), true).unwrap();
        assert!(service.1.claim_challenge(&receipt, &bound).is_err());
        let receipt = service.1.issue(legacy.clone()).unwrap();
        service.1.claim(&receipt, &legacy).unwrap();
        let receipt = service.1.issue_menu(legacy.clone(), true).unwrap();
        service.1.claim_menu(&receipt, &legacy).unwrap();
        let receipt = service.1.issue_challenge(bound.clone(), true).unwrap();
        snapshot.revision += 1;
        snapshot.devices[0].display_name = "user label changed".into();
        let same = challenge_binding_with(&service, &snapshot, &input, &FakeRoute).unwrap();
        assert_eq!(same, bound);
        let proof = service.1.claim_challenge(&receipt, &same).unwrap();
        snapshot.devices[0].networks[0].dhcp_server = Some("192.168.100.1".into());
        service.2.observe(&snapshot, &FakeRoute, connection::now());
        assert!(proof.validate(&same).is_err());
        assert!(!proof.take_target(
            &connection::parse_address(input.endpoint.as_deref().unwrap()).unwrap(),
            input.interface_id.as_deref().unwrap()
        ));
        let fresh = challenge_binding_with(&service, &snapshot, &input, &FakeRoute).unwrap();
        assert_ne!(fresh, same);
        let receipt = service.1.issue_challenge(fresh.clone(), true).unwrap();
        service.2.invalidate(&input.device_id);
        assert!(service.1.claim_challenge(&receipt, &fresh).is_err());
        assert!(!service.1 .0.lock().unwrap().contains_key(&receipt));
    }
    #[test]
    fn stage4_change_during_controlled_challenge_discards_success_and_recovers() {
        let (snapshot, input, base) = bound_fixture();
        let service = Service::default();
        let bound = challenge_binding_with(&service, &snapshot, &input, &FakeRoute).unwrap();
        let receipt = service.1.issue_challenge(bound.clone(), true).unwrap();
        let proof = service.1.claim_challenge(&receipt, &bound).unwrap();
        let mut calls = 0;
        let result = super::super::challenge::collect(
            &base,
            &snapshot.devices[0],
            4,
            &input,
            &proof,
            &AtomicBool::new(false),
            |_, _, _| {
                calls += 1;
                service.2.invalidate(&input.device_id);
                ChallengeObservation::failure("observed", Some(401))
            },
        );
        assert_eq!(calls, 1);
        assert!(result.is_err());
        let fresh = challenge_binding_with(&service, &snapshot, &input, &FakeRoute).unwrap();
        let receipt = service.1.issue_challenge(fresh.clone(), true).unwrap();
        let proof = service.1.claim_challenge(&receipt, &fresh).unwrap();
        let report = super::super::challenge::collect(
            &base,
            &snapshot.devices[0],
            4,
            &input,
            &proof,
            &AtomicBool::new(false),
            |_, _, _| ChallengeObservation::failure("challenge_not_usable", Some(401)),
        )
        .unwrap();
        assert!(!report.can_obtain);
    }
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
        let b = test_challenge_binding(binding(&d, 4, &input).unwrap());
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
            binding: b.auth.as_ref().unwrap().stamp(),
        };
        assert!(a.claim_challenge(&receipt, &b).is_err());
        for field in 0..3 {
            let receipt = a.issue_challenge(b.clone(), true).unwrap();
            let mut changed = b.clone();
            match field {
                0 => changed.endpoint = "http://192.168.100.2/".into(),
                1 => changed.interface = "other".into(),
                _ => changed.auth = Some(crate::endpoint_binding::BoundEndpoint::synthetic()),
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
                auth: None,
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
