//! Entirely synthetic captures; no owner device data or traffic.
use super::*;
use serde_json::{json, Value};

fn har(entries: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&json!({"log":{"version":"1.2","entries":entries}})).unwrap()
}
fn entry(method: &str, url: &str) -> Value {
    json!({"request":{"method":method,"url":url,"headers":[]},"response":{"status":200,"content":{"mimeType":"application/json","size":42},"headers":[]}})
}

#[test]
fn synthetic_har_get_reset_is_dangerous_and_not_read_qualified() {
    let report = parse_har(
        "synthetic-device",
        &har(vec![entry(
            "GET",
            "http://192.168.100.1/api/reset?confirm=yes",
        )]),
    )
    .unwrap();
    assert_eq!(
        report.entries[0].classification,
        Classification::FactoryReset
    );
    assert!(!report.entries[0].read_only_qualified);
    assert!(!report.can_acquire);
}

#[test]
fn synthetic_har_secrets_body_values_headers_queries_and_identifiers_are_absent() {
    let mut item = entry("POST", "http://admin:password-in-userinfo@192.168.100.1/login.cgi?password=secret-password&csrf=secret-csrf&token=secret-token&imei=123456789012345");
    item["request"]["headers"] = json!([{"name":"Authorization","value":"Basic secret-auth"},{"name":"Cookie","value":"session=secret-cookie"},{"name":"X-CSRF-Token","value":"secret-header"},{"name":"Content-Type","value":"application/json; token=secret-mime"}]);
    item["request"]["postData"] = json!({"mimeType":"application/json","text":"{\"username\":\"admin\",\"password\":\"secret-body\",\"imei\":\"123456789012345\",\"safe_unknown\":\"secret-unknown\"}"});
    item["response"]["headers"] = json!([{"name":"Set-Cookie","value":"secret-response-cookie"},{"name":"WWW-Authenticate","value":"Digest realm=\"secret-realm\", nonce=\"secret-nonce\""},{"name":"Content-Length","value":"123456789012345"}]);
    item["response"]["content"]["text"] = json!("response secret-body-value");
    item["response"]["content"]["size"] = json!(123456789012345u64);
    item["_initiator"] = json!({"url":"http://secret-initiator-token/", "type":"script"});
    let report = parse_har("synthetic-device", &har(vec![item])).unwrap();
    let wire = serde_json::to_string(&report).unwrap();
    for secret in [
        "secret-",
        "123456789012345",
        "password-in-userinfo",
        "safe_unknown",
    ] {
        assert!(!wire.contains(secret), "leaked {secret}: {wire}");
    }
    assert!(report.auth_candidates.contains(&"digest".to_string()));
    assert_eq!(
        report.entries[0].classification,
        Classification::Authentication
    );
    assert!(report.entries[0]
        .request_body_names
        .contains(&"password".to_string()));
}

#[test]
fn synthetic_har_configuration_export_is_never_firmware() {
    let report = parse_har(
        "d",
        &har(vec![entry(
            "GET",
            "http://192.168.1.1/api/config/backup.bin",
        )]),
    )
    .unwrap();
    assert_eq!(
        report.entries[0].classification,
        Classification::ConfigBackup
    );
    assert!(!report.can_acquire);
}

#[test]
fn synthetic_har_upload_is_not_download_and_methods_are_conservative() {
    let report = parse_har(
        "d",
        &har(vec![
            entry("POST", "http://192.168.1.1/api/firmware/upload"),
            entry("GET", "http://192.168.1.1/api/firmware/download"),
            entry("DELETE", "http://192.168.1.1/api/status"),
        ]),
    )
    .unwrap();
    assert_eq!(
        report
            .entries
            .iter()
            .find(|v| v.method == "POST")
            .unwrap()
            .classification,
        Classification::FirmwareUpload
    );
    assert!(report
        .entries
        .iter()
        .any(|v| v.classification == Classification::FirmwareDownload));
    assert!(report.entries.iter().all(|v| !v.read_only_qualified));
}

