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
    assert!(
        candidates
            .iter()
            .all(|c| c.capability.state == CapabilityState::NotImplemented
                && !c.capability.executable)
    );
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
    assert_eq!(
        serde_json::to_value(candidates).unwrap(),
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../fixtures/acquisition-catalogue-schema1.json"
        ))
        .unwrap()
    );
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
    assert!(
        r.candidates
            .iter()
            .all(|c| c.capability.state == CapabilityState::NotImplemented
                && !c.capability.executable)
    );
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
        approval::binding(&d, 4, &request).unwrap_err().code,
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
        approval::binding(&d, 4, &request).unwrap_err().code,
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

#[test]
fn r1_captured_fixture_and_positive_family_candidates() {
    let page = include_bytes!("../../fixtures/asr-root-redacted.html");
    assert_eq!(
        page.len(),
        16384,
        "fixture must be the reviewed captured prefix"
    );
    for body in [page.as_slice(), &page[..16000]] {
        let p = recognize(body, None);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].family, "asr_duster_xml");
        assert_eq!(p[0].confidence, CapabilityState::Candidate);
    }
    let open = recognize(b"<title>OpenWrt</title>", Some("uhttpd"));
    assert_eq!(open[0].family, "openwrt");
    assert_eq!(open[0].confidence, CapabilityState::Candidate);
}
#[test]
fn r2_visible_text_comments_and_script_strings_are_not_resource_attributes() {
    for body in [b"<p>js/base/ajax_calls.js js/base/xml_helper.js</p>".as_slice(),b"<!-- <script src='js/base/ajax_calls.js'></script><script src='js/base/xml_helper.js'></script> -->",b"<script>var example=\"<img src='js/base/ajax_calls.js'><img src='js/base/xml_helper.js'>\";</script>"] {assert!(recognize(body,None).is_empty());}
}
#[test]
fn r8_stray_less_than_keeps_following_resources_candidate_only() {
    let tags = "<script src='js/base/ajax_calls.js'></script><script src='js/base/xml_helper.js'></script>";
    for prefix in ["a < b", "a < 4", "a <", "a < 💡"] {
        let found = recognize(format!("{prefix}{tags}").as_bytes(), None);
        assert_eq!(found.len(), 1, "stray text prefix: {prefix}");
        assert_eq!(found[0].family, "asr_duster_xml");
        assert_eq!(found[0].confidence, CapabilityState::Candidate);
    }
    // R8 must not alter the genuine captured page's candidate evidence.
    let real = include_bytes!("../../fixtures/asr-root-redacted.html");
    let mut prefixed = b"a < b".to_vec();
    prefixed.extend_from_slice(real);
    assert_eq!(
        serde_json::to_value(recognize(real, None)).unwrap(),
        serde_json::to_value(recognize(&prefixed, None)).unwrap()
    );
}
#[test]
fn r3_network_gate_cannot_rely_on_token_presence() {
    assert!(!include_str!("mod.rs").contains("if input.approval_token.is_some()"));
}
#[test]
fn r5_busy_admission_precedes_claim() {
    let src = include_str!("ipc.rs");
    assert!(src.find(".begin(").unwrap() < src.find(".claim(").unwrap());
}
#[test]
fn r5_confirmation_values_are_single_line_and_capped() {
    let value = super::approval::confirmation_value(&format!("bad\r\n\0\t{}", "x".repeat(200)));
    assert!(value.chars().count() <= 80);
    assert!(!value.chars().any(char::is_control));
}

