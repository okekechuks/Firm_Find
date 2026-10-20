//! Cold starts deliberately have no authenticated-management or supported-modem evidence.
use super::mock::{Fault, MockAdapter};
use super::*;
use crate::{
    discovery::{replay_fixture, Device},
    profiles::{
        self, Attribute, CompatibilityReport, EvidenceOrigin, EvidenceRecord, MatchConfidence,
    },
};
use std::{
    sync::{atomic::Ordering, Arc, Barrier},
    time::Duration,
};
const NOW: u64 = 2_000_000;
const PROFILE: &str =
    include_str!("../../../../../device-profiles/tplink-archer-c7-v5-openwrt.json");
struct Cold {
    device: Device,
    profiles: profiles::Registry,
    report: CompatibilityReport,
    registry: Registry,
    binding: SessionBinding,
}
impl Cold {
    fn new(mock: &MockAdapter) -> Self {
        let device = replay_fixture(include_str!("../../fixtures/discovery-lifecycle.json"))
            .unwrap()[1]
            .devices[0]
            .clone();
        let profiles = profiles::Registry::load(vec![(PROFILE, "bundled", true)]);
        let mut evidence = profiles::gather_evidence(&device, &[]);
        evidence.push(
            EvidenceRecord::user_reported(
                device.id.clone(),
                Attribute::Model,
                "Archer C7".into(),
                NOW,
            )
            .unwrap(),
        );
        let report = profiles::evaluate(&profiles, &device, evidence, NOW, 1);
        assert_ne!(report.confidence, MatchConfidence::Verified);
        assert!(report.selected_profile.is_none());
        assert!(report.evidence.iter().all(|e| !matches!(
            e.source,
            EvidenceOrigin::AuthenticatedManagement | EvidenceOrigin::SupportedModem
        )));
        let mut registry = Registry::test_registry();
        registry.register(mock.clone()).unwrap();
        let p = &profiles.profiles[0];
        let binding = SessionBinding {
            device_id: device.id.clone(),
            adapter_id: mock.descriptor.id.clone(),
            adapter_version: "1.0.0".into(),
            profile_id: p.profile.profile_id.clone(),
            profile_version: p.profile.profile_version.clone(),
            profile_hash: p.content_hash.clone(),
            evidence_digest: report.input_digest.clone(),
            discovery_revision: 1,
            endpoint: "fixture://trusted-endpoint".into(),
            interface_id: device.networks[0].id.clone(),
            endpoint_identity: "FIXTURE-PIN".into(),
            expires_at: NOW + 30_000,
            simulated: true,
        };
        Self {
            device,
            profiles,
            report,
            registry,
            binding,
        }
    }
    fn ctx(&self) -> DeviceContext<'_> {
        DeviceContext {
            device: &self.device,
            profile: &self.report,
            profiles: &self.profiles,
            revision: 1,
            now: NOW,
        }
    }
    fn approve(&self) -> HandshakeApproval {
        HandshakeApproval::confirm_selected_target(self.binding.clone(), NOW).unwrap()
    }
    fn connect(
        &self,
        manager: &SessionManager,
        approval: &HandshakeApproval,
        secret: &[u8],
        control: &OperationControl,
    ) -> Result<SessionStatus, AdapterError> {
        manager.connect_initial(
            &self.registry,
            &self.ctx(),
            self.binding.clone(),
            approval,
            Secret::new(secret.to_vec())?,
            control,
        )
    }
}
fn control() -> OperationControl {
    OperationControl::new(
        Cancellation::default(),
        Stage::Connection,
        AdapterOperation::AuthorizedIdentityProbe,
        Duration::from_secs(1),
    )
    .unwrap()
}
fn error<T>(r: Result<T, AdapterError>) -> AdapterErrorCode {
    r.err().expect("must deny").code
}
fn clean(manager: &SessionManager, mock: &MockAdapter) {
    assert_eq!(manager.active_count(), 0);
    assert_eq!(mock.resources.load(Ordering::Acquire), 0);
}
const SECRET: &[u8] = b"test-only-credential";

#[test]
fn cold_start_requires_live_selected_interface_and_matching_provenance() {
    let mock = MockAdapter::new("cold");
    for case in 0..4 {
        let mut f = Cold::new(&mock);
        match case {
            0 => f.binding.interface_id = "unrelated-interface".into(),
            1 => f.device.networks[0].link_up = false,
            2 => f.device.networks[0].virtual_interface = true,
            _ => f.binding.simulated = false,
        }
        let manager = SessionManager::default();
        assert_eq!(
            error(f.connect(&manager, &f.approve(), SECRET, &control())),
            AdapterErrorCode::EndpointUntrusted
        );
        clean(&manager, &mock);
    }
}

