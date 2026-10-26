//! Abrupt process exits: SQLite and migration guards do not get Rust Drop cleanup.
//! All input paths belong to newly created OS-temp fixture directories.
use super::*;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

const OWNER: &[u8] = b"OBTAINER isolated schema-6 crash fixture v1";
const FIXTURE_ENV: &str = "OBTAINER_STAGE3_CRASH_FIXTURE";
const STAGE_ENV: &str = "OBTAINER_STAGE3_CRASH_STAGE";

fn tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut pending = vec![root.to_path_buf()];
    let mut entries = BTreeMap::new();
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let metadata = std::fs::symlink_metadata(&path).unwrap();
            assert!(!metadata.file_type().is_symlink());
            let key = path.strip_prefix(root).unwrap().to_path_buf();
            if metadata.is_dir() {
                entries.insert(key, None);
                pending.push(path);
            } else {
                entries.insert(key, Some(std::fs::read(path).unwrap()));
            }
        }
    }
    entries
}

#[test]
#[ignore = "subprocess entry; executed by stage3_abrupt_crashes_require_consent_and_recover_without_data_loss"]
fn stage3_crash_fixture_process() {
    let root = PathBuf::from(std::env::var_os(FIXTURE_ENV).expect("owned fixture required"))
        .canonicalize()
        .unwrap();
    let temporary = std::env::temp_dir().canonicalize().unwrap();
    assert!(root.starts_with(&temporary) && root != temporary);
    assert_eq!(
        std::fs::read(root.join(".stage3-crash-fixture")).unwrap(),
        OWNER
    );
    let stage = match std::env::var(STAGE_ENV).unwrap().as_str() {
        "before_commit" => schema6::Stage::AfterDdl,
        "after_commit" => schema6::Stage::AfterCommit,
        "after_checkpoint" => schema6::Stage::AfterCheckpoint,
        _ => panic!("unknown crash boundary"),
    };
    let _ = schema6::with_hook(
        &root.join("fixture.db"),
        |_| true,
        |current, _, _| {
            if current == stage {
                std::process::exit(86);
            }
            Ok(())
        },
    );
    panic!("requested crash boundary was not reached");
}

#[test]
fn stage3_abrupt_crashes_require_consent_and_recover_without_data_loss() {
    for stage in ["before_commit", "after_commit", "after_checkpoint"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("fixture.db");
        std::fs::write(directory.path().join(".stage3-crash-fixture"), OWNER).unwrap();
        drop(legacy(&path));
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "storage::schema6_tests::crash::stage3_crash_fixture_process",
                "--ignored",
                "--nocapture",
            ])
            .env(FIXTURE_ENV, directory.path())
            .env(STAGE_ENV, stage)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let mut child = command.spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                result => {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("bounded crash-fixture wait failed: {stage}: {result:?}");
                }
            }
        }
        let output = child.wait_with_output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(86),
            "{stage}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(schema6::has_pending(&path).unwrap());
        assert!(matches!(
            Repository::open(&path),
            Err(StorageError::UpgradeIncomplete)
        ));
        if stage == "before_commit" {
            assert_eq!(header_schema_version(&path), Some(5));
        }
        if stage == "after_checkpoint" {
            assert_eq!(header_schema_version(&path), Some(6));
        }
        let before_no = tree(directory.path());
        assert!(schema6::open_with_upgrade(&path, |_| false)
            .unwrap()
            .is_none());
        assert_eq!(
            before_no,
            tree(directory.path()),
            "No changed fixture files/directories: {stage}"
        );
        let recovered = schema6::open_with_upgrade(&path, |_| true)
            .unwrap()
            .unwrap();
        recovered.ensure_ready().unwrap();
        assert_eq!(header_schema_version(&path), Some(6));
        assert!(!schema6::has_pending(&path).unwrap());
        assert_eq!(
            recovered.settings().unwrap().theme,
            crate::domain::Theme::Dark
        );
        let device: (String, u32) = recovered
            .connection
            .query_row(
                "SELECT display_name,connected FROM devices WHERE id='synthetic-device'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(device, ("Synthetic router".into(), 1));
        let retained: (String,u32) = recovered.connection.query_row(
            "SELECT artifact_class,(SELECT COUNT(*) FROM firmware_artifacts) FROM firmware_admissions", [],
            |row| Ok((row.get(0)?,row.get(1)?))).unwrap();
        assert_eq!(retained, ("legacy_unclassified".into(), 1));
        drop(recovered);
        Repository::open(&path).unwrap().ensure_ready().unwrap();
    }
}
