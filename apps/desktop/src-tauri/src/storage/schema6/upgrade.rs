use super::*;
use crate::library::files;
use std::{
    fs::{self, File, OpenOptions},
    path::PathBuf,
    sync::atomic::AtomicBool,
};
#[path = "snapshot.rs"]
mod snapshot;
use snapshot::Snapshot;

pub(crate) struct UpgradePlan {
    database: PathBuf,
    pub backup_path: PathBuf,
    pub retained_backup: Option<PathBuf>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stage {
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
    AfterCommit,
    BeforeCheckpoint,
    AfterCheckpoint,
}

pub(crate) fn open_with_upgrade(
    path: &Path,
    approve: impl FnOnce(&UpgradePlan) -> bool,
) -> Result<Option<Repository>, StorageError> {
    open_impl(path, approve, &mut |_, _, _| Ok(()))
}
#[cfg(test)]
pub(crate) fn with_hook(
    path: &Path,
    approve: impl FnOnce(&UpgradePlan) -> bool,
    mut hook: impl FnMut(Stage, &Path, &Connection) -> Result<(), StorageError>,
) -> Result<Option<Repository>, StorageError> {
    open_impl(path, approve, &mut hook)
}

fn regular(path: &Path) -> Result<(), StorageError> {
    let m = fs::symlink_metadata(path)?;
    #[cfg(windows)]
    let reparse = {
        use std::os::windows::fs::MetadataExt;
        m.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let reparse = m.file_type().is_symlink();
    if !m.is_file() || reparse {
        return Err(StorageError::UpgradeRefused(
            "Database and backup paths must be regular local files.",
        ));
    }
    Ok(())
}
fn open_impl(
    path: &Path,
    approve: impl FnOnce(&UpgradePlan) -> bool,
    hook: &mut impl FnMut(Stage, &Path, &Connection) -> Result<(), StorageError>,
) -> Result<Option<Repository>, StorageError> {
    files::safe_path(path)?;
    let _admission = pending::lock(path)?;
    if path.exists() {
        regular(path)?;
    }
    let header = header_schema_version(path);
    if header.is_some_and(|v| v > SUPPORTED_SCHEMA_VERSION) {
        return Err(StorageError::NewerSchema);
    }
    let interrupted = if pending::exists(path)? {
        Some(pending::read(path)?)
    } else {
        None
    };
    if interrupted.is_some() && !path.exists() {
        return Err(StorageError::UpgradeIncomplete);
    }
    if interrupted.is_none() && (header.is_none() || header == Some(SUPPORTED_SCHEMA_VERSION)) {
        if !path.exists() {
            for suffix in ["-wal", "-shm", "-journal"] {
                if PathBuf::from(format!("{}{suffix}", path.display())).exists() {
                    return Err(StorageError::UpgradeIncomplete);
                }
            }
            let parent = path
                .parent()
                .ok_or(StorageError::UpgradeRefused("Database has no parent."))?;
            fs::create_dir_all(parent)?;
        }
        return Repository::open_locked(path).map(Some);
    }
    let parent = path
        .parent()
        .ok_or(StorageError::UpgradeRefused("Database has no parent."))?;
    let _root_guard = files::guard_directories(parent)?;
    let plan = UpgradePlan {
        database: path.into(),
        retained_backup: interrupted
            .as_ref()
            .and_then(|r| r.backup.as_ref())
            .map(|b| parent.join("backups").join(&b.filename)),
        backup_path: parent
            .join("backups")
            .join(format!("schema-before-6-{}.sqlite", files::nonce()?)),
    };
    if !approve(&plan) {
        return Ok(None);
    } // No SQLite connection, directories or logs before this decision.
    regular(path)?;
    let mut connection = connect_supported(&plan.database, SUPPORTED_SCHEMA_VERSION)?;
    let version: u32 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version == SUPPORTED_SCHEMA_VERSION {
        return resume(
            connection,
            path,
            interrupted.ok_or(StorageError::UpgradeIncomplete)?,
            hook,
        )
        .map(Some)
        .map_err(|_| StorageError::UpgradeIncomplete);
    }
    let before = {
        let read = connection.transaction()?;
        valid_database(&read)?;
        valid_legacy_hashes(&read)?;
        let snapshot = Snapshot::capture(&read)?;
        read.commit()?;
        snapshot
    };
    let backup = create_backup(&connection, &plan, &before, hook)?;
    let mut marker = pending::Lease::begin(
        path,
        Some(pending::BackupRef {
            filename: backup.filename.clone(),
            hash: backup.hash.clone(),
            size: backup.size,
            source_version: version,
        }),
    )?;
    hook(Stage::BeforeTransaction, path, &connection)?;
    let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    // Detect an external writer between the verified backup and write admission.
    if Snapshot::capture(&tx)? != before {
        return Err(StorageError::UpgradeRefused(
            "Source changed after backup. Retry to create a fresh backup.",
        ));
    }
    for sql in LEGACY_SQL.iter().skip(version as usize) {
        tx.execute_batch(sql)?;
    }
    hook(Stage::AfterLegacy, path, &tx)?;
    let (ddl, rest) =
        CLASS_SQL
            .split_once("-- CLASS BACKFILL")
            .ok_or(StorageError::UpgradeRefused(
                "Migration asset is incomplete.",
            ))?;
    let (backfill, guards) =
        rest.split_once("-- CLASS GUARDS")
            .ok_or(StorageError::UpgradeRefused(
                "Migration asset is incomplete.",
            ))?;
    tx.execute_batch(ddl)?;
    hook(Stage::AfterDdl, path, &tx)?;
    tx.execute_batch(backfill)?;
    hook(Stage::AfterBackfill, path, &tx)?;
    tx.execute_batch(guards)?;
    hook(Stage::AfterGuards, path, &tx)?;
    tx.execute(
        "INSERT INTO artifact_schema_receipt VALUES(6,?1,?2,?3,?4,?5)",
        params![
            version,
            backup.filename,
            backup.hash,
            backup.size,
            crate::connection::now()
        ],
    )?;
    before.original_values_unchanged(&tx)?;
    valid_database(&tx)?;
    hook(Stage::BeforeVersion, path, &tx)?;
    tx.pragma_update(None, "user_version", SUPPORTED_SCHEMA_VERSION)?;
    hook(Stage::AfterVersion, path, &tx)?;
    tx.commit()?;
    marker.committed();
    if hook(Stage::AfterCommit, path, &connection).is_err() {
        return Err(StorageError::UpgradeIncomplete);
    }
    finish(&mut connection, path, &mut marker, hook)?;
    drop(backup);
    Ok(Some(Repository { connection }))
}
fn valid_database(connection: &Connection) -> Result<(), StorageError> {
    let checks: Vec<String> = connection
        .prepare("PRAGMA integrity_check")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    if checks != ["ok"] {
        return Err(StorageError::UpgradeRefused(
            "Database integrity validation failed.",
        ));
    }
    let invalid: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check)",
        [],
        |r| r.get(0),
    )?;
    if invalid {
        return Err(StorageError::UpgradeRefused(
            "Foreign-key validation failed.",
        ));
    }
    Ok(())
}
fn valid_legacy_hashes(connection: &Connection) -> Result<(), StorageError> {
    let exists:bool=connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type='table' AND name='firmware_artifacts')",[],|r|r.get(0))?;
    if !exists {
        return Ok(());
    }
    let mut statement = connection.prepare("SELECT sha256 FROM firmware_artifacts")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let value = row.get_ref(0)?;
        if !matches!(value,rusqlite::types::ValueRef::Text(v) if v.len()==64 && v.iter().all(|b|b.is_ascii_digit()||matches!(*b,b'a'..=b'f')))
        {
            return Err(StorageError::UpgradeRefused(
                "A stored artifact hash is not lowercase SHA-256. No value was corrected.",
            ));
        }
    }
    Ok(())
}
struct VerifiedBackup {
    filename: String,
    hash: String,
    size: u64,
    _lease: File,
    _directories: Vec<File>,
}
fn immutable(path: &Path) -> Result<Connection, StorageError> {
    let mut uri = url::Url::from_file_path(path)
        .map_err(|_| StorageError::UpgradeRefused("Backup path is not representable."))?;
    uri.query_pairs_mut().append_pair("immutable", "1");
    Ok(Connection::open_with_flags(
        uri.as_str(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
            | rusqlite::OpenFlags::SQLITE_OPEN_URI
            | rusqlite::OpenFlags::SQLITE_OPEN_PRIVATE_CACHE,
    )?)
}
fn verify_copy(path: &Path, before: &Snapshot) -> Result<(), StorageError> {
    regular(path)?;
    let copy = immutable(path)?;
    valid_database(&copy)?;
    if Snapshot::capture(&copy)? != *before {
        return Err(StorageError::UpgradeRefused(
            "Backup does not match the source logical snapshot.",
        ));
    }
    Ok(())
}
fn create_backup(
    connection: &Connection,
    plan: &UpgradePlan,
    before: &Snapshot,
    hook: &mut impl FnMut(Stage, &Path, &Connection) -> Result<(), StorageError>,
) -> Result<VerifiedBackup, StorageError> {
    let parent = plan
        .backup_path
        .parent()
        .ok_or(StorageError::UpgradeRefused("Backup has no parent."))?;
    let source_size = fs::metadata(&plan.database)?.len();
    let wal = fs::metadata(format!("{}-wal", plan.database.display()))
        .map(|m| m.len())
        .unwrap_or(0);
    let needed = source_size
        .checked_add(wal)
        .and_then(|v| v.checked_mul(3))
        .and_then(|v| v.checked_add(1024 * 1024))
        .ok_or(StorageError::UpgradeRefused("Backup size overflow."))?;
    files::free_space(
        plan.database
            .parent()
            .ok_or(StorageError::UpgradeRefused("Database has no parent."))?,
        needed,
    )?;
    let directories = files::directory(parent)?;
    let candidate = plan.backup_path.with_extension("sqlite.partial");
    hook(Stage::BackupAllocation, &candidate, connection)?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(3).custom_flags(0x00200000);
    }
    let file = options.open(&candidate)?;
    connection.execute(
        "VACUUM main INTO ?1",
        [candidate
            .to_str()
            .ok_or(StorageError::UpgradeRefused("Backup path is not Unicode."))?],
    )?;
    hook(Stage::BackupCopied, &candidate, connection)?;
    verify_copy(&candidate, before)?;
    hook(Stage::BackupVerified, &candidate, connection)?;
    file.sync_all()?;
    hook(Stage::BackupFlushed, &candidate, connection)?;
    drop(file);
    let (lease, hash, size) =
        files::hash_file(&candidate, u64::MAX, &AtomicBool::new(false), |_| Ok(()))?;
    verify_copy(&candidate, before)?;
    drop(lease);
    rename_new(&candidate, &plan.backup_path)?;
    hook(Stage::BackupPublished, &plan.backup_path, connection)?;
    let (lease, published_hash, published_size) =
        files::hash_file(&plan.backup_path, u64::MAX, &AtomicBool::new(false), |_| {
            Ok(())
        })?;
    if published_hash != hash || published_size != size {
        return Err(StorageError::UpgradeRefused("Published backup changed."));
    }
    verify_copy(&plan.backup_path, before)?;
    Ok(VerifiedBackup {
        filename: plan
            .backup_path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or(StorageError::UpgradeRefused("Backup filename is invalid."))?
            .into(),
        hash,
        size,
        _lease: lease,
        _directories: directories,
    })
}
fn rename_new(source: &Path, destination: &Path) -> Result<(), StorageError> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
        let destination: Vec<u16> = destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        if unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    #[cfg(not(windows))]
    {
        fs::hard_link(source, destination)?;
        fs::remove_file(source)?;
    }
    Ok(())
}
fn finish(
    connection: &mut Connection,
    path: &Path,
    marker: &mut pending::Lease,
    hook: &mut impl FnMut(Stage, &Path, &Connection) -> Result<(), StorageError>,
) -> Result<(), StorageError> {
    validate_and_repair(connection).map_err(|_| StorageError::UpgradeIncomplete)?;
    if hook(Stage::BeforeCheckpoint, path, connection).is_err() {
        return Err(StorageError::UpgradeIncomplete);
    }
    checkpoint(connection, path).map_err(|_| StorageError::UpgradeIncomplete)?;
    if hook(Stage::AfterCheckpoint, path, connection).is_err() {
        return Err(StorageError::UpgradeIncomplete);
    }
    marker
        .complete()
        .map_err(|_| StorageError::UpgradeIncomplete)?;
    Ok(())
}
fn resume(
    mut connection: Connection,
    path: &Path,
    record: pending::Record,
    hook: &mut impl FnMut(Stage, &Path, &Connection) -> Result<(), StorageError>,
) -> Result<Repository, StorageError> {
    let receipt:(Option<u32>,Option<String>,Option<String>,Option<u64>)=connection.query_row("SELECT source_version,backup_filename,backup_sha256,backup_bytes FROM artifact_schema_receipt WHERE schema_version=6",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
    match &record.backup {
        Some(b)
            if receipt.0 == Some(b.source_version)
                && receipt.1.as_deref() == Some(b.filename.as_str())
                && receipt.2.as_deref() == Some(b.hash.as_str())
                && receipt.3 == Some(b.size) => {}
        None if receipt.0.is_none()
            && receipt.1.is_none()
            && receipt.2.is_none()
            && receipt.3.is_none() => {}
        _ => return Err(StorageError::UpgradeIncomplete),
    }
    let mut marker = pending::Lease::resume(path, record);
    let _backup = if receipt.0.is_some() {
        let name = receipt.1.ok_or(StorageError::UpgradeIncomplete)?;
        if !name.starts_with("schema-before-6-")
            || !name.ends_with(".sqlite")
            || name.contains(['/', '\\', ':'])
        {
            return Err(StorageError::UpgradeIncomplete);
        }
        let parent = path
            .parent()
            .ok_or(StorageError::UpgradeIncomplete)?
            .join("backups");
        let guards = files::guard_directories(&parent)?;
        let backup = parent.join(&name);
        let (lease, hash, size) =
            files::hash_file(&backup, u64::MAX, &AtomicBool::new(false), |_| Ok(()))?;
        if Some(&hash) != receipt.2.as_ref() || Some(size) != receipt.3 {
            return Err(StorageError::UpgradeIncomplete);
        }
        let copy = immutable(&backup)?;
        valid_database(&copy)?;
        let source_version: u32 = copy.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if Some(source_version) != receipt.0 {
            return Err(StorageError::UpgradeIncomplete);
        }
        Some(VerifiedBackup {
            filename: name,
            hash,
            size,
            _lease: lease,
            _directories: guards,
        })
    } else {
        None
    };
    finish(&mut connection, path, &mut marker, hook)?;
    Ok(Repository { connection })
}
