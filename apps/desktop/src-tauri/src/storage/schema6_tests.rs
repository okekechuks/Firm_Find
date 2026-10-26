//! Every database in this module belongs to a newly created temporary directory.
use super::*;

#[path = "schema6_crash_tests.rs"]
mod crash;

#[path = "schema6_contract_tests.rs"]
mod contract;

#[cfg(windows)]
#[test]
fn stage3_windows_path_aliases_share_migration_admission_without_creating_data() {
    let directory = tempfile::tempdir().unwrap();
    for existing in [false, true] {
        let path = if existing {
            directory.path().join("existing.db")
        } else {
            directory.path().join("not-created").join("new.db")
        };
        if existing {
            drop(legacy(&path));
        }
        let alias = if existing {
            path.canonicalize().unwrap()
        } else {
            directory
                .path()
                .canonicalize()
                .unwrap()
                .join("not-created")
                .join("new.db")
        };
        assert_ne!(
            path, alias,
            "fixture must exercise ordinary/verbatim Windows paths"
        );
        let before = directory_bytes(directory.path());
        let owner = schema6::admission_lock(&path).unwrap();
        assert!(
            schema6::admission_lock(&alias).is_err(),
            "path alias bypassed migration ownership: {existing}"
        );
        drop(owner);
        let reopened = schema6::admission_lock(&alias).unwrap();
        assert!(schema6::admission_lock(&path).is_err());
        drop(reopened);
        assert_eq!(before, directory_bytes(directory.path()));
        assert!(!directory.path().join("not-created").exists());
    }
}

fn legacy(path: &Path) -> Connection {
    let connection = Connection::open(path).unwrap();
    connection
        .execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;",
        )
        .unwrap();
    for sql in [
        include_str!("../../migrations/001_foundation.sql"),
        include_str!("../../migrations/002_discovery.sql"),
        include_str!("../../migrations/003_profiles.sql"),
        include_str!("../../migrations/004_firmware_library.sql"),
        include_str!("../../migrations/005_desktop_workspace.sql"),
    ] {
        connection.execute_batch(sql).unwrap();
    }
    connection.pragma_update(None, "user_version", 5).unwrap();
    connection.execute_batch("UPDATE settings SET value='{\"theme\":\"dark\"}'; INSERT INTO devices VALUES('synthetic-device','Synthetic router',1); INSERT INTO jobs VALUES('legacy-job','import','completed');").unwrap();
    let hash = "a".repeat(64);
    connection
        .execute("INSERT INTO firmware_artifacts VALUES(?1,4)", [&hash])
        .unwrap();
    connection.execute("INSERT INTO artifact_details(sha256,storage_key,integrity_status,first_imported_at,last_verified_at) VALUES(?1,?2,'verified',123,456)",params![hash,format!("sha256/aa/{hash}.bin")]).unwrap();
    connection.execute("INSERT INTO artifact_imports VALUES('legacy-import',?1,'synthetic.bin',123,'user_import','unverified',NULL,'unverified-historical-label','legacy-job')",[&hash]).unwrap();
    connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .unwrap();
    connection
}
fn directory_bytes(path: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    let mut values = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(path).unwrap() {
        let p = entry.unwrap().path();
        if p.is_file() {
            values.insert(p.file_name().unwrap().into(), std::fs::read(p).unwrap());
        }
    }
    values
}

#[test]
fn stage3_busy_checkpoint_cannot_be_bypassed_by_reopening_a_header_six_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.db");
    drop(legacy(&path));
    let mut reader = None;
    let result = schema6::with_hook(
        &path,
        |_| true,
        |stage, _, writer| {
            if stage == schema6::Stage::BeforeCheckpoint {
                let peer = Connection::open(&path)?;
                peer.execute_batch("BEGIN; SELECT COUNT(*) FROM firmware_artifacts;")?;
                writer.busy_timeout(std::time::Duration::ZERO)?;
                let (busy, _, _): (i32, i32, i32) =
                    writer.query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |r| {
                        Ok((r.get(0)?, r.get(1)?, r.get(2)?))
                    })?;
                assert_eq!(busy, 0);
                reader = Some(peer);
            }
            Ok(())
        },
    );
    assert!(matches!(result, Err(StorageError::UpgradeIncomplete)));
    assert_eq!(header_schema_version(&path), Some(6));
    assert!(
        Repository::open(&path).is_err(),
        "a header alone cannot bypass a pending checkpoint admission gate"
    );
    drop(reader);
    let reopened = schema6::open_with_upgrade(&path, |_| true)
        .unwrap()
        .unwrap();
    assert_eq!(
        reopened
            .connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        6
    );
}

