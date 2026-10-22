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

fn os_inspection_profile() -> String {
    mutate(CURATED, |p| {
        p["schemaVersion"] = 2.into();
        p["inspectionOnly"] = true.into();
        p["profileId"] = "openwrt.os-inspection-test".into();
        p["manufacturer"] = "".into();
        p["hardwareRevisions"] = serde_json::json!([]);
        p["matchRules"][0]["values"] = serde_json::json!(["board|x86/64|build"]);
        for (_, feature) in p["capabilities"].as_object_mut().unwrap() {
            feature["assessment"] = "UNKNOWN".into();
            feature["verifiedRange"] = serde_json::Value::Null;
            for (_, sub) in feature["subCapabilities"].as_object_mut().unwrap() {
                sub["assessment"] = "UNKNOWN".into();
            }
        }
    })
}

#[test]
fn inspection_profile_cannot_upgrade_hardware_or_legacy_schema_authority() {
    let document = os_inspection_profile();
    assert!(validate(&document).unwrap().inspection_only);
    assert!(validate(&mutate(&document, |p| p["schemaVersion"] = 1.into())).is_err());
    assert!(validate(&mutate(&document, |p| p["inspectionOnly"] = false.into())).is_err());
    assert!(validate(&mutate(&document, |p| p["manufacturer"] = "Invented".into())).is_err());
    assert!(validate(&mutate(&document, |p| p["capabilities"]["dns"]
        ["assessment"] =
        "SUPPORTED".into()))
    .is_err());
    assert!(
        validate(&mutate(&document, |p| p["capabilities"]["wifiUserLimit"]
            ["subCapabilities"]["clientLimit"]["assessment"] =
            "SUPPORTED".into()))
        .is_err()
    );
    assert!(
        validate(&mutate(&document, |p| p["matchRules"][0]["values"] =
            serde_json::json!(["OpenWrt"])))
        .is_err()
    );
    let legacy = validate(CURATED).unwrap();
    assert!(!legacy.inspection_only);
    assert!(
        serde_json::to_value(legacy)
            .unwrap()
            .get("inspectionOnly")
            .is_none(),
        "Legacy content serialization stays unchanged"
    );
}

#[test]
fn authenticated_os_profile_has_unknown_features_and_requires_exact_build_identity() {
    let document = os_inspection_profile();
    let registry = Registry::load(vec![(&document, "bundled", true)]);
    let d = device();
    let mut evidence = strong(&d);
    evidence.retain(|e| {
        !matches!(
            e.attribute,
            Attribute::Manufacturer | Attribute::HardwareRevision
        )
    });
    evidence
        .iter_mut()
        .find(|e| e.attribute == Attribute::ManagementIdentity)
        .unwrap()
        .value = "board|x86/64|build".into();
    let report = evaluate(&registry, &d, evidence.clone(), NOW, 1);
    assert_eq!(report.confidence, MatchConfidence::Verified);
    assert!(report
        .features
        .values()
        .all(|f| f.assessment == Assessment::Unknown && !f.can_apply));
    evidence
        .iter_mut()
        .find(|e| e.attribute == Attribute::ManagementIdentity)
        .unwrap()
        .value = "board|wrong-target|build".into();
    assert!(evaluate(&registry, &d, evidence, NOW, 1)
        .selected_profile
        .is_none());
}

