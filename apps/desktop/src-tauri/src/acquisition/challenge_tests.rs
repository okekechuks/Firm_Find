//! Synthetic header fixtures only; no captured challenges or physical device traffic.
use super::*;
use challenge::parse_headers;
const NONCE: &str = "d00df00d1234abcdd00df00d1234abcd";
fn raw(value: &str) -> Vec<u8> {
    format!("HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: {value}\r\n\r\n").into_bytes()
}
fn digest() -> String {
    format!("Digest realm=\"Synthetic realm\", nonce=\"{NONCE}\", qop=\"auth\"")
}
#[test]
fn challenge_synthetic_valid_case_space_and_optional_algorithm() {
    for value in [digest(),format!("dIgEsT   REALM = \"Synthetic realm\" , NONCE=\"{NONCE}\", QOP=\"auth, auth-int\", algorithm=\"MD5\"")] {
        let r=parse_headers(401,&raw(&value));
        assert_eq!(r.code,"observed");assert_eq!(r.realm.as_deref(),Some("Synthetic realm"));assert!(r.nonce_present);assert_eq!(r.http_status,Some(401));
        assert_eq!(r.verification,Verification::Observed);assert_eq!(r.reliability,Reliability::Observed);assert!(!serde_json::to_string(&r).unwrap().contains(NONCE));
    }
    assert_eq!(parse_headers(401, &raw(&digest())).algorithm, None);
}
#[test]
fn challenge_synthetic_rejects_ambiguous_malformed_and_oversized_headers() {
    let value = digest();
    let cases=vec![
        b"HTTP/1.1 401 Unauthorized\r\n\r\n".to_vec(),
        raw(&value.replace(", qop=\"auth\"","")),raw("Basic realm=\"synthetic\""),
        raw(&(value.clone()+", Basic realm=\"synthetic\"")),
        format!("HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: {value}\r\nWWW-Authenticate: {value}\r\n\r\n").into_bytes(),
        raw(&value.replace(NONCE,"not-a-hex-nonce")),raw(&value.replace("Synthetic realm",&"x".repeat(65))),
        raw(&value.replace("qop=\"auth\"","qop=auth")),raw(&(value.clone()+", realm=\"duplicate\"")),
        raw(&value.replace("Synthetic realm","bad\tvalue")),raw(&value.replace("auth",&"x".repeat(33))),
        raw(&(value.clone()+", algorithm=\""+&"x".repeat(17)+"\"")),
        raw(&(value.clone()+", opaque=\""+&"x".repeat(2050)+"\"")),
        format!("HTTP/1.1 401 Unauthorized\r\nX-Synthetic: {}\r\nWWW-Authenticate: {value}\r\n\r\n","x".repeat(8192)).into_bytes(),
    ];
    for bytes in cases {
        let r = parse_headers(401, &bytes);
        assert_eq!(r.code, "challenge_not_usable");
        assert!(r.realm.is_none() && r.qop.is_none() && r.algorithm.is_none());
        assert!(!serde_json::to_string(&r).unwrap().contains(NONCE));
    }
}
#[test]
fn challenge_synthetic_status_cookie_other_schemes_and_nonce_privacy() {
    for status in [200, 403, 500, 503, 302] {
        let r = parse_headers(status, &raw(&digest()));
        assert_eq!(
            r.code,
            if status == 302 {
                "redirect_blocked"
            } else {
                "unexpected_status"
            }
        );
        assert!(r.realm.is_none());
    }
    let bytes=format!("HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: {}\r\nSet-Cookie: synthetic_private_cookie=do_not_export\r\nX-Private: do_not_export\r\n\r\n",digest());
    let r = parse_headers(401, bytes.as_bytes());
    assert!(r.set_cookie_present);
    assert_eq!(r.code, "observed");
    let r = parse_headers(401, &raw(&(digest() + ", Basic realm=\"synthetic\"")));
    assert!(r.other_schemes_present);
    assert_eq!(r.code, "challenge_not_usable");
    let r = parse_headers(401, bytes.as_bytes());
    let encoded = serde_json::to_string(&r).unwrap();
    for secret in [
        NONCE,
        "do_not_export",
        "synthetic_private_cookie",
        "X-Private",
    ] {
        assert!(!encoded.contains(secret));
    }
}
#[test]
fn challenge_registry_is_four_closed_specs_and_no_session_operations() {
    let registry = RequestRegistry::default();
    let entries = registry.registered(ProtocolFamilyId::AsrDusterXml);
    assert_eq!(entries.len(), 4);
    assert!(entries
        .iter()
        .any(|spec| spec.canonical_path() == "/login.cgi"));
    for id in [
        RequestId::Login,
        RequestId::Logout,
        RequestId::XmlAction,
        RequestId::StatusRead,
    ] {
        assert!(registry
            .lookup(ProtocolFamilyId::AsrDusterXml, id)
            .is_none());
    }
    let spec = registry
        .lookup(ProtocolFamilyId::AsrDusterXml, RequestId::Challenge)
        .unwrap();
    assert_eq!(spec.max_bytes(), 0);
    assert_eq!(spec.max_header_bytes(), Some(8192));
    assert_eq!(spec.max_auth_header_bytes(), Some(2048));
    assert_eq!(spec.max_attempts(), 1);
    assert_eq!(spec.budget(), std::time::Duration::from_secs(5));
    for path in [
        "/login.cgi?Action=Digest",
        "/xml_action.cgi?method=get&module=duster&file=status1",
        "/xml_action.cgi?Action=logout",
        "/%6cogin.cgi",
        "/login.cgi#fragment",
    ] {
        let url = format!("http://192.168.100.1{path}");
        assert!(registry
            .validate(
                ProtocolFamilyId::AsrDusterXml,
                RequestId::Challenge,
                "http://192.168.100.1/",
                &RequestEnvelope::get(&url)
            )
            .is_err());
    }
    for denied in [
        "set", "upload", "upgrade", "reboot", "restore", "reset", "factory", "download", "delete",
        "format", "flash",
    ] {
        let url = format!("http://192.168.100.1/login.cgi?Action={denied}");
        assert!(registry
            .validate(
                ProtocolFamilyId::AsrDusterXml,
                RequestId::Challenge,
                "http://192.168.100.1/",
                &RequestEnvelope::get(&url)
            )
            .is_err());
    }
}