#[test]
fn r3_bogus_token_without_claimed_proof_is_local_only() {
    let d = device();
    let mut request = input(&d);
    request.approval_token = Some("forged".into());
    request.endpoint = Some("http://192.168.100.1/".into());
    let result = discover(&d, vec![], 4, &request, &AtomicBool::new(false)).unwrap();
    assert!(!result.probe.attempted);
    type Probe = fn(
        &ClaimedApproval,
        &connection::Target,
        &str,
        &[crate::discovery::NetworkAdapter],
        &AtomicBool,
    ) -> (connection::EndpointResult, Option<Vec<u8>>);
    let _proof_required: Probe = connection::root_metadata;
}
#[test]
fn r5_busy_does_not_consume_receipt() {
    let mut d = device();
    d.networks[0].gateways = vec!["192.168.100.1".into()];
    let mut request = input(&d);
    request.endpoint = Some("http://192.168.100.1/".into());
    request.interface_id = Some(d.networks[0].id.clone());
    let service = Service::default();
    let binding = approval::binding(&d, 4, &request).unwrap();
    request.approval_token = Some(service.1.issue(binding).unwrap());
    let connections = connection::ConnectionService::default();
    let first = connections.begin("held-one").unwrap();
    let second = connections.begin("held-two").unwrap();
    assert!(matches!(
        ipc::reserve_probe(&connections, &service, &d, 4, &request),
        Err(DiscoveryError { code: "BUSY", .. })
    ));
    drop(first);
    drop(second);
    let (_lease, proof) = ipc::reserve_probe(&connections, &service, &d, 4, &request).unwrap();
    assert!(proof.is_some());
}

#[test]
fn captured_static_version_with_xml_declaration_and_rgw_wrapper_is_observed() {
    let d = device();
    let e = version_evidence(
        &d,
        "http://192.168.100.1/",
        "usb",
        include_bytes!("../../fixtures/asr-version-redacted.xml"),
    )
    .unwrap();
    assert_eq!(e.value, "REL_MRVL_UAPXC_v2.6.0");
    assert_eq!(e.verification, Verification::Observed);
    assert_eq!(e.reliability, Reliability::Observed);
    assert!(identity(vec![e], connection::now(), None, None)
        .verified_firmware
        .is_none());
}

#[test]
fn b1_compiled_strategy_registry_runs_only_unimplemented_probes() {
    use super::strategy_api::StrategyProbeApproval;
    use super::{AcquisitionStrategyRegistry, ProbeContext, StrategyId, StrategyKind};
    let registry = AcquisitionStrategyRegistry::default();
    let strategies = registry.strategies().collect::<Vec<_>>();
    assert_eq!(strategies.len(), 5);
    assert_eq!(
        strategies.iter().map(|s| s.id()).collect::<Vec<_>>(),
        vec![
            StrategyId::OfficialImage,
            StrategyId::DocumentedExport,
            StrategyId::AuthorizedStorageRead,
            StrategyId::ManufacturerPackage,
            StrategyId::ExternalLaboratory,
        ]
    );
    assert_eq!(
        strategies.iter().map(|s| s.kind()).collect::<Vec<_>>(),
        vec![
            StrategyKind::OfficialImage,
            StrategyKind::DeviceExport,
            StrategyKind::StorageRead,
            StrategyKind::ManufacturerPackage,
            StrategyKind::ExternalLaboratory,
        ]
    );
    let mut context = ProbeContext::new();
    for strategy in strategies {
        let approval = StrategyProbeApproval::for_test(strategy.id());
        let report = strategy.probe(&mut context, &approval).unwrap();
        assert_eq!(report.state, CapabilityState::NotImplemented);
        assert!(!report.executable);
        let wrong_scope = StrategyProbeApproval::for_test(StrategyId::OfficialImage);
        if strategy.id() != StrategyId::OfficialImage {
            assert_eq!(
                strategy.probe(&mut context, &wrong_scope),
                Err(super::StrategyError::ApprovalScopeMismatch)
            );
        }
    }
    assert!(!registry.can_obtain());
}