#[test]
fn synthetic_har_classifies_action_query_without_retaining_values() {
    let report = parse_har(
        "d",
        &har(vec![entry(
            "GET",
            "http://192.168.1.1/xml_action.cgi?Action=Reboot&token=secret-token",
        )]),
    )
    .unwrap();
    assert_eq!(report.entries[0].classification, Classification::Reboot);
    let wire = serde_json::to_string(&report).unwrap();
    assert!(!wire.contains("secret-token"));
    assert!(!report.entries[0].path.contains('?'));
}

#[test]
fn synthetic_har_rejects_invalid_truncated_and_oversize_json() {
    assert_eq!(parse_har("d", b"{").unwrap_err().code, "INVALID_HAR");
    assert_eq!(parse_har("d", b"[]").unwrap_err().code, "INVALID_HAR");
    assert_eq!(
        parse_har("d", &vec![b' '; MAX_HAR_BYTES + 1])
            .unwrap_err()
            .code,
        "INPUT_TOO_LARGE"
    );
}

#[test]
fn synthetic_bundle_extracts_only_string_candidates_without_execution() {
    let js = br#"fetch('/api/status'); var x = new XMLHttpRequest(); x.open('POST','/api/firmware/upload'); window.location='/api/reset'; eval('throw new Error()'); const fota='https://vendor.example/firmware/latest.json?token=secret-token';"#;
    let report = analyze_bundle("d", "http://192.168.1.1", "app.js", js).unwrap();
    assert!(report.entries.iter().any(|v| v.path == "/api/status"));
    assert!(report
        .entries
        .iter()
        .any(|v| v.classification == Classification::FirmwareUpload));
    assert!(report
        .entries
        .iter()
        .any(|v| v.context.as_deref() == Some("fetch_wrapper")));
    assert!(report
        .entries
        .iter()
        .any(|v| v.context.as_deref() == Some("xmlhttprequest_wrapper")));
    assert!(report
        .entries
        .iter()
        .all(|v| v.evidence == EvidenceSource::StaticCandidate && !v.read_only_qualified));
    assert!(!serde_json::to_string(&report)
        .unwrap()
        .contains("secret-token"));
}

#[test]
fn synthetic_bundle_resource_references_are_same_origin_and_static_only() {
    let html = br#"<script src='/js/app.js'></script><link href='/css/main.css' rel='stylesheet'><img src='/reset.cgi'><script src='https://foreign.example/a.js'></script><script>var x='/api/status';</script>"#;
    let report = analyze_bundle("d", "http://192.168.1.1", "index.html", html).unwrap();
    assert_eq!(report.resources.len(), 2);
    assert!(report
        .resources
        .iter()
        .all(|r| r.origin == "http://192.168.1.1"));
    assert!(!report.actions.collect_resources);
}

#[test]
fn synthetic_bundle_missing_origin_and_nontext_input_fail_closed() {
    assert_eq!(
        analyze_bundle("d", "file:///C:/private", "app.js", b"fetch('/status')")
            .unwrap_err()
            .code,
        "INVALID_ORIGIN"
    );
    assert_eq!(
        analyze_bundle("d", "http://192.168.1.1", "app.exe", b"MZ")
            .unwrap_err()
            .code,
        "UNSUPPORTED_RESOURCE"
    );
}

#[test]
fn synthetic_bundle_never_turns_firmware_strings_into_read_authority() {
    let report = analyze_bundle(
        "d",
        "http://192.168.1.1",
        "app.js",
        br#"const x='/firmware/download';"#,
    )
    .unwrap();
    assert!(!report.can_acquire);
    assert!(!report.actions.authenticate);
    assert!(!report.entries[0].read_only_qualified);
}

