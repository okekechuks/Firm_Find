use super::*;
use profiles::{Attribute, Reliability, Verification};
fn setup() -> (tempfile::TempDir, Repository, DiscoverySnapshot) {
    let dir = tempfile::tempdir().unwrap();
    let mut repo = Repository::open(&dir.path().join("fixture.db")).unwrap();
    let snapshot = lab::seed(&mut repo, now()).unwrap();
    (dir, repo, snapshot)
}
#[test]
fn unique_exact_target_links_only_qualified_source() {
    let (_dir, repo, snapshot) = setup();
    let (identity, binding, sources) = assess(&repo, &snapshot, lab::ID, now()).unwrap();
    assert_eq!(identity.verified_hardware_revision.as_deref(), Some("v5"));
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].revision, "v5");
    validate_acquisition(&repo, &snapshot, &binding, &sources[0].id, now()).unwrap();
    assert!(validate_acquisition(&repo, &snapshot, &binding, "unregistered", now()).is_err());
}
#[test]
fn unchanged_scan_and_user_labels_do_not_revoke_existing_binding() {
    let (_dir, repo, mut snapshot) = setup();
    let time = now();
    let (_, binding, sources) = assess(&repo, &snapshot, lab::ID, time).unwrap();
    snapshot.scanned_at = Some(time + 1000);
    snapshot.devices[0].last_seen_at = time + 1000;
    snapshot.devices[0].display_name = "Different user label".into();
    // Inventory freshness may extend; the identity evidence's expiry remains
    // fixed and cannot be renewed by passive scans.
    validate_acquisition(&repo, &snapshot, &binding, &sources[0].id, time + 1000).unwrap();
}
#[test]
fn proprietary_observed_and_reported_values_cannot_authorize() {
    let (_dir, repo, snapshot) = setup();
    let (mut identity, _, _) = assess(&repo, &snapshot, lab::ID, now()).unwrap();
    for e in &mut identity.evidence {
        e.verification = Verification::UserReported;
        e.reliability = Reliability::Hint;
    }
    let identity = acquisition::identity(identity.evidence, now(), None, None);
    assert!(matching_sources(&identity).unwrap().is_empty());
    assert!(identity.verified_model.is_none());
}
#[test]
fn same_model_different_revision_never_inherits_firmware() {
    let (_dir, repo, snapshot) = setup();
    let (mut identity, _, _) = assess(&repo, &snapshot, lab::ID, now()).unwrap();
    identity.verified_hardware_revision = Some("v2".into());
    let sources = matching_sources(&identity).unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].revision, "v2");
    identity.verified_hardware_revision = Some("v5-newer-unqualified".into());
    assert!(matching_sources(&identity).unwrap().is_empty());
    identity.verified_hardware_revision = None;
    assert!(matching_sources(&identity).unwrap().is_empty());
    identity.confidence = "conflicting".into();
    assert!(matching_sources(&identity).unwrap().is_empty());
}
#[test]
fn cancelled_failed_stale_inventory_disconnection_and_changed_binding_refuse_publication() {
    let (_dir, repo, snapshot) = setup();
    let time = now();
    let (_, binding, sources) = assess(&repo, &snapshot, lab::ID, time).unwrap();
    let source = &sources[0].id;
    for mutation in 0..7 {
        let mut next = snapshot.clone();
        match mutation {
            0 => next.cancelled = true,
            1 => next.status = DiscoveryStatus::Failed,
            2 => next.scanned_at = Some(time - profiles::EVIDENCE_TTL_MS),
            3 => next.devices[0].connected = false,
            4 => next.revision += 1,
            5 => next.devices[0].networks[0].id = "substituted".into(),
            _ => next.devices[0].networks[0].gateways = vec!["192.0.2.9".into()],
        };
        assert!(
            validate_acquisition(&repo, &next, &binding, source, time).is_err(),
            "mutation {mutation}"
        );
    }
    assert!(validate_acquisition(&repo, &snapshot, &binding, source, binding.expires_at).is_err());
}
#[test]
fn conflict_stale_evidence_and_labels_fail_closed() {
    let (_dir, mut repo, mut snapshot) = setup();
    let time = now();
    let original = assess(&repo, &snapshot, lab::ID, time).unwrap().2;
    snapshot.devices[0].display_name = "Any unrelated user label".into();
    assert_eq!(assess(&repo, &snapshot, lab::ID, time).unwrap().2, original);
    let mut conflicting = repo
        .identity_observations(lab::ID)
        .unwrap()
        .into_iter()
        .find(|e| e.attribute == Attribute::Model)
        .unwrap();
    conflicting.id = "conflicting-model".into();
    conflicting.value = "Different model".into();
    repo.save_identity_observation(&conflicting).unwrap();
    assert!(assess(&repo, &snapshot, lab::ID, time)
        .unwrap()
        .2
        .is_empty());
    assert!(
        assess(&repo, &snapshot, lab::ID, time + profiles::EVIDENCE_TTL_MS)
            .unwrap()
            .2
            .is_empty()
    );
}
#[test]
fn denial_stale_approval_and_protected_host_create_no_job_or_artifact() {
    let (dir, repo, mut snapshot) = setup();
    let repo = Arc::new(Mutex::new(repo));
    let service = LibraryService::start(
        repo.clone(),
        dir.path().into(),
        DEFAULT_MAX,
        Arc::new(|_| {}),
    )
    .unwrap();
    let (_, binding, sources) = assess(&lock(&repo).unwrap(), &snapshot, lab::ID, now()).unwrap();
    let live = Arc::new(Mutex::new(snapshot.clone()));
    assert_eq!(
        service
            .acquire_for_device(live.clone(), binding.clone(), &sources[0].id, false)
            .unwrap_err()
            .code,
        "SOURCE_APPROVAL_REQUIRED"
    );
    let mut expired = binding;
    expired.expires_at = 0;
    assert!(service
        .acquire_for_device(live, expired, &sources[0].id, true)
        .is_err());
    snapshot.devices[0].category = Category::GenericNetworkInterface;
    assert!(assess(&lock(&repo).unwrap(), &snapshot, lab::ID, now())
        .unwrap()
        .2
        .is_empty());
    assert!(service.jobs().unwrap().is_empty());
    let result = service
        .device_workflow(&snapshot, lab::ID, &profiles::Registry::bundled())
        .unwrap();
    assert!(result.artifacts.is_empty());
    assert!(!result.stages.iter().any(|s| s.executable));
    assert!(result.configuration.iter().all(|s| !s.executable));
    service.stop();
}
#[test]
fn stage_progress_and_failure_are_projected_from_durable_jobs() {
    let (dir, repo, snapshot) = setup();
    let repo = Arc::new(Mutex::new(repo));
    let service = LibraryService::start(
        repo.clone(),
        dir.path().into(),
        DEFAULT_MAX,
        Arc::new(|_| {}),
    )
    .unwrap();
    // Synthetic job metadata only: no task, transport or successful artifact.
    let id = "2".repeat(32);
    lock(&repo)
        .unwrap()
        .create_library_job(
            &id,
            "official_image_download",
            1,
            JobInput {
                device: Some(lab::ID),
                ..Default::default()
            },
        )
        .unwrap();
    lock(&repo)
        .unwrap()
        .library_transition(&id, JobStatus::Running, None)
        .unwrap();
    let value = serde_json::to_value(
        service
            .device_workflow(&snapshot, lab::ID, &profiles::Registry::bundled())
            .unwrap(),
    )
    .unwrap();
    let acquisition = value["stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "acquisition")
        .unwrap();
    assert_eq!(acquisition["job"]["status"], "running");
    lock(&repo)
        .unwrap()
        .library_transition(
            &id,
            JobStatus::Failed,
            Some(&LibraryError::new(
                "SYNTHETIC_INTERRUPTION",
                "Synthetic interrupted operation; no image admitted.",
            )),
        )
        .unwrap();
    let value = serde_json::to_value(
        service
            .device_workflow(&snapshot, lab::ID, &profiles::Registry::bundled())
            .unwrap(),
    )
    .unwrap();
    let acquisition = value["stages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "acquisition")
        .unwrap();
    assert_eq!(acquisition["state"], "failed");
    assert_eq!(acquisition["reasonCode"], "SYNTHETIC_INTERRUPTION");
    assert!(value["artifacts"].as_array().unwrap().is_empty());
    service.stop();
}
