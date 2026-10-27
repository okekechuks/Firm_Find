use super::*;
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
fn setup() -> (tempfile::TempDir, Arc<Mutex<Repository>>, LibraryService) {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("data");
    fs::create_dir(&root).unwrap();
    let repo = Arc::new(Mutex::new(Repository::open(&root.join("test.db")).unwrap()));
    let service =
        LibraryService::start(Arc::clone(&repo), root, DEFAULT_MAX, Arc::new(|_| {})).unwrap();
    (d, repo, service)
}
fn source(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let p = dir.join(name);
    fs::write(&p, bytes).unwrap();
    p
}
fn start(service: &LibraryService, path: &Path) -> JobRecord {
    let selected = service.select_path(path).unwrap();
    service
        .import(ImportRequest {
            ticket: selected.ticket,
            description: Some("Harmless generated test fixture".into()),
            associated_device_id: None,
        })
        .unwrap()
}
fn wait(service: &LibraryService, id: &str) -> JobRecord {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let j = service.job(id).unwrap();
        if j.status.terminal() && !service.inner.controls.lock().unwrap().contains_key(id) {
            return j;
        }
        assert!(Instant::now() < deadline, "Job timed out: {j:?}");
        thread::sleep(Duration::from_millis(10));
    }
}
fn completed(service: &LibraryService, path: &Path) -> JobRecord {
    let job = start(service, path);
    let result = wait(service, &job.id);
    assert_eq!(result.status, JobStatus::Completed, "{result:?}");
    result
}

#[test]
fn review_restore_existing_artifact_preserves_its_name_and_kind() {
    let (d, _, s) = setup();
    let original = completed(
        &s,
        &source(d.path(), "original.bin", b"harmless restore fixture"),
    );
    let hash = original.related_artifact.unwrap();
    assert_eq!(
        wait(&s, &s.create_backup(&hash).unwrap().id).status,
        JobStatus::Completed
    );
    let backup = s.backups().unwrap().remove(0);
    assert_eq!(
        wait(&s, &s.restore_backup(&backup.id, true).unwrap().id).status,
        JobStatus::Completed
    );
    let details = s.details(&hash).unwrap();
    assert_eq!(details.artifact.original_filename, "original.bin");
    assert_eq!(details.artifact.kind, "imported_original");
    assert_eq!(details.artifact.source_count, 2);
}