#[test]
fn stage3_declining_upgrade_opens_no_sqlite_and_preserves_every_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.db");
    let connection = legacy(&path);
    connection.execute_batch("INSERT INTO audit_events(kind,message) VALUES('synthetic-wal','Committed WAL sentinel');").unwrap();
    std::fs::write(
        dir.path().join("unrelated.bin"),
        b"harmless synthetic bytes",
    )
    .unwrap();
    let before = directory_bytes(dir.path());
    let mut dialogs = 0;
    let result = schema6::open_with_upgrade(&path, |plan| {
        dialogs += 1;
        assert!(!plan.backup_path.exists());
        assert!(!plan.backup_path.parent().unwrap().exists());
        false
    })
    .unwrap();
    assert!(result.is_none());
    assert_eq!(dialogs, 1);
    assert_eq!(before, directory_bytes(dir.path()));
    assert!(!dir.path().join("backups").exists());
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        5
    );
}

#[test]
fn stage3_approved_wal_upgrade_preserves_rows_and_has_verified_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.db");
    let connection = legacy(&path);
    connection.execute_batch("INSERT INTO audit_events(kind,message) VALUES('synthetic-wal','Committed WAL sentinel');").unwrap();
    let repository = schema6::open_with_upgrade(&path, |_| true)
        .unwrap()
        .unwrap();
    assert_eq!(header_schema_version(&path), Some(6));
    assert_eq!(
        repository.settings().unwrap().theme,
        crate::domain::Theme::Dark
    );
    let original: (String, u32) = repository
        .connection
        .query_row(
            "SELECT display_name,connected FROM devices WHERE id='synthetic-device'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(original, ("Synthetic router".into(), 1));
    let backup: String = repository
        .connection
        .query_row(
            "SELECT backup_filename FROM artifact_schema_receipt WHERE schema_version=6",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let copied = Connection::open_with_flags(
        dir.path().join("backups").join(backup),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        copied
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        5
    );
    assert_eq!(
        copied
            .query_row(
                "SELECT COUNT(*) FROM audit_events WHERE kind='synthetic-wal'",
                [],
                |r| r.get::<_, u32>(0)
            )
            .unwrap(),
        1
    );
    assert_eq!(
        repository
            .connection
            .query_row("SELECT artifact_class FROM firmware_admissions", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
        "legacy_unclassified"
    );
    assert_eq!(
        repository
            .connection
            .query_row(
                "SELECT associated_device_id FROM artifact_imports",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "unverified-historical-label"
    );
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        6
    );
}

#[test]
fn stage3_fresh_database_has_classified_schema() {
    let dir = tempfile::tempdir().unwrap();
    let repository = Repository::open(&dir.path().join("fixture.db")).unwrap();
    let version: u32 = repository
        .connection
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(version, 6);
    for name in [
        "artifact_blobs",
        "artifact_occurrences",
        "artifact_blob_state",
        "firmware_admissions",
    ] {
        let exists: bool = repository
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type='table' AND name=?1)",
                [name],
                |r| r.get(0),
            )
            .unwrap();
        assert!(exists, "{name}");
    }
}

#[test]
fn stage3_raw_membership_insert_requires_separate_firmware_admission() {
    let dir = tempfile::tempdir().unwrap();
    let repository = Repository::open(&dir.path().join("fixture.db")).unwrap();
    let result = repository.connection.execute(
        "INSERT INTO firmware_artifacts(sha256,size_bytes) VALUES(?1,1)",
        ["a".repeat(64)],
    );
    assert!(
        result.is_err(),
        "unclassified bytes must not enter firmware membership"
    );
}

