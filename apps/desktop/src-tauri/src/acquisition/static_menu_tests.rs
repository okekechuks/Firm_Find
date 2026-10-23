//! Synthetic menu fixtures only. No captured device menu or physical traffic.
use super::*;

#[test]
fn menu_synthetic_shapes_and_exact_discriminator() {
    for xml in [
        r#"<?xml version="1.0" encoding="UTF-8"?><Root><Tab Name="D" type="submenuabsent" implFunction="objdashboard" xmlName="synthetic_dashboard"/></Root>"#,
        r#"<Root><Tab Name="Dashboard" type="submenuabsent" implFunction="objdashboard" xmlName="synthetic_dashboard"/></Root>"#,
        r#"<Root><Tab Name="Dashboard" type="submenu"><Menu id="synthetic" implFunction="objdashboard" xmlName="synthetic_dashboard"/></Tab></Root>"#,
    ] {
        assert_eq!(
            static_menu::parse_menu(xml.as_bytes()).as_deref(),
            Some("synthetic_dashboard")
        );
    }
}
#[test]
fn menu_synthetic_rejects_ambiguous_entities_namespaces_and_tokens() {
    for xml in [
        "<Root/>",
        "<Root>",
        "<!DOCTYPE Root><Root/>",
        "<!ENTITY e 'x'><Root/>",
        r#"<Root xmlns="synthetic"><Tab Name="D" type="submenuabsent" implFunction="objdashboard" xmlName="synthetic"/></Root>"#,
    ] {
        assert!(static_menu::parse_menu(xml.as_bytes()).is_none());
    }
    let base = r#"<Root><Tab Name="D" type="submenuabsent" implFunction="objdashboard" xmlName="TOKEN"/></Root>"#;
    for token in [
        "",
        "a/b",
        "../x",
        "a?x",
        "a#x",
        "%61",
        "a b",
        "a&amp;b",
        "x.cgi",
        "é",
        "Action=Download",
    ] {
        assert!(static_menu::parse_menu(base.replace("TOKEN", token).as_bytes()).is_none());
    }
    assert!(
        static_menu::parse_menu(base.replace("objdashboard", "Objdashboard").as_bytes()).is_none()
    );
    assert!(static_menu::parse_menu(base.replace(" xmlName=\"TOKEN\"", "").as_bytes()).is_none());
    assert!(static_menu::parse_menu(base.replace("TOKEN", &"x".repeat(65)).as_bytes()).is_none());
    let duplicate=base.replace("</Root>", r#"<Tab Name="Other" type="submenu"><Menu id="s" implFunction="objdashboard" xmlName="other"/></Tab></Root>"#);
    assert!(static_menu::parse_menu(duplicate.as_bytes()).is_none());
    assert!(static_menu::parse_menu(&vec![b'x'; 16385]).is_none());
}
#[test]
fn menu_closed_table_registers_only_third_static_get() {
    let registry = RequestRegistry::default();
    let entries = registry.registered(ProtocolFamilyId::AsrDusterXml);
    assert_eq!(entries.len(), 3);
    let spec = registry
        .lookup(ProtocolFamilyId::AsrDusterXml, RequestId::StaticMenu)
        .unwrap();
    assert_eq!(spec.canonical_path(), "/xml/ui_mifi.xml");
    assert_eq!(spec.max_bytes(), 16384);
    assert_eq!(spec.response_types(), &[ResponseType::Xml]);
}

#[test]
fn menu_wrong_media_type_limitation_exports_only_sanitized_type() {
    let (d, input, base) = synthetic_context();
    for (raw, expected) in [
        (Some("text/plain"), "text/plain"),
        (Some("text/html"), "text/html"),
        (Some("text/html/private@secret"), "unavailable"),
        (
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            "unavailable",
        ),
        (None, "unavailable"),
    ] {
        let proof = StaticMenuApproval::for_test(&d, 4, &input);
        let report = static_menu::collect(
            &base,
            &d,
            4,
            &input,
            &proof,
            &AtomicBool::new(false),
            |_, _, _| {
                let mut result = serde_json::to_value(connection::EndpointResult::failure(
                    "wrong_content_type",
                    "private_response_message",
                ))
                .unwrap();
                result["observedMediaType"] = serde_json::json!(raw);
                result["server"] = serde_json::json!("private_server");
                (
                    serde_json::from_value(result).unwrap(),
                    Some(b"private_body".to_vec()),
                )
            },
        )
        .unwrap();
        let encoded = serde_json::to_value(&report).unwrap();
        assert_eq!(encoded["staticMenu"]["observedMediaType"], expected);
        let exported = diagnostics(&report);
        assert_eq!(
            exported["staticMenu"],
            serde_json::json!({"code":"wrong_content_type", "observedMediaType":expected})
        );
        let json = exported.to_string();
        for secret in [
            "private_response_message",
            "private_server",
            "private_body",
            "private@secret",
            "192.168.100.1",
            &d.id,
        ] {
            assert!(!json.contains(secret));
        }
        assert!(!report.can_obtain);
    }
}

fn synthetic_context() -> (Device, DiscoveryRequest, AcquisitionCapabilityReport) {
    let mut d = super::tests::device();
    d.networks[0].gateways = vec!["192.168.100.1".into()];
    let input = DiscoveryRequest {
        device_id: d.id.clone(),
        endpoint: Some("http://192.168.100.1/".into()),
        interface_id: Some(d.networks[0].id.clone()),
        approval_token: None,
        advanced_endpoint: false,
    };
    let mut r = discover_on_networks(
        &d,
        vec![],
        4,
        &input,
        &AtomicBool::new(false),
        &d.networks,
        None,
    )
    .unwrap();
    r.fingerprint.endpoint = input.endpoint.clone();
    r.fingerprint.interface_id = input.interface_id.clone();
    r.protocols=recognize(br#"<script src="js/base/xml_helper.js"></script><script src="js/base/ajax_calls.js"></script>"#,None);
    (d, input, r)
}
#[test]
fn menu_dispatch_observation_redaction_and_no_stage_authority() {
    #[derive(Clone)]
    struct Captured(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
    impl std::io::Write for Captured {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let logs = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let writer = logs.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_max_level(tracing::Level::TRACE)
        .with_writer(move || Captured(writer.clone()))
        .finish();
    let _subscriber = tracing::subscriber::set_default(subscriber);

    let (d, input, base) = synthetic_context();
    let proof = StaticMenuApproval::for_test(&d, 4, &input);
    let report=static_menu::collect(&base,&d,4,&input,&proof,&AtomicBool::new(false),|_,_,_| {
        let mut r=*connection::EndpointResult::failure("identity_unknown", "synthetic");r.status=Some(200);
        (r,Some(br#"<Root><private>synthetic_secret_not_exported</private><Tab Name="D" type="submenuabsent" implFunction="objdashboard" xmlName="synthetic_dashboard"/></Root>"#.to_vec()))
    }).unwrap();
    let menu = report.static_menu.as_ref().unwrap();
    assert_eq!(menu.file_name.as_deref(), Some("synthetic_dashboard"));
    assert_eq!(menu.source, EvidenceOrigin::EndpointResponse);
    assert_eq!(menu.verification, Verification::Observed);
    assert_eq!(menu.reliability, Reliability::Observed);
    assert!(!serde_json::to_string(&report)
        .unwrap()
        .contains("synthetic_secret_not_exported"));
    let exported = diagnostics(&report).to_string();
    assert!(!exported.contains("synthetic_dashboard"));
    assert!(!exported.contains("synthetic_secret_not_exported"));
    let recorded = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
    assert!(!recorded.contains("synthetic_secret_not_exported"));
    assert!(!recorded.contains("<Root>"));
    assert_eq!(
        serde_json::to_value(&report.stages).unwrap(),
        serde_json::to_value(&base.stages).unwrap()
    );
    assert!(!report.can_obtain);
    assert_eq!(report.identity.evidence, base.identity.evidence);
}
#[test]
fn menu_dispatch_stops_on_failures_cancellation_and_stale_input() {
    let (d, input, base) = synthetic_context();
    let proof = StaticMenuApproval::for_test(&d, 4, &input);
    for (status, code, expected) in [
        (
            Some(401),
            "authentication_required",
            "menu_not_readable_without_login",
        ),
        (Some(403), "authentication_required", "forbidden"),
        (Some(302), "redirect_blocked", "redirect_blocked"),
        (Some(500), "unsupported_management", "transport_failed"),
        (None, "wrong_content_type", "wrong_content_type"),
        (None, "metadata_too_large", "metadata_too_large"),
        (None, "timeout", "timeout"),
    ] {
        let r = static_menu::collect(
            &base,
            &d,
            4,
            &input,
            &proof,
            &AtomicBool::new(false),
            |_, _, _| {
                let mut result = *connection::EndpointResult::failure(code, "not exported");
                result.status = status;
                (result, Some(b"not saved".to_vec()))
            },
        )
        .unwrap();
        assert_eq!(r.static_menu.as_ref().unwrap().code, expected);
        assert!(r.static_menu.unwrap().file_name.is_none());
    }
    let token = AtomicBool::new(true);
    assert!(
        static_menu::collect(&base, &d, 4, &input, &proof, &token, |_, _, _| panic!(
            "cancelled dispatch"
        ))
        .is_err()
    );
    let mut stale = base.clone();
    stale.expires_at = 0;
    assert!(static_menu::collect(
        &stale,
        &d,
        4,
        &input,
        &proof,
        &AtomicBool::new(false),
        |_, _, _| panic!("stale dispatch")
    )
    .is_err());
    assert!(static_menu::collect(
        &base,
        &d,
        5,
        &input,
        &proof,
        &AtomicBool::new(false),
        |_, _, _| panic!("changed revision")
    )
    .is_err());
    let token = AtomicBool::new(false);
    assert!(
        static_menu::collect(&base, &d, 4, &input, &proof, &token, |_, _, _| {
            token.store(true, Ordering::Release);
            (
                *connection::EndpointResult::failure("cancelled", "stopped"),
                None,
            )
        })
        .is_err()
    );
    let mut unrecognized = base.clone();
    unrecognized.protocols.clear();
    let r = static_menu::collect(
        &unrecognized,
        &d,
        4,
        &input,
        &proof,
        &AtomicBool::new(false),
        |_, _, _| panic!("no family traffic"),
    )
    .unwrap();
    assert_eq!(r.static_menu.unwrap().code, "no_recognized_family");
}
#[test]
fn menu_validator_rejects_cgi_and_all_unregistered_paths() {
    let r = RequestRegistry::default();
    for path in [
        "/login.cgi",
        "/logout",
        "/xml_action.cgi?Action=Download",
        "/xml/ui_mifi.xml?file=x",
        "/xml/%75i_mifi.xml",
        "/xml/../ui_mifi.xml",
    ] {
        assert!(r
            .validate(
                ProtocolFamilyId::AsrDusterXml,
                RequestId::StaticMenu,
                "http://192.168.100.1/",
                &RequestEnvelope::get(&format!("http://192.168.100.1{path}"))
            )
            .is_err());
    }
    for id in [RequestId::Login, RequestId::Logout, RequestId::XmlAction] {
        assert!(r.lookup(ProtocolFamilyId::AsrDusterXml, id).is_none());
    }
}

#[test]
fn menu_malformed_xml_names_are_not_observations() {
    for xml in [
        r#"<Root>bad]]><Tab Name="D" type="submenuabsent" implFunction="objdashboard" xmlName="synthetic"/></Root>"#,
        r#"<Root bad="<"><Tab Name="D" type="submenuabsent" implFunction="objdashboard" xmlName="synthetic"/></Root>"#,
        r#"<!--synthetic--><?xml version="1.0"?><Root><Tab Name="D" type="submenuabsent" implFunction="objdashboard" xmlName="synthetic"/></Root>"#,
        r#"<?xml version="1.0" version="1.1"?><Root><Tab Name="D" type="submenuabsent" implFunction="objdashboard" xmlName="synthetic"/></Root>"#,
    ] {
        assert!(static_menu::parse_menu(xml.as_bytes()).is_none());
    }

    for xml in [
        r#"<1><Tab Name="D" type="submenuabsent" implFunction="objdashboard" xmlName="synthetic"/></1>"#,
        r#"<Root 1="invalid"><Tab Name="D" type="submenuabsent" implFunction="objdashboard" xmlName="synthetic"/></Root>"#,
    ] {
        assert!(static_menu::parse_menu(xml.as_bytes()).is_none());
    }
}

#[test]
fn menu_commands_are_closed_and_parser_has_no_request_authority() {
    let acl: serde_json::Value =
        serde_json::from_str(include_str!("../../capabilities/main.json")).unwrap();
    for (command, permission) in [
        ("approve_static_menu", "allow-approve-static-menu"),
        ("read_static_menu", "allow-read-static-menu"),
    ] {
        assert!(acl["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == permission));
        assert!(include_str!("../../build.rs").contains(command));
    }
    let (d, mut input, base) = synthetic_context();
    let proof = StaticMenuApproval::for_test(&d, 4, &input);
    input.endpoint = Some("http://192.168.100.1/login.cgi".into());
    assert!(static_menu::collect(
        &base,
        &d,
        4,
        &input,
        &proof,
        &AtomicBool::new(false),
        |_, _, _| panic!("CGI dispatch")
    )
    .is_err());
}
