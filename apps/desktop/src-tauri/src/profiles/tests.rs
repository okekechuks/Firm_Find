use super::engine::{EvidenceOrigin, Reliability, Verification};
use super::*;
use crate::discovery::{replay_fixture, Device};

const CURATED: &str =
    include_str!("../../../../../device-profiles/tplink-archer-c7-v5-openwrt.json");
const OBSERVED: &str = include_str!("../../../../../device-profiles/observed-comfi-mg11.json");
const NOW: u64 = 2_000_000;
fn device() -> Device {
    let mut d = replay_fixture(include_str!("../../fixtures/discovery-lifecycle.json")).unwrap()[1]
        .devices[0]
        .clone();
    d.last_seen_at = NOW;
    d
}
fn strong(d: &Device) -> Vec<EvidenceRecord> {
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
        collection_method: "SYNTHETIC trusted-adapter fixture, never collected from live hardware"
            .into(),
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
fn registry() -> Registry {
    Registry::load(vec![(CURATED, "bundled", true)])
}
fn unknown(report: &CompatibilityReport) {
    assert!(report.selected_profile.is_none());
    assert_eq!(report.features.len(), 4);
    for f in report.features.values() {
        assert_eq!(f.assessment, Assessment::Unknown);
        assert!(!f.can_apply);
        assert!(!f.adapter_implemented);
        assert!(f.readiness.contains(&Readiness::NotImplemented));
    }
}
fn mutate(document: &str, f: impl FnOnce(&mut serde_json::Value)) -> String {
    let mut value = serde_json::from_str(document).unwrap();
    f(&mut value);
    serde_json::to_string(&value).unwrap()
}

#[test]
fn bundled_profiles_validate_and_ids_are_deterministic() {
    let a = Registry::bundled();
    assert_eq!(a.profiles.len(), 2, "{:?}", a.diagnostics);
    assert!(a.diagnostics.is_empty());
    assert!(!a.profiles[0].authoritative);
    assert!(a.profiles[1].authoritative);
    let b = Registry::load(vec![
        (CURATED, "bundled", true),
        (OBSERVED, "bundled_observation", false),
    ]);
    assert_eq!(a.digest, b.digest);
    assert!(a.by_id("observed.comfi-mg11", Some("1.0.0")).is_some());
    assert!(a.by_id("observed.comfi-mg11", Some("9.9.9")).is_none());
    assert!(a.by_id("missing", None).is_none());
}

#[test]
fn explicit_alias_matches_but_experimental_provenance_cannot_grant_support() {
    let d = device();
    let mut e = strong(&d);
    e.iter_mut()
        .find(|e| e.attribute == Attribute::Model)
        .unwrap()
        .value = "TP-Link Archer C7 v5".into();
    assert_eq!(
        evaluate(&registry(), &d, e.clone(), NOW, 1).confidence,
        MatchConfidence::Verified
    );
    let experimental = mutate(CURATED, |v| {
        v["provenance"]["category"] = "experimental".into()
    });
    let registry = Registry::load(vec![(&experimental, "bundled", true)]);
    assert!(!registry.profiles[0].authoritative);
    unknown(&evaluate(&registry, &d, e, NOW, 1));
}
#[test]
fn malformed_future_unknown_enum_and_scripts_are_rejected() {
    for text in [
        "{".into(),
        mutate(CURATED, |v| v["schemaVersion"] = 2.into()),
        mutate(CURATED, |v| v["script"] = "evil.exe".into()),
        mutate(CURATED, |v| {
            v["capabilities"]["dns"]["execute"] = "echo data".into()
        }),
        mutate(CURATED, |v| {
            v["managementInterfaces"][0]["protocol"] = "shell".into()
        }),
        mutate(CURATED, |v| {
            v["deviceCategory"] = serde_json::json!(["unknown_category"])
        }),
    ] {
        assert!(validate(&text).is_err(), "Accepted invalid profile");
    }
}
#[test]
fn duplicate_keys_in_structs_and_maps_are_rejected() {
    assert!(validate(&CURATED.replacen(
        "\"schemaVersion\": 1",
        "\"schemaVersion\": 1, \"schemaVersion\": 1",
        1
    ))
    .is_err());
    assert!(
        validate(&CURATED.replacen("\"upstream\": {", "\"upstream\": {}, \"upstream\": {", 1))
            .is_err()
    );
}
#[test]
fn size_depth_string_collection_date_and_source_bounds_reject() {
    let too_big = " ".repeat(MAX_PROFILE_BYTES + 1);
    assert!(validate(&too_big).is_err());
    for text in [
        mutate(CURATED, |v| v["model"] = "x".repeat(1025).into()),
        mutate(CURATED, |v| v["modelAliases"] = vec!["x"; 65].into()),
        mutate(CURATED, |v| v["lastReviewed"] = "2026-02-30".into()),
        mutate(CURATED, |v| {
            v["evidenceSources"][0]["reference"] = "javascript:alert(1)".into()
        }),
        mutate(CURATED, |v| {
            v["capabilities"]["dns"]["evidenceSources"] = serde_json::json!(["missing-source"])
        }),
        mutate(CURATED, |v| {
            v["nested"] = serde_json::from_str::<serde_json::Value>(&format!(
                "{}0{}",
                "[".repeat(20),
                "]".repeat(20)
            ))
            .unwrap()
        }),
    ] {
        assert!(validate(&text).is_err());
    }
}
#[test]
fn invalid_ranges_subkeys_and_app_owned_implementation_fields_reject() {
    for text in [
        mutate(CURATED, |v| {
            v["capabilities"]["wifiUserLimit"]["verifiedRange"] =
                serde_json::json!({"min":1,"max":32})
        }),
        mutate(CURATED, |v| {
            v["capabilities"]["dns"]["subCapabilities"]
                .as_object_mut()
                .unwrap()
                .remove("lanIpv6")
                .map(|_| ())
                .unwrap()
        }),
        mutate(CURATED, |v| {
            v["capabilities"]["dns"]["requirements"] = serde_json::json!(["NOT_IMPLEMENTED"])
        }),
        mutate(
            CURATED,
            |v| {
                v["firmwareVersionConstraints"] =
                    serde_json::json!({"exact":[],"minInclusive":"24.0.0","maxExclusive":"23.0.0"})
            },
        ),
    ] {
        assert!(validate(&text).is_err());
    }
}
#[test]
fn duplicates_quarantine_all_and_bad_document_does_not_crash_registry() {
    let r = Registry::load(vec![
        (CURATED, "bundled", true),
        (CURATED, "local_unverified", false),
        ("{}", "local_unverified", false),
    ]);
    assert!(r.profiles.is_empty());
    assert_eq!(r.diagnostics.len(), 2);
}
#[test]
fn exact_fixture_matches_but_missing_adapter_is_not_unsupported() {
    let d = device();
    let r = evaluate(&registry(), &d, strong(&d), NOW, 1);
    assert_eq!(r.confidence, MatchConfidence::Verified);
    assert!(r.selected_profile.is_some());
    assert_eq!(r.features["dns"].assessment, Assessment::Supported);
    assert!(r.features["dns"]
        .readiness
        .contains(&Readiness::RequiresAuthentication));
    for f in r.features.values() {
        assert!(!f.can_apply);
        assert!(f.readiness.contains(&Readiness::NotImplemented));
    }
    for key in ["wifiUserLimit", "apn", "ussd"] {
        assert_eq!(r.features[key].assessment, Assessment::Unknown);
    }
    assert!(r.features["wifiUserLimit"].verified_range.is_none());
    assert_eq!(
        r.features["dns"].sub_capabilities["lanIpv6"].assessment,
        Assessment::Unknown
    );
}
#[test]
fn usb_labels_gateway_and_http_alone_cannot_verify_model_or_capabilities() {
    let mut d = device();
    d.vendor_id = Some("1286".into());
    d.product_id = Some("4E31".into());
    let mut e = gather_evidence(&d, &[]);
    e.push(
        EvidenceRecord::user_reported(d.id.clone(), Attribute::Model, "Archer C7".into(), NOW)
            .unwrap(),
    );
    let r = evaluate(&Registry::bundled(), &d, e, NOW, 1);
    unknown(&r);
    assert_ne!(r.confidence, MatchConfidence::Verified);
}
#[test]
fn wrong_revision_wrong_release_unparseable_and_missing_identity_fail_closed() {
    let d = device();
    for (attribute, value) in [
        (Attribute::HardwareRevision, "v4"),
        (Attribute::FirmwareVersion, "24.10.0"),
        (Attribute::FirmwareVersion, "MF69_AM802L_ZX_DD_N_241106_EU"),
    ] {
        let mut e = strong(&d);
        e.iter_mut()
            .find(|e| e.attribute == attribute)
            .unwrap()
            .value = value.into();
        unknown(&evaluate(&registry(), &d, e, NOW, 1));
    }
    let mut e = strong(&d);
    e.retain(|e| e.attribute != Attribute::HardwareRevision);
    unknown(&evaluate(&registry(), &d, e, NOW, 1));
}
#[test]
fn stale_future_disconnected_cross_subject_and_endpoint_split_cannot_verify() {
    let d = device();
    for at in [NOW - EVIDENCE_TTL_MS - 1, NOW + 30_001] {
        let mut e = strong(&d);
        for e in &mut e {
            e.collected_at = at;
        }
        unknown(&evaluate(&registry(), &d, e, NOW, 1));
    }
    let mut removed = d.clone();
    removed.connected = false;
    unknown(&evaluate(&registry(), &removed, strong(&d), NOW, 1));
    let mut e = strong(&d);
    e[0].subject_id = "another-device".into();
    unknown(&evaluate(&registry(), &d, e, NOW, 1));
    let mut e = strong(&d);
    e[0].endpoint = Some("fixture://unrelated".into());
    unknown(&evaluate(&registry(), &d, e, NOW, 1));
}
#[test]
fn conflicts_preserve_both_records_and_deny_selection() {
    let d = device();
    let mut e = strong(&d);
    let mut other = e[1].clone();
    other.id = "conflicting-model".into();
    other.value = "Archer A7".into();
    e.push(other);
    let r = evaluate(&registry(), &d, e, NOW, 1);
    unknown(&r);
    assert_eq!(r.confidence, MatchConfidence::Conflicting);
    assert!(r.conflicts.contains(&Attribute::Model));
    assert_eq!(
        r.evidence
            .iter()
            .filter(|e| e.attribute == Attribute::Model && !e.conflicts.is_empty())
            .count(),
        2
    );
}
#[test]
fn multiple_matches_are_ambiguous_and_imported_data_never_authoritative() {
    let second = mutate(CURATED, |v| v["profileId"] = "test.second".into());
    let d = device();
    let r = Registry::load(vec![(CURATED, "bundled", true), (&second, "bundled", true)]);
    let r = evaluate(&r, &d, strong(&d), NOW, 1);
    assert_eq!(r.confidence, MatchConfidence::Ambiguous);
    unknown(&r);
    let imported = Registry::load(vec![(CURATED, "local_unverified", true)]);
    assert!(!imported.profiles[0].authoritative);
    unknown(&evaluate(&imported, &d, strong(&d), NOW, 1));
}
#[test]
fn version_ranges_are_numeric_bounded_and_opaque_is_unknown() {
    let c = VersionConstraint {
        exact: vec![],
        min_inclusive: Some("23.5.0".into()),
        max_exclusive: Some("24.0.0".into()),
    };
    assert_eq!(version_allowed("23.10.1", &c), Some(true));
    assert_eq!(version_allowed("24.0.0", &c), Some(false));
    assert_eq!(version_allowed("MF69_AM802L_ZX_DD_N_241106_EU", &c), None);
}
#[test]
fn feature_and_subcapability_assessments_remain_independent() {
    let text = mutate(CURATED, |v| {
        v["capabilities"]["apn"]["assessment"] = "SUPPORTED".into();
        v["capabilities"]["apn"]["evidenceSources"] = serde_json::json!(["board"]);
        v["capabilities"]["apn"]["requirements"] = serde_json::json!(["REQUIRES_PACKAGE"]);
        v["capabilities"]["apn"]["subCapabilities"]["manual"]["assessment"] = "SUPPORTED".into();
        v["capabilities"]["apn"]["subCapabilities"]["manual"]["evidenceSources"] =
            serde_json::json!(["board"]);
        v["capabilities"]["ussd"]["assessment"] = "UNSUPPORTED".into();
        v["capabilities"]["ussd"]["evidenceSources"] = serde_json::json!(["board"]);
    });
    // Synthetic feature variation: no claim about the actual Archer C7 or MG11.
    let registry = Registry::load(vec![(&text, "bundled", true)]);
    assert_eq!(registry.profiles.len(), 1, "{:?}", registry.diagnostics);
    let d = device();
    let r = evaluate(&registry, &d, strong(&d), NOW, 1);
    assert_eq!(r.features["apn"].assessment, Assessment::Supported);
    assert!(r.features["apn"]
        .readiness
        .contains(&Readiness::RequiresPackage));
    assert_eq!(
        r.features["apn"].sub_capabilities["automatic"].assessment,
        Assessment::Unknown
    );
    assert_eq!(r.features["ussd"].assessment, Assessment::Unsupported);
    assert_eq!(
        r.features["ussd"].sub_capabilities["routerWebUi"].assessment,
        Assessment::Unknown
    );
}
#[test]
fn decisions_explain_version_evidence_and_freshness_changes() {
    let d = device();
    let before = evaluate(&registry(), &d, strong(&d), NOW, 1);
    let mut after = evaluate(&registry(), &d, strong(&d), NOW + EVIDENCE_TTL_MS + 1, 2);
    record_change(&mut after, &before);
    assert!(after.change.is_some());
    unknown(&after);
    let updated = mutate(CURATED, |v| v["profileVersion"] = "1.0.1".into());
    let mut after = evaluate(
        &Registry::load(vec![(&updated, "bundled", true)]),
        &d,
        strong(&d),
        NOW,
        2,
    );
    record_change(&mut after, &before);
    assert!(after
        .change
        .unwrap()
        .reasons
        .iter()
        .any(|r| r.contains("registry")));
}
#[test]
fn user_reports_are_bounded_never_verified_and_deduplicate() {
    let a = EvidenceRecord::user_reported(
        "device".into(),
        Attribute::ReportedIdentifier,
        "LT800/".into(),
        NOW,
    )
    .unwrap();
    let b = EvidenceRecord::user_reported(
        "device".into(),
        Attribute::ReportedIdentifier,
        "LT800/".into(),
        NOW + 1,
    )
    .unwrap();
    assert_eq!(a.id, b.id);
    assert_eq!(a.verification, Verification::UserReported);
    assert_eq!(a.reliability, Reliability::Hint);
    for attribute in [
        Attribute::ManagementIdentity,
        Attribute::Manufacturer,
        Attribute::UsbVid,
    ] {
        assert!(
            EvidenceRecord::user_reported("device".into(), attribute, "test".into(), NOW).is_err()
        );
    }
    assert!(
        EvidenceRecord::user_reported("device".into(), Attribute::Model, "x".repeat(257), NOW)
            .is_err()
    );
}