#[test]
fn sprint_analysis_reports_persist_separately_and_never_admit_report_as_firmware() {
    let (d, repo, s) = setup();
    let bytes = b"\x27\x05\x19\x56harmless synthetic analysis hsqs";
    let original = completed(&s, &source(d.path(), "synthetic-analysis.bin", bytes));
    let hash = original.related_artifact.unwrap();
    let analyzed = wait(&s, &s.analyze(&hash).unwrap().id);
    assert_eq!(analyzed.status, JobStatus::Completed, "{analyzed:?}");
    assert_eq!(analyzed.related_artifact.as_deref(), Some(hash.as_str()));
    let report = s.analysis_report(&hash).unwrap().unwrap();
    assert_eq!(report.input_sha256, hash);
    assert_eq!(report.format, "U-Boot legacy");
    assert_eq!(
        repo.lock()
            .unwrap()
            .connection
            .query_row("SELECT COUNT(*) FROM firmware_artifacts", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        repo.lock()
            .unwrap()
            .connection
            .query_row(
                "SELECT COUNT(*) FROM artifact_occurrences WHERE artifact_class='metadata_report'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    let root = s.inner.root.clone();
    s.stop();
    drop(s);
    drop(repo);
    let repo = Arc::new(Mutex::new(Repository::open(&root.join("test.db")).unwrap()));
    let reopened = LibraryService::start(repo, root, DEFAULT_MAX, Arc::new(|_| {})).unwrap();
    assert_eq!(
        reopened
            .analysis_report(&hash)
            .unwrap()
            .unwrap()
            .input_sha256,
        hash
    );
    assert_eq!(
        fs::read(reopened.inner.vault.blob(&hash).unwrap().0).unwrap(),
        bytes
    );
    reopened.stop();
}

#[test]
fn sprint_unapproved_official_source_queues_nothing_and_cannot_promote_import() {
    let (d, repo, s) = setup();
    assert!(s
        .acquire_official("https://attacker.invalid/image.bin", true)
        .is_err());
    assert!(s
        .acquire_official(&official::catalogue().unwrap()[0].id, false)
        .is_err());
    assert!(s.jobs().unwrap().is_empty());
    let original = completed(
        &s,
        &source(
            d.path(),
            "synthetic-official-negative.bin",
            b"not official firmware",
        ),
    );
    let hash = original.related_artifact.unwrap();
    let proof = s
        .inner
        .vault
        .verify_published(&hash, DEFAULT_MAX, &AtomicBool::new(false), |_| Ok(()))
        .unwrap();
    let source = official::qualify(&official::catalogue().unwrap()[0].id, true).unwrap();
    let mut repo = repo.lock().unwrap();
    let tx = repo.connection.transaction().unwrap();
    assert!(admission::admit_official(&tx, &original.id, &proof, &source).is_err());
    tx.rollback().unwrap();
    drop(repo);
    assert!(s.official_provenance(&hash).unwrap().is_empty());
    assert_eq!(
        s.details(&hash).unwrap().artifact.artifact_class,
        "user_imported_unverified"
    );
    s.stop();
}

#[test]
fn sprint_interrupted_download_and_analysis_fail_without_retry_or_admission() {
    let (_dir, repo, s) = setup();
    let root = s.inner.root.clone();
    let mut ids = Vec::new();
    for kind in ["official_image_download", "firmware_analysis"] {
        let id = nonce().unwrap();
        repo.lock()
            .unwrap()
            .create_library_job(
                &id,
                kind,
                32,
                JobInput {
                    filename: Some("synthetic-interrupted.bin"),
                    ..Default::default()
                },
            )
            .unwrap();
        repo.lock()
            .unwrap()
            .library_transition(&id, JobStatus::Running, None)
            .unwrap();
        fs::write(
            s.inner.vault.stage(&id).unwrap(),
            b"harmless incomplete bytes",
        )
        .unwrap();
        ids.push(id);
    }
    s.stop();
    drop(s);
    let reopened =
        LibraryService::start(Arc::clone(&repo), root, DEFAULT_MAX, Arc::new(|_| {})).unwrap();
    for id in ids {
        let job = reopened.job(&id).unwrap();
        assert_eq!(job.status, JobStatus::Failed);
        assert_eq!(job.error_code.as_deref(), Some("INTERRUPTED"));
        assert!(!reopened.inner.vault.stage(&id).unwrap().exists());
    }
    assert_eq!(
        repo.lock()
            .unwrap()
            .connection
            .query_row("SELECT COUNT(*) FROM firmware_artifacts", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(reopened.inner.controls.lock().unwrap().is_empty());
    reopened.stop();
}

#[test]
fn review_hash_lookup_never_selects_a_filename_match() {
    let (d, repo, s) = setup();
    let original = completed(&s, &source(d.path(), "first.bin", b"first harmless blob"));
    let hash = original.related_artifact.unwrap();
    let decoy = completed(
        &s,
        &source(
            d.path(),
            &format!("contains-{hash}.bin"),
            b"different harmless blob",
        ),
    );
    repo.lock()
        .unwrap()
        .connection
        .execute(
            "UPDATE artifact_details SET first_imported_at=first_imported_at+1000 WHERE sha256=?1",
            [decoy.related_artifact.unwrap()],
        )
        .unwrap();
    assert_eq!(s.details(&hash).unwrap().artifact.sha256, hash);
}

#[test]
fn review_missing_configured_library_keeps_startup_and_metadata() {
    let (d, repo, s) = setup();
    let original = completed(
        &s,
        &source(d.path(), "retained.bin", b"harmless retained bytes"),
    );
    let hash = original.related_artifact.unwrap();
    let details = s.details(&hash).unwrap();
    let root = s.inner.root.clone();
    s.stop();
    drop(s);
    let unavailable = d.path().join("disconnected-volume").join("library");
    {
        let repository = repo.lock().unwrap();
        let mut preferences = repository.desktop_preferences().unwrap();
        preferences.library_directory = Some(unavailable.to_string_lossy().into_owned());
        repository
            .connection
            .execute(
                "UPDATE desktop_preferences SET value=?1 WHERE id=1",
                [serde_json::to_string(&preferences).unwrap()],
            )
            .unwrap();
    }
    let reopened =
        LibraryService::start(repo.clone(), root, DEFAULT_MAX, Arc::new(|_| {})).unwrap();
    assert!(!unavailable.exists(), "must not recreate relocated storage");
    assert_eq!(
        serde_json::to_value(reopened.details(&hash).unwrap()).unwrap(),
        serde_json::to_value(details).unwrap()
    );
    assert_eq!(
        reopened
            .select_path(&d.path().join("anything.bin"))
            .unwrap_err()
            .code,
        "LIBRARY_UNAVAILABLE"
    );
    assert_eq!(
        repo.lock()
            .unwrap()
            .desktop_preferences()
            .unwrap()
            .library_directory
            .unwrap(),
        unavailable.to_string_lossy()
    );
    reopened.stop();
}

#[test]
fn unavailable_library_can_recover_default_without_losing_records_or_backup() {
    let (d, repo, s) = setup();
    let original = completed(
        &s,
        &source(d.path(), "recoverable.bin", b"generated recovery data"),
    );
    let hash = original.related_artifact.unwrap();
    wait(&s, &s.create_backup(&hash).unwrap().id);
    let backup = s.backups().unwrap();
    let root = s.inner.root.clone();
    s.stop();
    drop(s);
    let lost = d.path().join("absent-drive");
    {
        let r = repo.lock().unwrap();
        let mut p = r.desktop_preferences().unwrap();
        p.library_directory = Some(lost.to_string_lossy().into());
        r.connection
            .execute(
                "UPDATE desktop_preferences SET value=?1 WHERE id=1",
                [serde_json::to_string(&p).unwrap()],
            )
            .unwrap();
    }
    let offline =
        LibraryService::start(repo.clone(), root.clone(), DEFAULT_MAX, Arc::new(|_| {})).unwrap();
    assert_eq!(
        offline.recover_default_library(false).unwrap_err().code,
        "CONFIRMATION_REQUIRED"
    );
    assert!(repo
        .lock()
        .unwrap()
        .desktop_preferences()
        .unwrap()
        .library_directory
        .is_some());
    offline.recover_default_library(true).unwrap();
    assert!(offline.desktop_state().unwrap().restart_required);
    assert!(offline.recover_default_library(true).is_err());
    assert!(!lost.exists());
    offline.stop();
    drop(offline);
    let recovered = LibraryService::start(repo, root, DEFAULT_MAX, Arc::new(|_| {})).unwrap();
    assert!(recovered.desktop_state().unwrap().library_available);
    assert!(recovered
        .desktop_state()
        .unwrap()
        .preferences
        .library_directory
        .is_none());
    assert_eq!(
        recovered.details(&hash).unwrap().artifact.original_filename,
        "recoverable.bin"
    );
    assert_eq!(recovered.backups().unwrap()[0].id, backup[0].id);
    assert_eq!(
        wait(&recovered, &recovered.verify(&hash).unwrap().id).status,
        JobStatus::Completed
    );
    recovered.stop();
}

#[cfg(windows)]
#[test]
fn review_directory_lease_blocks_rename_until_released() {
    let d = tempfile::tempdir().unwrap();
    let original = d.path().join("leased");
    fs::create_dir(&original).unwrap();
    let guards = guard_directories(&original).unwrap();
    let renamed = d.path().join("renamed");
    assert!(fs::rename(&original, &renamed).is_err());
    drop(guards);
    fs::rename(&original, &renamed).unwrap();
}

#[test]
fn stream_hashes_empty_small_and_large_with_bounded_reads() {
    struct Bounded<R>(R);
    impl<R: Read> Read for Bounded<R> {
        fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
            assert!(b.len() <= CHUNK);
            self.0.read(b)
        }
    }
    for data in [vec![], b"abc".to_vec(), vec![0x42; 17 * CHUNK + 123]] {
        let mut out = vec![];
        let (hash, n) = stream(
            &mut Bounded(Cursor::new(&data)),
            &mut out,
            data.len() as u64,
            &AtomicBool::new(false),
            |_| Ok(()),
        )
        .unwrap();
        assert_eq!(hash, format!("{:x}", Sha256::digest(&data)));
        assert_eq!(n, data.len() as u64);
        assert_eq!(out, data);
    }
}
#[test]
fn stream_cancels_between_chunks_and_maps_disk_full() {
    let flag = AtomicBool::new(false);
    let mut source = Cursor::new(vec![1; 3 * CHUNK]);
    let mut out = vec![];
    let e = stream(&mut source, &mut out, 3 * CHUNK as u64, &flag, |_| {
        flag.store(true, Ordering::Release);
        Ok(())
    })
    .unwrap_err();
    assert_eq!(e.code, "CANCELLED");
    assert_eq!(out.len(), CHUNK);
    struct Full;
    impl Write for Full {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from_raw_os_error(112))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    assert_eq!(
        stream(
            &mut Cursor::new(b"abc"),
            &mut Full,
            3,
            &AtomicBool::new(false),
            |_| Ok(())
        )
        .unwrap_err()
        .code,
        "DISK_FULL"
    );
}
#[test]
fn validated_paths_and_digest_names_prevent_traversal() {
    let (d, _, service) = setup();
    for hash in ["../evil", &"A".repeat(64), &"x".repeat(64)] {
        assert!(service.inner.vault.blob(hash).is_err());
    }
    assert!(service.inner.vault.stage("../source").is_err());
    assert!(safe_path(Path::new("relative.bin")).is_err());
    assert!(safe_path(&d.path().join("..\\escape")).is_err());
    let hash = "a".repeat(64);
    let (p, _) = service.inner.vault.blob(&hash).unwrap();
    assert!(p.ends_with(format!("sha256/aa/{hash}.bin")));
    #[cfg(windows)]
    for p in [
        r"\\server\share\file.bin",
        r"C:\file.bin:secret",
        r"\\.\PhysicalDrive0",
    ] {
        assert!(safe_path(Path::new(p)).is_err());
    }
}
#[test]
fn fresh_import_preserves_source_and_commits_metadata_and_progress() {
    let (d, _, s) = setup();
    let p = source(
        d.path(),
        "untrusted-vendor-name.bin",
        b"harmless fixture bytes",
    );
    let job = completed(&s, &p);
    let hash = job.related_artifact.unwrap();
    let details = s.details(&hash).unwrap();
    assert_eq!(details.artifact.integrity_status, "verified");
    assert_eq!(details.artifact.format, "UNKNOWN");
    assert_eq!(details.imports[0].source_category, "user_import");
    assert_eq!(details.imports[0].provenance_status, "unverified");
    assert_eq!(job.progress_bytes, job.total_bytes);
    assert_eq!(fs::read(&p).unwrap(), b"harmless fixture bytes");
    assert_eq!(
        fs::read(s.inner.vault.blob(&hash).unwrap().0).unwrap(),
        fs::read(p).unwrap()
    );
    assert!(!s.inner.vault.stage(&job.id).unwrap().exists());
}
#[test]
fn dedup_same_and_different_names_but_same_name_changed_bytes_are_distinct() {
    let (d, _, s) = setup();
    let p = source(d.path(), "first.bin", b"same harmless bytes");
    let a = completed(&s, &p);
    let b = completed(&s, &p);
    let q = source(d.path(), "second.bin", b"same harmless bytes");
    let c = completed(&s, &q);
    assert!(b.duplicate && c.duplicate);
    assert_eq!(a.related_artifact, b.related_artifact);
    assert_eq!(
        s.details(a.related_artifact.as_deref().unwrap())
            .unwrap()
            .artifact
            .source_count,
        3
    );
    fs::write(&p, b"changed harmless bytes").unwrap();
    let different = completed(&s, &p);
    assert_ne!(a.related_artifact, different.related_artifact);
    assert_eq!(s.list(&LibraryQuery::default()).unwrap().total, 2);
}
#[test]
fn corrupt_and_missing_blobs_are_detected_and_never_overwritten() {
    let (d, _, s) = setup();
    let p = source(d.path(), "fixture.bin", b"test-original");
    let first = completed(&s, &p);
    let hash = first.related_artifact.unwrap();
    let blob = s.inner.vault.blob(&hash).unwrap().0;
    fs::write(&blob, b"corrupt bytes").unwrap();
    let j = s.verify(&hash).unwrap();
    assert_eq!(wait(&s, &j.id).error_code.as_deref(), Some("HASH_MISMATCH"));
    assert_eq!(
        s.details(&hash).unwrap().artifact.integrity_status,
        "hash_mismatch"
    );
    let j = start(&s, &p);
    assert_eq!(wait(&s, &j.id).status, JobStatus::Failed);
    assert_eq!(fs::read(&blob).unwrap(), b"corrupt bytes");
    assert_eq!(s.details(&hash).unwrap().artifact.source_count, 1);
    fs::remove_file(&blob).unwrap();
    let j = s.verify(&hash).unwrap();
    assert_eq!(wait(&s, &j.id).error_code.as_deref(), Some("FILE_MISSING"));
    assert_eq!(s.details(&hash).unwrap().artifact.storage_status, "missing");
}
#[test]
fn selected_files_are_bounded_single_use_and_windows_write_locked() {
    let (d, _, s) = setup();
    assert_eq!(
        s.select_path(&d.path().join("missing.bin"))
            .unwrap_err()
            .code,
        "FILE_MISSING"
    );
    assert!(s.select_path(d.path()).is_err());
    let p = source(d.path(), "held.bin", b"selected fixture");
    let selected = s.select_path(&p).unwrap();
    #[cfg(windows)]
    assert!(std::fs::OpenOptions::new().write(true).open(&p).is_err());
    let input = || ImportRequest {
        ticket: selected.ticket.clone(),
        description: None,
        associated_device_id: None,
    };
    let j = s.import(input()).unwrap();
    assert!(s.import(input()).is_err());
    assert_eq!(wait(&s, &j.id).status, JobStatus::Completed);
    let too_big = source(d.path(), "large.bin", b"big");
    let small_root = d.path().join("limited");
    fs::create_dir(&small_root).unwrap();
    let repo = Arc::new(Mutex::new(
        Repository::open(&small_root.join("db")).unwrap(),
    ));
    let limited = LibraryService::start(repo, small_root, 2, Arc::new(|_| {})).unwrap();
    assert_eq!(
        limited.select_path(&too_big).unwrap_err().code,
        "FILE_TOO_LARGE"
    );
}
#[test]
fn cancellation_during_copy_has_no_catalog_entry_and_is_idempotent() {
    let (d, repo, s) = setup();
    let p = source(d.path(), "cancel.bin", &vec![5; 8 * CHUNK]);
    // Deterministic cancellation at a real worker progress callback, no production delay hooks.
    let root = d.path().join("cancel-data");
    fs::create_dir(&root).unwrap();
    let r = Arc::new(Mutex::new(Repository::open(&root.join("db")).unwrap()));
    let (arrive, arrived) = mpsc::channel();
    let (release, proceed) = mpsc::channel();
    let proceed = Mutex::new(proceed);
    let service = LibraryService::start(
        r,
        root,
        DEFAULT_MAX,
        Arc::new(move |job| {
            if job.stage == "copying" && job.progress_bytes == 0 {
                arrive.send(()).unwrap();
                proceed.lock().unwrap().recv().unwrap();
            }
        }),
    )
    .unwrap();
    let j = start(&service, &p);
    arrived.recv_timeout(Duration::from_secs(5)).unwrap();
    // The public cancellation fence is authoritative even if completion wins a race.
    let cancelled = service.cancel(&j.id).unwrap();
    release.send(()).unwrap();
    let final_job = wait(&service, &j.id);
    assert_eq!(cancelled.status, JobStatus::Running);
    assert_eq!(final_job.status, JobStatus::Cancelled);
    assert_eq!(service.cancel(&j.id).unwrap().status, JobStatus::Cancelled);
    assert_eq!(service.list(&LibraryQuery::default()).unwrap().total, 0);
    assert_eq!(repo.lock().unwrap().library_jobs().unwrap().len(), 0);
    drop(s);
}
#[test]
fn concurrent_identical_imports_share_one_blob_and_finish_durably() {
    let (d, _, s) = setup();
    let p = source(d.path(), "parallel.bin", &vec![7; 2 * CHUNK]);
    let a = start(&s, &p);
    let b = start(&s, &p);
    let a = wait(&s, &a.id);
    let b = wait(&s, &b.id);
    assert_eq!(a.status, JobStatus::Completed);
    assert_eq!(b.status, JobStatus::Completed);
    assert!(b.duplicate);
    assert_eq!(a.related_artifact, b.related_artifact);
    assert_eq!(s.list(&LibraryQuery::default()).unwrap().total, 1);
}
#[test]
fn database_failure_after_publication_preserves_orphan_and_retry_reuses_verified_bytes() {
    let (d, repo, s) = setup();
    repo.lock().unwrap().connection.execute_batch("CREATE TRIGGER fail_import BEFORE INSERT ON artifact_imports BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    let p = source(
        d.path(),
        "db-failure.bin",
        b"database failure harmless fixture",
    );
    let j = start(&s, &p);
    let failed = wait(&s, &j.id);
    assert_eq!(failed.status, JobStatus::Failed);
    assert_eq!(failed.error_code.as_deref(), Some("DATABASE_UNAVAILABLE"));
    assert_eq!(s.list(&LibraryQuery::default()).unwrap().total, 0);
    let hash = failed.related_artifact.unwrap();
    for table in [
        "artifact_blobs",
        "artifact_occurrences",
        "firmware_admissions",
        "artifact_blob_state",
    ] {
        assert_eq!(
            repo.lock()
                .unwrap()
                .connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r
                    .get::<_, u32>(0))
                .unwrap(),
            0,
            "{table} rolls back with the failed import"
        );
    }
    assert!(s.inner.vault.blob(&hash).unwrap().0.exists());
    repo.lock()
        .unwrap()
        .connection
        .execute_batch("DROP TRIGGER fail_import;")
        .unwrap();
    let retry = completed(&s, &p);
    assert!(retry.duplicate);
    assert_eq!(retry.related_artifact.as_deref(), Some(hash.as_str()));
}
#[test]
fn restart_recovers_partial_and_published_jobs_without_false_success() {
    let (d, repo, s) = setup();
    let root = s.inner.root.clone();
    let ids = [nonce().unwrap(), nonce().unwrap()];
    for id in &ids {
        repo.lock()
            .unwrap()
            .create_library_job(
                id,
                "firmware_import",
                3,
                JobInput {
                    filename: Some("interrupted.bin"),
                    ..Default::default()
                },
            )
            .unwrap();
        repo.lock()
            .unwrap()
            .library_transition(id, JobStatus::Running, None)
            .unwrap();
        fs::write(s.inner.vault.stage(id).unwrap(), b"abc").unwrap();
    }
    let hash = format!("{:x}", Sha256::digest(b"abc"));
    let blob = s.inner.vault.blob(&hash).unwrap().0;
    fs::hard_link(s.inner.vault.stage(&ids[1]).unwrap(), &blob).unwrap();
    repo.lock()
        .unwrap()
        .library_candidate(&ids[1], &hash)
        .unwrap();
    let unknown = s.inner.vault.staging.join("unknown-owner.part");
    fs::write(&unknown, b"leave alone").unwrap();
    s.stop();
    drop(s);
    let s = LibraryService::start(repo, root, DEFAULT_MAX, Arc::new(|_| {})).unwrap();
    for id in ids {
        let j = s.job(&id).unwrap();
        assert_eq!(j.status, JobStatus::Failed);
        assert_eq!(j.error_code.as_deref(), Some("INTERRUPTED"));
        assert!(!s.inner.vault.stage(&id).unwrap().exists());
    }
    assert!(unknown.exists() && blob.exists());
    assert_eq!(s.list(&LibraryQuery::default()).unwrap().total, 0);
    assert!(d.path().exists());
}
#[test]
fn jobs_enforce_transitions_and_cannot_commit_after_cancellation() {
    let (dir, repo, service) = setup();
    let fixture = source(dir.path(), "cancel-proof.bin", b"harmless cancel proof");
    let imported = completed(&service, &fixture);
    let proof = service
        .inner
        .vault
        .verify_published(
            imported.related_artifact.as_deref().unwrap(),
            DEFAULT_MAX,
            &AtomicBool::new(false),
            |_| Ok(()),
        )
        .unwrap();
    let id = nonce().unwrap();
    let mut repo = repo.lock().unwrap();
    repo.create_library_job(
        &id,
        "firmware_import",
        0,
        JobInput {
            filename: Some("empty.bin"),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(repo
        .library_transition(&id, JobStatus::Completed, None)
        .is_err());
    repo.library_transition(&id, JobStatus::Cancelled, None)
        .unwrap();
    assert!(repo
        .library_transition(&id, JobStatus::Running, None)
        .is_err());
    assert!(repo.commit_library_import(&id, &proof, false).is_err());
}
#[test]
fn metadata_search_pagination_manifest_privacy_and_no_overwrite() {
    let (d, _, s) = setup();
    let p = source(d.path(), "searchable.bin", b"manifest-safe-fixture");
    let j = completed(&s, &p);
    let hash = j.related_artifact.unwrap();
    assert_eq!(
        s.list(&LibraryQuery {
            search: Some("SEARCHABLE".into()),
            ..Default::default()
        })
        .unwrap()
        .total,
        1
    );
    assert_eq!(
        s.list(&LibraryQuery {
            integrity: Some("verified".into()),
            source: Some("user_import".into()),
            ..Default::default()
        })
        .unwrap()
        .total,
        1
    );
    assert_eq!(
        s.list(&LibraryQuery {
            search: Some("%".into()),
            ..Default::default()
        })
        .unwrap()
        .total,
        0
    );
    assert!(s
        .list(&LibraryQuery {
            page: Some(100001),
            ..Default::default()
        })
        .is_err());
    let out = d.path().join("manifest.json");
    s.export_path(&hash, &out).unwrap();
    let text = fs::read_to_string(&out).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["manifestSchemaVersion"], 1);
    assert_eq!(v["sha256"], hash);
    assert!(!text.contains(&s.inner.root.to_string_lossy().to_string()));
    assert!(!text.contains("Harmless generated test fixture"));
    assert!(!text.contains("associatedDeviceId"));
    assert!(s.export_path(&hash, &out).is_err());
    assert_eq!(fs::read_to_string(out).unwrap(), text);
}
#[test]
fn manifest_saved_survives_activity_failure_without_partial_audit_or_overwrite() {
    let (d, repo, s) = setup();
    let input = source(
        d.path(),
        "harmless-export.bin",
        b"generated metadata export bytes",
    );
    let job = completed(&s, &input);
    let hash = job.related_artifact.unwrap();
    let events = repo
        .lock()
        .unwrap()
        .connection
        .query_row("SELECT COUNT(*) FROM audit_events", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap();
    repo.lock().unwrap().connection.execute_batch("CREATE TRIGGER fail_export_activity BEFORE INSERT ON activity_links WHEN (SELECT kind FROM audit_events WHERE id=NEW.event_id)='manifest_exported' BEGIN SELECT RAISE(FAIL,'injected activity failure'); END;").unwrap();
    let path = d.path().join("saved-despite-log-failure.json");
    let result = s.export_path(&hash, &path).unwrap();
    assert!(result.saved);
    assert!(!result.activity_recorded);
    assert!(result.warning.unwrap().contains("Manifest saved"));
    let bytes = fs::read(&path).unwrap();
    let data: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(data["sha256"], hash);
    assert_eq!(
        repo.lock()
            .unwrap()
            .connection
            .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        events
    );
    assert!(s.export_path(&hash, &path).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    repo.lock()
        .unwrap()
        .connection
        .execute_batch("DROP TRIGGER fail_export_activity;")
        .unwrap();
    let result = s.export_path(&hash, &d.path().join("logged.json")).unwrap();
    assert!(result.saved && result.activity_recorded && result.warning.is_none());
}

#[test]
fn shutdown_during_verification_cancels_and_repeated_startup_keeps_terminal_state() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("data");
    fs::create_dir(&root).unwrap();
    let repo = Arc::new(Mutex::new(Repository::open(&root.join("db")).unwrap()));
    let gate = Arc::new(AtomicBool::new(false));
    let active = Arc::clone(&gate);
    let (tx, rx) = mpsc::channel();
    let (release, go) = mpsc::channel();
    let go = Mutex::new(go);
    let s = LibraryService::start(
        Arc::clone(&repo),
        root.clone(),
        DEFAULT_MAX,
        Arc::new(move |j| {
            if active.load(Ordering::Acquire)
                && j.kind == "integrity_verify"
                && j.status == JobStatus::Running
                && j.stage == "running"
            {
                tx.send(()).unwrap();
                go.lock().unwrap().recv().unwrap();
            }
        }),
    )
    .unwrap();
    let j = completed(
        &s,
        &source(d.path(), "verify-shutdown.bin", b"harmless verified bytes"),
    );
    let hash = j.related_artifact.unwrap();
    gate.store(true, Ordering::Release);
    let j = s.verify(&hash).unwrap();
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let copy = s.clone();
    let closing = thread::spawn(move || copy.stop());
    while !s.inner.shutdown.load(Ordering::Acquire) {
        thread::yield_now();
    }
    release.send(()).unwrap();
    closing.join().unwrap();
    assert_eq!(wait(&s, &j.id).status, JobStatus::Cancelled);
    drop(s);
    for _ in 0..2 {
        let service = LibraryService::start(
            Arc::clone(&repo),
            root.clone(),
            DEFAULT_MAX,
            Arc::new(|_| {}),
        )
        .unwrap();
        assert_eq!(service.job(&j.id).unwrap().status, JobStatus::Cancelled);
        assert_ne!(
            service.details(&hash).unwrap().artifact.integrity_status,
            "checking"
        );
        service.stop();
        drop(service);
    }
}
#[test]
fn nonempty_schema_three_upgrade_and_repeated_startup_preserve_all_legacy_rows() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute_batch(include_str!("../../migrations/001_foundation.sql"))
        .unwrap();
    c.execute_batch(include_str!("../../migrations/002_discovery.sql"))
        .unwrap();
    c.execute_batch(include_str!("../../migrations/003_profiles.sql"))
        .unwrap();
    c.execute_batch("PRAGMA user_version=3; UPDATE settings SET value='{\"theme\":\"dark\"}'; INSERT INTO devices VALUES('old-device','Keep me',1); INSERT INTO jobs VALUES('old-job','existing','completed');").unwrap();
    c.execute(
        "INSERT INTO firmware_artifacts VALUES(?1,123)",
        ["a".repeat(64)],
    )
    .unwrap();
    drop(c);
    for _ in 0..2 {
        let r = crate::storage::schema6::open_with_upgrade(&path, |_| true)
            .unwrap()
            .unwrap();
        assert_eq!(r.snapshot(dir.path()).unwrap().database_version, 6);
        assert_eq!(
            r.connection
                .query_row("SELECT COUNT(*) FROM jobs WHERE id='old-job'", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            r.connection
                .query_row("SELECT COUNT(*) FROM firmware_artifacts", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            r.connection
                .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        assert_eq!(
            r.connection
                .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
            0
        );
        assert!(matches!(
            r.settings().unwrap().theme,
            crate::domain::Theme::Dark
        ));
    }
}

#[test]
fn empty_import_and_selection_expiry_are_explicit() {
    let (dir, _, s) = setup();
    let p = source(dir.path(), "empty.bin", b"");
    let job = completed(&s, &p);
    let hash = format!("{:x}", Sha256::digest(b""));
    assert_eq!(job.related_artifact.as_deref(), Some(hash.as_str()));
    assert_eq!(s.details(&hash).unwrap().artifact.size_bytes, 0);
    let picked = s.select_path(&p).unwrap();
    s.inner
        .tickets
        .lock()
        .unwrap()
        .get_mut(&picked.ticket)
        .unwrap()
        .preview
        .expires_at = 1;
    assert_eq!(
        s.import(ImportRequest {
            ticket: picked.ticket,
            description: None,
            associated_device_id: None
        })
        .unwrap_err()
        .code,
        "SELECTION_EXPIRED"
    );
}
#[test]
fn changed_source_snapshot_fails_without_a_catalog_entry() {
    let (dir, _, s) = setup();
    let p = source(dir.path(), "changed.bin", b"original bytes");
    let picked = s.select_path(&p).unwrap();
    // Windows prevents mutation while selected; inject mismatched before/after metadata instead.
    let different = source(dir.path(), "different.bin", b"short");
    s.inner
        .tickets
        .lock()
        .unwrap()
        .get_mut(&picked.ticket)
        .unwrap()
        .metadata = fs::metadata(different).unwrap();
    let job = s
        .import(ImportRequest {
            ticket: picked.ticket,
            description: None,
            associated_device_id: None,
        })
        .unwrap();
    let job = wait(&s, &job.id);
    assert_eq!(job.error_code.as_deref(), Some("SOURCE_CHANGED"));
    assert_eq!(s.list(&LibraryQuery::default()).unwrap().total, 0);
    assert_eq!(fs::read(p).unwrap(), b"original bytes");
}
#[test]
fn cancellation_at_publication_does_not_poison_an_existing_valid_artifact() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("data");
    fs::create_dir(&root).unwrap();
    let repo = Arc::new(Mutex::new(Repository::open(&root.join("db")).unwrap()));
    let gate = Arc::new(AtomicBool::new(false));
    let active = Arc::clone(&gate);
    let (tx, rx) = mpsc::channel();
    let (release, go) = mpsc::channel();
    let go = Mutex::new(go);
    let s = LibraryService::start(
        repo,
        root,
        DEFAULT_MAX,
        Arc::new(move |j| {
            if active.load(Ordering::Acquire)
                && j.stage == "checking_vault"
                && j.progress_bytes == 0
            {
                tx.send(()).unwrap();
                go.lock().unwrap().recv().unwrap();
            }
        }),
    )
    .unwrap();
    let p = source(d.path(), "fixture.bin", b"cancel near completion");
    let original = completed(&s, &p);
    gate.store(true, Ordering::Release);
    let job = start(&s, &p);
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    s.cancel(&job.id).unwrap();
    release.send(()).unwrap();
    assert_eq!(wait(&s, &job.id).status, JobStatus::Cancelled);
    let d = s
        .details(original.related_artifact.as_deref().unwrap())
        .unwrap();
    assert_eq!(d.artifact.integrity_status, "verified");
    assert_eq!(d.artifact.source_count, 1);
}
#[cfg(windows)]
#[test]
fn directory_junctions_and_exclusive_source_handles_are_rejected() {
    use std::os::windows::fs::OpenOptionsExt;
    let (d, _, s) = setup();
    let target = d.path().join("real-folder");
    fs::create_dir(&target).unwrap();
    source(&target, "fixture.bin", b"junction test bytes");
    let link = d.path().join("linked-folder");
    let create = |link: &Path, target: &Path| {
        let status=std::process::Command::new("powershell.exe").args(["-NoProfile","-Command","$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:OBTAINER_TEST_LINK -Target $env:OBTAINER_TEST_TARGET | Out-Null"]).env("OBTAINER_TEST_LINK",link).env("OBTAINER_TEST_TARGET",target).status().unwrap();
        assert!(status.success());
    };
    create(&link, &target);
    assert_eq!(
        s.select_path(&link.join("fixture.bin")).unwrap_err().code,
        "UNSAFE_PATH"
    );
    let destination = s.inner.vault.root.join("sha256/aa");
    create(&destination, &target);
    assert!(s.inner.vault.blob(&"a".repeat(64)).is_err());
    assert_eq!(
        fs::read(target.join("fixture.bin")).unwrap(),
        b"junction test bytes"
    );
    let p = source(d.path(), "exclusive.bin", b"locked harmless bytes");
    let _exclusive = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&p)
        .unwrap();
    assert!(s.select_path(&p).is_err());
}

#[test]
fn gui_backup_restore_verifies_copies_and_preserves_original_history() {
    let (d, repo, s) = setup();
    let bytes = b"harmless original backup fixture";
    let original = completed(&s, &source(d.path(), "router.bin", bytes));
    let hash = original.related_artifact.unwrap();
    let backup_job = wait(&s, &s.create_backup(&hash).unwrap().id);
    assert_eq!(backup_job.status, JobStatus::Completed);
    let b = s.backups().unwrap().remove(0);
    assert_eq!(fs::read(&b.location).unwrap(), bytes);
    while !s.inner.controls.lock().unwrap().is_empty() {
        thread::yield_now();
    }
    assert!(s.remove_item(&hash, false, false).is_err());
    s.remove_item(&hash, false, true).unwrap();
    assert_eq!(s.list(&LibraryQuery::default()).unwrap().total, 0);
    let restored = wait(&s, &s.restore_backup(&b.id, true).unwrap().id);
    assert_eq!(restored.status, JobStatus::Completed);
    let details = s.details(&hash).unwrap();
    assert_eq!(details.artifact.original_filename, "router_DEFAULT.bin");
    assert_eq!(details.artifact.kind, "restored_default");
    assert_eq!(details.artifact.source_count, 2);
    assert_eq!(fs::read(&b.location).unwrap(), bytes);
    assert_eq!(
        repo.lock()
            .unwrap()
            .library_job(&original.id)
            .unwrap()
            .status,
        JobStatus::Completed
    );
    assert!(s.restore_backup(&b.id, false).is_err());
    while !s.inner.controls.lock().unwrap().is_empty() {
        thread::yield_now();
    }
    s.remove_item(&b.id, true, true).unwrap();
    assert!(s.backups().unwrap().is_empty());
    assert!(Path::new(&b.location).is_file());
    assert_eq!(
        repo.lock()
            .unwrap()
            .connection
            .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}
#[test]
fn gui_backup_and_restore_reject_corruption_without_claiming_success() {
    let (d, _, s) = setup();
    let original = completed(
        &s,
        &source(d.path(), "corruption.bin", b"generated test bytes"),
    );
    let hash = original.related_artifact.unwrap();
    let good = wait(&s, &s.create_backup(&hash).unwrap().id);
    assert_eq!(good.status, JobStatus::Completed);
    let b = s.backups().unwrap().remove(0);
    fs::write(&b.location, b"tampered backup bytes").unwrap();
    let restored = wait(&s, &s.restore_backup(&b.id, true).unwrap().id);
    assert_eq!(restored.status, JobStatus::Failed);
    assert_eq!(restored.error_code.as_deref(), Some("HASH_MISMATCH"));
    assert_eq!(s.details(&hash).unwrap().artifact.source_count, 1);
    let (blob, _guards) = s.inner.vault.blob(&hash).unwrap();
    fs::write(blob, b"tampered vault bytes").unwrap();
    let backup = wait(&s, &s.create_backup(&hash).unwrap().id);
    assert_eq!(backup.status, JobStatus::Failed);
    assert_eq!(backup.error_code.as_deref(), Some("HASH_MISMATCH"));
    assert_eq!(s.backups().unwrap().len(), 1);
}
#[test]
fn gui_restored_names_keep_extensions_and_avoid_casefold_conflicts() {
    use super::desktop::restored_name;
    assert_eq!(
        restored_name("Huawei_E5577.bin", &[]),
        "Huawei_E5577_DEFAULT.bin"
    );
    assert_eq!(restored_name("router", &[]), "router_DEFAULT");
    assert_eq!(
        restored_name("router_DEFAULT.img", &[]),
        "router_DEFAULT.img"
    );
    assert_eq!(
        restored_name(
            "router.bin",
            &["ROUTER_default.BIN".into(), "router_DEFAULT_1.bin".into()]
        ),
        "router_DEFAULT_2.bin"
    );
}
#[test]
fn gui_preferences_are_strict_persistent_and_cannot_override_storage_paths() {
    use super::desktop::PreferenceInput;
    let (d, repo, s) = setup();
    let old = repo.lock().unwrap().settings().unwrap();
    assert!(serde_json::from_str::<PreferenceInput>(r#"{"autoRefresh":false,"notifyCompleted":false,"notifyFailed":true,"notifyDevices":true,"libraryDirectory":"C:/unsafe"}"#).is_err());
    let saved = repo
        .lock()
        .unwrap()
        .save_desktop_preferences(PreferenceInput {
            auto_refresh: false,
            notify_completed: false,
            notify_failed: true,
            notify_devices: true,
        })
        .unwrap();
    s.stop();
    drop(s);
    drop(repo);
    let real = d.path().join("data/test.db");
    let reopened = Repository::open(&real).unwrap();
    assert_eq!(reopened.desktop_preferences().unwrap(), saved);
    assert_eq!(reopened.settings().unwrap(), old);
}
#[test]
fn gui_type_sort_and_visibility_filters_preserve_shared_bytes() {
    let (d, _, s) = setup();
    let a = completed(&s, &source(d.path(), "a.bin", b"short"));
    let b = completed(&s, &source(d.path(), "b.bin", b"longer generated bytes"));
    let page = s
        .list(&LibraryQuery {
            sort: Some("largest".into()),
            kind: Some("imported_original".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(page.total, 2);
    assert_eq!(page.artifacts[0].original_filename, "b.bin");
    assert_eq!(
        s.list(&LibraryQuery {
            kind: Some("extracted".into()),
            ..Default::default()
        })
        .unwrap()
        .total,
        0
    );
    assert!(s
        .list(&LibraryQuery {
            sort: Some("DROP TABLE jobs".into()),
            ..Default::default()
        })
        .is_err());
    while !s.inner.controls.lock().unwrap().is_empty() {
        thread::yield_now();
    }
    s.remove_item(a.related_artifact.as_deref().unwrap(), false, true)
        .unwrap();
    let page = s.list(&LibraryQuery::default()).unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.unique_bytes, b"longer generated bytes".len() as u64);
    assert_eq!(page.artifacts[0].sha256, b.related_artifact.unwrap());
    assert!(s
        .inner
        .vault
        .blob(a.related_artifact.as_deref().unwrap())
        .unwrap()
        .0
        .exists());
}
#[test]
fn gui_storage_copy_switches_only_after_verified_commit_and_restart() {
    let (d, repo, s) = setup();
    let input = source(
        d.path(),
        "relocate.bin",
        b"generated storage migration bytes",
    );
    let j = completed(&s, &input);
    let hash = j.related_artifact.unwrap();
    let backup = wait(&s, &s.create_backup(&hash).unwrap().id);
    assert_eq!(backup.status, JobStatus::Completed);
    while !s.inner.controls.lock().unwrap().is_empty() {
        thread::yield_now();
    }
    let parent = d.path().join("new-library");
    fs::create_dir(&parent).unwrap();
    let plan = s.plan_directory(&parent, Some(d.path())).unwrap();
    assert_eq!(plan.file_count, 1);
    assert!(s.relocate(&plan.ticket, false).is_err());
    let job = wait(&s, &s.relocate(&plan.ticket, true).unwrap().id);
    assert_eq!(job.status, JobStatus::Completed);
    assert!(s.desktop_state().unwrap().restart_required);
    let old = s.inner.vault.blob(&hash).unwrap().0;
    let new = Path::new(&plan.destination).join(Vault::key(&hash));
    assert_eq!(fs::read(&old).unwrap(), fs::read(&new).unwrap());
    assert!(s.select_path(&input).is_err());
    let backup_dir = s.desktop_state().unwrap().backup_directory;
    s.stop();
    drop(s);
    let reopened =
        LibraryService::start(repo, d.path().join("data"), DEFAULT_MAX, Arc::new(|_| {})).unwrap();
    assert_eq!(
        reopened.desktop_state().unwrap().library_directory,
        plan.destination
    );
    assert_eq!(
        reopened.desktop_state().unwrap().backup_directory,
        backup_dir
    );
    assert!(!reopened.desktop_state().unwrap().restart_required);
    assert_eq!(
        reopened.details(&hash).unwrap().artifact.size_bytes,
        fs::metadata(&input).unwrap().len()
    );
    assert!(old.exists());
}
#[test]
fn gui_storage_conflicts_scope_and_corruption_fail_without_switching() {
    let (d, _, s) = setup();
    let j = completed(
        &s,
        &source(d.path(), "unsafe-move.bin", b"generated safe bytes"),
    );
    let hash = j.related_artifact.unwrap();
    while !s.inner.controls.lock().unwrap().is_empty() {
        thread::yield_now();
    }
    assert!(s.plan_directory(d.path(), Some(d.path())).is_err());
    let outside = tempfile::tempdir().unwrap();
    assert_eq!(
        s.plan_directory(outside.path(), Some(d.path()))
            .unwrap_err()
            .code,
        "TEST_SCOPE_REQUIRED"
    );
    let parent = d.path().join("copy");
    fs::create_dir(&parent).unwrap();
    fs::create_dir(parent.join("OBTAINER-Library")).unwrap();
    assert_eq!(
        s.plan_directory(&parent, None).unwrap_err().code,
        "STORAGE_CONFLICT"
    );
    fs::remove_dir(parent.join("OBTAINER-Library")).unwrap();
    let plan = s.plan_directory(&parent, None).unwrap();
    fs::write(
        s.inner.vault.blob(&hash).unwrap().0,
        b"corrupt generated bytes",
    )
    .unwrap();
    let job = wait(&s, &s.relocate(&plan.ticket, true).unwrap().id);
    assert_eq!(job.status, JobStatus::Failed);
    assert!(s
        .desktop_state()
        .unwrap()
        .preferences
        .library_directory
        .is_none());
    assert!(!s.desktop_state().unwrap().restart_required);
    assert!(Path::new(&plan.destination).is_dir());
}

#[test]
fn gui_active_backup_blocks_removal_and_cancels_without_a_false_copy() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("data");
    fs::create_dir(&root).unwrap();
    let repo = Arc::new(Mutex::new(Repository::open(&root.join("test.db")).unwrap()));
    let (tx, rx) = mpsc::channel();
    let (release, go) = mpsc::channel();
    let go = Mutex::new(go);
    let s = LibraryService::start(
        repo,
        root,
        DEFAULT_MAX,
        Arc::new(move |j| {
            if j.kind == "firmware_backup" && j.status == JobStatus::Running && j.stage == "running"
            {
                tx.send(()).unwrap();
                go.lock().unwrap().recv().unwrap();
            }
        }),
    )
    .unwrap();
    let j = completed(
        &s,
        &source(d.path(), "busy.bin", b"harmless backup cancellation bytes"),
    );
    let hash = j.related_artifact.unwrap();
    let queued = s.create_backup(&hash).unwrap();
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        s.remove_item(&hash, false, true).unwrap_err().code,
        "FILE_IN_USE"
    );
    assert!(s.cancel(&queued.id).unwrap().cancel_requested);
    release.send(()).unwrap();
    assert_eq!(wait(&s, &queued.id).status, JobStatus::Cancelled);
    assert!(s.backups().unwrap().is_empty());
    assert_eq!(s.details(&hash).unwrap().artifact.source_count, 1);
}
#[test]
fn gui_schema_four_upgrade_preserves_existing_rows_and_theme_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy-four.db");
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute_batch(include_str!("../../migrations/001_foundation.sql"))
        .unwrap();
    c.execute_batch(include_str!("../../migrations/002_discovery.sql"))
        .unwrap();
    c.execute_batch(include_str!("../../migrations/003_profiles.sql"))
        .unwrap();
    c.execute_batch("UPDATE settings SET value='{\"theme\":\"dark\"}'; INSERT INTO devices VALUES('old-device','Original MiFi label',1); INSERT INTO jobs VALUES('old-job','historical','completed'); INSERT INTO audit_events(kind,message) VALUES('retained','Original history');").unwrap();
    c.execute(
        "INSERT INTO firmware_artifacts VALUES(?1,42)",
        ["b".repeat(64)],
    )
    .unwrap();
    c.execute_batch(include_str!("../../migrations/004_firmware_library.sql"))
        .unwrap();
    c.execute_batch("PRAGMA user_version=4").unwrap();
    let before: String = c
        .query_row(
            "SELECT value FROM settings WHERE key='preferences'",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| {
            c.query_row("SELECT value FROM settings LIMIT 1", [], |r| r.get(0))
                .unwrap()
        });
    let events: i64 = c
        .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r.get(0))
        .unwrap();
    drop(c);
    for _ in 0..2 {
        let repo = crate::storage::schema6::open_with_upgrade(&path, |_| true)
            .unwrap()
            .unwrap();
        assert_eq!(
            repo.connection
                .query_row("SELECT value FROM settings LIMIT 1", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            before
        );
        assert_eq!(
            repo.connection
                .query_row("SELECT COUNT(*) FROM audit_events", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            events
        );
        assert_eq!(
            repo.connection
                .query_row(
                    "SELECT display_name FROM devices WHERE id='old-device'",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
            "Original MiFi label"
        );
        assert_eq!(
            repo.connection
                .query_row("SELECT size_bytes FROM firmware_artifacts", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            42
        );
        assert!(repo.desktop_preferences().unwrap().auto_refresh);
        assert_eq!(repo.snapshot(dir.path()).unwrap().database_version, 6);
    }
}

#[test]
fn stage3_user_import_has_separate_immutable_occurrence_and_unverified_class() {
    let (dir, repo, service) = setup();
    let p = source(
        dir.path(),
        "arbitrary-not-official.bin",
        b"harmless stage3 class test",
    );
    let first = completed(&service, &p);
    let hash = first.related_artifact.unwrap();
    completed(&service, &p);
    let repo = repo.lock().unwrap();
    let class: String = repo
        .connection
        .query_row(
            "SELECT artifact_class FROM firmware_admissions WHERE sha256=?1",
            [&hash],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(class, "user_imported_unverified");
    assert_eq!(repo.connection.query_row("SELECT COUNT(*) FROM artifact_occurrences WHERE blob_sha256=?1 AND artifact_class='user_imported_unverified' AND provenance_kind='user_import'", [&hash], |r|r.get::<_,u32>(0)).unwrap(), 2);
    assert_eq!(
        repo.connection
            .query_row(
                "SELECT COUNT(*) FROM artifact_blobs WHERE sha256=?1",
                [&hash],
                |r| r.get::<_, u32>(0)
            )
            .unwrap(),
        1
    );
}

#[test]
fn stage3_same_bytes_require_independent_occurrence_and_do_not_reclassify_configuration() {
    let (dir, repo, service) = setup();
    let bytes = b"harmless shared class fixture";
    let hash = format!("{:x}", Sha256::digest(bytes));
    {
        let c = &repo.lock().unwrap().connection;
        c.execute(
            "INSERT INTO artifact_blobs VALUES(?1,?2)",
            rusqlite::params![hash, bytes.len()],
        )
        .unwrap();
        c.execute("INSERT INTO artifact_occurrences(occurrence_id,blob_sha256,artifact_class,provenance_kind,acquisition_occurrence_id,collected_at) VALUES('synthetic-config',?1,'configuration_backup','synthetic','synthetic-config',1)",[&hash]).unwrap();
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM firmware_artifacts", [], |r| r
                .get::<_, u32>(0))
                .unwrap(),
            0
        );
    }
    completed(&service, &source(dir.path(), "local assertion.bin", bytes));
    let c = &repo.lock().unwrap().connection;
    assert_eq!(c.query_row("SELECT artifact_class FROM artifact_occurrences WHERE occurrence_id='synthetic-config'",[],|r|r.get::<_,String>(0)).unwrap(),"configuration_backup");
    assert_eq!(
        c.query_row("SELECT COUNT(*) FROM artifact_blobs", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        c.query_row("SELECT COUNT(*) FROM artifact_occurrences", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        2
    );
}
#[test]
fn stage3_legacy_restore_and_reimport_preserve_admission_class() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("data");
    fs::create_dir(&root).unwrap();
    let path = root.join("fixture.db");
    let c = rusqlite::Connection::open(&path).unwrap();
    for sql in [
        include_str!("../../migrations/001_foundation.sql"),
        include_str!("../../migrations/002_discovery.sql"),
        include_str!("../../migrations/003_profiles.sql"),
        include_str!("../../migrations/004_firmware_library.sql"),
        include_str!("../../migrations/005_desktop_workspace.sql"),
    ] {
        c.execute_batch(sql).unwrap();
    }
    let bytes = b"harmless legacy restore fixture";
    let hash = format!("{:x}", Sha256::digest(bytes));
    c.execute(
        "INSERT INTO firmware_artifacts VALUES(?1,?2)",
        rusqlite::params![hash, bytes.len()],
    )
    .unwrap();
    c.execute(
        "INSERT INTO artifact_details(sha256,storage_key,first_imported_at) VALUES(?1,?2,1)",
        rusqlite::params![hash, Vault::key(&hash)],
    )
    .unwrap();
    c.execute_batch("PRAGMA user_version=5;").unwrap();
    drop(c);
    let repo = Arc::new(Mutex::new(
        crate::storage::schema6::open_with_upgrade(&path, |_| true)
            .unwrap()
            .unwrap(),
    ));
    let service =
        LibraryService::start(Arc::clone(&repo), root, DEFAULT_MAX, Arc::new(|_| {})).unwrap();
    fs::write(service.inner.vault.blob(&hash).unwrap().0, bytes).unwrap();
    assert_eq!(
        wait(&service, &service.create_backup(&hash).unwrap().id).status,
        JobStatus::Completed
    );
    let backup = service.backups().unwrap().remove(0);
    assert_eq!(
        wait(
            &service,
            &service.restore_backup(&backup.id, true).unwrap().id
        )
        .status,
        JobStatus::Completed
    );
    completed(&service, &source(dir.path(), "reimport.bin", bytes));
    let c = &repo.lock().unwrap().connection;
    assert_eq!(
        c.query_row("SELECT artifact_class FROM firmware_admissions", [], |r| {
            r.get::<_, String>(0)
        })
        .unwrap(),
        "legacy_unclassified"
    );
    assert_eq!(
        c.query_row(
            "SELECT COUNT(*) FROM artifact_occurrences WHERE artifact_class='legacy_unclassified'",
            [],
            |r| r.get::<_, u32>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(c.query_row("SELECT COUNT(*) FROM artifact_occurrences WHERE artifact_class='user_imported_unverified'",[],|r|r.get::<_,u32>(0)).unwrap(),1);
}

#[test]
fn stage3_misaddressed_bytes_cannot_mint_published_blob_proof() {
    let (_dir, _repo, service) = setup();
    let expected = "a".repeat(64);
    fs::write(
        service.inner.vault.blob(&expected).unwrap().0,
        b"harmless wrongly addressed bytes",
    )
    .unwrap();
    assert!(
        matches!(service.inner.vault.verify_published(&expected,DEFAULT_MAX,&AtomicBool::new(false),|_|Ok(())),Err(e) if e.code=="HASH_MISMATCH")
    );
    assert_eq!(service.list(&LibraryQuery::default()).unwrap().total, 0);
}

#[test]
fn extraction_blobs_and_reports_survive_library_relocation_without_firmware_admission() {
    let (d, repo, s) = setup();
    let original = completed(&s, &source(d.path(), "synthetic.bin", b"synthetic source"));
    let hash = original.related_artifact.unwrap();
    assert_eq!(
        wait(&s, &s.analyze(&hash).unwrap().id).status,
        JobStatus::Completed
    );
    // Synthetic non-firmware data blob with the same schema relationship as an extraction bundle.
    let raw = b"synthetic unclassified extracted bytes";
    let bundle_hash = format!("{:x}", Sha256::digest(raw));
    fs::write(s.inner.vault.blob(&bundle_hash).unwrap().0, raw).unwrap();
    repo.lock()
        .unwrap()
        .connection
        .execute(
            "INSERT INTO artifact_blobs VALUES(?1,?2)",
            rusqlite::params![bundle_hash, raw.len()],
        )
        .unwrap();
    let parent = d.path().join("relocation-with-derived-data");
    fs::create_dir(&parent).unwrap();
    let plan = s.plan_directory(&parent, Some(d.path())).unwrap();
    assert_eq!(plan.file_count, 3);
    assert_eq!(
        wait(&s, &s.relocate(&plan.ticket, true).unwrap().id).status,
        JobStatus::Completed
    );
    assert_eq!(
        fs::read(Path::new(&plan.destination).join(Vault::key(&bundle_hash))).unwrap(),
        raw
    );
    assert_eq!(
        repo.lock()
            .unwrap()
            .connection
            .query_row("SELECT COUNT(*) FROM firmware_artifacts", [], |r| r
                .get::<_, u64>(0))
            .unwrap(),
        1
    );
    s.stop();
    drop(s);
    let reopened =
        LibraryService::start(repo, d.path().join("data"), DEFAULT_MAX, Arc::new(|_| {})).unwrap();
    assert!(reopened.analysis_report(&hash).unwrap().is_some());
}
#[test]
fn repeated_extraction_start_reuses_pending_job_and_cancellation_is_supported() {
    let (d, repo, s) = setup();
    let original = completed(&s, &source(d.path(), "synthetic.bin", b"synthetic source"));
    let hash = original.related_artifact.unwrap();
    let id = nonce().unwrap();
    repo.lock()
        .unwrap()
        .create_library_job(
            &id,
            "firmware_extraction",
            16,
            JobInput {
                hash: Some(&hash),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(s.extract(&hash).unwrap().id, id);
    assert!(s.cancel(&id).unwrap().cancel_requested);
    s.stop();
    drop(s);
    let reopened =
        LibraryService::start(repo, d.path().join("data"), DEFAULT_MAX, Arc::new(|_| {})).unwrap();
    let job = reopened.job(&id).unwrap();
    assert_eq!(job.status, JobStatus::Failed);
    assert_eq!(job.error_code.as_deref(), Some("INTERRUPTED"));
}

#[test]
fn extracted_export_preserves_existing_destination_and_refuses_internal_paths() {
    let (d, repo, s) = setup();
    let original = completed(
        &s,
        &source(d.path(), "synthetic-export-source.bin", &[0; 1024]),
    );
    let hash = original.related_artifact.unwrap();
    // Explicitly synthetic persisted report exercises export, not a decoder claim.
    let bytes = b"synthetic exported text\n";
    let bundle = format!("{:x}", Sha256::digest(bytes));
    fs::write(s.inner.vault.blob(&bundle).unwrap().0, bytes).unwrap();
    let report = serde_json::json!({"unextractedRegions":[],"version":1,"inputSha256":hash,"inputBytes":1024,
        "region":{"format":"squashfs","offset":0,"length":1024,"compression":4},"container":"raw","expandedBytes":1024,
        "extractor":"synthetic export fixture","toolSha256":"0".repeat(64),
        "entries":[{"path":"safe.txt","kind":"file","size":bytes.len(),"sha256":bundle,"linkTarget":null,"offset":0}],
        "totalBytes":bytes.len(),"fileCount":1,"directoryCount":0,"filesystemRoots":["/"],"workerExit":0,"status":"extracted_verified","limitations":[],"isolation":{},"bundleSha256":bundle,"createdAt":1});
    let raw = serde_json::to_vec(&report).unwrap();
    let report_hash = format!("{:x}", Sha256::digest(&raw));
    fs::write(s.inner.vault.blob(&report_hash).unwrap().0, &raw).unwrap();
    {
        let repo = repo.lock().unwrap();
        for (sha, size) in [(&bundle, bytes.len()), (&report_hash, raw.len())] {
            repo.connection
                .execute(
                    "INSERT INTO artifact_blobs VALUES(?1,?2)",
                    rusqlite::params![sha, size],
                )
                .unwrap();
        }
        repo.connection.execute("INSERT INTO artifact_occurrences(occurrence_id,blob_sha256,artifact_class,provenance_kind,source_reference,acquisition_occurrence_id,collected_at) VALUES('synthetic-export',?1,'metadata_report','isolated_filesystem_extraction',?2,'synthetic-export',1)",rusqlite::params![report_hash,format!("extraction:{hash}")]).unwrap();
    }
    let destination = d.path().join("existing.tar");
    fs::write(&destination, b"owner bytes remain").unwrap();
    assert!(s
        .export_extraction_path(&hash, &["safe.txt".into()], &destination)
        .is_err());
    assert_eq!(fs::read(&destination).unwrap(), b"owner bytes remain");
    assert!(s
        .export_extraction_path(
            &hash,
            &["safe.txt".into()],
            &s.inner.root.join("internal.tar")
        )
        .is_err());
    assert!(!s.inner.root.join("internal.tar").exists());
    assert_eq!(s.list(&LibraryQuery::default()).unwrap().total, 1);
    assert!(!fs::read_dir(d.path()).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".obtainer-export-")));
}
