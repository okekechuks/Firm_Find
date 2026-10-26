//! Local, consent-gated schema upgrade. No device, vault-content or network access.
use super::*;
mod pending;
mod upgrade;
pub(super) use pending::{exists as has_pending, lock as admission_lock};
pub(crate) use upgrade::{open_with_upgrade, UpgradePlan};
#[cfg(test)]
pub(super) use upgrade::{with_hook, Stage};

const LEGACY_SQL: [&str; 5] = [
    include_str!("../../migrations/001_foundation.sql"),
    include_str!("../../migrations/002_discovery.sql"),
    include_str!("../../migrations/003_profiles.sql"),
    include_str!("../../migrations/004_firmware_library.sql"),
    include_str!("../../migrations/005_desktop_workspace.sql"),
];
const CLASS_SQL: &str = include_str!("../../migrations/006_artifact_classes.sql");

pub(super) fn initialize(connection: &mut Connection) -> Result<(), StorageError> {
    let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    for sql in LEGACY_SQL {
        tx.execute_batch(sql)?;
    }
    tx.execute_batch(CLASS_SQL)?;
    tx.execute(
        "INSERT INTO artifact_schema_receipt VALUES(6,NULL,NULL,NULL,NULL,?1)",
        [crate::connection::now()],
    )?;
    tx.pragma_update(None, "user_version", SUPPORTED_SCHEMA_VERSION)?;
    tx.commit()?;
    Ok(())
}

pub(super) fn checkpoint(connection: &Connection, path: &Path) -> Result<(), StorageError> {
    let (busy, _, _): (i32, i32, i32) =
        connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?;
    if busy != 0 || header_schema_version(path) != Some(SUPPORTED_SCHEMA_VERSION) {
        return Err(StorageError::UpgradeIncomplete);
    }
    Ok(())
}

pub(super) fn validate_and_repair(connection: &mut Connection) -> Result<(), StorageError> {
    validate_constraints(connection)?;
    let receipts: u32 = connection.query_row(
        "SELECT COUNT(*) FROM artifact_schema_receipt WHERE schema_version=6",
        [],
        |r| r.get(0),
    )?;
    if receipts != 1 {
        return Err(StorageError::UpgradeIncomplete);
    }
    let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let conflict:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM firmware_artifacts a LEFT JOIN artifact_blobs b USING(sha256) LEFT JOIN firmware_admissions f USING(sha256) WHERE b.sha256 IS NULL OR f.sha256 IS NULL OR a.size_bytes!=b.size_bytes) OR EXISTS(SELECT 1 FROM firmware_admissions f LEFT JOIN artifact_occurrences o ON o.occurrence_id=f.occurrence_id WHERE o.occurrence_id IS NULL OR f.sha256!=o.blob_sha256 OR f.artifact_class!=o.artifact_class) OR EXISTS(SELECT 1 FROM artifact_blob_state s JOIN firmware_artifacts a USING(sha256) LEFT JOIN artifact_details d USING(sha256) WHERE d.sha256 IS NULL)",[],|r|r.get(0))?;
    if conflict {
        return Err(StorageError::UpgradeRefused(
            "Artifact identity or derived state conflicts with retained records.",
        ));
    }
    tx.execute("INSERT INTO artifact_blob_state SELECT sha256,storage_key,CASE WHEN storage_status='present' THEN 'present' WHEN storage_status='missing' THEN 'missing' ELSE 'unknown' END,integrity_status,last_verified_at FROM artifact_details WHERE 1 ON CONFLICT(sha256) DO UPDATE SET storage_key=excluded.storage_key,storage_status=excluded.storage_status,integrity_status=excluded.integrity_status,last_verified_at=excluded.last_verified_at WHERE artifact_blob_state.storage_key IS NOT excluded.storage_key OR artifact_blob_state.storage_status IS NOT excluded.storage_status OR artifact_blob_state.integrity_status IS NOT excluded.integrity_status OR artifact_blob_state.last_verified_at IS NOT excluded.last_verified_at",[])?;
    let foreign_keys: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check)",
        [],
        |r| r.get(0),
    )?;
    if foreign_keys {
        return Err(StorageError::UpgradeRefused(
            "Foreign-key validation failed.",
        ));
    }
    tx.commit()?;
    Ok(())
}

pub(super) fn begin_fresh(path: &Path) -> Result<pending::Lease, StorageError> {
    pending::Lease::begin(path, None)
}

// Compare the additive contract with its compiled migration asset, including SQL
// bodies, rather than accepting a matching object name with a missing CHECK/trigger.
fn validate_constraints(connection: &Connection) -> Result<(), StorageError> {
    let reference = Connection::open_in_memory()?;
    for sql in LEGACY_SQL {
        reference.execute_batch(sql)?;
    }
    let old: Vec<String> = reference
        .prepare("SELECT name FROM sqlite_schema")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    reference.execute_batch(CLASS_SQL)?;
    let objects: Vec<(String, String, Option<String>)> = reference
        .prepare("SELECT type,name,sql FROM sqlite_schema")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    for (kind, name, expected) in objects
        .into_iter()
        .filter(|(_, name, _)| !old.contains(name))
    {
        let actual: Option<Option<String>> = connection
            .query_row(
                "SELECT sql FROM sqlite_schema WHERE type=?1 AND name=?2",
                params![kind, name],
                |r| r.get(0),
            )
            .optional()?;
        if actual != Some(expected) {
            return Err(StorageError::UpgradeRefused(
                "Classified schema constraints are incomplete or changed.",
            ));
        }
    }
    Ok(())
}
