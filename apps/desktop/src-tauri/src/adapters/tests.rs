use super::mock::{Fault, MockAdapter};
use super::*;

#[test]
fn authentication_transitions_require_a_handshake_and_expire_explicitly() {
    let mut state = AuthenticationLifecycle::default();
    assert_eq!(state.state(), AuthenticationState::Required);
    assert!(state.accept().is_err());
    state.begin().unwrap();
    assert_eq!(state.state(), AuthenticationState::Authenticating);
    state.accept().unwrap();
    assert_eq!(state.state(), AuthenticationState::Authenticated);
    state.expire();
    assert_eq!(state.state(), AuthenticationState::Expired);
    assert!(state.begin().is_err());
    let mut failed = AuthenticationLifecycle::default();
    failed.begin().unwrap();
    failed.reject();
    assert_eq!(failed.state(), AuthenticationState::Failed);
}
#[test]
fn identity_mismatch_cannot_establish_a_connected_session() {
    let mut mock = MockAdapter::new("fixture");
    mock.fault = Fault::IdentityMismatch;
    let f = Fixture::new(&mock);
    let m = SessionManager::default();
    assert_eq!(
        code(f.connect(&m, &control())),
        AdapterErrorCode::IdentityConflict
    );
    assert_eq!(m.active_count(), 0);
    assert_eq!(mock.resources.load(Ordering::Acquire), 0);
}

#[test]
fn discovery_changes_during_connection_prevent_late_session_publication() {
    let (started, receiver) = std::sync::mpsc::channel();
    let (proceed, release) = std::sync::mpsc::channel();
    let mut mock = MockAdapter::new("fixture");
    mock.fault = Fault::ControlledConnect;
    mock.started = Some(started);
    mock.proceed = Some(Arc::new(std::sync::Mutex::new(release)));
    let f = Fixture::new(&mock);
    let m = SessionManager::default();
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            let c = OperationControl::new(
                Cancellation::default(),
                Stage::Connection,
                AdapterOperation::AuthorizedIdentityProbe,
                Duration::from_secs(10),
            )
            .unwrap();
            f.connect(&m, &c)
        });
        receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("connector did not signal admission");
        m.observe_discovery(&crate::discovery::DiscoverySnapshot {
            revision: 2,
            ..Default::default()
        });
        proceed.send(()).unwrap();
        assert!(worker.join().unwrap().is_err());
    });
    assert_eq!(m.active_count(), 0);
    assert_eq!(mock.resources.load(Ordering::Acquire), 0);
}
#[test]
fn controlled_connector_can_complete_without_a_discovery_change() {
    let (started, receiver) = std::sync::mpsc::channel();
    let (proceed, release) = std::sync::mpsc::channel();
    let mut mock = MockAdapter::new("fixture");
    mock.fault = Fault::ControlledConnect;
    mock.started = Some(started);
    mock.proceed = Some(Arc::new(std::sync::Mutex::new(release)));
    let f = Fixture::new(&mock);
    let m = SessionManager::default();
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| f.connect(&m, &control()));
        receiver.recv_timeout(Duration::from_secs(10)).unwrap();
        proceed.send(()).unwrap();
        assert!(worker.join().unwrap().is_ok());
    });
    m.shutdown();
    assert_eq!(mock.resources.load(Ordering::Acquire), 0);
}
#[test]
fn malformed_adapter_queries_reject_paths_unknown_fields_and_noncanonical_ids() {
    for id in ["", "../data", &"A".repeat(64), &"0".repeat(65)] {
        assert!(AdapterQuery {
            device_id: id.into(),
            target_kind: AdapterTargetKind::Discovered
        }
        .validate()
        .is_err());
    }
    assert!(serde_json::from_value::<AdapterQuery>(
        serde_json::json!({"deviceId":"a".repeat(64),"authorized":true})
    )
    .is_err());
}
#[test]
fn shutdown_prevents_new_connections_and_expiry_reaps_idle_resources() {
    let mock = MockAdapter::new("fixture");
    let f = Fixture::new(&mock);
    let m = SessionManager::default();
    let id = f.connect(&m, &control()).unwrap().id.parse().unwrap();
    assert_eq!(
        m.status(id, NOW).unwrap().authentication,
        AuthenticationState::Authenticated
    );
    m.reap_expired(NOW + 40_000);
    assert_eq!(m.active_count(), 0);
    assert_eq!(mock.resources.load(Ordering::Acquire), 0);
    m.shutdown();
    assert_eq!(
        code(f.connect(&m, &control())),
        AdapterErrorCode::OperationCancelled
    );
}
use crate::{
    discovery::{replay_fixture, Device},
    profiles::{
        self, Attribute, CompatibilityReport, EvidenceOrigin, EvidenceRecord, MatchConfidence,
        Reliability, Verification,
    },
};
use std::{
    sync::{atomic::Ordering, Arc, Barrier},
    time::Duration,
};
const NOW: u64 = 2_000_000;
const PROFILE: &str =
    include_str!("../../../../../device-profiles/tplink-archer-c7-v5-openwrt.json");
