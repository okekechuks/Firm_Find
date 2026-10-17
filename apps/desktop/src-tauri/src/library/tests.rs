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
    let (_dir, repo, _s) = setup();
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
    assert!(repo
        .commit_library_import(&id, &"a".repeat(64), 0, "invalid", false)
        .is_err());
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
        let r = Repository::open(&path).unwrap();
        assert_eq!(r.snapshot(dir.path()).unwrap().database_version, 4);
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