fn still_original(connection: &Connection) {
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        5
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_schema WHERE name='artifact_blobs'",
                [],
                |r| r.get::<_, u32>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT value FROM settings WHERE key='preferences'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "{\"theme\":\"dark\"}"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT size_bytes FROM firmware_artifacts WHERE sha256=?1",
                ["a".repeat(64)],
                |r| r.get::<_, u32>(0)
            )
            .unwrap(),
        4
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT original_filename FROM artifact_imports WHERE import_id='legacy-import'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "synthetic.bin"
    );
}
#[test]
fn stage3_every_precommit_failure_rolls_back_schema_and_original_values() {
    use schema6::Stage::*;
    for fault in [
        BackupAllocation,
        BackupCopied,
        BackupVerified,
        BackupFlushed,
        BackupPublished,
        BeforeTransaction,
        AfterLegacy,
        AfterDdl,
        AfterBackfill,
        AfterGuards,
        BeforeVersion,
        AfterVersion,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.db");
        let original = legacy(&path);
        let result = schema6::with_hook(
            &path,
            |_| true,
            |stage, _, _| {
                if stage == fault {
                    Err(StorageError::UpgradeRefused("Injected synthetic failure."))
                } else {
                    Ok(())
                }
            },
        );
        assert!(result.is_err(), "fault {fault:?}");
        still_original(&original);
    }
}
#[test]
fn stage3_invalid_legacy_hashes_are_refused_without_correction_or_backup() {
    for bad in [
        "F".repeat(64),
        "abc".into(),
        "a".repeat(63),
        "0".repeat(65),
        "\u{ff41}".repeat(64),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.db");
        let original = legacy(&path);
        original
            .execute_batch("PRAGMA ignore_check_constraints=ON;")
            .unwrap();
        original
            .execute("INSERT INTO firmware_artifacts VALUES(?1,8)", [&bad])
            .unwrap();
        original
            .execute_batch("PRAGMA ignore_check_constraints=OFF;")
            .unwrap();
        assert!(matches!(
            schema6::open_with_upgrade(&path, |_| true),
            Err(StorageError::UpgradeRefused(_))
        ));
        still_original(&original);
        assert_eq!(
            original
                .query_row(
                    "SELECT sha256 FROM firmware_artifacts WHERE size_bytes=8",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
            bad
        );
        assert!(!dir.path().join("backups").exists());
    }
}
#[test]
fn stage3_backup_corruption_and_destination_collision_never_migrate_or_replace() {
    for corrupt in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.db");
        let original = legacy(&path);
        let destination = std::cell::RefCell::new(None::<std::path::PathBuf>);
        let result = schema6::with_hook(
            &path,
            |plan| {
                *destination.borrow_mut() = Some(plan.backup_path.clone());
                true
            },
            |stage, candidate, _| {
                if corrupt && stage == schema6::Stage::BackupCopied {
                    std::fs::write(candidate, b"synthetic corrupt backup")?;
                }
                if !corrupt && stage == schema6::Stage::BackupFlushed {
                    std::fs::write(
                        destination.borrow().as_ref().unwrap(),
                        b"synthetic destination sentinel",
                    )?;
                }
                Ok(())
            },
        );
        assert!(result.is_err());
        still_original(&original);
        if !corrupt {
            assert_eq!(
                std::fs::read(destination.borrow().as_ref().unwrap()).unwrap(),
                b"synthetic destination sentinel"
            );
        }
    }
}
#[test]
fn stage3_external_write_after_backup_aborts_and_keeps_that_write() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.db");
    let original = legacy(&path);
    let result = schema6::with_hook(
        &path,
        |_| true,
        |stage, _, _| {
            if stage == schema6::Stage::BeforeTransaction {
                Connection::open(&path)?.execute("INSERT INTO audit_events(kind,message) VALUES('external_fixture','Keep this separately committed value')",[])?;
            }
            Ok(())
        },
    );
    assert!(matches!(result, Err(StorageError::UpgradeRefused(_))));
    still_original(&original);
    assert_eq!(
        original
            .query_row(
                "SELECT COUNT(*) FROM audit_events WHERE kind='external_fixture'",
                [],
                |r| r.get::<_, u32>(0)
            )
            .unwrap(),
        1
    );
}
#[test]
fn stage3_postcommit_failures_hold_admission_and_decline_preserves_pending_state() {
    for fault in [
        schema6::Stage::AfterCommit,
        schema6::Stage::BeforeCheckpoint,
        schema6::Stage::AfterCheckpoint,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.db");
        drop(legacy(&path));
        let result = schema6::with_hook(
            &path,
            |_| true,
            |stage, _, _| {
                if stage == fault {
                    Err(StorageError::UpgradeIncomplete)
                } else {
                    Ok(())
                }
            },
        );
        assert!(matches!(result, Err(StorageError::UpgradeIncomplete)));
        assert!(Repository::open(&path).is_err());
        let before = directory_bytes(dir.path());
        assert!(schema6::open_with_upgrade(&path, |_| false)
            .unwrap()
            .is_none());
        assert_eq!(before, directory_bytes(dir.path()));
        let recovered = schema6::open_with_upgrade(&path, |_| true)
            .unwrap()
            .unwrap();
        recovered.ensure_ready().unwrap();
        assert_eq!(header_schema_version(&path), Some(6));
    }
}
#[test]
fn stage3_nonfirmware_cannot_be_promoted_by_hash_labels_or_raw_admission() {
    let dir = tempfile::tempdir().unwrap();
    let repository = Repository::open(&dir.path().join("fixture.db")).unwrap();
    let c = &repository.connection;
    let hash = "b".repeat(64);
    c.execute("INSERT INTO artifact_blobs VALUES(?1,4)", [&hash])
        .unwrap();
    for (id, class) in [
        ("config", "configuration_backup"),
        ("report", "metadata_report"),
    ] {
        c.execute("INSERT INTO artifact_occurrences(occurrence_id,blob_sha256,artifact_class,provenance_kind,source_reference,acquisition_occurrence_id,collected_at) VALUES(?1,?2,?3,'synthetic','official_firmware.bin',?1,1)",params![id,hash,class]).unwrap();
        for declared in [class, "official_image_download", "user_imported_unverified"] {
            assert!(c
                .execute(
                    "INSERT INTO firmware_admissions VALUES(?1,?2,?3)",
                    params![hash, id, declared]
                )
                .is_err());
        }
        assert!(c.execute("UPDATE artifact_occurrences SET artifact_class='official_image_download' WHERE occurrence_id=?1",[id]).is_err());
        assert!(c
            .execute(
                "DELETE FROM artifact_occurrences WHERE occurrence_id=?1",
                [id]
            )
            .is_err());
    }
    assert_eq!(
        c.query_row("SELECT COUNT(*) FROM firmware_artifacts", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        0
    );
    assert!(c.execute("INSERT INTO artifact_occurrences(occurrence_id,blob_sha256,artifact_class,provenance_kind,acquisition_occurrence_id,collected_at) VALUES('legacy-forged',?1,'legacy_unclassified','synthetic','legacy-forged',1)",[&hash]).is_err());
    assert!(c.execute("INSERT INTO artifact_occurrences(occurrence_id,blob_sha256,artifact_class,provenance_kind,acquisition_occurrence_id,collected_at) VALUES('wrong-user',?1,'user_imported_unverified','official','wrong-user',1)",[&hash]).is_err());
}
#[test]
fn stage3_coverage_foreign_keys_and_installability_are_constrained() {
    let dir = tempfile::tempdir().unwrap();
    let repository = Repository::open(&dir.path().join("fixture.db")).unwrap();
    let c = &repository.connection;
    let hash = "b".repeat(64);
    c.execute("INSERT INTO artifact_blobs VALUES(?1,4)", [&hash])
        .unwrap();
    for (id, device, job, complete, observed, manifest) in [
        ("no-device", Some("absent"), None, 0, 2, None),
        ("no-job", None, Some("absent"), 0, 2, None),
        ("partial-complete", None, None, 1, 2, Some(hash.as_str())),
        ("missing-manifest", None, None, 1, 4, None),
    ] {
        assert!(c.execute("INSERT INTO artifact_occurrences(occurrence_id,blob_sha256,artifact_class,provenance_kind,acquisition_occurrence_id,device_id,job_id,collected_at,coverage_total_bytes,coverage_observed_bytes,coverage_complete,coverage_manifest_sha256) VALUES(?1,?2,'physical_flash_dump','synthetic',?1,?3,?4,1,4,?5,?6,?7)",params![id,hash,device,job,observed,complete,manifest]).is_err());
    }
    c.execute("INSERT INTO artifact_occurrences(occurrence_id,blob_sha256,artifact_class,provenance_kind,acquisition_occurrence_id,collected_at,coverage_total_bytes,coverage_observed_bytes,coverage_complete,coverage_manifest_sha256) VALUES('qualified-synthetic',?1,'physical_flash_dump','synthetic','qualified-synthetic',1,4,4,1,?1)",[&hash]).unwrap();
    assert!(c.execute("UPDATE artifact_occurrences SET installability='verified' WHERE occurrence_id='qualified-synthetic'",[]).is_err());
}

#[test]
fn stage3_unexpected_original_schema_change_aborts_before_version_bump() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.db");
    let original = legacy(&path);
    let result = schema6::with_hook(
        &path,
        |_| true,
        |stage, _, connection| {
            if stage == schema6::Stage::AfterGuards {
                connection
                    .execute_batch("ALTER TABLE devices ADD COLUMN accidental_change TEXT;")?;
            }
            Ok(())
        },
    );
    assert!(
        result.is_err(),
        "preserved row values alone do not authorize changing an original schema"
    );
    still_original(&original);
    assert_eq!(
        original
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('devices') WHERE name='accidental_change'",
                [],
                |r| r.get::<_, u32>(0)
            )
            .unwrap(),
        0
    );
}