fn device() -> Device {
    let mut d = replay_fixture(include_str!("../../fixtures/discovery-lifecycle.json")).unwrap()[1]
        .devices[0]
        .clone();
    d.last_seen_at = NOW;
    d
}
fn evidence(d: &Device) -> Vec<EvidenceRecord> {
    [
        (Attribute::Manufacturer, "TP-Link"),
        (Attribute::Model, "Archer C7"),
        (Attribute::HardwareRevision, "v5"),
        (Attribute::FirmwareFamily, "OpenWrt"),
        (Attribute::FirmwareVersion, "23.05.5"),
        (Attribute::ManagementIdentity, "tplink,archer-c7-v5"),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (attribute, value))| EvidenceRecord {
        id: format!("fixture-{i}"),
        subject_id: d.id.clone(),
        attribute,
        value: value.into(),
        source: EvidenceOrigin::AuthenticatedManagement,
        collection_method: "SIMULATED contract fixture only".into(),
        collected_at: NOW,
        reliability: Reliability::Strong,
        verification: Verification::Verified,
        interface_id: Some("fixture-interface".into()),
        endpoint: Some("fixture://trusted-endpoint".into()),
        reference: None,
        hardware_applicability: Some("v5".into()),
        firmware_applicability: Some("23.05.5".into()),
        conflicts: vec![],
        stale: false,
    })
    .collect()
}
struct Fixture {
    device: Device,
    profiles: profiles::Registry,
    report: CompatibilityReport,
    registry: Registry,
}
impl Fixture {
    fn new(mock: &MockAdapter) -> Self {
        let d = device();
        let p = profiles::Registry::load(vec![(PROFILE, "bundled", true)]);
        let report = profiles::evaluate(&p, &d, evidence(&d), NOW, 1);
        assert_eq!(report.confidence, MatchConfidence::Verified);
        let mut registry = Registry::test_registry();
        registry.register(mock.clone()).unwrap();
        Self {
            device: d,
            profiles: p,
            report,
            registry,
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
    fn binding(&self) -> SessionBinding {
        SessionBinding {
            device_id: self.device.id.clone(),
            adapter_id: self.registry.descriptors()[0].id.clone(),
            adapter_version: "1.0.0".into(),
            profile_id: self.report.selected_profile.clone().unwrap(),
            profile_version: self.report.profile_version.clone().unwrap(),
            profile_hash: self.report.profile_content_hash.clone().unwrap(),
            evidence_digest: self.report.input_digest.clone(),
            discovery_revision: 1,
            endpoint: "fixture://trusted-endpoint".into(),
            interface_id: "fixture-interface".into(),
            endpoint_identity: "FIXTURE-PIN".into(),
            expires_at: NOW + 30_000,
            simulated: true,
        }
    }
    fn grant(&self) -> AuthorizationGrant {
        AuthorizationGrant::confirm_selected_target(
            self.binding(),
            vec![
                AdapterOperation::AuthorizedIdentityProbe,
                AdapterOperation::InspectCapabilities,
                AdapterOperation::ReadDns,
                AdapterOperation::ReadApn,
                AdapterOperation::ReadWifiLimit,
                AdapterOperation::ReadUssdCapability,
                AdapterOperation::BackupConfiguration,
            ],
            NOW,
        )
        .unwrap()
    }
    fn connect(
        &self,
        m: &SessionManager,
        c: &OperationControl,
    ) -> Result<SessionStatus, AdapterError> {
        m.connect(
            &self.registry,
            &self.ctx(),
            self.binding(),
            &self.grant(),
            secret(),
            c,
        )
    }
}
fn secret() -> Secret {
    Secret::new(b"test-only-credential".to_vec()).unwrap()
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
fn short(stage: Stage) -> OperationControl {
    OperationControl::new(
        Cancellation::default(),
        stage,
        AdapterOperation::AuthorizedIdentityProbe,
        Duration::from_millis(15),
    )
    .unwrap()
}
fn code<T>(r: Result<T, AdapterError>) -> AdapterErrorCode {
    match r {
        Err(e) => e.code,
        Ok(_) => panic!("unexpected success"),
    }
}

#[test]
fn descriptors_validate_duplicate_ids_versions_bounds_and_production_test_isolation() {
    let mock = MockAdapter::new("fixture");
    assert!(Registry::default().register(mock.clone()).is_err());
    for mutate in [0, 1, 2, 3, 4] {
        let mut bad = mock.clone();
        match mutate {
            0 => bad.descriptor.id = "../bad".into(),
            1 => bad.descriptor.version = "x".into(),
            2 => bad.descriptor.transports.clear(),
            3 => bad
                .descriptor
                .operations
                .push(AdapterOperation::InspectCapabilities),
            _ => bad.descriptor.firmware_families.clear(),
        }
        assert_eq!(
            code(Registry::test_registry().register(bad)),
            AdapterErrorCode::AdapterNotCompatible
        );
    }
    let mut r = Registry::test_registry();
    r.register(mock.clone()).unwrap();
    assert_eq!(
        code(r.register(mock)),
        AdapterErrorCode::InternalAdapterError
    );
    assert!(Registry::production()
        .unwrap()
        .descriptors()
        .iter()
        .all(|d| d.implementation_status == AdapterImplementationStatus::BuiltIn));
}
#[test]
fn exact_profile_matches_without_claiming_a_management_connection() {
    let f = Fixture::new(&MockAdapter::new("fixture"));
    let r = f.registry.evaluate(&f.ctx());
    assert_eq!(r.matches[0].status, AdapterMatchStatus::Exact);
    assert_eq!(r.selected_adapter.as_deref(), Some("fixture"));
    assert_ne!(r.connection_state, ConnectionState::ConnectedReadOnly);
    assert!(r.matches[0]
        .features
        .values()
        .all(|f| !f.can_write && !f.can_read && !f.can_verify && !f.can_rollback));
    let production = Registry::production().unwrap().evaluate(&f.ctx());
    assert_eq!(production.matches[0].status, AdapterMatchStatus::Exact);
    assert_eq!(
        production.connection_state,
        ConnectionState::AuthorizationRequired
    );
}
#[test]
fn unknown_labels_usb_and_http_success_cannot_grant_matching_authority() {
    let mut f = Fixture::new(&MockAdapter::new("fixture"));
    f.report = profiles::evaluate(
        &f.profiles,
        &f.device,
        profiles::gather_evidence(&f.device, &[]),
        NOW,
        1,
    );
    assert!(f.registry.evaluate(&f.ctx()).selected_adapter.is_none());
    assert!(f
        .report
        .features
        .values()
        .all(|v| v.assessment == profiles::Assessment::Unknown));
}
#[test]
fn wrong_revision_missing_firmware_and_user_reported_firmware_do_not_select() {
    for case in 0..3 {
        let mut f = Fixture::new(&MockAdapter::new("fixture"));
        let mut e = evidence(&f.device);
        match case {
            0 => {
                e.iter_mut()
                    .find(|e| e.attribute == Attribute::HardwareRevision)
                    .unwrap()
                    .value = "v99".into()
            }
            1 => e.retain(|e| e.attribute != Attribute::FirmwareVersion),
            _ => {
                let v = e
                    .iter_mut()
                    .find(|e| e.attribute == Attribute::FirmwareVersion)
                    .unwrap();
                v.source = EvidenceOrigin::UserSupplied;
                v.reliability = Reliability::Hint;
                v.verification = Verification::UserReported;
            }
        }
        f.report = profiles::evaluate(&f.profiles, &f.device, e, NOW, 1);
        assert!(f.registry.evaluate(&f.ctx()).selected_adapter.is_none());
    }
}
#[test]
fn conflicting_identity_and_profile_ambiguity_reject() {
    let mut f = Fixture::new(&MockAdapter::new("fixture"));
    let mut e = evidence(&f.device);
    let mut conflicting = e[1].clone();
    conflicting.id = "conflict".into();
    conflicting.value = "Other model".into();
    e.push(conflicting);
    f.report = profiles::evaluate(&f.profiles, &f.device, e, NOW, 1);
    assert_eq!(
        f.registry.evaluate(&f.ctx()).matches[0].status,
        AdapterMatchStatus::Rejected
    );
}
#[test]
fn unique_exact_match_beats_weak_candidate_independent_of_registration_order() {
    let base: serde_json::Value = serde_json::from_str(PROFILE).unwrap();
    let mut weak = base.clone();
    weak["profileId"] = "weak.profile".into();
    let text = serde_json::to_string(&weak).unwrap();
    for reverse in [false, true] {
        let mut f = Fixture::new(&MockAdapter::new("exact"));
        f.profiles = profiles::Registry::load(vec![
            (PROFILE, "bundled", true),
            (&text, "local_unverified", false),
        ]);
        f.report = profiles::evaluate(&f.profiles, &f.device, evidence(&f.device), NOW, 1);
        let mut exact = MockAdapter::new("exact");
        exact.descriptor.profile_ids = vec![base["profileId"].as_str().unwrap().into()];
        let mut weak = MockAdapter::new("weak");
        weak.descriptor.profile_ids = vec!["weak.profile".into()];
        f.registry = Registry::test_registry();
        if reverse {
            f.registry.register(weak).unwrap();
            f.registry.register(exact).unwrap();
        } else {
            f.registry.register(exact).unwrap();
            f.registry.register(weak).unwrap();
        }
        let r = f.registry.evaluate(&f.ctx());
        assert_eq!(r.selected_adapter.as_deref(), Some("exact"));
        assert_eq!(
            r.matches[1].status,
            AdapterMatchStatus::RequiresVerification
        );
    }
}
#[test]
fn two_exact_adapters_are_ambiguous() {
    let mut f = Fixture::new(&MockAdapter::new("one"));
    f.registry.register(MockAdapter::new("two")).unwrap();
    let r = f.registry.evaluate(&f.ctx());
    assert_eq!(r.selection, AdapterSelectionStatus::Ambiguous);
    assert!(r.selected_adapter.is_none());
}
#[test]
fn disconnected_stale_wrong_subject_and_revision_reject_before_connect() {
    for case in 0..4 {
        let mut f = Fixture::new(&MockAdapter::new("fixture"));
        match case {
            0 => f.device.connected = false,
            1 => f.report.expires_at = NOW,
            _ if case == 2 => f.report.device_id = "other".into(),
            _ => f.report.discovery_revision = 2,
        }
        assert!(f.registry.evaluate(&f.ctx()).selected_adapter.is_none());
        assert!(f.connect(&SessionManager::default(), &control()).is_err());
    }
}
#[test]
fn pci_wifi_storage_hid_and_spoofed_categories_are_protected() {
    let mut f = Fixture::new(&MockAdapter::new("fixture"));
    for class in ["DiskDrive", "HIDClass", "Keyboard", "System"] {
        f.device.interfaces[0].class_name = class.into();
        assert_eq!(
            f.registry.evaluate(&f.ctx()).selection,
            AdapterSelectionStatus::Protected
        );
        assert_eq!(
            code(f.connect(&SessionManager::default(), &control())),
            AdapterErrorCode::ProtectedDevice
        );
    }
}
#[test]
fn all_physical_writes_acquisition_and_ussd_are_denied_in_backend() {
    let f = Fixture::new(&MockAdapter::new("fixture"));
    for op in [
        AdapterOperation::ModifyConfiguration,
        AdapterOperation::RestoreConfiguration,
        AdapterOperation::FirmwareInstallation,
        AdapterOperation::FirmwareAcquisition,
        AdapterOperation::UssdSession,
        AdapterOperation::FirmwareBuild,
    ] {
        assert_eq!(
            code(authorize(
                &f.registry,
                &f.ctx(),
                &f.binding(),
                op,
                &f.grant()
            )),
            AdapterErrorCode::OperationNotAuthorized
        );
    }
}
#[test]
fn authorization_binds_endpoint_interface_adapter_profile_evidence_and_operation() {
    let f = Fixture::new(&MockAdapter::new("fixture"));
    for case in 0..7 {
        let mut b = f.binding();
        match case {
            0 => b.endpoint = "fixture://substitute".into(),
            1 => b.interface_id = "other".into(),
            2 => b.adapter_id = "other".into(),
            3 => b.profile_hash = "other".into(),
            4 => b.evidence_digest = "other".into(),
            5 => b.discovery_revision = 2,
            _ => b.endpoint_identity = "OTHER-PIN".into(),
        }
        assert!(authorize(
            &f.registry,
            &f.ctx(),
            &b,
            AdapterOperation::AuthorizedIdentityProbe,
            &f.grant()
        )
        .is_err());
    }
    let g = AuthorizationGrant::confirm_selected_target(
        f.binding(),
        vec![AdapterOperation::InspectCapabilities],
        NOW,
    )
    .unwrap();
    assert_eq!(
        code(authorize(
            &f.registry,
            &f.ctx(),
            &f.binding(),
            AdapterOperation::AuthorizedIdentityProbe,
            &g
        )),
        AdapterErrorCode::OperationNotAuthorized
    );
}
#[test]
fn endpoint_pin_substitution_is_rejected_even_with_new_scoped_grant() {
    let f = Fixture::new(&MockAdapter::new("fixture"));
    let m = SessionManager::default();
    let mut b = f.binding();
    b.endpoint_identity = "WRONG".into();
    let g = AuthorizationGrant::confirm_selected_target(
        b.clone(),
        vec![AdapterOperation::AuthorizedIdentityProbe],
        NOW,
    )
    .unwrap();
    assert_eq!(
        code(m.connect(&f.registry, &f.ctx(), b, &g, secret(), &control())),
        AdapterErrorCode::EndpointUntrusted
    );
    assert_eq!(m.active_count(), 0);
}
#[test]
fn missing_and_invalid_credentials_fail_without_leaking() {
    assert!(Secret::new(vec![]).is_err());
    let mock = MockAdapter::new("fixture");
    let f = Fixture::new(&mock);
    let m = SessionManager::default();
    let s = Secret::new(b"private-value".to_vec()).unwrap();
    assert_eq!(format!("{s:?}"), "Secret([REDACTED])");
    let err = m
        .connect(
            &f.registry,
            &f.ctx(),
            f.binding(),
            &f.grant(),
            s,
            &control(),
        )
        .unwrap_err();
    assert_eq!(err.code, AdapterErrorCode::AuthenticationFailed);
    assert!(!serde_json::to_string(&err)
        .unwrap()
        .contains("private-value"));
    assert_eq!(mock.resources.load(Ordering::Acquire), 0);
    assert_eq!(m.active_count(), 0);
}
#[test]
fn successful_simulated_handshake_identity_capabilities_and_idempotent_disconnect() {
    let mock = MockAdapter::new("fixture");
    let f = Fixture::new(&mock);
    let m = SessionManager::default();
    let status = f.connect(&m, &control()).unwrap();
    assert!(status.simulated);
    let id = status.id.parse().unwrap();
    let identity = m
        .read_identity(id, &f.registry, &f.ctx(), &f.grant(), &control())
        .unwrap();
    assert!(identity.simulated);
    assert_eq!(identity.hardware_revision, "v5");
    let caps = m
        .probe(id, &f.registry, &f.ctx(), &f.grant(), &control())
        .unwrap();
    assert!(caps[0].can_read);
    assert!(!caps[0].can_apply);
    m.disconnect(id).unwrap();
    m.disconnect(id).unwrap();
    assert_eq!(mock.resources.load(Ordering::Acquire), 0);
    assert_eq!(m.active_count(), 0);
}
#[test]
fn unsupported_feature_reads_and_backup_never_report_success() {
    for op in [
        AdapterOperation::ReadDns,
        AdapterOperation::ReadApn,
        AdapterOperation::ReadWifiLimit,
        AdapterOperation::ReadUssdCapability,
        AdapterOperation::BackupConfiguration,
    ] {
        let mock = MockAdapter::new("fixture");
        let f = Fixture::new(&mock);
        let m = SessionManager::default();
        let id = f.connect(&m, &control()).unwrap().id.parse().unwrap();
        let c = control();
        let result = match op {
            AdapterOperation::ReadDns => {
                code(m.read_dns(id, &f.registry, &f.ctx(), &f.grant(), &c))
            }
            AdapterOperation::ReadApn => {
                code(m.read_apn(id, &f.registry, &f.ctx(), &f.grant(), &c))
            }
            AdapterOperation::ReadWifiLimit => {
                code(m.read_wifi(id, &f.registry, &f.ctx(), &f.grant(), &c))
            }
            AdapterOperation::ReadUssdCapability => {
                code(m.read_ussd(id, &f.registry, &f.ctx(), &f.grant(), &c))
            }
            _ => code(authorize(
                &f.registry,
                &f.ctx(),
                &f.binding(),
                op,
                &f.grant(),
            )),
        };
        assert_eq!(result, AdapterErrorCode::OperationUnsupported);
        assert_eq!(
            m.active_count(),
            1,
            "unsupported feature must not log out the session"
        );
        assert!(m
            .read_identity(id, &f.registry, &f.ctx(), &f.grant(), &control())
            .is_ok());
        m.shutdown();
        assert_eq!(mock.resources.load(Ordering::Acquire), 0);
    }
}
#[test]
fn expired_or_changed_discovery_invalidates_and_releases_session() {
    for expire in [true, false] {
        let mock = MockAdapter::new("fixture");
        let f = Fixture::new(&mock);
        let m = SessionManager::default();
        let id = f.connect(&m, &control()).unwrap().id.parse().unwrap();
        let mut ctx = f.ctx();
        if expire {
            ctx.now = NOW + 31_000;
        } else {
            ctx.revision = 2;
        }
        assert!(m
            .read_identity(id, &f.registry, &ctx, &f.grant(), &control())
            .is_err());
        assert_eq!(m.active_count(), 0);
        assert_eq!(mock.resources.load(Ordering::Acquire), 0);
    }
}
#[test]
fn cancellation_before_and_after_connection_does_not_publish_success() {
    let mock = MockAdapter::new("fixture");
    let f = Fixture::new(&mock);
    let m = SessionManager::default();
    let c = control();
    c.cancellation().cancel();
    assert_eq!(
        code(f.connect(&m, &c)),
        AdapterErrorCode::OperationCancelled
    );
    assert_eq!(m.active_count(), 0);
    let id = f.connect(&m, &control()).unwrap().id.parse().unwrap();
    assert_eq!(
        code(m.read_identity(id, &f.registry, &f.ctx(), &f.grant(), &c)),
        AdapterErrorCode::OperationCancelled
    );
    assert_eq!(mock.resources.load(Ordering::Acquire), 0);
}
#[test]
fn cancellation_during_connection_releases_resource_and_concurrency_slot() {
    let gate = Arc::new(Barrier::new(2));
    let mut mock = MockAdapter::new("fixture");
    mock.fault = Fault::ConnectWait;
    mock.gate = Some(gate.clone());
    let f = Fixture::new(&mock);
    let c = control();
    let token = c.cancellation();
    std::thread::scope(|s| {
        let worker = s.spawn(|| f.connect(&SessionManager::default(), &c));
        gate.wait();
        token.cancel();
        assert_eq!(
            code(worker.join().unwrap()),
            AdapterErrorCode::OperationCancelled
        );
    });
    assert_eq!(mock.resources.load(Ordering::Acquire), 0);
}
#[test]
fn real_stage_deadlines_bound_connect_auth_identity_and_capability_work() {
    for fault in [
        Fault::ConnectWait,
        Fault::AuthWait,
        Fault::IdentityWait,
        Fault::CapabilitiesWait,
    ] {
        let mut mock = MockAdapter::new("fixture");
        mock.fault = fault;
        let f = Fixture::new(&mock);
        let m = SessionManager::default();
        let result = if matches!(
            fault,
            Fault::ConnectWait | Fault::AuthWait | Fault::IdentityWait
        ) {
            code(f.connect(&m, &short(Stage::Connection)))
        } else {
            let id = f.connect(&m, &control()).unwrap().id.parse().unwrap();
            if matches!(fault, Fault::IdentityWait) {
                code(m.read_identity(
                    id,
                    &f.registry,
                    &f.ctx(),
                    &f.grant(),
                    &short(Stage::Identity),
                ))
            } else {
                code(m.probe(
                    id,
                    &f.registry,
                    &f.ctx(),
                    &f.grant(),
                    &short(Stage::Capabilities),
                ))
            }
        };
        assert!(matches!(
            result,
            AdapterErrorCode::ConnectionTimeout | AdapterErrorCode::OperationTimeout
        ));
        assert_eq!(mock.resources.load(Ordering::Acquire), 0);
        assert_eq!(m.active_count(), 0);
    }
}
#[test]
fn transport_failure_cleans_session_and_preserves_typed_error() {
    let mut mock = MockAdapter::new("fixture");
    mock.fault = Fault::Transport;
    let f = Fixture::new(&mock);
    let m = SessionManager::default();
    assert_eq!(
        code(f.connect(&m, &control())),
        AdapterErrorCode::TransportError
    );
    assert_eq!(mock.resources.load(Ordering::Acquire), 0);
    assert_eq!(m.active_count(), 0);
}
#[test]
fn bounded_concurrency_and_shutdown_release_all_sessions() {
    let mock = MockAdapter::new("fixture");
    let f = Fixture::new(&mock);
    let m = SessionManager::default();
    f.connect(&m, &control()).unwrap();
    f.connect(&m, &control()).unwrap();
    assert_eq!(
        code(f.connect(&m, &control())),
        AdapterErrorCode::TransportError
    );
    assert_eq!(m.active_count(), 2);
    m.shutdown();
    assert_eq!(mock.resources.load(Ordering::Acquire), 0);
    assert_eq!(m.active_count(), 0);
}
#[test]
fn connection_drop_releases_resource_even_if_close_times_out() {
    let mut mock = MockAdapter::new("fixture");
    mock.fault = Fault::CloseWait;
    let f = Fixture::new(&mock);
    let mut c = mock.connect(&f.binding(), &secret(), &control()).unwrap();
    assert_eq!(
        code(c.close(&short(Stage::Disconnection))),
        AdapterErrorCode::OperationTimeout
    );
    drop(c);
    assert_eq!(mock.resources.load(Ordering::Acquire), 0);
}
#[test]
fn saved_address_matching_is_passive_and_never_creates_a_discovery_device() {
    let r = Registry::production()
        .unwrap()
        .evaluate_saved_endpoint(&"a".repeat(64), 1, NOW);
    assert_eq!(r.selection, AdapterSelectionStatus::NoMatch);
    assert!(r.matches.is_empty());
    assert!(r.selected_adapter.is_none());
}
#[test]
fn evaluation_does_not_mutate_existing_sqlite_rows_or_settings() {
    let t = tempfile::tempdir().unwrap();
    let mut repo = crate::storage::Repository::open(&t.path().join("data.db")).unwrap();
    repo.save_settings(&crate::domain::Settings {
        theme: crate::domain::Theme::Dark,
    })
    .unwrap();
    let before = repo.settings().unwrap();
    let f = Fixture::new(&MockAdapter::new("fixture"));
    let _ = f.registry.evaluate(&f.ctx());
    assert_eq!(repo.settings().unwrap(), before);
    assert!(repo.routers().unwrap().is_empty());
}