#[test]
fn cold_start_approval_uses_monotonic_expiry_even_when_context_clock_is_unchanged() {
    let mock = MockAdapter::new("cold");
    let mut f = Cold::new(&mock);
    f.binding.expires_at = NOW + 1;
    let approval = f.approve();
    std::thread::sleep(Duration::from_millis(5));
    let manager = SessionManager::default();
    assert_eq!(
        error(f.connect(&manager, &approval, SECRET, &control())),
        AdapterErrorCode::SessionExpired
    );
    clean(&manager, &mock);
}

#[test]
fn cold_start_reproduces_established_gate_block_without_seeded_authentication() {
    let mock = MockAdapter::new("cold");
    let f = Cold::new(&mock);
    let manager = SessionManager::default();
    let grant = AuthorizationGrant::confirm_selected_target(
        f.binding.clone(),
        vec![AdapterOperation::AuthorizedIdentityProbe],
        NOW,
    )
    .unwrap();
    assert_eq!(
        error(manager.connect(
            &f.registry,
            &f.ctx(),
            f.binding.clone(),
            &grant,
            Secret::new(SECRET.to_vec()).unwrap(),
            &control()
        )),
        AdapterErrorCode::IdentityUnverified
    );
    clean(&manager, &mock);
}

#[test]
fn cold_start_authenticates_then_requires_separate_established_read_approval() {
    let mock = MockAdapter::new("cold");
    let f = Cold::new(&mock);
    let manager = SessionManager::default();
    let approval = f.approve();
    let session = f.connect(&manager, &approval, SECRET, &control()).unwrap();
    assert_eq!(session.state, ConnectionState::ConnectedReadOnly);
    assert_eq!(session.authentication, AuthenticationState::Authenticated);
    assert!(session.simulated);
    let id = session.id.parse().unwrap();
    let grant = manager
        .confirm_session_reads(
            id,
            &f.registry,
            &f.ctx(),
            vec![
                AdapterOperation::AuthorizedIdentityProbe,
                AdapterOperation::InspectCapabilities,
            ],
        )
        .unwrap();
    let identity = manager
        .read_identity(id, &f.registry, &f.ctx(), &grant, &control())
        .unwrap();
    assert_eq!(identity.management_identity, "tplink,archer-c7-v5");
    assert!(
        !manager
            .probe(id, &f.registry, &f.ctx(), &grant, &control())
            .unwrap()[0]
            .can_apply
    );
    assert_eq!(
        error(f.connect(&manager, &approval, SECRET, &control())),
        AdapterErrorCode::OperationNotAuthorized
    );
    // The original report remains unverified; authenticated evidence is session-local only.
    assert!(f.report.selected_profile.is_none());
    manager.disconnect(id).unwrap();
    clean(&manager, &mock);
}

#[test]
fn cold_start_also_works_without_user_identity_hints() {
    let mock = MockAdapter::new("cold");
    let mut f = Cold::new(&mock);
    f.report = profiles::evaluate(&f.profiles, &f.device, vec![], NOW, 1);
    f.binding.evidence_digest = f.report.input_digest.clone();
    let manager = SessionManager::default();
    let id = f
        .connect(&manager, &f.approve(), SECRET, &control())
        .unwrap()
        .id
        .parse()
        .unwrap();
    manager.disconnect(id).unwrap();
    clean(&manager, &mock);
}

#[test]
fn cold_start_bad_authentication_never_publishes_session_or_reuses_approval() {
    let mock = MockAdapter::new("cold");
    let f = Cold::new(&mock);
    let manager = SessionManager::default();
    let approval = f.approve();
    assert_eq!(
        error(f.connect(&manager, &approval, b"wrong", &control())),
        AdapterErrorCode::AuthenticationFailed
    );
    assert_eq!(
        error(f.connect(&manager, &approval, SECRET, &control())),
        AdapterErrorCode::OperationNotAuthorized
    );
    clean(&manager, &mock);
}

#[test]
fn cold_start_endpoint_interface_pin_and_evidence_substitution_denied() {
    let mock = MockAdapter::new("cold");
    for field in 0..7 {
        let mut f = Cold::new(&mock);
        let approval = f.approve();
        match field {
            0 => f.binding.endpoint = "fixture://different".into(),
            1 => f.binding.interface_id = "another-interface".into(),
            2 => f.binding.endpoint_identity = "OTHER-PIN".into(),
            3 => f.binding.evidence_digest = "other-evidence".into(),
            4 => f.binding.adapter_id = "other-adapter".into(),
            5 => f.binding.profile_hash = "other-profile".into(),
            _ => f.binding.discovery_revision = 2,
        }
        let manager = SessionManager::default();
        assert_eq!(
            error(f.connect(&manager, &approval, SECRET, &control())),
            AdapterErrorCode::OperationNotAuthorized
        );
        clean(&manager, &mock);
    }
    // Fresh approval of a substituted peer still cannot bypass connector pin verification.
    let mut f = Cold::new(&mock);
    f.binding.endpoint_identity = "OTHER-PIN".into();
    let manager = SessionManager::default();
    assert_eq!(
        error(f.connect(&manager, &f.approve(), SECRET, &control())),
        AdapterErrorCode::EndpointUntrusted
    );
    clean(&manager, &mock);
}