#[test]
fn challenge_nonce_never_reaches_trace_report_diagnostics_or_errors() {
    #[derive(Clone)]
    struct Captured(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
    impl std::io::Write for Captured {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
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
    let _guard = tracing::subscriber::set_default(subscriber);
    let (d, input, base) = context();
    let proof = AuthChallengeApproval::for_test(&d, 4, &input);
    let r = challenge::collect(
        &base,
        &d,
        4,
        &input,
        &proof,
        &AtomicBool::new(false),
        |_, _, _| parse_headers(401, &raw(&digest())),
    )
    .unwrap();
    for output in [
        serde_json::to_string(&r).unwrap(),
        diagnostics(&r).to_string(),
        String::from_utf8(logs.lock().unwrap().clone()).unwrap(),
    ] {
        assert!(!output.contains(NONCE));
    }
    let error = challenge::collect(
        &base,
        &d,
        5,
        &input,
        &proof,
        &AtomicBool::new(false),
        |_, _, _| panic!("stale proof"),
    )
    .unwrap_err();
    assert!(!format!("{error:?}").contains(NONCE));
}

fn context() -> (Device, DiscoveryRequest, AcquisitionCapabilityReport) {
    let mut d = super::tests::device();
    d.networks[0].gateways = vec!["192.168.100.1".into()];
    let input = DiscoveryRequest {
        device_id: d.id.clone(),
        endpoint: Some("http://192.168.100.1/".into()),
        interface_id: Some(d.networks[0].id.clone()),
        approval_token: None,
        advanced_endpoint: false,
    };
    let mut base = discover_on_networks(
        &d,
        vec![],
        4,
        &input,
        &AtomicBool::new(false),
        &d.networks,
        None,
    )
    .unwrap();
    base.fingerprint.endpoint = input.endpoint.clone();
    base.fingerprint.interface_id = input.interface_id.clone();
    base.probe.attempted = true;
    base.probe.http_status = Some(200);
    base.protocols=recognize(br#"<script src="js/base/xml_helper.js"></script><script src="js/base/ajax_calls.js"></script>"#,None);
    (d, input, base)
}
#[test]
fn challenge_collector_requires_fresh_same_root_and_preserves_authority() {
    let (d, input, base) = context();
    let proof = AuthChallengeApproval::for_test(&d, 4, &input);
    let r = challenge::collect(
        &base,
        &d,
        4,
        &input,
        &proof,
        &AtomicBool::new(false),
        |_, _, _| parse_headers(401, &raw(&digest())),
    )
    .unwrap();
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["authChallenge"]["code"], "observed");
    assert_eq!(
        serde_json::to_value(&r.stages).unwrap(),
        serde_json::to_value(&base.stages).unwrap()
    );
    assert!(!r.can_obtain);
    assert_eq!(r.identity.evidence, base.identity.evidence);
    for altered in 0..5 {
        let mut b = base.clone();
        match altered {
            0 => b.protocols.clear(),
            1 => b.expires_at = 0,
            2 => b.fingerprint.endpoint = Some("http://192.168.100.2/".into()),
            3 => b.fingerprint.interface_id = Some("other".into()),
            _ => b.probe.attempted = false,
        };
        let r = challenge::collect(
            &b,
            &d,
            4,
            &input,
            &proof,
            &AtomicBool::new(false),
            |_, _, _| panic!("no fresh matched root; no dispatch"),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(r).unwrap()["authChallenge"]["code"],
            "no_recognized_family"
        );
    }
    assert!(challenge::collect(
        &base,
        &d,
        4,
        &input,
        &proof,
        &AtomicBool::new(true),
        |_, _, _| panic!("cancelled")
    )
    .is_err());
    assert!(challenge::collect(
        &base,
        &d,
        5,
        &input,
        &proof,
        &AtomicBool::new(false),
        |_, _, _| panic!("stale proof")
    )
    .is_err());
}
#[test]
fn challenge_diagnostics_keep_only_validated_metadata_and_flags() {
    let (_, _, base) = context();
    let mut value = serde_json::to_value(&base).unwrap();
    let observation = parse_headers(401, &raw(&digest()));
    value["authChallenge"] = serde_json::to_value(observation).unwrap();
    let report: AcquisitionCapabilityReport = serde_json::from_value(value.clone()).unwrap();
    let exported = diagnostics(&report);
    assert_eq!(exported["authChallenge"]["realm"], "Synthetic realm");
    assert!(!exported.to_string().contains(NONCE));
    value["authChallenge"]["realm"] = serde_json::json!("bad\nraw_response");
    let report: AcquisitionCapabilityReport = serde_json::from_value(value).unwrap();
    let exported = diagnostics(&report);
    assert!(exported["authChallenge"]["realm"].is_null());
    assert!(!exported.to_string().contains("raw_response"));
}
