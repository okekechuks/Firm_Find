//! Durable admission barrier, distinct from the main-file header and immutable receipt.
use super::*;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct BackupRef {
    pub filename: String,
    pub hash: String,
    pub size: u64,
    pub source_version: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Record {
    version: u32,
    operation: String,
    database: String,
    pub backup: Option<BackupRef>,
}
fn name(database: &Path) -> Result<String, StorageError> {
    Ok(database
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or(StorageError::UpgradeIncomplete)?
        .into())
}
pub(super) fn path(database: &Path) -> Result<PathBuf, StorageError> {
    Ok(database.with_file_name(format!("{}.schema6-pending.json", name(database)?)))
}
pub(crate) fn exists(database: &Path) -> Result<bool, StorageError> {
    match fs::symlink_metadata(path(database)?) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}
pub(crate) fn lock(database: &Path) -> Result<crate::startup::InstanceLock, StorageError> {
    crate::library::files::safe_path(database)?;
    // Ordinary and verbatim Windows spellings of the same location must acquire
    // the same mutex. Resolve only existing ancestors; never create data to lock it.
    let mut ancestor = database;
    let mut suffix = Vec::new();
    let mut identity = loop {
        match fs::canonicalize(ancestor) {
            Ok(path) => break path,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                suffix.push(
                    ancestor
                        .file_name()
                        .ok_or(StorageError::UpgradeIncomplete)?
                        .to_owned(),
                );
                ancestor = ancestor.parent().ok_or(StorageError::UpgradeIncomplete)?;
            }
            Err(e) => return Err(e.into()),
        }
    };
    for component in suffix.into_iter().rev() {
        identity.push(component);
    }
    Ok(crate::startup::InstanceLock::acquire(&format!(
        "schema6:{}",
        identity.to_string_lossy().to_lowercase()
    ))?)
}
pub(super) fn read(database: &Path) -> Result<Record, StorageError> {
    let file = crate::library::files::read_file(&path(database)?)?;
    let mut data = vec![];
    file.take(4097).read_to_end(&mut data)?;
    if data.len() > 4096 {
        return Err(StorageError::UpgradeIncomplete);
    }
    let record: Record =
        serde_json::from_slice(&data).map_err(|_| StorageError::UpgradeIncomplete)?;
    if record.version != 6
        || !crate::library::valid_id(&record.operation)
        || record.database.to_lowercase() != name(database)?.to_lowercase()
    {
        return Err(StorageError::UpgradeIncomplete);
    }
    if let Some(backup) = &record.backup {
        let nonce = backup
            .filename
            .strip_prefix("schema-before-6-")
            .and_then(|s| s.strip_suffix(".sqlite"))
            .ok_or(StorageError::UpgradeIncomplete)?;
        if !crate::library::valid_id(nonce)
            || !crate::library::valid_hash(&backup.hash)
            || backup.size == 0
            || backup.source_version > 5
        {
            return Err(StorageError::UpgradeIncomplete);
        }
    }
    Ok(record)
}
pub(crate) struct Lease {
    database: PathBuf,
    record: Record,
    committed: bool,
    cleared: bool,
}
impl Lease {
    pub(super) fn begin(database: &Path, backup: Option<BackupRef>) -> Result<Self, StorageError> {
        if exists(database)? {
            read(database)?;
        }
        let record = Record {
            version: 6,
            operation: crate::library::files::nonce()?,
            database: name(database)?,
            backup,
        };
        let destination = path(database)?;
        let temporary = destination.with_extension(format!("json.{}", record.operation));
        let mut file = crate::library::files::new_file(&temporary)?;
        let data = serde_json::to_vec(&record).map_err(|_| StorageError::UpgradeIncomplete)?;
        file.write_all(&data)?;
        file.sync_all()?;
        drop(file);
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Storage::FileSystem::*;
            let source: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
            let target: Vec<u16> = destination
                .as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect();
            if unsafe {
                MoveFileExW(
                    source.as_ptr(),
                    target.as_ptr(),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            } == 0
            {
                return Err(std::io::Error::last_os_error().into());
            }
        }
        #[cfg(not(windows))]
        fs::rename(&temporary, &destination)?;
        Ok(Self {
            database: database.into(),
            record,
            committed: false,
            cleared: false,
        })
    }
    pub(super) fn resume(database: &Path, record: Record) -> Self {
        Self {
            database: database.into(),
            record,
            committed: true,
            cleared: false,
        }
    }
    pub(crate) fn committed(&mut self) {
        self.committed = true;
    }
    pub(crate) fn complete(&mut self) -> Result<(), StorageError> {
        if read(&self.database)?.operation != self.record.operation {
            return Err(StorageError::UpgradeIncomplete);
        }
        fs::remove_file(path(&self.database)?)?;
        self.cleared = true;
        Ok(())
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        if !self.committed
            && !self.cleared
            && read(&self.database).is_ok_and(|r| r.operation == self.record.operation)
        {
            if let Ok(p) = path(&self.database) {
                let _ = fs::remove_file(p);
            }
        }
    }
}