#[test]
fn cold_start_connector_must_attest_approved_peer_and_validate_actual_identity() {
    for (fault, expected) in [
        (Fault::MissingTrust, AdapterErrorCode::EndpointUntrusted),
        (
            Fault::EndpointSubstitution,
            AdapterErrorCode::EndpointUntrusted,
        ),
        (Fault::IdentityMismatch, AdapterErrorCode::IdentityConflict),
        (
            Fault::ManagementIdentityMismatch,
            AdapterErrorCode::IdentityConflict,
        ),
    ] {
        let mut mock = MockAdapter::new("cold");
        mock.fault = fault;
        let f = Cold::new(&mock);
        let manager = SessionManager::default();
        assert_eq!(
            error(f.connect(&manager, &f.approve(), SECRET, &control())),
            expected
        );
        clean(&manager, &mock);
    }
}

#[test]
fn cold_start_cancellation_before_and_during_authentication_cleans_resources() {
    let mock = MockAdapter::new("cold");
    let f = Cold::new(&mock);
    let manager = SessionManager::default();
    let c = control();
    c.cancellation().cancel();
    assert_eq!(
        error(f.connect(&manager, &f.approve(), SECRET, &c)),
        AdapterErrorCode::OperationCancelled
    );
    clean(&manager, &mock);
    let mut mock = MockAdapter::new("cold");
    mock.fault = Fault::AuthWait;
    let gate = Arc::new(Barrier::new(2));
    mock.gate = Some(gate.clone());
    let f = Cold::new(&mock);
    let approval = f.approve();
    let c = control();
    std::thread::scope(|s| {
        let worker = s.spawn(|| f.connect(&manager, &approval, SECRET, &c));
        gate.wait();
        c.cancellation().cancel();
        assert_eq!(
            error(worker.join().unwrap()),
            AdapterErrorCode::OperationCancelled
        );
    });
    clean(&manager, &mock);
}

#[test]
fn cold_start_connection_authentication_and_identity_timeouts_are_bounded() {
    for (fault, expected) in [
        (Fault::ConnectWait, AdapterErrorCode::ConnectionTimeout),
        (Fault::AuthWait, AdapterErrorCode::OperationTimeout),
        (Fault::IdentityWait, AdapterErrorCode::OperationTimeout),
    ] {
        let mut mock = MockAdapter::new("cold");
        mock.fault = fault;
        let f = Cold::new(&mock);
        let manager = SessionManager::default();
        let c = OperationControl::new(
            Cancellation::default(),
            Stage::Connection,
            AdapterOperation::AuthorizedIdentityProbe,
            Duration::from_millis(40),
        )
        .unwrap();
        assert_eq!(
            error(f.connect(&manager, &f.approve(), SECRET, &c)),
            expected
        );
        clean(&manager, &mock);
    }
}

#[test]
fn cold_start_approval_expiry_and_stale_discovery_deny_before_connector() {
    let mock = MockAdapter::new("cold");
    let f = Cold::new(&mock);
    let manager = SessionManager::default();
    let approval = f.approve();
    let mut ctx = f.ctx();
    ctx.now = f.binding.expires_at;
    assert_eq!(
        error(manager.connect_initial(
            &f.registry,
            &ctx,
            f.binding.clone(),
            &approval,
            Secret::new(SECRET.to_vec()).unwrap(),
            &control()
        )),
        AdapterErrorCode::SessionExpired
    );
    manager.observe_discovery(&crate::discovery::DiscoverySnapshot {
        revision: 2,
        ..Default::default()
    });
    assert_eq!(
        error(f.connect(&manager, &approval, SECRET, &control())),
        AdapterErrorCode::OperationCancelled
    );
    clean(&manager, &mock);
}