#[test]
fn synthetic_unknown_path_schema_and_dns_labels_do_not_retain_values() {
    let mut item = entry(
        "GET",
        "https://secret-host.example/secret-path/status?secret-name=secret-value",
    );
    item["request"]["postData"] =
        json!({"mimeType":"application/json","text":"{\"secret-field\":\"secret-body\"}"});
    let report = parse_har("d", &har(vec![item])).unwrap();
    assert!(!serde_json::to_string(&report).unwrap().contains("secret-"));
    assert!(report.entries[0].origin.contains("redacted-host"));
}

#[test]
fn synthetic_har_classification_matrix_includes_camelcase_cgi_operations() {
    let cases = [
        ("GET", "/api/device/version", Classification::IdentityRead),
        ("GET", "/api/status", Classification::StatusRead),
        ("GET", "/login.cgi", Classification::Authentication),
        ("GET", "/config/read", Classification::ConfigRead),
        ("GET", "/config/set", Classification::ConfigWrite),
        ("GET", "/config/backup", Classification::ConfigBackup),
        ("GET", "/config/restore", Classification::ConfigRestore),
        (
            "GET",
            "/firmware/manifest.json",
            Classification::FirmwareMetadata,
        ),
        (
            "GET",
            "/firmware/update/check",
            Classification::FirmwareUpdateCheck,
        ),
        ("POST", "/firmware/upload", Classification::FirmwareUpload),
        (
            "GET",
            "/firmware/download",
            Classification::FirmwareDownload,
        ),
        ("GET", "/file/export", Classification::FileExport),
        (
            "GET",
            "/xml_action.cgi?Action=Reboot",
            Classification::Reboot,
        ),
        (
            "GET",
            "/xml_action.cgi?Action=FactoryReset",
            Classification::FactoryReset,
        ),
        (
            "GET",
            "/xml_action.cgi?Action=GetStatus",
            Classification::StatusRead,
        ),
        ("DELETE", "/api/status", Classification::Unknown),
    ];
    for (method, route, expected) in cases {
        let report = parse_har(
            "d",
            &har(vec![entry(method, &format!("http://192.0.2.1{route}"))]),
        )
        .unwrap();
        assert_eq!(
            report.entries[0].classification, expected,
            "{method} {route}"
        );
        assert!(!report.entries[0].read_only_qualified);
    }
}

#[test]
fn synthetic_bundle_candidate_cap_is_reported_without_executing_anything() {
    let source: String = (0..600)
        .map(|n| format!("fetch('/api/{n}/status');"))
        .collect();
    let report = analyze_bundle("d", "http://192.0.2.1", "app.js", source.as_bytes()).unwrap();
    assert_eq!(report.entries.len(), 512);
    assert!(report
        .warnings
        .iter()
        .any(|w| w.contains("512-candidate cap")));
    assert!(!report.can_acquire);
}

#[test]
fn synthetic_har_separate_query_schema_is_sanitized_and_cannot_hide_reset() {
    let mut item = entry("GET", "http://192.0.2.1/api/status");
    item["request"]["queryString"] =
        json!([{"name":"Action","value":"FactoryReset"},{"name":"token","value":"secret-token"}]);
    let report = parse_har("d", &har(vec![item])).unwrap();
    assert_eq!(
        report.entries[0].classification,
        Classification::FactoryReset
    );
    assert_eq!(report.entries[0].query_names, vec!["action", "token"]);
    assert!(!serde_json::to_string(&report)
        .unwrap()
        .contains("secret-token"));
}

#[test]
fn synthetic_source_assessment_is_published_without_promoting_metadata_to_a_package() {
    let report = parse_har(
        "d",
        &har(vec![
            entry("GET", "http://192.0.2.1/firmware/manifest.json"),
            entry("GET", "http://192.0.2.1/firmware/update/check"),
        ]),
    )
    .unwrap();
    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(value["strategy"]["strategy"], "web_management_acquisition");
    assert_eq!(value["strategy"]["canAcquire"], false);
    let sources = value["strategy"]["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 2);
    assert!(sources
        .iter()
        .all(|source| source["state"] == "metadata_only"));
    assert!(sources
        .iter()
        .all(|source| source["sourceType"] == "unknown"));
}