#[test]
fn os_inspection_profile_still_requires_trusted_evidence_and_registry_origin() {
    let document = os_inspection_profile();
    let d = device();
    let mut evidence = strong(&d);
    evidence.retain(|e| {
        !matches!(
            e.attribute,
            Attribute::Manufacturer | Attribute::HardwareRevision
        )
    });
    evidence
        .iter_mut()
        .find(|e| e.attribute == Attribute::ManagementIdentity)
        .unwrap()
        .value = "board|x86/64|build".into();
    let imported = Registry::load(vec![(&document, "imported", true)]);
    assert_ne!(
        evaluate(&imported, &d, evidence.clone(), NOW, 1).confidence,
        MatchConfidence::Verified
    );
    for e in &mut evidence {
        e.source = EvidenceOrigin::UserSupplied;
        e.verification = Verification::UserReported;
    }
    let bundled = Registry::load(vec![(&document, "bundled", true)]);
    assert_ne!(
        evaluate(&bundled, &d, evidence, NOW, 1).confidence,
        MatchConfidence::Verified
    );
    let mut missing_hardware = strong(&d);
    missing_hardware.retain(|e| {
        !matches!(
            e.attribute,
            Attribute::Manufacturer | Attribute::HardwareRevision
        )
    });
    assert_ne!(
        evaluate(&super::tests::registry(), &d, missing_hardware, NOW, 1).confidence,
        MatchConfidence::Verified
    );
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
        scheme: FirmwareVersionScheme::NumericTriplet,
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
    let mut registry = Registry::load(vec![(&text, "bundled", true)]);
    assert_eq!(registry.profiles.len(), 1, "{:?}", registry.diagnostics);
    // Reserved readiness is engine-owned; inject internally after schema validation
    // to exercise a nonadjacent duplicate without accepting it from profile JSON.
    registry.profiles[0]
        .profile
        .capabilities
        .apn
        .requirements
        .insert(0, Readiness::NotImplemented);
    let d = device();
    let r = evaluate(&registry, &d, strong(&d), NOW, 1);
    assert_eq!(r.features["apn"].assessment, Assessment::Supported);
    assert_eq!(
        r.features["apn"]
            .readiness
            .iter()
            .filter(|entry| **entry == Readiness::NotImplemented)
            .count(),
        1
    );
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

const OPAQUE: &str = "MF69_AM802L_ZX_DD_N_241106_EU";
fn opaque_profile() -> String {
    // Synthetic compatibility fixture; never bundled or attributed to physical hardware.
    mutate(CURATED, |p| {
        p["profileId"] = "fixture.proprietary".into();
        p["firmwareFamilies"] = serde_json::json!(["FixtureVendorFirmware"]);
        p["firmwareVersionConstraints"] = serde_json::json!({"scheme":"opaque_exact","exact":[OPAQUE],"minInclusive":null,"maxExclusive":null});
    })
}
fn opaque_evidence(d: &Device) -> Vec<EvidenceRecord> {
    let mut evidence = strong(d);
    for e in &mut evidence {
        if e.attribute == Attribute::FirmwareFamily {
            e.value = "FixtureVendorFirmware".into();
        }
        if e.attribute == Attribute::FirmwareVersion {
            e.value = OPAQUE.into();
        }
    }
    evidence
}

#[test]
fn legacy_schema_one_versions_and_canonical_profile_content_stay_compatible() {
    let legacy = validate(CURATED).unwrap();
    assert_eq!(
        legacy.firmware_version_constraints.scheme,
        FirmwareVersionScheme::NumericTriplet
    );
    let canonical = serde_json::to_value(&legacy).unwrap();
    assert_eq!(
        canonical,
        serde_json::from_str::<serde_json::Value>(CURATED).unwrap()
    );
    assert!(canonical["firmwareVersionConstraints"]
        .get("scheme")
        .is_none());
    let explicit = mutate(CURATED, |p| {
        p["firmwareVersionConstraints"]["scheme"] = "numeric_triplet".into()
    });
    assert_eq!(
        serde_json::to_value(validate(&explicit).unwrap()).unwrap(),
        canonical
    );
    let c = &legacy.firmware_version_constraints;
    assert_eq!(version_allowed("23.05.5", c), Some(true));
    assert_eq!(version_allowed("23.5.5", c), Some(true));
    assert_eq!(version_allowed("23.05.6", c), Some(false));
    assert_eq!(version_allowed("", c), None);
    assert_eq!(version_allowed(OPAQUE, c), None);
}
#[test]
fn opaque_identifiers_use_literal_case_sensitive_equality_without_numeric_or_date_inference() {
    let p = validate(&opaque_profile()).unwrap();
    let c = &p.firmware_version_constraints;
    assert_eq!(p.schema_version, 1);
    assert_eq!(c.scheme, FirmwareVersionScheme::OpaqueExact);
    assert_eq!(version_allowed(OPAQUE, c), Some(true));
    for value in [
        "MF69_AM802L_ZX_DD_N_241106_US",
        "mf69_am802l_zx_dd_n_241106_eu",
        "2024-11-06",
    ] {
        assert_eq!(version_allowed(value, c), Some(false));
    }
    for value in ["", " ", " MF69_AM802L_ZX_DD_N_241106_EU", "version\n"] {
        assert_eq!(version_allowed(value, c), None);
    }
    let numeric_looking = VersionConstraint {
        scheme: FirmwareVersionScheme::OpaqueExact,
        exact: vec!["23.05.5".into(), "241106".into()],
        min_inclusive: None,
        max_exclusive: None,
    };
    assert_eq!(version_allowed("23.05.5", &numeric_looking), Some(true));
    assert_eq!(version_allowed("23.5.5", &numeric_looking), Some(false));
    assert_eq!(version_allowed("241106", &numeric_looking), Some(true));
    assert_eq!(version_allowed("2024-11-06", &numeric_looking), Some(false));
}
#[test]
fn malformed_version_schemes_constraints_and_opaque_ranges_are_rejected() {
    for constraints in [
        serde_json::json!({"scheme":"unknown","exact":[OPAQUE],"minInclusive":null,"maxExclusive":null}),
        serde_json::json!({"scheme":null,"exact":[OPAQUE],"minInclusive":null,"maxExclusive":null}),
        serde_json::json!({"scheme":"opaque_exact","exact":[],"minInclusive":null,"maxExclusive":null}),
        serde_json::json!({"scheme":"opaque_exact","exact":[""],"minInclusive":null,"maxExclusive":null}),
        serde_json::json!({"scheme":"opaque_exact","exact":[" version"],"minInclusive":null,"maxExclusive":null}),
        serde_json::json!({"scheme":"opaque_exact","exact":["version\n"],"minInclusive":null,"maxExclusive":null}),
        serde_json::json!({"scheme":"opaque_exact","exact":["x".repeat(1025)],"minInclusive":null,"maxExclusive":null}),
        serde_json::json!({"scheme":"opaque_exact","exact":[OPAQUE],"minInclusive":"1.0.0","maxExclusive":"2.0.0"}),
        serde_json::json!({"scheme":"opaque_exact","exact":[OPAQUE],"minInclusive":OPAQUE,"maxExclusive":null}),
        serde_json::json!({"scheme":"numeric_triplet","exact":[OPAQUE],"minInclusive":null,"maxExclusive":null}),
        serde_json::json!({"exact":[OPAQUE],"minInclusive":null,"maxExclusive":null}),
        serde_json::json!({"exact":["23.05.5"],"minInclusive":"23.0.0","maxExclusive":"24.0.0"}),
        serde_json::json!({"exact":[],"minInclusive":"23.0.0","maxExclusive":null}),
        serde_json::json!({"exact":[],"minInclusive":"24.0.0","maxExclusive":"23.0.0"}),
    ] {
        let document = mutate(CURATED, |p| p["firmwareVersionConstraints"] = constraints);
        assert!(validate(&document).is_err());
        // Malformed known-scheme constraints are also safe if constructed directly in Rust.
        if let Ok(p) = serde_json::from_str::<Profile>(&document) {
            assert_eq!(
                version_allowed(OPAQUE, &p.firmware_version_constraints),
                None
            );
        }
    }
}
#[test]
fn exact_proprietary_match_needs_independently_verified_firmware_evidence() {
    let text = opaque_profile();
    let registry = Registry::load(vec![(&text, "bundled", true)]);
    let d = device();
    let e = opaque_evidence(&d);
    let r = evaluate(&registry, &d, e.clone(), NOW, 1);
    assert_eq!(r.confidence, MatchConfidence::Verified);
    assert_eq!(r.selected_profile.as_deref(), Some("fixture.proprietary"));
    assert_eq!(r.features["dns"].assessment, Assessment::Supported);
    assert!(r
        .features
        .values()
        .all(|f| !f.can_apply && !f.adapter_implemented));
    for (source, verification, reliability) in [
        (
            EvidenceOrigin::UserSupplied,
            Verification::UserReported,
            Reliability::Hint,
        ),
        (
            EvidenceOrigin::AuthenticatedManagement,
            Verification::Observed,
            Reliability::Observed,
        ),
    ] {
        let mut unverified = e.clone();
        let version = unverified
            .iter_mut()
            .find(|e| e.attribute == Attribute::FirmwareVersion)
            .unwrap();
        version.source = source;
        version.verification = verification;
        version.reliability = reliability;
        unknown(&evaluate(&registry, &d, unverified, NOW, 1));
    }
    let mut missing = e.clone();
    missing.retain(|e| e.attribute != Attribute::FirmwareVersion);
    unknown(&evaluate(&registry, &d, missing, NOW, 1));
    for value in ["MF69_AM802L_ZX_DD_N_241106_US", "", "23.05.5"] {
        let mut wrong = e.clone();
        wrong
            .iter_mut()
            .find(|e| e.attribute == Attribute::FirmwareVersion)
            .unwrap()
            .value = value.into();
        unknown(&evaluate(&registry, &d, wrong, NOW, 1));
    }
    unknown(&evaluate(&registry, &d, e, NOW + EVIDENCE_TTL_MS + 1, 1));
}
#[test]
fn conflicting_opaque_versions_preserve_case_and_block_authority() {
    let text = opaque_profile();
    let registry = Registry::load(vec![(&text, "bundled", true)]);
    let d = device();
    let mut e = opaque_evidence(&d);
    let mut other = e
        .iter()
        .find(|e| e.attribute == Attribute::FirmwareVersion)
        .unwrap()
        .clone();
    other.id = "conflicting-opaque-case".into();
    other.value = OPAQUE.to_lowercase();
    e.push(other);
    let r = evaluate(&registry, &d, e, NOW, 1);
    unknown(&r);
    assert_eq!(r.confidence, MatchConfidence::Conflicting);
    assert!(r.conflicts.contains(&Attribute::FirmwareVersion));
    assert_eq!(
        r.evidence
            .iter()
            .filter(|e| e.attribute == Attribute::FirmwareVersion && !e.conflicts.is_empty())
            .count(),
        2
    );
}
#[test]
fn unique_verified_match_beats_weaker_profiles_even_when_they_sort_first() {
    let d = device();
    let e = strong(&d);
    // One probable profile lacks revision applicability; the other is a complete untrusted preview.
    for authoritative in [false, true] {
        let weak = mutate(CURATED, |p| {
            p["profileId"] = "a.fixture.weak".into();
            if authoritative {
                p["hardwareRevisions"] = serde_json::json!([]);
            }
        });
        let origin = if authoritative {
            "bundled"
        } else {
            "local_unverified"
        };
        let forward = Registry::load(vec![
            (&weak, origin, authoritative),
            (CURATED, "bundled", true),
        ]);
        let reverse = Registry::load(vec![
            (CURATED, "bundled", true),
            (&weak, origin, authoritative),
        ]);
        for registry in [forward, reverse] {
            let r = evaluate(&registry, &d, e.clone(), NOW, 1);
            assert_eq!(r.candidates[0].profile_id, "a.fixture.weak");
            assert_eq!(r.candidates[0].confidence, MatchConfidence::Probable);
            assert_eq!(r.confidence, MatchConfidence::Verified);
            assert_eq!(
                r.selected_profile.as_deref(),
                Some("tplink.archer-c7-v5.openwrt-23-05-5")
            );
            assert_eq!(r.features["dns"].assessment, Assessment::Supported);
            assert!(r.features.values().all(|f| !f.can_apply));
        }
    }
}
#[test]
fn usb_hints_remain_visible_but_cannot_override_verified_identity() {
    let mut d = device();
    d.vendor_id = Some("1286".into());
    d.product_id = Some("4E31".into());
    let mut e = strong(&d);
    e.extend(gather_evidence(&d, &[]));
    let r = evaluate(&Registry::bundled(), &d, e, NOW, 1);
    assert_eq!(r.confidence, MatchConfidence::Verified);
    let hint = r
        .candidates
        .iter()
        .find(|p| p.profile_id == "observed.comfi-mg11")
        .unwrap();
    assert!(hint.matched_fields.contains(&Attribute::UsbVid));
    assert!(hint.matched_fields.contains(&Attribute::UsbPid));
    assert_eq!(hint.confidence, MatchConfidence::NoMatch);
    assert!(!hint.authoritative);
}
#[test]
fn two_verified_profiles_with_conflicting_capabilities_stay_ambiguous() {
    let second = mutate(CURATED, |p| {
        p["profileId"] = "a.fixture.conflicting".into();
        p["capabilities"]["dns"]["assessment"] = "UNSUPPORTED".into();
    });
    let d = device();
    let registry = Registry::load(vec![(&second, "bundled", true), (CURATED, "bundled", true)]);
    let r = evaluate(&registry, &d, strong(&d), NOW, 1);
    unknown(&r);
    assert_eq!(r.confidence, MatchConfidence::Ambiguous);
    assert_eq!(
        r.candidates
            .iter()
            .filter(|c| c.confidence == MatchConfidence::Verified && c.authoritative)
            .count(),
        2
    );
}
#[test]
fn no_verified_match_retains_conservative_probable_candidate_and_ambiguous_behavior() {
    let d = device();
    let weak = mutate(CURATED, |p| p["hardwareRevisions"] = serde_json::json!([]));
    let registry = Registry::load(vec![(&weak, "bundled", true)]);
    let r = evaluate(&registry, &d, strong(&d), NOW, 1);
    assert_eq!(r.confidence, MatchConfidence::Probable);
    unknown(&r);
    let mut d = d;
    d.vendor_id = Some("1286".into());
    d.product_id = Some("4E31".into());
    let mut e = gather_evidence(&d, &[]);
    e.push(
        EvidenceRecord::user_reported(d.id.clone(), Attribute::Model, "MG11".into(), NOW).unwrap(),
    );
    let r = evaluate(&Registry::bundled(), &d, e.clone(), NOW, 1);
    unknown(&r);
    assert_eq!(r.confidence, MatchConfidence::Candidate);
    let second = mutate(OBSERVED, |p| p["profileId"] = "fixture.second-hint".into());
    let registry = Registry::load(vec![
        (OBSERVED, "bundled_observation", false),
        (&second, "bundled_observation", false),
    ]);
    let r = evaluate(&registry, &d, e, NOW, 1);
    unknown(&r);
    assert_eq!(r.confidence, MatchConfidence::Ambiguous);
}

#[test]
fn unmatched_optional_usb_hint_cannot_demote_a_fully_verified_profile() {
    let profile = mutate(CURATED, |p| {
        p["matchRules"].as_array_mut().unwrap().push(serde_json::json!({"attribute":"usb_vid","values":["FFFF"],"purpose":"candidate_hint","explanation":"Optional USB product-family hint; never identity proof."}))
    });
    let registry = Registry::load(vec![(&profile, "bundled", true)]);
    let d = device();
    let mut evidence = strong(&d);
    evidence.extend(gather_evidence(&d, &[]));
    let r = evaluate(&registry, &d, evidence, NOW, 1);
    assert_eq!(r.confidence, MatchConfidence::Verified);
    assert!(r.selected_profile.is_some());
    assert!(r.candidates[0]
        .reasons
        .iter()
        .any(|reason| reason.contains("Optional CandidateHint")));
    assert!(r.features.values().all(|f| !f.can_apply));
}
#[test]
fn weak_firmware_reports_remain_visible_without_overriding_verified_firmware() {
    let profile = opaque_profile();
    let registry = Registry::load(vec![(&profile, "bundled", true)]);
    let d = device();
    let mut evidence = opaque_evidence(&d);
    for value in [OPAQUE, "unverified-other-release"] {
        evidence.push(
            EvidenceRecord::user_reported(
                d.id.clone(),
                Attribute::FirmwareVersion,
                value.into(),
                NOW,
            )
            .unwrap(),
        );
    }
    let r = evaluate(&registry, &d, evidence.clone(), NOW, 1);
    assert_eq!(r.confidence, MatchConfidence::Verified);
    assert_eq!(
        r.evidence
            .iter()
            .filter(|e| e.verification == Verification::UserReported)
            .count(),
        2
    );
    assert!(r.features.values().all(|f| !f.can_apply));
    evidence.retain(|e| {
        e.attribute != Attribute::FirmwareVersion || e.verification == Verification::UserReported
    });
    unknown(&evaluate(&registry, &d, evidence, NOW, 1));
}