#[test]
fn b1_metadata_approval_cannot_be_used_for_a_strategy_probe() {
    use super::{approval::ApprovalScope, StrategyId};
    let mut d = device();
    d.networks[0].gateways = vec!["192.168.100.1".into()];
    let mut request = input(&d);
    request.endpoint = Some("http://192.168.100.1/".into());
    request.interface_id = Some(d.networks[0].id.clone());
    let binding = approval::binding(&d, 4, &request).unwrap();
    let approvals = approval::Approvals::default();
    let token = approvals.issue(binding.clone()).unwrap();
    let proof = approvals.claim(&token, &binding).unwrap();
    assert_eq!(proof.scope(), ApprovalScope::EndpointMetadata);
    assert!(proof.strategy_probe(StrategyId::OfficialImage).is_err());
}

#[test]
fn b1_reported_or_observed_values_never_satisfy_verified_requirements() {
    use super::{EvaluationContext, Requirement};
    let d = device();
    let now = connection::now();
    let reported = identity(
        vec![EvidenceRecord::user_reported(
            d.id.clone(),
            Attribute::Model,
            "same model".into(),
            now,
        )
        .unwrap()],
        now,
        None,
        None,
    );
    let requirement = Requirement::VerifiedModel;
    assert!(!requirement.is_satisfied(&EvaluationContext::new(&reported)));

    let mut observed = record(&d, Attribute::Model, "same model", now);
    observed.source = EvidenceOrigin::EndpointResponse;
    observed.verification = Verification::Observed;
    observed.reliability = Reliability::Observed;
    let assessment = identity(vec![observed], now, None, None);
    assert!(!requirement.is_satisfied(&EvaluationContext::new(&assessment)));

    let verified_except_revision = identity(
        vec![
            record(&d, Attribute::Manufacturer, "Fixture Vendor", now),
            record(&d, Attribute::Model, "Fixture Model", now),
            record(&d, Attribute::FirmwareFamily, "Fixture OS", now),
            record(&d, Attribute::FirmwareVersion, "vendor_opaque_123", now),
        ],
        now,
        None,
        None,
    );
    let registry = AcquisitionStrategyRegistry::default();
    let route = registry.get(StrategyId::DocumentedExport).unwrap();
    let plan = route
        .plan(
            &EvaluationContext::new(&verified_except_revision),
            &ProbeReport::not_implemented(StrategyId::DocumentedExport),
        )
        .unwrap();
    assert!(plan
        .missing
        .contains(&Requirement::VerifiedHardwareRevision));
    assert!(!plan.executable);
}

