//! Local desktop workflows share the existing bounded library worker. No device writes.
use super::*;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::io::{Seek, SeekFrom};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopPreferences {
    pub auto_refresh: bool,
    pub notify_completed: bool,
    pub notify_failed: bool,
    pub notify_devices: bool,
    pub library_directory: Option<String>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreferenceInput {
    pub auto_refresh: bool,
    pub notify_completed: bool,
    pub notify_failed: bool,
    pub notify_devices: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub id: String,
    pub artifact_sha256: String,
    pub filename: String,
    pub size_bytes: u64,
    pub created_at: u64,
    pub verified_at: u64,
    pub source_kind: String,
    pub location: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopState {
    pub library_available: bool,
    pub preferences: DesktopPreferences,
    pub library_directory: String,
    pub backup_directory: String,
    pub restart_required: bool,
    pub library_bytes: u64,
    pub backup_bytes: u64,
    pub extracted: u64,
    pub customized: u64,
    pub max_concurrent_jobs: u8,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoragePlan {
    pub ticket: String,
    pub source: String,
    pub destination: String,
    pub file_count: u64,
    pub bytes: u64,
    pub expires_at: u64,
}
impl Repository {
    pub fn desktop_preferences(&self) -> Result<DesktopPreferences> {
        let json: String = self.connection.query_row(
            "SELECT value FROM desktop_preferences WHERE id=1",
            [],
            |r| r.get(0),
        )?;
        serde_json::from_str(&json).map_err(|_| {
            LibraryError::new(
                "INVALID_PREFERENCES",
                "Stored workspace preferences are invalid; existing storage was retained.",
            )
        })
    }
    pub fn save_desktop_preferences(
        &mut self,
        input: PreferenceInput,
    ) -> Result<DesktopPreferences> {
        let mut p = self.desktop_preferences()?;
        p.auto_refresh = input.auto_refresh;
        p.notify_completed = input.notify_completed;
        p.notify_failed = input.notify_failed;
        p.notify_devices = input.notify_devices;
        let tx = self.connection.transaction()?;
        tx.execute(
            "UPDATE desktop_preferences SET value=?1 WHERE id=1",
            [serde_json::to_string(&p).expect("preferences serialize")],
        )?;
        db::event(
            &tx,
            "preferences_changed",
            "Device refresh and notification preferences saved.",
            None,
            None,
            "info",
        )?;
        tx.commit()?;
        Ok(p)
    }
}
fn checked_backup(repo: &Repository, id: &str, root: &Path) -> Result<Backup> {
    if !valid_id(id) {
        return Err(LibraryError::new(
            "INVALID_BACKUP",
            "Invalid backup identity.",
        ));
    }
    repo.connection.query_row("SELECT id,artifact_sha256,filename,size_bytes,created_at,verified_at,source_kind FROM firmware_backups WHERE id=?1 AND removed_at IS NULL",[id],|r|Ok(Backup{id:r.get(0)?,artifact_sha256:r.get(1)?,filename:r.get(2)?,size_bytes:r.get(3)?,created_at:r.get(4)?,verified_at:r.get(5)?,source_kind:r.get(6)?,location:root.join(format!("{id}.bin")).to_string_lossy().into_owned()})).optional()?.ok_or_else(||LibraryError::new("BACKUP_NOT_FOUND","Backup is no longer available."))
}
/// Metadata names are separate from content-addressed disk paths.
pub fn restored_name(original: &str, existing: &[String]) -> String {
    let path = Path::new(original);
    let stem = path
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or("firmware");
    let extension = path
        .extension()
        .and_then(|v| v.to_str())
        .filter(|v| !v.is_empty())
        .map(|s| format!(".{s}"))
        .unwrap_or_default();
    let stem = if stem.len() > 160 {
        stem.chars().take(80).collect::<String>()
    } else {
        stem.into()
    };
    let base = if stem.ends_with("_DEFAULT") {
        stem
    } else {
        format!("{stem}_DEFAULT")
    };
    for i in 0u64.. {
        let name = if i == 0 {
            format!("{base}{extension}")
        } else {
            format!("{base}_{i}{extension}")
        };
        if !existing.iter().any(|v| v.eq_ignore_ascii_case(&name)) {
            return name;
        }
    }
    unreachable!()
}
impl LibraryService {
    pub fn recover_default_library(&self, confirmed: bool) -> Result<()> {
        if !confirmed {
            return Err(LibraryError::new("CONFIRMATION_REQUIRED", "Confirm use of the default Library location. Missing files will not be copied or deleted."));
        }
        let controls = lock(&self.inner.controls)?;
        if !self.inner.vault.unavailable
            || self.inner.shutdown.load(Ordering::Acquire)
            || self.inner.relocating.load(Ordering::Acquire)
            || !controls.is_empty()
        {
            return Err(LibraryError::new("RECOVERY_UNAVAILABLE", "Recovery requires unavailable storage, no active jobs and no pending location change."));
        }
        let _checked = Vault::open(&self.inner.root.join("vault"))?;
        let mut repo = lock(&self.inner.repo)?;
        let mut preferences = repo.desktop_preferences()?;
        preferences.library_directory = None;
        let tx = repo.connection.transaction()?;
        tx.execute(
            "UPDATE desktop_preferences SET value=?1 WHERE id=1",
            [serde_json::to_string(&preferences).expect("preferences serialize")],
        )?;
        db::event(&tx, "library_recovery_selected", "Default Library selected after unavailable storage; restart required. Existing records, backups and previous storage retained.", None, None, "warning")?;
        tx.commit()?;
        self.inner.relocating.store(true, Ordering::Release);
        Ok(())
    }
    pub fn desktop_state(&self) -> Result<DesktopState> {
        let repo = lock(&self.inner.repo)?;
        let (library_bytes,extracted,customized)=repo.connection.query_row("SELECT COALESCE(SUM(a.size_bytes),0),COALESCE(SUM(CASE WHEN p.kind='extracted' OR (p.kind IS NULL AND EXISTS(SELECT 1 FROM artifact_imports i WHERE i.artifact_sha256=a.sha256 AND i.source_category='authorized_device_acquisition')) THEN 1 ELSE 0 END),0),COALESCE(SUM(CASE WHEN p.kind='customized' THEN 1 ELSE 0 END),0) FROM firmware_artifacts a JOIN artifact_details d USING(sha256) LEFT JOIN artifact_presentation p USING(sha256) WHERE d.removed_at IS NULL",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        Ok(DesktopState {
            library_available: !self.inner.vault.unavailable,
            preferences: repo.desktop_preferences()?,
            library_directory: self.inner.vault.root.to_string_lossy().into_owned(),
            backup_directory: self
                .inner
                .root
                .join("backups")
                .to_string_lossy()
                .into_owned(),
            restart_required: self.inner.relocating.load(Ordering::Acquire),
            library_bytes,
            backup_bytes: repo.connection.query_row(
                "SELECT COALESCE(SUM(size_bytes),0) FROM firmware_backups WHERE removed_at IS NULL",
                [],
                |r| r.get(0),
            )?,
            extracted,
            customized,
            max_concurrent_jobs: 1,
        })
    }
    pub fn backups(&self) -> Result<Vec<Backup>> {
        let repo = lock(&self.inner.repo)?;
        let mut q=repo.connection.prepare("SELECT id FROM firmware_backups WHERE removed_at IS NULL ORDER BY created_at DESC,id DESC LIMIT 100")?;
        let ids = q
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ids.iter()
            .map(|id| checked_backup(&repo, id, &self.inner.root.join("backups")))
            .collect()
    }
    fn queue_desktop(
        &self,
        kind: &str,
        total: u64,
        input: JobInput<'_>,
        task: impl FnOnce(String) -> Task,
    ) -> Result<JobRecord> {
        self.ensure_open()?;
        let mut controls = lock(&self.inner.controls)?;
        self.ensure_open()?;
        if controls.len() >= 16 {
            return Err(LibraryError::new(
                "QUEUE_FULL",
                "Wait for a library job to finish.",
            ));
        }
        let id = nonce()?;
        lock(&self.inner.repo)?.create_library_job(&id, kind, total, input)?;
        controls.insert(id.clone(), Arc::new(AtomicBool::new(false)));
        if self.inner.sender.try_send(task(id.clone())).is_err() {
            controls.remove(&id);
            lock(&self.inner.repo)?.library_transition(
                &id,
                JobStatus::Failed,
                Some(&LibraryError::new(
                    "QUEUE_FULL",
                    "The worker could not accept this operation.",
                )),
            )?;
        }
        drop(controls);
        self.job(&id)
    }
    pub fn create_backup(&self, hash: &str) -> Result<JobRecord> {
        let d = self.details(hash)?;
        self.queue_desktop(
            "firmware_backup",
            d.artifact.size_bytes,
            JobInput {
                filename: Some(&d.artifact.original_filename),
                hash: Some(hash),
                ..Default::default()
            },
            |job| Task::Backup {
                job,
                hash: hash.into(),
            },
        )
    }
    pub fn restore_backup(&self, id: &str, confirmed: bool) -> Result<JobRecord> {
        if !confirmed {
            return Err(LibraryError::new(
                "CONFIRMATION_REQUIRED",
                "Confirm the backup and Library destination before restoring.",
            ));
        }
        let repo = lock(&self.inner.repo)?;
        let b = checked_backup(&repo, id, &self.inner.root.join("backups"))?;
        let mut q=repo.connection.prepare("SELECT original_filename FROM artifact_imports UNION SELECT display_name FROM artifact_presentation UNION SELECT original_filename FROM job_details WHERE original_filename IS NOT NULL")?;
        let names = q
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<String>>>()?;
        let name = restored_name(&b.filename, &names);
        drop(q);
        drop(repo);
        self.queue_desktop(
            "backup_restore",
            b.size_bytes,
            JobInput {
                filename: Some(&name),
                hash: Some(&b.artifact_sha256),
                description: Some(
                    "Restored from a preserved local backup; provenance remains unverified.",
                ),
                ..Default::default()
            },
            |job| Task::Restore {
                job,
                backup: id.into(),
            },
        )
    }
    pub fn remove_item(&self, id: &str, backup: bool, confirmed: bool) -> Result<()> {
        self.ensure_open()?;
        if !confirmed {
            return Err(LibraryError::new(
                "CONFIRMATION_REQUIRED",
                "Confirm the selected file before removing it.",
            ));
        }
        let _fence = lock(&self.inner.finalize)?;
        let controls = lock(&self.inner.controls)?;
        if !controls.is_empty() {
            return Err(LibraryError::new(
                "FILE_IN_USE",
                "Wait for all active library jobs before removing a file.",
            ));
        }
        let mut repo = lock(&self.inner.repo)?;
        let (hash, name) = if backup {
            let b = checked_backup(&repo, id, &self.inner.root.join("backups"))?;
            (b.artifact_sha256, b.filename)
        } else {
            let d = repo.library_details(id)?;
            (d.artifact.sha256, d.artifact.original_filename)
        };
        let tx = repo.connection.transaction()?;
        if backup {
            tx.execute(
                "UPDATE firmware_backups SET removed_at=?2 WHERE id=?1 AND removed_at IS NULL",
                params![id, now()],
            )?;
        } else {
            tx.execute(
                "UPDATE artifact_details SET removed_at=?2 WHERE sha256=?1 AND removed_at IS NULL",
                params![id, now()],
            )?;
        }
        db::event(&tx,if backup{"backup_removed"}else{"firmware_removed"},&format!("Removed {name} from the active inventory. Stored bytes and history retained for recovery."),None,Some(&hash),"warning")?;
        tx.commit()?;
        Ok(())
    }
    pub fn open_folder(&self, target: &str, id: Option<&str>) -> Result<()> {
        let folder = match target {
            "library" => self.inner.vault.root.clone(),
            "backups" => self.inner.root.join("backups"),
            "artifact" => {
                let hash =
                    id.ok_or_else(|| LibraryError::new("INVALID_TARGET", "Select an artifact."))?;
                self.details(hash)?;
                self.inner
                    .vault
                    .blob(hash)?
                    .0
                    .parent()
                    .expect("blob parent")
                    .to_path_buf()
            }
            _ => {
                return Err(LibraryError::new(
                    "INVALID_TARGET",
                    "Only managed Library and Backup folders can be opened.",
                ))
            }
        };
        let _guards = guard_directories(&folder)?;
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            std::process::Command::new("explorer.exe")
                .arg(&folder)
                .creation_flags(0x08000000)
                .spawn()?;
            Ok(())
        }
        #[cfg(not(windows))]
        {
            Err(LibraryError::new(
                "WINDOWS_REQUIRED",
                "Open Folder requires the Windows app.",
            ))
        }
    }
    pub fn choose_directory(
        &self,
        owner: isize,
        scope: Option<&Path>,
    ) -> Result<Option<StoragePlan>> {
        self.ensure_open()?;
        if self.inner.dialog_open.swap(true, Ordering::AcqRel) {
            return Err(LibraryError::new(
                "DIALOG_BUSY",
                "Finish the open dialog first.",
            ));
        }
        let result = folder_dialog(owner)
            .and_then(|folder| folder.map(|p| self.plan_directory(&p, scope)).transpose());
        self.inner.dialog_open.store(false, Ordering::Release);
        result
    }
    pub(crate) fn plan_directory(
        &self,
        folder: &Path,
        scope: Option<&Path>,
    ) -> Result<StoragePlan> {
        let _guards = guard_directories(folder)?;
        if let Some(scope) = scope {
            if !inside(folder, scope)? {
                return Err(LibraryError::new(
                    "TEST_SCOPE_REQUIRED",
                    "Native test storage must remain inside its marked run directory.",
                ));
            }
        }
        let destination = folder.join("OBTAINER-Library");
        if destination.exists()
            || inside(&destination, &self.inner.root)?
            || inside(&self.inner.root, folder)?
            || inside(&destination, &self.inner.vault.root)?
        {
            return Err(LibraryError::new("STORAGE_CONFLICT","Choose a different empty parent folder outside current application storage. An existing destination is never overwritten."));
        }
        let probe = folder.join(format!(".obtainer-write-check-{}", nonce()?));
        let test = new_file(&probe)?;
        test.sync_all()?;
        drop(test);
        fs::remove_file(probe)?;
        let repo = lock(&self.inner.repo)?;
        let (count,bytes)=repo.connection.query_row("SELECT COUNT(*),COALESCE(SUM(a.size_bytes),0) FROM firmware_artifacts a JOIN artifact_details d USING(sha256) WHERE d.storage_key<>''",[],|r|Ok((r.get(0)?,r.get(1)?)))?;
        drop(repo);
        free_space(folder, bytes)?;
        let plan = StoragePlan {
            ticket: nonce()?,
            source: self.inner.vault.root.to_string_lossy().into_owned(),
            destination: destination.to_string_lossy().into_owned(),
            file_count: count,
            bytes,
            expires_at: now() + 300000,
        };
        let mut plans = lock(&self.inner.plans)?;
        plans.retain(|_, p| p.expires_at > now());
        if plans.len() >= 8 {
            return Err(LibraryError::new(
                "PLAN_LIMIT",
                "Too many storage plans. Reopen Settings in a few minutes.",
            ));
        }
        plans.insert(plan.ticket.clone(), plan.clone());
        Ok(plan)
    }
    pub fn relocate(&self, ticket: &str, confirmed: bool) -> Result<JobRecord> {
        self.ensure_open()?;
        if !confirmed {
            return Err(LibraryError::new(
                "CONFIRMATION_REQUIRED",
                "Review and confirm the storage copy plan.",
            ));
        }
        let mut controls = lock(&self.inner.controls)?;
        self.ensure_open()?;
        if !controls.is_empty() {
            return Err(LibraryError::new(
                "FILE_IN_USE",
                "Finish all jobs before moving Library storage.",
            ));
        }
        let plan = lock(&self.inner.plans)?
            .remove(ticket)
            .filter(|p| p.expires_at > now())
            .ok_or_else(|| {
                LibraryError::new(
                    "PLAN_EXPIRED",
                    "Select the folder again; this storage plan expired.",
                )
            })?;
        let current = {
            let repo = lock(&self.inner.repo)?;
            repo.connection.query_row("SELECT COUNT(*),COALESCE(SUM(a.size_bytes),0) FROM firmware_artifacts a JOIN artifact_details d USING(sha256) WHERE d.storage_key<>''",[],|r|Ok((r.get::<_,u64>(0)?,r.get::<_,u64>(1)?)))?
        };
        if current != (plan.file_count, plan.bytes) {
            return Err(LibraryError::new("PLAN_CHANGED","Library contents changed. Select the destination again to review an updated copy plan."));
        }
        let id = nonce()?;
        lock(&self.inner.repo)?.create_library_job(
            &id,
            "library_relocation",
            plan.bytes,
            JobInput::default(),
        )?;
        self.inner.relocating.store(true, Ordering::Release);
        controls.insert(id.clone(), Arc::new(AtomicBool::new(false)));
        if self
            .inner
            .sender
            .try_send(Task::Relocate {
                job: id.clone(),
                plan,
            })
            .is_err()
        {
            self.inner.relocating.store(false, Ordering::Release);
            controls.remove(&id);
            lock(&self.inner.repo)?.library_transition(
                &id,
                JobStatus::Failed,
                Some(&LibraryError::new(
                    "QUEUE_FULL",
                    "Storage copy was not queued.",
                )),
            )?;
        }
        drop(controls);
        self.job(&id)
    }
}
impl Inner {
    pub(super) fn run_backup(&self, id: &str, hash: &str, cancel: &AtomicBool) -> Result<()> {
        let d = lock(&self.repo)?.library_details(hash)?;
        let (source, _source_guards) = self.vault.blob(hash)?;
        let destination = self.root.join("backups");
        let _guards = guard_directories(&destination)?;
        free_space(&destination, d.artifact.size_bytes)?;
        // Staging remains on the same application volume as backups, even with an external Library.
        let stage = destination.join(format!("{id}.part"));
        let result = (|| {
            self.copy_checked(id, &source, &stage, hash, d.artifact.size_bytes, cancel)?;
            let final_path = destination.join(format!("{id}.bin"));
            let _fence = lock(&self.finalize)?;
            cancelled(cancel)?;
            fs::hard_link(&stage, &final_path)?;
            drop(_fence);
            let (lease, actual, n) = hash_file(&final_path, self.max, cancel, |_| Ok(()))?;
            if actual != hash || n != d.artifact.size_bytes {
                return Err(LibraryError::new(
                    "HASH_MISMATCH",
                    "Published backup did not match its source; no backup success recorded.",
                ));
            }
            let _fence = lock(&self.finalize)?;
            cancelled(cancel)?;
            let mut repo = lock(&self.repo)?;
            // Completion and backup registration share one SQLite transaction.
            let tx = repo.connection.transaction()?;
            tx.execute(
                "INSERT INTO firmware_backups VALUES(?1,?2,?3,?4,?5,?5,?6,NULL)",
                params![
                    id,
                    hash,
                    d.artifact.original_filename,
                    n,
                    now(),
                    d.artifact.kind
                ],
            )?;
            tx.execute(
                "UPDATE jobs SET status='completed' WHERE id=?1 AND status='running'",
                [id],
            )?;
            tx.execute("UPDATE job_details SET finished_at=?2,updated_at=?2,progress_bytes=total_bytes,stage='completed',message='Backup created and independently verified.',recovery_info=NULL WHERE job_id=?1",params![id,now()])?;
            db::event(
                &tx,
                "backup_created",
                "Original file copy preserved and independently verified.",
                Some(id),
                Some(hash),
                "info",
            )?;
            tx.commit()?;
            drop(lease);
            Ok(())
        })();
        // Only this worker's validated regular stage is removed, never published or unknown data.
        if stage.exists() && read_file(&stage).is_ok() {
            let _ = fs::remove_file(&stage);
        }
        result
    }
    fn copy_checked(
        &self,
        id: &str,
        source: &Path,
        stage: &Path,
        expected: &str,
        size: u64,
        cancel: &AtomicBool,
    ) -> Result<()> {
        let mut file = read_file(source)?;
        let mut target = new_file(stage)?;
        let mut last = Instant::now();
        self.progress(id, 0, size, "copying")?;
        let (hash, n) = stream(&mut file, &mut target, size, cancel, |bytes| {
            if last.elapsed() >= Duration::from_millis(100) {
                self.progress(id, bytes, size, "copying")?;
                last = Instant::now();
            }
            Ok(())
        })?;
        if hash != expected || n != size {
            return Err(LibraryError::new(
                "HASH_MISMATCH",
                "Source bytes do not match the saved checksum. Nothing was overwritten.",
            ));
        }
        target.flush()?;
        target.sync_all()?;
        drop(target);
        self.progress(id, 0, size, "verifying")?;
        let (_lease, hash, n) = hash_file(stage, self.max, cancel, |bytes| {
            if last.elapsed() >= Duration::from_millis(100) {
                self.progress(id, bytes, size, "verifying")?;
                last = Instant::now();
            }
            Ok(())
        })?;
        if hash != expected || n != size {
            return Err(LibraryError::new(
                "HASH_MISMATCH",
                "Copied bytes failed verification.",
            ));
        }
        Ok(())
    }
    pub(super) fn run_restore(&self, id: &str, backup: &str, cancel: &AtomicBool) -> Result<()> {
        let b = {
            let repo = lock(&self.repo)?;
            checked_backup(&repo, backup, &self.root.join("backups"))?
        };
        let _guards = guard_directories(&self.root.join("backups"))?;
        let (mut file, hash, n) = hash_file(Path::new(&b.location), self.max, cancel, |bytes| {
            self.progress(id, bytes, b.size_bytes, "verifying_backup")
        })?;
        if hash != b.artifact_sha256 || n != b.size_bytes {
            return Err(LibraryError::new(
                "HASH_MISMATCH",
                "Backup checksum failed. Original backup was retained; no restored image recorded.",
            ));
        }
        file.seek(SeekFrom::Start(0))?;
        let metadata = file.metadata()?;
        let name = lock(&self.repo)?.connection.query_row(
            "SELECT original_filename FROM job_details WHERE job_id=?1",
            [id],
            |r| r.get(0),
        )?;
        let selection = Selection {
            file,
            metadata,
            preview: FileSelection {
                ticket: id.into(),
                filename: name,
                size_bytes: b.size_bytes,
                expires_at: now() + 300000,
                source_category: "user_import".into(),
                max_file_bytes: self.max,
            },
        };
        self.run_import(id, selection, cancel)
    }
    pub(super) fn run_relocation(
        &self,
        id: &str,
        plan: &StoragePlan,
        cancel: &AtomicBool,
    ) -> Result<()> {
        let destination = Path::new(&plan.destination);
        // A new, dedicated directory is mandatory. Never adopt or overwrite an existing tree.
        let _parent = guard_directories(
            destination
                .parent()
                .ok_or_else(|| LibraryError::new("UNSAFE_PATH", "Missing destination parent."))?,
        )?;
        fs::create_dir(destination)?;
        let vault = Vault::open(destination)?;
        let files = {
            let repo = lock(&self.repo)?;
            let mut q=repo.connection.prepare("SELECT a.sha256,a.size_bytes FROM firmware_artifacts a JOIN artifact_details d USING(sha256) WHERE d.storage_key<>'' ORDER BY a.sha256")?;
            let rows = q
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u64>(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows
        };
        let total: u64 = files.iter().map(|f| f.1).sum();
        free_space(destination, total)?;
        let mut copied = 0;
        for (hash, size) in files {
            cancelled(cancel)?;
            let (source, _src) = self.vault.blob(&hash)?;
            let stage = vault.stage(id)?;
            self.copy_checked(id, &source, &stage, &hash, size, cancel)?;
            let (final_path, _dest) = vault.blob(&hash)?;
            fs::hard_link(&stage, &final_path)?;
            let (_lease, actual, n) = hash_file(&final_path, self.max, cancel, |_| Ok(()))?;
            if actual != hash || n != size {
                return Err(LibraryError::new(
                    "HASH_MISMATCH",
                    "New storage failed verification; the original Library remains active.",
                ));
            }
            vault.remove_stage(id)?;
            copied += size;
            self.progress(id, copied, total, "copying_library")?;
        }
        let _fence = lock(&self.finalize)?;
        cancelled(cancel)?;
        let mut repo = lock(&self.repo)?;
        let mut preferences = repo.desktop_preferences()?;
        preferences.library_directory = Some(plan.destination.clone());
        let tx = repo.connection.transaction()?;
        tx.execute(
            "UPDATE desktop_preferences SET value=?1 WHERE id=1",
            [serde_json::to_string(&preferences).expect("preferences serialize")],
        )?;
        tx.execute(
            "UPDATE jobs SET status='completed' WHERE id=?1 AND status='running'",
            [id],
        )?;
        tx.execute("UPDATE job_details SET finished_at=?2,updated_at=?2,progress_bytes=total_bytes,stage='completed',message='Library copied and verified. Restart OBTAINER to use the new location; original bytes are preserved.' WHERE job_id=?1",params![id,now()])?;
        db::event(
            &tx,
            "library_relocated",
            "Library copy verified. Restart required; previous storage preserved.",
            Some(id),
            None,
            "info",
        )?;
        tx.commit()?;
        Ok(())
    }
}
#[cfg(windows)]
fn folder_dialog(owner: isize) -> Result<Option<PathBuf>> {
    use windows_sys::Win32::{System::Com::*, UI::Shell::*};
    let init = unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };
    if init < 0 {
        return Err(LibraryError::new(
            "DIALOG_FAILED",
            "Folder picker could not initialize.",
        ));
    }
    let title: Vec<u16> = "Select Library location\0".encode_utf16().collect();
    let mut info: BROWSEINFOW = unsafe { std::mem::zeroed() };
    info.hwndOwner = owner as _;
    info.lpszTitle = title.as_ptr();
    info.ulFlags = BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE | BIF_EDITBOX;
    let item = unsafe { SHBrowseForFolderW(&info) };
    if item.is_null() {
        unsafe { CoUninitialize() };
        return Ok(None);
    }
    let mut path = [0u16; 260];
    let ok = unsafe { SHGetPathFromIDListW(item, path.as_mut_ptr()) };
    unsafe {
        CoTaskMemFree(item.cast());
        CoUninitialize();
    }
    if ok == 0 {
        return Err(LibraryError::new(
            "INVALID_FOLDER",
            "Choose a normal local folder.",
        ));
    }
    let end = path.iter().position(|v| *v == 0).unwrap_or(path.len());
    Ok(Some(PathBuf::from(
        String::from_utf16(&path[..end]).map_err(|_| {
            LibraryError::new("INVALID_FOLDER", "Folder name is not valid Unicode.")
        })?,
    )))
}
#[cfg(not(windows))]
fn folder_dialog(_: isize) -> Result<Option<PathBuf>> {
    Err(LibraryError::new(
        "WINDOWS_REQUIRED",
        "Folder selection requires Windows.",
    ))
}