#[test]
fn stage3_missing_class_guard_blocks_startup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.db");
    let repo = Repository::open(&path).unwrap();
    repo.connection
        .execute_batch("DROP TRIGGER firmware_requires_admission;")
        .unwrap();
    drop(repo);
    assert!(
        Repository::open(&path).is_err(),
        "incomplete admission constraints must fail closed"
    );
}
#[test]
fn stage3_repair_uses_original_state_and_never_promotes_or_reads_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.db");
    drop(legacy(&path));
    let repo = schema6::open_with_upgrade(&path, |_| true)
        .unwrap()
        .unwrap();
    repo.connection.execute_batch("UPDATE artifact_details SET storage_status='unreadable',integrity_status='hash_mismatch',last_verified_at=789; UPDATE artifact_blob_state SET storage_status='present',integrity_status='verified',last_verified_at=999;").unwrap();
    drop(repo);
    let repo = Repository::open(&path).unwrap();
    let pair: (String, String, u64) = repo
        .connection
        .query_row(
            "SELECT storage_status,integrity_status,last_verified_at FROM artifact_blob_state",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(pair, ("unknown".into(), "hash_mismatch".into(), 789));
    assert_eq!(
        repo.connection
            .query_row("SELECT storage_status FROM artifact_details", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
        "unreadable"
    );
    repo.connection
        .execute("DELETE FROM artifact_blob_state", [])
        .unwrap();
    drop(repo);
    let repo = Repository::open(&path).unwrap();
    assert_eq!(
        repo.connection
            .query_row("SELECT COUNT(*) FROM artifact_blob_state", [], |r| r
                .get::<_, u32>(0))
            .unwrap(),
        1
    );
}
#[test]
fn stage3_live_migration_excludes_other_open_and_workers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.db");
    drop(legacy(&path));
    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(0);
    let (continue_tx, continue_rx) = std::sync::mpsc::sync_channel(0);
    let worker_path = path.clone();
    let worker = std::thread::spawn(move || {
        schema6::with_hook(
            &worker_path,
            |_| true,
            |stage, _, _| {
                if stage == schema6::Stage::BeforeTransaction {
                    ready_tx.send(()).unwrap();
                    continue_rx
                        .recv_timeout(std::time::Duration::from_secs(10))
                        .unwrap();
                }
                Ok(())
            },
        )
        .map(|r| r.is_some())
    });
    ready_rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    assert!(Repository::open(&path).is_err());
    // Even an internal pre-admission handle cannot start Library workers.
    let raw = Repository {
        connection: Connection::open(&path).unwrap(),
    };
    assert!(raw.ensure_ready().is_err());
    continue_tx.send(()).unwrap();
    assert!(worker.join().unwrap().unwrap());
    Repository::open(&path).unwrap().ensure_ready().unwrap();
}
#[test]
fn stage3_lost_or_corrupt_backup_cannot_complete_pending_upgrade() {
    for corrupt in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.db");
        drop(legacy(&path));
        let failed = schema6::with_hook(
            &path,
            |_| true,
            |stage, _, _| {
                if stage == schema6::Stage::AfterCommit {
                    Err(StorageError::UpgradeIncomplete)
                } else {
                    Ok(())
                }
            },
        );
        assert!(failed.is_err());
        let backup = std::fs::read_dir(dir.path().join("backups"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| p.extension().is_some_and(|v| v == "sqlite"))
            .unwrap();
        if corrupt {
            std::fs::write(&backup, b"synthetic corruption").unwrap();
        } else {
            std::fs::remove_file(&backup).unwrap();
        }
        assert!(schema6::open_with_upgrade(&path, |_| true).is_err());
        assert!(Repository::open(&path).is_err());
    }
}