#[test]
fn cold_start_protected_disconnected_ambiguous_and_conflicting_devices_deny() {
    let mock = MockAdapter::new("cold");
    for case in 0..4 {
        let mut f = Cold::new(&mock);
        match case {
            0 => f.device.category = crate::discovery::identity::Category::GenericNetworkInterface,
            1 => f.device.connected = false,
            2 => f.registry.register(MockAdapter::new("second")).unwrap(),
            _ => {
                let mut evidence = f.report.evidence.clone();
                // Two trusted incompatible observations simulate a pre-existing real conflict.
                for (i, value) in ["A", "B"].iter().enumerate() {
                    let mut e = evidence[0].clone();
                    e.id = format!("conflict-{i}");
                    e.attribute = Attribute::Model;
                    e.value = (*value).into();
                    e.collected_at = NOW;
                    e.source = EvidenceOrigin::AuthenticatedManagement;
                    e.verification = profiles::Verification::Verified;
                    e.reliability = profiles::Reliability::Strong;
                    evidence.push(e);
                }
                f.report = profiles::evaluate(&f.profiles, &f.device, evidence, NOW, 1);
                f.binding.evidence_digest = f.report.input_digest.clone();
            }
        }
        let manager = SessionManager::default();
        let expected = match case {
            0 => AdapterErrorCode::ProtectedDevice,
            1 => AdapterErrorCode::DeviceDisconnected,
            _ => AdapterErrorCode::IdentityConflict,
        };
        assert_eq!(
            error(f.connect(&manager, &f.approve(), SECRET, &control())),
            expected
        );
        clean(&manager, &mock);
    }
}

#[test]
fn cold_start_read_scope_cannot_reuse_unverified_grant_or_enable_writes() {
    let mock = MockAdapter::new("cold");
    let f = Cold::new(&mock);
    let manager = SessionManager::default();
    let id = f
        .connect(&manager, &f.approve(), SECRET, &control())
        .unwrap()
        .id
        .parse()
        .unwrap();
    for op in [
        AdapterOperation::ModifyConfiguration,
        AdapterOperation::FirmwareAcquisition,
        AdapterOperation::FirmwareBuild,
        AdapterOperation::FirmwareInstallation,
        AdapterOperation::UssdSession,
    ] {
        assert!(manager
            .confirm_session_reads(id, &f.registry, &f.ctx(), vec![op])
            .is_err());
    }
    let grant = AuthorizationGrant::confirm_selected_target(
        f.binding.clone(),
        vec![AdapterOperation::InspectCapabilities],
        NOW,
    )
    .unwrap();
    assert_eq!(
        error(manager.probe(id, &f.registry, &f.ctx(), &grant, &control())),
        AdapterErrorCode::OperationNotAuthorized
    );
    clean(&manager, &mock);
}

#[test]
fn cold_start_established_reads_invalidate_changed_evidence_and_session_expiry() {
    let mock = MockAdapter::new("cold");
    for expire in [false, true] {
        let mut f = Cold::new(&mock);
        let manager = SessionManager::default();
        let id = f
            .connect(&manager, &f.approve(), SECRET, &control())
            .unwrap()
            .id
            .parse()
            .unwrap();
        let grant = manager
            .confirm_session_reads(
                id,
                &f.registry,
                &f.ctx(),
                vec![AdapterOperation::InspectCapabilities],
            )
            .unwrap();
        if !expire {
            let e = EvidenceRecord::user_reported(
                f.device.id.clone(),
                Attribute::FirmwareVersion,
                "new-report".into(),
                NOW,
            )
            .unwrap();
            let mut evidence = f.report.evidence.clone();
            evidence.push(e);
            f.report = profiles::evaluate(&f.profiles, &f.device, evidence, NOW, 1);
        }
        let mut ctx = f.ctx();
        if expire {
            ctx.now = f.binding.expires_at;
        }
        assert!(manager
            .probe(id, &f.registry, &ctx, &grant, &control())
            .is_err());
        clean(&manager, &mock);
    }
}

#[test]
fn cold_start_discovery_race_and_shutdown_cannot_publish_late_session() {
    let mut mock = MockAdapter::new("cold");
    mock.fault = Fault::IdentityWait;
    let gate = Arc::new(Barrier::new(2));
    mock.gate = Some(gate.clone());
    let f = Cold::new(&mock);
    let manager = SessionManager::default();
    let approval = f.approve();
    let c = control();
    std::thread::scope(|s| {
        let worker = s.spawn(|| f.connect(&manager, &approval, SECRET, &c));
        gate.wait();
        manager.observe_discovery(&crate::discovery::DiscoverySnapshot {
            revision: 2,
            ..Default::default()
        });
        c.cancellation().cancel();
        assert_eq!(
            error(worker.join().unwrap()),
            AdapterErrorCode::OperationCancelled
        );
    });
    clean(&manager, &mock);
    let manager = SessionManager::default();
    manager.shutdown();
    assert_eq!(
        error(f.connect(&manager, &f.approve(), SECRET, &control())),
        AdapterErrorCode::OperationCancelled
    );
    clean(&manager, &mock);
}
