use super::*;
#[test]
fn selecting_an_endpoint_cannot_hide_conflicting_trusted_device_evidence() {
    let d = device();
    let now = connection::now();
    let first = record(&d, Attribute::Model, "Model A", now);
    let mut other = record(&d, Attribute::Model, "Model B", now);
    other.endpoint = Some("http://192.168.200.1/".into());
    let result = identity(
        vec![first, other],
        now,
        Some("http://192.168.100.1/"),
        Some("usb-network"),
    );
    assert_eq!(result.confidence, "conflicting");
    assert!(result.verified_model.is_none());
}
#[test]
fn exact_identity_does_not_skip_independent_source_and_export_qualification() {
    let d = device();
    let now = connection::now();
    let identity = identity(
        vec![
            record(&d, Attribute::Manufacturer, "Fixture Vendor", now),
            record(&d, Attribute::Model, "Fixture Model", now),
            record(&d, Attribute::HardwareRevision, "V1", now),
            record(&d, Attribute::FirmwareFamily, "Fixture OS", now),
            record(&d, Attribute::FirmwareVersion, "vendor_opaque_123", now),
        ],
        now,
        None,
        None,
    );
    let candidates = AcquisitionStrategyRegistry::default().evaluate(&identity);
    assert_eq!(candidates.len(), 5);
    assert!(candidates
        .iter()
        .all(|c| c.capability.state == CapabilityState::Unknown && !c.capability.executable));
    assert!(candidates[0]
        .capability
        .missing_requirements
        .iter()
        .any(|r| r.contains("Publisher")));
    assert!(candidates[1]
        .capability
        .missing_requirements
        .iter()
        .any(|r| r.contains("authentication")));
    assert!(candidates[2]
        .capability
        .missing_requirements
        .iter()
        .any(|r| r.contains("ranges")));
    assert!(!candidates[0]
        .capability
        .missing_requirements
        .iter()
        .any(|r| r == "Exact hardware revision"));
}
#[test]
fn discovery_commands_are_registered_in_the_production_acl() {
    let acl: serde_json::Value =
        serde_json::from_str(include_str!("../../capabilities/main.json")).unwrap();
    let permissions = acl["permissions"].as_array().unwrap();
    for (command, permission) in [
        (
            "approve_firmware_route_probe",
            "allow-approve-firmware-route-probe",
        ),
        ("discover_firmware_routes", "allow-discover-firmware-routes"),
        (
            "cancel_firmware_route_discovery",
            "allow-cancel-firmware-route-discovery",
        ),
        (
            "export_acquisition_diagnostics",
            "allow-export-acquisition-diagnostics",
        ),
    ] {
        assert!(permissions.iter().any(|p| p == permission));
        assert!(include_str!("../../build.rs").contains(&format!("\"{command}\"")));
    }
    for forbidden in [
        "allow-execute-command",
        "allow-extract-device-firmware",
        "allow-flash-firmware",
    ] {
        assert!(!permissions.iter().any(|p| p == forbidden));
    }
}
#[test]
fn cancelled_or_superseded_route_lease_cannot_publish_or_release_its_successor() {
    let service = connection::ConnectionService::default();
    let first = service.begin("route").unwrap();
    let next = service.begin("route").unwrap();
    assert!(first.if_current(|| panic!("old route published")).is_none());
    drop(first);
    assert_eq!(next.if_current(|| 7), Some(7));
    service.cancel("route");
    assert!(next.if_current(|| 9).is_none());
    drop(next);
    let after = service.begin("route").unwrap();
    assert_eq!(after.if_current(|| 11), Some(11));
}
pub(super) fn device() -> Device {
    crate::discovery::replay_fixture(include_str!("../../fixtures/discovery-lifecycle.json"))
        .unwrap()[1]
        .devices[0]
        .clone()
}
fn input(d: &Device) -> DiscoveryRequest {
    DiscoveryRequest {
        device_id: d.id.clone(),
        endpoint: None,
        interface_id: None,
        approval_token: None,
        advanced_endpoint: false,
    }
}
fn record(d: &Device, attribute: Attribute, value: &str, now: u64) -> EvidenceRecord {
    // Trusted adapter fixture; manufacturer remains forbidden in the user-report API.
    let mut e =
        EvidenceRecord::user_reported(d.id.clone(), Attribute::Model, value.into(), now).unwrap();
    e.attribute = attribute;
    e.source = EvidenceOrigin::AuthenticatedManagement;
    e.verification = Verification::Verified;
    e.reliability = Reliability::Strong;
    e.endpoint = Some("http://192.168.100.1/".into());
    e.interface_id = Some("usb-network".into());
    e
}
#[test]
fn mg11_no_longer_hits_a_hardcoded_global_failure() {
    let mut d = device();
    d.vendor_id = Some("1286".into());
    d.product_id = Some("4E31".into());
    let evidence = vec![EvidenceRecord::user_reported(
        d.id.clone(),
        Attribute::Model,
        "comFi MG11".into(),
        connection::now(),
    )
    .unwrap()];
    let r = discover(&d, evidence, 4, &input(&d), &AtomicBool::new(false)).unwrap();
    assert!(!r.probe.attempted);
    assert_eq!(r.candidates.len(), 5);
    assert!(!r.can_obtain);
    assert_eq!(r.identity.confidence, "unverified");
    assert_eq!(r.identity.reported_model.as_deref(), Some("comFi MG11"));
    assert!(r.identity.verified_model.is_none());
    assert!(r
        .candidates
        .iter()
        .all(|c| c.capability.state == CapabilityState::Unknown && !c.capability.executable));
    assert_eq!(r.capabilities.len(), 16);
    assert_eq!(
        r.capabilities["deviceDetected"].state,
        CapabilityState::Observed
    );
}
#[test]
fn root_family_signals_never_verify_model_or_export() {
    let families = recognize(
        include_bytes!("../../fixtures/asr-root-redacted.html"),
        None,
    );
    assert_eq!(families.len(), 1);
    assert_eq!(families[0].family, "asr_duster_xml");
    assert_eq!(families[0].confidence, CapabilityState::Candidate);
    assert!(recognize(b"<script>Huawei ZTE VIDA D-Link</script>", None).is_empty());
    assert_eq!(
        recognize(b"<title>OpenWrt</title>", Some("uhttpd")).len(),
        1
    );
    assert!(recognize(&vec![b'x'; 20000], None).is_empty());
}
#[test]
fn stale_conflicting_and_reported_evidence_never_verify_identity() {
    let d = device();
    let now = connection::now();
    let e = vec![
        record(&d, Attribute::Model, "A", now),
        record(&d, Attribute::Model, "B", now),
    ];
    let r = identity(e, now, None, None);
    assert_eq!(r.confidence, "conflicting");
    assert!(r.verified_model.is_none());
    let r = identity(
        vec![record(&d, Attribute::Model, "A", now - 300_001)],
        now,
        None,
        None,
    );
    assert_eq!(r.confidence, "unverified");
    let r = identity(
        vec![record(&d, Attribute::Model, "A", now)],
        now,
        Some("http://192.168.200.1/"),
        Some("usb-network"),
    );
    assert!(r.verified_model.is_none());
    let r = identity(
        vec![record(&d, Attribute::Model, "A", now)],
        now,
        None,
        None,
    );
    assert_eq!(r.verified_model.as_deref(), Some("A"));
}
#[test]
fn hardware_revisions_are_not_interchangeable() {
    let d = device();
    let now = connection::now();
    let r = identity(
        vec![
            record(&d, Attribute::Model, "same model", now),
            record(&d, Attribute::HardwareRevision, "V1", now),
            record(&d, Attribute::HardwareRevision, "V2", now),
        ],
        now,
        None,
        None,
    );
    assert_eq!(r.confidence, "conflicting");
    assert!(r.verified_hardware_revision.is_none());
}
#[test]
fn public_endpoint_disconnect_protected_device_and_cancel_are_rejected() {
    let mut d = device();
    let token = AtomicBool::new(false);
    let mut request = input(&d);
    request.approval_token = Some("test".into());
    request.endpoint = Some("https://8.8.8.8/".into());
    request.interface_id = Some("network".into());
    assert_eq!(
        discover(&d, vec![], 4, &request, &token).unwrap_err().code,
        "INVALID_ENDPOINT"
    );
    d.connected = false;
    assert_eq!(
        discover(&d, vec![], 4, &input(&d), &token)
            .unwrap_err()
            .code,
        "DEVICE_DISCONNECTED"
    );
    d.connected = true;
    let mut protected = d.clone();
    protected.interfaces[0].class_name = "DiskDrive".into();
    assert_eq!(
        discover(&protected, vec![], 4, &input(&protected), &token)
            .unwrap_err()
            .code,
        "PROTECTED_DEVICE"
    );
    token.store(true, Ordering::Release);
    assert_eq!(
        discover(&d, vec![], 4, &input(&d), &token)
            .unwrap_err()
            .code,
        "CANCELLED"
    );
}
#[test]
fn missing_interface_and_mismatched_subject_cannot_probe() {
    let d = device();
    let mut request = input(&d);
    request.approval_token = Some("test".into());
    request.endpoint = Some("http://192.168.100.1/".into());
    assert_eq!(
        discover(&d, vec![], 4, &request, &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "INTERFACE_REQUIRED"
    );
    let mut e = record(&d, Attribute::Model, "other", connection::now());
    e.subject_id = "other-device".into();
    assert_eq!(
        discover(&d, vec![e], 4, &input(&d), &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "INVALID_EVIDENCE"
    );
}
#[test]
fn diagnostics_exclude_endpoint_ids_and_all_raw_peer_and_owner_text() {
    let d = device();
    let e = EvidenceRecord::user_reported(
        d.id.clone(),
        Attribute::Model,
        "private owner label".into(),
        connection::now(),
    )
    .unwrap();
    let r = discover(&d, vec![e], 4, &input(&d), &AtomicBool::new(false)).unwrap();
    let json = diagnostics(&r).to_string();
    assert!(!json.contains(&d.id));
    assert!(!json.contains("private owner label"));
    assert!(!json.contains("endpoint"));
    assert!(!json.contains(&d.display_name));
}

#[test]
fn a1_legacy_boolean_cannot_authorize_network() {
    assert!(serde_json::from_value::<DiscoveryRequest>(
        serde_json::json!({"deviceId":"a","endpoint":null,"interfaceId":null,"authorized":true})
    )
    .is_err());
}
#[test]
fn a2_title_only_is_not_a_protocol_family() {
    assert!(recognize(
        b"<title>ZTE Wireless Router</title><p>unrelated page</p>",
        None
    )
    .is_empty());
    assert!(recognize(b"<title>E-LINK Wireless Router</title>", None).is_empty());
}
#[test]
fn a3_static_version_is_observed_not_authoritative() {
    let d = device();
    let e = version_evidence(
        &d,
        "http://192.168.100.1/",
        "usb",
        b"<firmware_version>REL_MRVL_UAPXC_v2.6.0</firmware_version>",
    )
    .unwrap();
    assert_eq!(e.source, EvidenceOrigin::EndpointResponse);
    assert_eq!(e.verification, Verification::Observed);
    assert_eq!(e.reliability, Reliability::Observed);
    assert!(identity(vec![e], connection::now(), None, None)
        .verified_firmware
        .is_none());
}
#[test]
fn a4_collision_names_other_interface() {
    let mut first = crate::discovery::NetworkAdapter {
        id: "usb".into(),
        display_name: "USB".into(),
        link_up: true,
        ..Default::default()
    };
    first.prefixes.push(crate::discovery::AddressPrefix {
        address: "192.168.100.10".into(),
        length: 24,
    });
    let mut second = first.clone();
    second.id = "wifi".into();
    second.display_name = "Wi-Fi".into();
    assert_eq!(
        connection::subnet_conflict(
            "192.168.100.1".parse().unwrap(),
            &first,
            &[first.clone(), second]
        )
        .unwrap()
        .code,
        "subnet_conflict"
    );
}
#[test]
fn a5_observation_and_unimplemented_are_distinct() {
    let d = device();
    let r = discover(&d, vec![], 4, &input(&d), &AtomicBool::new(false)).unwrap();
    assert_eq!(
        serde_json::to_value(r.capabilities["deviceDetected"].state).unwrap(),
        "observed"
    );
    assert_eq!(
        serde_json::to_value(r.capabilities["installation"].state).unwrap(),
        "not_implemented"
    );
}
#[test]
fn a6_signals_after_metadata_cap_are_ignored() {
    let mut page = vec![b' '; 16384];
    page.extend_from_slice(b"<title>OpenWrt</title>");
    assert!(recognize(&page, Some("uhttpd")).is_empty());
}

#[test]
fn a3_malformed_static_version_cannot_supply_identity() {
    let d = device();
    for b in [
        b"<firmware_version></firmware_version>".as_slice(),
        b"<firmware_version><script>unsafe</script></firmware_version>",
        b"unstructured",
    ] {
        assert!(version_evidence(&d, "http://192.168.100.1/", "usb", b).is_none());
    }
    let observed = version_evidence(
        &d,
        "http://192.168.100.1/",
        "usb",
        b"<firmware_version>REL_MRVL_UAPXC_v2.6.0</firmware_version>",
    )
    .unwrap();
    let reported = EvidenceRecord::user_reported(
        d.id.clone(),
        Attribute::FirmwareVersion,
        "MF69_AM802L_ZX_DD_N_241106_EU".into(),
        connection::now(),
    )
    .unwrap();
    let r = discover(
        &d,
        vec![observed, reported],
        4,
        &input(&d),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(r.identity.evidence.len(), 2);
    assert!(r.identity.verified_firmware.is_none());
    assert!(!r.can_obtain);
}

#[test]
fn collision_diagnostics_do_not_export_interface_names() {
    let d = device();
    let mut report = discover(&d, vec![], 4, &input(&d), &AtomicBool::new(false)).unwrap();
    report.probe.code = "subnet_conflict".into();
    report.probe.message = "Another network: private owner's interface".into();
    assert!(!diagnostics(&report).to_string().contains("private owner"));
}
