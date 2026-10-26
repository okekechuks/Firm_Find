mod admission;
pub mod analysis;
mod db;
pub mod desktop;
pub(crate) mod files;
pub mod ipc;
mod manifest_contract;
pub mod official;
pub mod types;
use crate::{connection::now, storage::Repository};
use files::*;
use std::{
    collections::HashMap,
    fs::{self, File, Metadata},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
pub use types::*;

pub const DEFAULT_MAX: u64 = 2 * 1024 * 1024 * 1024;
struct Selection {
    file: File,
    metadata: Metadata,
    preview: FileSelection,
}
enum Task {
    Analyze {
        job: String,
        hash: String,
    },
    Official {
        job: String,
        source: Box<official::QualifiedSource>,
    },
    Import {
        job: String,
        selection: Selection,
    },
    Verify {
        job: String,
        hash: String,
    },
    Backup {
        job: String,
        hash: String,
    },
    Restore {
        job: String,
        backup: String,
    },
    Relocate {
        job: String,
        plan: desktop::StoragePlan,
    },
}
type Notify = Arc<dyn Fn(JobRecord) + Send + Sync>;
struct Inner {
    repo: Arc<Mutex<Repository>>,
    vault: Vault,
    root: PathBuf,
    max: u64,
    tickets: Mutex<HashMap<String, Selection>>,
    controls: Mutex<HashMap<String, Arc<AtomicBool>>>,
    sender: mpsc::SyncSender<Task>,
    shutdown: AtomicBool,
    dialog_open: AtomicBool,
    finalize: Mutex<()>,
    notify: Notify,
    relocating: AtomicBool,
    plans: Mutex<HashMap<String, desktop::StoragePlan>>,
}
#[derive(Clone)]
pub struct LibraryService {
    inner: Arc<Inner>,
    worker: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}
fn lock<T>(m: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>> {
    m.lock().map_err(|_| {
        LibraryError::new(
            "SERVICE_UNAVAILABLE",
            "Library state is unavailable. Restart the app.",
        )
    })
}
impl LibraryService {
    pub fn acquire_official(
        &self,
        source_id: &str,
        confirm_exact_target: bool,
    ) -> Result<JobRecord> {
        self.ensure_open()?;
        let source = official::qualify(source_id, confirm_exact_target)?;
        let entry = source.entry();
        if entry.size_bytes > self.inner.max {
            return Err(LibraryError::new(
                "FILE_TOO_LARGE",
                "Approved image exceeds the configured Library limit.",
            ));
        }
        free_space(&self.inner.vault.root, entry.size_bytes)?;
        let mut controls = lock(&self.inner.controls)?;
        self.ensure_open()?;
        if controls.len() >= 16 {
            return Err(LibraryError::new(
                "QUEUE_FULL",
                "Wait for an existing Library job to finish.",
            ));
        }
        let id = nonce()?;
        lock(&self.inner.repo)?.create_library_job(
            &id,
            "official_image_download",
            entry.size_bytes,
            JobInput {
                filename: Some(&entry.filename),
                description: Some(&entry.id),
                hash: Some(&entry.sha256),
                ..Default::default()
            },
        )?;
        controls.insert(id.clone(), Arc::new(AtomicBool::new(false)));
        if self
            .inner
            .sender
            .try_send(Task::Official {
                job: id.clone(),
                source: Box::new(source),
            })
            .is_err()
        {
            controls.remove(&id);
            lock(&self.inner.repo)?.library_transition(
                &id,
                JobStatus::Failed,
                Some(&LibraryError::new(
                    "QUEUE_FULL",
                    "Download was not queued; no request sent.",
                )),
            )?;
        }
        lock(&self.inner.repo)?.library_job(&id)
    }
    pub fn start(
        repo: Arc<Mutex<Repository>>,
        root: PathBuf,
        max: u64,
        notify: Notify,
    ) -> Result<Self> {
        lock(&repo)?.ensure_ready().map_err(|_|LibraryError::new("UPGRADE_INCOMPLETE","Library upgrade is incomplete. Finish recovery with this version before starting library jobs."))?;
        if max == 0 || max > 8 * 1024 * 1024 * 1024 {
            return Err(LibraryError::new(
                "INVALID_LIMIT",
                "Import size limit must be between 1 byte and 8 GiB.",
            ));
        }
        let configured = lock(&repo)?.desktop_preferences()?.library_directory;
        let vault = match configured {
            None => Vault::open(&root.join("vault"))?,
            Some(path) => {
                let path = PathBuf::from(path);
                // Default storage may be initialized automatically. A relocated root
                // must already exist, and loss of access must not prevent app startup.
                match guard_directories(&path).and_then(|_lease| Vault::open(&path)) {
                    Ok(vault) => vault,
                    Err(error) => {
                        tracing::warn!(
                            code = error.code,
                            "Configured Library unavailable; records retained"
                        );
                        Vault::unavailable(path)
                    }
                }
            }
        };
        let _backup_guards = directory(&root.join("backups"))?;
        let (sender, receiver) = mpsc::sync_channel(16);
        let inner = Arc::new(Inner {
            repo,
            vault,
            root,
            max,
            tickets: Mutex::new(HashMap::new()),
            controls: Mutex::new(HashMap::new()),
            sender,
            shutdown: AtomicBool::new(false),
            dialog_open: AtomicBool::new(false),
            finalize: Mutex::new(()),
            notify,
            relocating: AtomicBool::new(false),
            plans: Mutex::new(HashMap::new()),
        });
        inner.recover()?;
        let service = Arc::clone(&inner);
        let handle = thread::Builder::new()
            .name("firmware-library".into())
            .spawn(move || loop {
                match receiver.recv_timeout(Duration::from_millis(100)) {
                    Ok(task) => service.execute(task),
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if service.shutdown.load(Ordering::Acquire) {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            })?;
        Ok(Self {
            inner,
            worker: Arc::new(Mutex::new(Some(handle))),
        })
    }
    pub fn stop(&self) {
        self.inner.shutdown.store(true, Ordering::Release);
        if let Ok(c) = self.inner.controls.lock() {
            for flag in c.values() {
                flag.store(true, Ordering::Release);
            }
        }
        if let Ok(mut t) = self.inner.tickets.lock() {
            t.clear();
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if let Ok(mut handle) = self.worker.lock() {
                if handle.as_ref().is_none_or(|h| h.is_finished()) {
                    if let Some(h) = handle.take() {
                        let _ = h.join();
                    }
                    return;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
    pub fn select(&self, owner: isize) -> Result<Option<FileSelection>> {
        self.ensure_open()?;
        if self.inner.dialog_open.swap(true, Ordering::AcqRel) {
            return Err(LibraryError::new(
                "DIALOG_BUSY",
                "Finish the current file dialog first.",
            ));
        }
        let result = dialog(false, owner).and_then(|p| p.map(|p| self.select_path(&p)).transpose());
        self.inner.dialog_open.store(false, Ordering::Release);
        result
    }
    fn ensure_open(&self) -> Result<()> {
        self.inner.vault.ensure_available()?;
        if self.inner.shutdown.load(Ordering::Acquire)
            || self.inner.relocating.load(Ordering::Acquire)
        {
            return Err(LibraryError::new(
                "SHUTTING_DOWN",
                "The library is restarting or relocating. Wait for the copy to finish, then reopen the application.",
            ));
        }
        Ok(())
    }
    pub(crate) fn select_path(&self, path: &Path) -> Result<FileSelection> {
        self.ensure_open()?;
        safe_path(path)?;
        if inside(path, &self.inner.root)? || inside(path, &self.inner.vault.root)? {
            return Err(LibraryError::new(
                "INTERNAL_SOURCE",
                "Select an external source copy, not application data or vault internals.",
            ));
        }
        let _guards =
            guard_directories(path.parent().ok_or_else(|| {
                LibraryError::new("INVALID_SOURCE", "Source has no parent folder.")
            })?)?;
        let file = read_file(path)?;
        let metadata = file.metadata()?;
        if metadata.len() > self.inner.max {
            return Err(LibraryError::new(
                "FILE_TOO_LARGE",
                "File exceeds the configured import limit.",
            ));
        }
        free_space(&self.inner.vault.root, metadata.len())?;
        let preview = FileSelection {
            ticket: nonce()?,
            filename: filename(path)?,
            size_bytes: metadata.len(),
            expires_at: now() + 300000,
            source_category: "user_import".into(),
            max_file_bytes: self.inner.max,
        };
        let mut tickets = lock(&self.inner.tickets)?;
        tickets.retain(|_, v| v.preview.expires_at > now());
        if tickets.len() >= 8 {
            return Err(LibraryError::new(
                "SELECTION_LIMIT",
                "Too many pending selections. Import or discard a selection first.",
            ));
        }
        tickets.insert(
            preview.ticket.clone(),
            Selection {
                file,
                metadata,
                preview: preview.clone(),
            },
        );
        Ok(preview)
    }
    pub fn discard(&self, ticket: &str) -> Result<()> {
        lock(&self.inner.tickets)?.remove(ticket);
        Ok(())
    }
    pub fn import(&self, input: ImportRequest) -> Result<JobRecord> {
        self.ensure_open()?;
        if !valid_id(&input.ticket)
            || input
                .description
                .as_ref()
                .is_some_and(|v| v.len() > 1024 || v.chars().any(char::is_control))
        {
            return Err(LibraryError::new(
                "INVALID_IMPORT",
                "Invalid selection or description (maximum 1024 bytes).",
            ));
        }
        if let Some(device) = &input.associated_device_id {
            if device.len() > 128
                || !lock(&self.inner.repo)?.connection.query_row(
                    "SELECT EXISTS(SELECT 1 FROM devices WHERE id=?1)",
                    [device],
                    |r| r.get::<_, bool>(0),
                )?
            {
                return Err(LibraryError::new("INVALID_DEVICE","Associated device does not exist. Device labels do not establish firmware compatibility."));
            }
        }
        let mut controls = lock(&self.inner.controls)?;
        self.ensure_open()?;
        if controls.len() >= 16 {
            return Err(LibraryError::new(
                "QUEUE_FULL",
                "Library queue is full. Wait for a job to finish.",
            ));
        }
        let selection = lock(&self.inner.tickets)?
            .remove(&input.ticket)
            .ok_or_else(|| {
                LibraryError::new(
                    "SELECTION_EXPIRED",
                    "Select the file again; its ticket is missing or already used.",
                )
            })?;
        if selection.preview.expires_at < now() {
            return Err(LibraryError::new(
                "SELECTION_EXPIRED",
                "Select the file again; its approval expired.",
            ));
        }
        let id = nonce()?;
        lock(&self.inner.repo)?.create_library_job(
            &id,
            "firmware_import",
            selection.metadata.len(),
            JobInput {
                filename: Some(&selection.preview.filename),
                description: input.description.as_deref(),
                device: input.associated_device_id.as_deref(),
                hash: None,
            },
        )?;
        controls.insert(id.clone(), Arc::new(AtomicBool::new(false)));
        if self
            .inner
            .sender
            .try_send(Task::Import {
                job: id.clone(),
                selection,
            })
            .is_err()
        {
            controls.remove(&id);
            lock(&self.inner.repo)?.library_transition(
                &id,
                JobStatus::Failed,
                Some(&LibraryError::new(
                    "QUEUE_FULL",
                    "Queue could not accept this import. Select the file again.",
                )),
            )?;
        }
        drop(controls);
        self.job(&id)
    }
    pub fn verify(&self, hash: &str) -> Result<JobRecord> {
        self.ensure_open()?;
        let details = lock(&self.inner.repo)?.library_details(hash)?;
        let mut controls = lock(&self.inner.controls)?;
        self.ensure_open()?;
        if controls.len() >= 16 {
            return Err(LibraryError::new("QUEUE_FULL", "Library queue is full."));
        }
        let id = nonce()?;
        lock(&self.inner.repo)?.create_library_job(
            &id,
            "integrity_verify",
            details.artifact.size_bytes,
            JobInput {
                hash: Some(hash),
                ..Default::default()
            },
        )?;
        controls.insert(id.clone(), Arc::new(AtomicBool::new(false)));
        if self
            .inner
            .sender
            .try_send(Task::Verify {
                job: id.clone(),
                hash: hash.into(),
            })
            .is_err()
        {
            controls.remove(&id);
            lock(&self.inner.repo)?.library_transition(
                &id,
                JobStatus::Failed,
                Some(&LibraryError::new(
                    "QUEUE_FULL",
                    "Queue could not accept verification.",
                )),
            )?;
        }
        drop(controls);
        self.job(&id)
    }
    pub fn cancel(&self, id: &str) -> Result<JobRecord> {
        let _fence = lock(&self.inner.finalize)?;
        let job = self.job(id)?;
        if job.status.terminal() {
            return Ok(job);
        }
        if ![
            "firmware_import",
            "integrity_verify",
            "firmware_backup",
            "backup_restore",
            "library_relocation",
        ]
        .contains(&job.kind.as_str())
        {
            return Err(LibraryError::new(
                "UNSUPPORTED_JOB",
                "Historical job retained; this service cannot cancel that job type.",
            ));
        }
        if let Some(flag) = lock(&self.inner.controls)?.get(id) {
            flag.store(true, Ordering::Release);
        }
        lock(&self.inner.repo)?.library_cancel_requested(id)
    }
    pub fn job(&self, id: &str) -> Result<JobRecord> {
        lock(&self.inner.repo)?.library_job(id)
    }
    pub fn jobs(&self) -> Result<Vec<JobRecord>> {
        lock(&self.inner.repo)?.library_jobs()
    }
    pub fn list(&self, q: &LibraryQuery) -> Result<LibraryPage> {
        lock(&self.inner.repo)?.library_artifacts(q, self.inner.max)
    }
    pub fn details(&self, hash: &str) -> Result<ArtifactDetails> {
        lock(&self.inner.repo)?.library_details(hash)
    }
    pub fn import_history(&self) -> Result<Vec<FirmwareImport>> {
        lock(&self.inner.repo)?.library_import_history()
    }
    pub fn activity(&self) -> Result<Vec<ActivityEvent>> {
        lock(&self.inner.repo)?.library_activity()
    }
    pub fn export(&self, hash: &str, owner: isize) -> Result<ManifestExportResult> {
        self.ensure_open()?;
        if self.inner.dialog_open.swap(true, Ordering::AcqRel) {
            return Err(LibraryError::new(
                "DIALOG_BUSY",
                "Finish the current file dialog first.",
            ));
        }
        let result = dialog(true, owner).and_then(|p| match p {
            Some(p) => self.export_path(hash, &p),
            None => Ok(ManifestExportResult {
                saved: false,
                activity_recorded: false,
                warning: None,
            }),
        });
        self.inner.dialog_open.store(false, Ordering::Release);
        result
    }
    /// Backend-generated, redacted metadata only. Never overwrite a selected file.
    pub fn export_redacted_report(&self, report: &serde_json::Value, owner: isize) -> Result<bool> {
        self.ensure_open()?;
        if self.inner.dialog_open.swap(true, Ordering::AcqRel) {
            return Err(LibraryError::new(
                "DIALOG_BUSY",
                "Finish the current file dialog first.",
            ));
        }
        let result = (|| {
            let Some(path) = diagnostics_dialog(owner)? else {
                return Ok(false);
            };
            safe_path(&path)?;
            if inside(&path, &self.inner.root)? || inside(&path, &self.inner.vault.root)? {
                return Err(LibraryError::new(
                    "INTERNAL_DESTINATION",
                    "Save reports outside application data.",
                ));
            }
            let parent = path
                .parent()
                .ok_or_else(|| LibraryError::new("UNSAFE_PATH", "No parent folder."))?;
            let _guards = guard_directories(parent)?;
            let temp = parent.join(format!(".obtainer-{}.tmp", nonce()?));
            let result = (|| {
                let mut file = new_file(&temp)?;
                file.write_all(&serde_json::to_vec_pretty(report).map_err(|_| {
                    LibraryError::new("EXPORT_FAILED", "Report serialization failed.")
                })?)?;
                file.sync_all()?;
                drop(file);
                fs::hard_link(&temp, &path)?;
                Ok(true)
            })();
            let _ = fs::remove_file(&temp);
            result
        })();
        self.inner.dialog_open.store(false, Ordering::Release);
        result
    }
    pub(crate) fn export_path(&self, hash: &str, path: &Path) -> Result<ManifestExportResult> {
        safe_path(path)?;
        if inside(path, &self.inner.root)? || inside(path, &self.inner.vault.root)? {
            return Err(LibraryError::new(
                "INTERNAL_DESTINATION",
                "Save manifests outside application data and the vault.",
            ));
        }
        let parent = path
            .parent()
            .ok_or_else(|| LibraryError::new("UNSAFE_PATH", "Destination has no parent."))?;
        let _guards = guard_directories(parent)?;
        if path.exists() {
            return Err(LibraryError::new(
                "DESTINATION_EXISTS",
                "Choose a new filename. Existing files are never overwritten.",
            ));
        }
        let d = self.details(hash)?;
        // Explicitly exclude source paths, descriptions, device identifiers and credentials.
        let sources:Vec<_>=d.imports.iter().map(|i|serde_json::json!({"originalFilename":i.original_filename,"importedAt":i.imported_at,"sourceCategory":i.source_category,"provenanceStatus":i.provenance_status})).collect();
        let manifest = serde_json::json!({"manifestSchemaVersion":1,"applicationVersion":env!("CARGO_PKG_VERSION"),"sha256":hash,"sizeBytes":d.artifact.size_bytes,"format":d.artifact.format,"integrityStatus":d.artifact.integrity_status,"lastVerifiedAt":d.artifact.last_verified_at,"sourceCount":d.artifact.source_count,"sources":sources,"omittedSourceCount":d.artifact.source_count.saturating_sub(d.imports.len() as u64),"notice":"Unsigned checksum metadata; not proof of authenticity, safety, compatibility or flashing support."});
        let mut manifest = manifest;
        manifest["officialSources"] = serde_json::to_value(self.official_provenance(hash)?)
            .map_err(|_| {
                LibraryError::new("EXPORT_FAILED", "Source provenance serialization failed.")
            })?;
        let temp = parent.join(format!(".obtainer-{}.tmp", nonce()?));
        let result = (|| -> Result<ManifestExportResult> {
            let mut f = new_file(&temp)?;
            f.write_all(&serde_json::to_vec_pretty(&manifest).map_err(|_| {
                LibraryError::new("EXPORT_FAILED", "Manifest serialization failed.")
            })?)?;
            f.sync_all()?;
            drop(f);
            fs::hard_link(&temp, path)?;
            // Publication is the save commit point. Audit failure cannot undo a valid export.
            let activity_recorded = lock(&self.inner.repo)
                .and_then(|mut repo| repo.library_exported(hash))
                .is_ok();
            Ok(ManifestExportResult {
                saved: true,
                activity_recorded,
                warning: (!activity_recorded).then(|| "Manifest saved, but workspace activity could not be recorded. Keep the saved file; do not retry using the same filename.".into()),
            })
        })();
        let _ = fs::remove_file(&temp);
        result
    }
}
impl Drop for LibraryService {
    fn drop(&mut self) {
        if Arc::strong_count(&self.worker) == 1 {
            self.stop();
        }
    }
}
impl Inner {
    fn progress(&self, id: &str, bytes: u64, total: u64, stage: &str) -> Result<()> {
        let report = {
            let mut repo = lock(&self.repo)?;
            repo.library_progress(id, bytes, total, stage)?;
            repo.library_job(id)?
        };
        (self.notify)(report);
        Ok(())
    }
    fn execute(&self, task: Task) {
        let id = match &task {
            Task::Analyze { job, .. }
            | Task::Official { job, .. }
            | Task::Import { job, .. }
            | Task::Verify { job, .. }
            | Task::Backup { job, .. }
            | Task::Restore { job, .. }
            | Task::Relocate { job, .. } => job.clone(),
        };
        let flag = match self.controls.lock().ok().and_then(|m| m.get(&id).cloned()) {
            Some(f) => f,
            None => return,
        };
        let result = (|| -> Result<()> {
            if self.shutdown.load(Ordering::Acquire) {
                flag.store(true, Ordering::Release);
            }
            cancelled(&flag)?;
            let started = lock(&self.repo)?.library_transition(&id, JobStatus::Running, None)?;
            (self.notify)(started);
            match task {
                Task::Analyze { hash, .. } => self.run_analysis(&id, &hash, &flag),
                Task::Official { source, .. } => self.run_official(&id, *source, &flag),
                Task::Import { selection, .. } => self.run_import(&id, selection, &flag),
                Task::Verify { hash, .. } => self.run_verify(&id, &hash, &flag),
                Task::Backup { hash, .. } => self.run_backup(&id, &hash, &flag),
                Task::Restore { backup, .. } => self.run_restore(&id, &backup, &flag),
                Task::Relocate { plan, .. } => self.run_relocation(&id, &plan, &flag),
            }
        })();
        if let Err(error) = result {
            if self
                .repo
                .lock()
                .ok()
                .and_then(|r| r.library_job(&id).ok())
                .is_some_and(|j| j.kind == "library_relocation")
            {
                self.relocating.store(false, Ordering::Release);
            }
            if let Ok(mut repo) = self.repo.lock() {
                let _=repo.connection.execute("UPDATE artifact_details SET integrity_status='not_checked' WHERE integrity_status='checking' AND sha256=(SELECT artifact_sha256 FROM job_details WHERE job_id=?1)",[&id]);
                let _ = repo.library_transition(
                    &id,
                    if error.code == "CANCELLED" {
                        JobStatus::Cancelled
                    } else {
                        JobStatus::Failed
                    },
                    Some(&error),
                );
            }
        }
        if self.vault.remove_stage(&id).is_err() {
            if let Ok(repo) = self.repo.lock() {
                let _=repo.connection.execute("UPDATE job_details SET recovery_info='Staging cleanup could not be verified; retained for manual review.' WHERE job_id=?1",[&id]);
            }
        }
        let final_job = self
            .repo
            .lock()
            .ok()
            .and_then(|repo| repo.library_job(&id).ok());
        if let Some(job) = final_job {
            (self.notify)(job);
        }
        if let Ok(mut controls) = self.controls.lock() {
            controls.remove(&id);
        }
    }
    fn run_import(&self, id: &str, mut selection: Selection, cancel: &AtomicBool) -> Result<()> {
        if !unchanged(&selection.metadata, &selection.file.metadata()?) {
            return Err(LibraryError::new(
                "SOURCE_CHANGED",
                "Source changed after selection. Select it again.",
            ));
        }
        let size = selection.metadata.len();
        free_space(&self.vault.root, size)?;
        let stage = self.vault.stage(id)?;
        let mut target = new_file(&stage)?;
        let mut last = Instant::now();
        self.progress(id, 0, size, "copying")?;
        let (hash, copied) = stream(&mut selection.file, &mut target, size, cancel, |bytes| {
            if last.elapsed() >= Duration::from_millis(100) {
                self.progress(id, bytes, size, "copying")?;
                last = Instant::now();
            }
            Ok(())
        })?;
        if copied != size || !unchanged(&selection.metadata, &selection.file.metadata()?) {
            return Err(LibraryError::new(
                "SOURCE_CHANGED",
                "Source changed during import; no artifact was committed.",
            ));
        }
        target.flush()?;
        target.sync_all()?;
        drop(target);
        self.progress(id, 0, size, "checking_staging")?;
        last = Instant::now();
        let (staged, staged_hash, staged_size) = hash_file(&stage, size, cancel, |bytes| {
            if last.elapsed() >= Duration::from_millis(100) {
                self.progress(id, bytes, size, "checking_staging")?;
                last = Instant::now();
            }
            Ok(())
        })?;
        drop(staged);
        if staged_hash != hash || staged_size != size {
            return Err(LibraryError::new(
                "STAGING_MISMATCH",
                "Staged bytes did not match the streamed source. No artifact committed.",
            ));
        }
        lock(&self.repo)?.library_candidate(id, &hash)?;
        let (blob, _guards) = self.vault.blob(&hash)?;
        let _fence = lock(&self.finalize)?;
        cancelled(cancel)?;
        let duplicate = match fs::hard_link(&stage, &blob) {
            Ok(()) => false,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => true,
            Err(e) => return Err(e.into()),
        };
        // Hold a read-only Windows sharing lease through metadata commit. Never overwrite blobs.
        // Cancellation remains accepted during hashing via the atomic flag; commit itself is fenced.
        drop(_fence);
        self.progress(id, 0, size, "checking_vault")?;
        last = Instant::now();
        let checked = self
            .vault
            .verify_published(&hash, self.max, cancel, |bytes| {
                if last.elapsed() >= Duration::from_millis(100) {
                    self.progress(id, bytes, size, "checking_vault")?;
                    last = Instant::now();
                }
                Ok(())
            });
        let proof = match checked {
            Ok(v) => v,
            Err(e) if e.code == "CANCELLED" => return Err(e),
            Err(e) => {
                self.mark_bad_duplicate(&hash, size, None, None, &e);
                return Err(e);
            }
        };
        if proof.hash() != hash || proof.size() != size {
            let error=LibraryError::new("HASH_MISMATCH","Existing vault content is corrupted. It was preserved and was not reused or overwritten.");
            self.mark_bad_duplicate(
                &hash,
                size,
                Some(proof.hash().into()),
                Some(proof.size()),
                &error,
            );
            return Err(error);
        }
        let _fence = lock(&self.finalize)?;
        cancelled(cancel)?;
        lock(&self.repo)?.commit_library_import(id, &proof, duplicate)?;
        drop(proof);
        Ok(())
    }
    fn run_official(
        &self,
        id: &str,
        source: official::QualifiedSource,
        cancel: &AtomicBool,
    ) -> Result<()> {
        let entry = source.entry();
        let stage = self.vault.stage(id)?;
        let mut output = new_file(&stage)?;
        let mut last = Instant::now();
        self.progress(id, 0, entry.size_bytes, "downloading_official_image")?;
        official::download(&source, &mut output, cancel, |bytes| {
            if last.elapsed() >= Duration::from_millis(100) {
                self.progress(id, bytes, entry.size_bytes, "downloading_official_image")?;
                last = Instant::now();
            }
            Ok(())
        })?;
        output.flush()?;
        output.sync_all()?;
        drop(output);
        self.progress(id, 0, entry.size_bytes, "verifying_official_hash")?;
        let (file, hash, size) = hash_file(&stage, entry.size_bytes, cancel, |_| Ok(()))?;
        drop(file);
        if hash != entry.sha256 || size != entry.size_bytes {
            return Err(LibraryError::new("OFFICIAL_HASH_MISMATCH","Downloaded bytes disagree with the signed-release catalogue. Nothing admitted to Library."));
        }
        let (blob, _guards) = self.vault.blob(&hash)?;
        let fence = lock(&self.finalize)?;
        cancelled(cancel)?;
        let duplicate = match fs::hard_link(&stage, &blob) {
            Ok(()) => false,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => true,
            Err(e) => return Err(e.into()),
        };
        drop(fence);
        let proof = self
            .vault
            .verify_published(&hash, self.max, cancel, |_| Ok(()))?;
        source.verify(&proof)?;
        let _fence = lock(&self.finalize)?;
        cancelled(cancel)?;
        lock(&self.repo)?.commit_official_download(id, &proof, duplicate, &source)?;
        Ok(())
    }
    fn mark_bad_duplicate(
        &self,
        hash: &str,
        size: u64,
        actual: Option<String>,
        actual_size: Option<u64>,
        error: &LibraryError,
    ) {
        if let Ok(mut repo) = self.repo.lock() {
            let status = if error.code == "FILE_MISSING" {
                "missing"
            } else if error.code == "ACCESS_DENIED" {
                "unreadable"
            } else if error.code == "HASH_MISMATCH" {
                "hash_mismatch"
            } else {
                "verification_failed"
            };
            let _ = repo.set_library_integrity(
                None,
                &IntegrityResult {
                    artifact_sha256: hash.into(),
                    expected_size: size,
                    actual_size,
                    actual_sha256: actual,
                    status: status.into(),
                    verified_at: now(),
                    error_code: Some(error.code.clone()),
                },
            );
        }
    }
    fn run_verify(&self, id: &str, hash: &str, cancel: &AtomicBool) -> Result<()> {
        let details = lock(&self.repo)?.library_details(hash)?;
        let (path, _guards) = self.vault.blob(hash)?;
        lock(&self.repo)?.library_mark(hash, "checking", "present")?;
        let size = details.artifact.size_bytes;
        let mut last = Instant::now();
        let checked = hash_file(&path, size.max(self.max), cancel, |bytes| {
            if last.elapsed() >= Duration::from_millis(100) {
                self.progress(id, bytes, size, "verifying")?;
                last = Instant::now();
            }
            Ok(())
        });
        let (lease, actual_hash, actual_size, status, error) = match checked {
            Ok((file, actual, n)) => {
                let valid = actual == hash && n == size;
                (
                    Some(file),
                    Some(actual),
                    Some(n),
                    if valid { "verified" } else { "hash_mismatch" },
                    if valid {
                        None
                    } else {
                        Some(LibraryError::new("HASH_MISMATCH","Stored blob no longer matches its expected hash or size. Content was preserved."))
                    },
                )
            }
            Err(e) if e.code == "CANCELLED" => return Err(e),
            Err(e) => {
                let status = match e.code.as_str() {
                    "FILE_MISSING" => "missing",
                    "ACCESS_DENIED" => "unreadable",
                    _ => "verification_failed",
                };
                (None, None, None, status, Some(e))
            }
        };
        let _fence = lock(&self.finalize)?;
        cancelled(cancel)?;
        let mut repo = lock(&self.repo)?;
        repo.set_library_integrity(
            Some(id),
            &IntegrityResult {
                artifact_sha256: hash.into(),
                expected_size: size,
                actual_size,
                actual_sha256: actual_hash,
                status: status.into(),
                verified_at: now(),
                error_code: error.as_ref().map(|e| e.code.clone()),
            },
        )?;
        if let Some(e) = error {
            return Err(e);
        }
        repo.library_progress(id, size, size, "verified")?;
        repo.library_transition(id, JobStatus::Completed, None)?;
        drop(lease);
        Ok(())
    }
    fn recover(&self) -> Result<()> {
        let pending: Vec<String> = {
            let repo = lock(&self.repo)?;
            let mut q=repo.connection.prepare("SELECT id FROM jobs WHERE kind IN ('firmware_import','integrity_verify','firmware_backup','backup_restore','library_relocation','official_image_download','firmware_analysis') AND status IN ('pending','running')")?;
            let rows = q.query_map([], |r| r.get(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        for id in pending {
            let cleanup = self.vault.remove_stage(&id);
            let error=LibraryError::new("INTERRUPTED","Operation interrupted before completion. Any published blob is retained for verified reuse; select the source again to retry.");
            let mut repo = lock(&self.repo)?;
            repo.library_transition(&id, JobStatus::Failed, Some(&error))?;
            if cleanup.is_err() {
                repo.connection.execute("UPDATE job_details SET recovery_info='Unexpected staging item retained for manual review.' WHERE job_id=?1",[&id])?;
            }
        }
        // An inaccessible volume says nothing about the integrity of individual
        // blobs. Keep their recorded state and all staging bytes for reconnection.
        if self.vault.unavailable {
            return Ok(());
        }
        // Reconcile in bounded pages; never reuse last session's verified label as current proof.
        let mut offset = 0;
        loop {
            let rows: Vec<(String, u64)> = {
                let repo = lock(&self.repo)?;
                let mut q=repo.connection.prepare("SELECT sha256,size_bytes FROM firmware_artifacts ORDER BY sha256 LIMIT 128 OFFSET ?1")?;
                let rows = q.query_map([offset], |r| Ok((r.get(0)?, r.get(1)?)))?;
                rows.collect::<rusqlite::Result<Vec<_>>>()?
            };
            if rows.is_empty() {
                break;
            }
            for (hash, size) in &rows {
                if !valid_hash(hash) {
                    continue;
                }
                let (p, _guards) = self.vault.blob(hash)?;
                let (integrity, storage) = match read_file(&p) {
                    Ok(f) if f.metadata()?.len() == *size => ("not_checked", "present"),
                    Ok(_) => ("hash_mismatch", "present"),
                    Err(e) if e.code == "FILE_MISSING" => ("missing", "missing"),
                    Err(_) => ("unreadable", "unreadable"),
                };
                lock(&self.repo)?.library_mark(hash, integrity, storage)?;
            }
            offset += rows.len() as i64;
        }
        let retained = fs::read_dir(&self.vault.staging)?
            .filter_map(|e| e.ok())
            .any(|e| e.path().file_name().is_some());
        if retained {
            let repo = lock(&self.repo)?;
            repo.connection.execute("INSERT INTO audit_events(kind,message) SELECT 'staging_review_required','Unrecognized or retained staging items require manual review; no unknown file was deleted.' WHERE NOT EXISTS(SELECT 1 FROM audit_events WHERE kind='staging_review_required')",[])?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests;