#[test]
fn b1_plan_rejects_stale_mixed_endpoint_changed_generation_and_conflicting_evidence() {
    use super::{AcquisitionStrategyRegistry, EvaluationContext, ProbeReport, StrategyId};
    let registry = AcquisitionStrategyRegistry::default();
    let strategy = registry.get(StrategyId::OfficialImage).unwrap();
    let d = device();
    let now = connection::now();
    let verified = identity(
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
    let probe = ProbeReport::not_implemented(StrategyId::OfficialImage);
    let stale = EvaluationContext::new(&verified).with_generation(9, 8);
    assert!(!strategy.plan(&stale, &probe).unwrap().executable);

    let mut other_endpoint = record(&d, Attribute::Model, "Fixture Model", now);
    other_endpoint.endpoint = Some("http://192.168.200.1/".into());
    let mixed = identity(
        vec![
            record(&d, Attribute::Model, "Fixture Model", now),
            other_endpoint,
        ],
        now,
        None,
        None,
    );
    assert!(
        !strategy
            .plan(&EvaluationContext::new(&mixed), &probe)
            .unwrap()
            .executable
    );

    let conflicting = identity(
        vec![
            record(&d, Attribute::Model, "Fixture Model", now),
            record(&d, Attribute::Model, "Other Model", now),
        ],
        now,
        None,
        None,
    );
    assert!(
        !strategy
            .plan(&EvaluationContext::new(&conflicting), &probe)
            .unwrap()
            .executable
    );

    let mut stale_record = record(&d, Attribute::Model, "Fixture Model", now);
    stale_record.stale = true;
    let expired = identity(vec![stale_record], now, None, None);
    assert!(
        !strategy
            .plan(&EvaluationContext::new(&expired), &probe)
            .unwrap()
            .executable
    );
}

#[test]
fn b3_compiled_request_registry_accepts_only_the_four_closed_reads() {
    use super::request_registry::{ProtocolFamilyId, RequestEnvelope, RequestId, RequestRegistry};
    let registry = RequestRegistry::default();
    let specs = registry.registered(ProtocolFamilyId::AsrDusterXml);
    assert_eq!(specs.len(), 4);
    assert_eq!(
        specs.iter().map(|spec| spec.id()).collect::<Vec<_>>(),
        vec![
            RequestId::Root,
            RequestId::StaticVersion,
            RequestId::StaticMenu,
            RequestId::Challenge
        ]
    );
    for spec in specs {
        let url = format!("http://192.168.100.1{}", spec.canonical_path());
        assert!(registry
            .validate(
                ProtocolFamilyId::AsrDusterXml,
                spec.id(),
                "http://192.168.100.1/",
                &RequestEnvelope::get(&url),
            )
            .is_ok());
    }
    assert!(registry
        .lookup(ProtocolFamilyId::AsrDusterXml, RequestId::Login)
        .is_none());
    assert!(registry
        .lookup(ProtocolFamilyId::AsrDusterXml, RequestId::Logout)
        .is_none());
    assert!(registry
        .lookup(ProtocolFamilyId::AsrDusterXml, RequestId::XmlAction)
        .is_none());
}

#[test]
fn b3_validator_rejects_every_unregistered_request_shape() {
    use super::request_registry::{ProtocolFamilyId, RequestEnvelope, RequestId, RequestRegistry};
    let registry = RequestRegistry::default();
    let root = "http://192.168.100.1/";
    let rejected = [
        RequestEnvelope {
            method: "POST",
            url: root,
            body: None,
            redirect: None,
        },
        RequestEnvelope {
            method: "GET",
            url: root,
            body: Some(b""),
            redirect: None,
        },
        RequestEnvelope::get("http://192.168.100.1/other"),
        RequestEnvelope::get("http://192.168.100.1/?caller=1"),
        RequestEnvelope::get("http://192.168.100.1/%2fetc"),
        RequestEnvelope::get("http://192.168.100.1/%252e%252e/version_num.txt"),
        RequestEnvelope::get("http://192.168.100.1/../version_num.txt"),
        RequestEnvelope::get("http://owner@192.168.100.1/"),
        RequestEnvelope::get("http://192.168.100.1/#fragment"),
        RequestEnvelope::get("ftp://192.168.100.1/"),
        RequestEnvelope::get("http://192.168.100.2/"),
        RequestEnvelope {
            method: "GET",
            url: root,
            body: None,
            redirect: Some("http://192.168.100.2/"),
        },
    ];
    for request in rejected {
        assert!(
            registry
                .validate(
                    ProtocolFamilyId::AsrDusterXml,
                    RequestId::Root,
                    root,
                    &request
                )
                .is_err(),
            "accepted {request:?}"
        );
    }
    for denied in [
        "set", "upload", "upgrade", "reboot", "restore", "reset", "factory", "download", "delete",
        "format", "flash",
    ] {
        let url = format!("{root}?Action={denied}");
        assert!(
            registry
                .validate(
                    ProtocolFamilyId::AsrDusterXml,
                    RequestId::Root,
                    root,
                    &RequestEnvelope::get(&url),
                )
                .is_err(),
            "accepted denied term {denied}"
        );
    }
    assert!(registry
        .validate(
            ProtocolFamilyId::AsrDusterXml,
            RequestId::StaticVersion,
            "http://192.168.100.1/",
            &RequestEnvelope::get("http://different-host/ version_num.txt"),
        )
        .is_err());
}

#[test]
fn b6_report_has_eleven_ordered_stages_and_identity_does_not_advance_them() {
    let d = device();
    let unknown_report = discover(&d, vec![], 4, &input(&d), &AtomicBool::new(false)).unwrap();
    assert_eq!(unknown_report.stages.len(), 11);
    assert_eq!(
        unknown_report
            .stages
            .iter()
            .map(|stage| stage.id)
            .collect::<Vec<_>>(),
        vec![
            CompatibilityStageId::Detected,
            CompatibilityStageId::ExactIdentity,
            CompatibilityStageId::ManagementApi,
            CompatibilityStageId::OfficialImage,
            CompatibilityStageId::FirmwareExport,
            CompatibilityStageId::PhysicalRead,
            CompatibilityStageId::FirmwareAcquired,
            CompatibilityStageId::FilesystemExtracted,
            CompatibilityStageId::Customization,
            CompatibilityStageId::Rebuild,
            CompatibilityStageId::Installation,
        ]
    );
    assert!(unknown_report
        .stages
        .iter()
        .all(|stage| !matches!(stage.state, CompatibilityStageState::NotSupported)));
    let now = connection::now();
    let verified_identity = identity(
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
    let updated = compatibility_stages(
        &verified_identity,
        &unknown_report.probe,
        &unknown_report.capabilities,
        &unknown_report.protocols,
    );
    assert_eq!(updated[1].state, CompatibilityStageState::Verified);
    for (index, (updated_stage, original_stage)) in
        updated.iter().zip(&unknown_report.stages).enumerate()
    {
        if index != 1 {
            assert_eq!(updated_stage.state, original_stage.state);
        }
    }
}

#[test]
fn acquisition_model_selection_does_not_depend_on_user_label() {
    let d = device();
    let now = connection::now();
    let assessment = identity(
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
    let registry = AcquisitionStrategyRegistry::default();
    let strategy = registry.get(StrategyId::OfficialImage).unwrap();
    let first = EvaluationContext::new(&assessment).with_user_label("Owner name one");
    let second = EvaluationContext::new(&assessment).with_user_label("Different owner name");
    assert_eq!(
        strategy.applicability(&first),
        strategy.applicability(&second)
    );
}

#[test]
fn acquisition_runtime_has_no_hardware_model_name_special_cases() {
    assert!(
        runtime_model_name_matches().is_empty(),
        "{:?}",
        runtime_model_name_matches()
    );
}

fn runtime_model_name_matches() -> Vec<String> {
    use std::{fs, path::Path};

    fn collect_rs_files(directory: &Path, files: &mut Vec<std::path::PathBuf>) {
        for entry in fs::read_dir(directory).expect("acquisition source directory is readable") {
            let path = entry.expect("source directory entry is readable").path();
            if path.is_dir() {
                if !path
                    .components()
                    .any(|part| part.as_os_str() == "fixtures" || part.as_os_str() == "tests")
                {
                    collect_rs_files(&path, files);
                }
            } else if path.extension().is_some_and(|extension| extension == "rs")
                && path.file_name().is_some_and(|name| name != "tests.rs")
            {
                files.push(path);
            }
        }
    }

    fn without_test_only_items(source: &str) -> String {
        let lines: Vec<_> = source.lines().collect();
        let mut output = String::new();
        let mut index = 0;
        while index < lines.len() {
            if lines[index].trim() != "#[cfg(test)]" {
                output.push_str(lines[index]);
                output.push('\n');
                index += 1;
                continue;
            }
            index += 1;
            if index >= lines.len() {
                break;
            }
            let item = lines[index];
            if item.trim_end().ends_with(';') {
                index += 1;
                continue;
            }
            let mut depth = 0_i32;
            let mut started = false;
            while index < lines.len() {
                let line = lines[index];
                for byte in line.bytes() {
                    if byte == b'{' {
                        depth += 1;
                        started = true;
                    } else if byte == b'}' {
                        depth -= 1;
                    }
                }
                index += 1;
                if started && depth == 0 {
                    break;
                }
            }
        }
        output
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/acquisition");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    let model_names: Vec<_> = include_str!("fixtures/hardware_model_names.txt")
        .lines()
        .map(str::trim)
        .filter(|name| !name.is_empty() && !name.starts_with('#'))
        .map(str::to_lowercase)
        .collect();
    let mut found = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path).expect("runtime acquisition source is UTF-8");
        let runtime_source = without_test_only_items(&source).to_lowercase();
        for model in &model_names {
            if runtime_source.contains(model) {
                found.push(format!("{} contains {model}", path.display()));
            }
        }
    }
    found
}

#[test]
fn static_version_xml_rejects_entities_duplicates_and_unbalanced_wrappers() {
    let d = device();
    for body in [b"<!DOCTYPE RGW [<!ENTITY e SYSTEM 'file:///private'>]><RGW><firmware_version>&e;</firmware_version></RGW>".as_slice(),b"<RGW><firmware_version>A</firmware_version><firmware_version>B</firmware_version></RGW>",b"<RGW><firmware_version>A</firmware_version>",b"<?xml version='1.0'?><RGW><firmware_version>&evil;</firmware_version></RGW>"] {assert!(version_evidence(&d,"http://192.168.100.1/","usb",body).is_none());}
}

#[test]
fn review_f1_default_cannot_qualify_from_identity_alone() {
    struct Empty;
    impl AcquisitionStrategy for Empty {
        fn id(&self) -> StrategyId {
            StrategyId::OfficialImage
        }
        fn kind(&self) -> StrategyKind {
            StrategyKind::OfficialImage
        }
        fn qualified_requirements(&self) -> &'static [&'static str] {
            &[]
        }
    }
    let d = device();
    let now = connection::now();
    let identity = identity(
        [
            Attribute::Manufacturer,
            Attribute::FirmwareFamily,
            Attribute::Model,
            Attribute::HardwareRevision,
            Attribute::FirmwareVersion,
        ]
        .into_iter()
        .map(|a| record(&d, a, "fixture", now))
        .collect(),
        now,
        None,
        None,
    );
    assert_eq!(
        Empty
            .applicability(&EvaluationContext::new(&identity))
            .state,
        CapabilityState::Unknown
    );
}
#[test]
fn review_f2_proof_constructor_is_not_a_bare_scope_factory() {
    assert!(!include_str!("strategy_api.rs").contains("fn from_scope("));
    assert!(include_str!("approval.rs").contains("compile_fail,E0451"));
}
#[test]
fn review_f3_candidates_report_missing_implementation() {
    let d = device();
    let r = discover(&d, vec![], 4, &input(&d), &AtomicBool::new(false)).unwrap();
    assert!(
        r.candidates
            .iter()
            .all(|c| c.capability.state == CapabilityState::NotImplemented
                && !c.capability.executable)
    );
}
#[test]
fn review_f4_web_response_requires_family_for_management_observation() {
    let d = device();
    for status in [200, 404, 500] {
        let mut r = discover(&d, vec![], 4, &input(&d), &AtomicBool::new(false)).unwrap();
        r.probe.attempted = true;
        r.probe.http_status = Some(status);
        let stages = compatibility_stages(&r.identity, &r.probe, &r.capabilities, &r.protocols);
        assert_eq!(stages[2].state, CompatibilityStageState::Unknown);
        assert_eq!(
            stages[2].reason,
            "A web server answered; no management family recognized"
        );
        let families = recognize(
            include_bytes!("../../fixtures/asr-root-redacted.html"),
            None,
        );
        let stages = compatibility_stages(&r.identity, &r.probe, &r.capabilities, &families);
        assert_eq!(stages[2].state, CompatibilityStageState::Observed);
    }
}
#[test]
fn review_f5_private_field_doctest_pins_error() {
    assert!(include_str!("request_registry.rs").contains("compile_fail,E0451"));
}
