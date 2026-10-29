//! Isolated file extraction. Output bytes never receive firmware admission.
use super::*;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io::{Read, Seek, SeekFrom};
use std::process::{Child, Command, Stdio};

const MAX_INPUT: u64 = 512 * 1024 * 1024;
const MAX_OUTPUT: u64 = 128 * 1024 * 1024;
const MAX_MANIFEST: u64 = 8 * 1024 * 1024;
const WORKER: &str = include_str!("../../../../../scripts/firmware-extract-worker.py");
const ENGINE: &str = include_str!("../../../../../scripts/extraction-engine-v2.py");
const PYTHON: &str = "/usr/bin/python3.14";
mod layers;
pub mod structure;
use layers::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    pub path: String,
    pub kind: String,
    pub size: u64,
    pub sha256: Option<String>,
    pub link_target: Option<String>,
    #[serde(default)]
    pub offset: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub export_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Region {
    pub format: String,
    pub offset: u64,
    pub length: u64,
    pub compression: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ByteRegion {
    pub offset: u64,
    pub length: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContainerInfo {
    pub format: String,
    pub version: u32,
    pub flags: u32,
    pub length: u64,
    pub partition_offsets: Vec<u64>,
    pub crc32: String,
    pub parser: String,
    pub source_commit: String,
    pub source_sha256: String,
    pub parser_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtractionReport {
    pub unextracted_regions: Vec<ByteRegion>,
    pub version: u32,
    pub input_sha256: String,
    pub input_bytes: u64,
    pub region: Region,
    pub container: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_info: Option<ContainerInfo>,
    pub expanded_bytes: u64,
    pub extractor: String,
    pub tool_sha256: String,
    pub entries: Vec<Entry>,
    pub total_bytes: u64,
    pub file_count: u64,
    pub directory_count: u64,
    pub filesystem_roots: Vec<String>,
    pub worker_exit: i32,
    pub status: String,
    pub limitations: Vec<String>,
    pub isolation: serde_json::Value,
    #[serde(default)]
    pub bundle_sha256: String,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completeness: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layers: Option<Vec<Layer>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unexplained_regions: Option<Vec<Unexplained>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entropy_map: Option<Vec<Entropy>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scan: Option<Scan>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budgets: Option<Budgets>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swupdate: Option<Vec<Swupdate>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub omitted_members: Option<Vec<OmittedMember>>,
}
fn invalid() -> LibraryError {
    LibraryError::new(
        "INVALID_EXTRACTION",
        "Extraction output failed independent validation. Original firmware retained.",
    )
}
struct WorkerProcess {
    child: Child,
    cancel: PathBuf,
}
impl Drop for WorkerProcess {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(Some(_))) {
            return;
        }
        let _ = fs::write(&self.cancel, b"cancel");
        for _ in 0..100 {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        // The Linux supervisor also revokes a lost heartbeat within ten seconds.
        // Its finally block kills/reaps the namespace process group.
    }
}
fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 240
        && name.split('/').count() <= 32
        && !name
            .chars()
            .any(|c| c.is_control() || matches!(c,'\u{061c}'|'\u{200e}'|'\u{200f}'|'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}') || "\\:*?\"<>|".contains(c))
        && name.split('/').all(|p| {
            let base = p.split('.').next().unwrap_or("").to_ascii_lowercase();
            !p.is_empty()
                && p != "."
                && p != ".."
                && !p.ends_with([' ', '.'])
                && !["con", "prn", "aux", "nul"].contains(&base.as_str())
                && !(base.len() == 4
                    && (base.starts_with("com") || base.starts_with("lpt"))
                    && matches!(base.as_bytes()[3], b'1'..=b'9'))
        })
}
fn validate_bundle(
    file: &mut File,
    hash: &str,
    size: u64,
    cancel: &AtomicBool,
) -> Result<ExtractionReport> {
    file.rewind()?;
    let mut header = [0; 10];
    file.read_exact(&mut header)?;
    let length = u32::from_le_bytes(header[6..10].try_into().unwrap()) as u64;
    if &header[..6] != b"OBTX1\n" || length == 0 || length > MAX_MANIFEST {
        return Err(invalid());
    }
    let mut raw = vec![0; length as usize];
    file.read_exact(&mut raw)?;
    let mut report: ExtractionReport = serde_json::from_slice(&raw).map_err(|_| invalid())?;
    if report.version == 2 {
        layers::validate(&report, hash, size)?;
    } else {
        if report.version != 1
            || report.input_sha256 != hash
            || report.input_bytes != size
            || report.worker_exit != 0
            || report.region.format != "squashfs"
            || report.region.length <= 96
            || report
                .region
                .offset
                .checked_add(report.region.length)
                .is_none_or(|end| end > report.expanded_bytes)
            || !(1..=6).contains(&report.region.compression)
            || !matches!(report.container.as_str(), "raw" | "gzip" | "trx")
            || (report.container != "gzip" && report.expanded_bytes != size)
            || (report.container == "gzip" && report.expanded_bytes > MAX_OUTPUT)
            || report.extractor != "unsquashfs 4.7.5"
            || report.tool_sha256
                != "003a7e9d70fbdcd05dfe132101c4b7bbb19a217976777f5393573eff21ddc202"
            || !matches!(
                report.status.as_str(),
                "extracted_verified" | "partially_extracted"
            )
            || report.entries.len() > 20000
            || report.total_bytes > MAX_OUTPUT
        {
            return Err(invalid());
        }
        if report.completeness.is_some()
            || report.layers.is_some()
            || report.provenance.is_some()
            || report.budgets.is_some()
            || report.scan.is_some()
            || report.entropy_map.is_some()
            || report.unexplained_regions.is_some()
            || report.omitted_members.is_some()
            || report.swupdate.is_some()
        {
            return Err(invalid());
        }
        if report.container == "trx" {
            let info = report.container_info.as_ref().ok_or_else(invalid)?;
            if info.format != "trx"
                || info.version != 1
                || info.flags != 0
                || info.length <= 28
                || info.length > size
                || info.partition_offsets.is_empty()
                || info.partition_offsets.len() > 3
                || info
                    .partition_offsets
                    .iter()
                    .any(|offset| *offset < 28 || *offset >= info.length)
                || info.partition_offsets.windows(2).any(|w| w[0] >= w[1])
                || info.partition_offsets.last() != Some(&report.region.offset)
                || report.region.offset + report.region.length > info.length
                || info.crc32.len() != 8
                || !info.crc32.bytes().all(|b| b.is_ascii_hexdigit())
                || info.parser != "OpenWrt otrx"
                || info.source_commit != "e3853149fb18dfa55f10c10c9aaf99c329747365"
                || info.source_sha256
                    != "e2ef4e7a08f84543945fec83c43bdebfdb1c252da1c440ebb90454913ec2dc3e"
                || info.parser_sha256
                    != "66b870899eca6a12079668c23cdd95db069978afc9af5bcbc3681cd9c1e968e5"
            {
                return Err(invalid());
            }
        } else if report.container_info.is_some() {
            return Err(invalid());
        }
        let end = report.region.offset + report.region.length;
        let mut omitted = vec![];
        if report.region.offset > 0 {
            omitted.push(ByteRegion {
                offset: 0,
                length: report.region.offset,
            });
        }
        if end < report.expanded_bytes {
            omitted.push(ByteRegion {
                offset: end,
                length: report.expanded_bytes - end,
            });
        }
        let partial = !omitted.is_empty()
            || report
                .entries
                .iter()
                .any(|e| matches!(e.kind.as_str(), "symlink_metadata" | "special_omitted"));
        if report.unextracted_regions != omitted
            || (report.status == "partially_extracted") != partial
            || report.isolation != expected_isolation()
        {
            return Err(invalid());
        }
    }
    let mut names = HashSet::new();
    let mut directories = HashSet::new();
    let mut total = 0u64;
    let mut count = 0;
    for entry in &mut report.entries {
        cancelled(cancel)?;
        entry.export_error = if entry.kind == "file" {
            tar_parts(&entry.path).err().map(|e| e.message)
        } else {
            None
        };
        if !safe_name(&entry.path) || !names.insert(entry.path.to_lowercase()) {
            return Err(invalid());
        }
        if (report.version == 2 && entry.mode.is_none())
            || entry.mode.is_some_and(|mode| mode > 0o777)
        {
            return Err(invalid());
        }
        match entry.kind.as_str() {
            "file" => {
                if entry.size > 32 * 1024 * 1024 || entry.link_target.is_some() {
                    return Err(invalid());
                }
                total = total.checked_add(entry.size).ok_or_else(invalid)?;
                if total > MAX_OUTPUT {
                    return Err(invalid());
                }
                entry.offset = file.stream_position()?;
                let mut remaining = entry.size;
                let mut digest = Sha256::new();
                let mut buffer = [0; 65536];
                while remaining > 0 {
                    cancelled(cancel)?;
                    let n = (remaining as usize).min(buffer.len());
                    file.read_exact(&mut buffer[..n])?;
                    digest.update(&buffer[..n]);
                    remaining -= n as u64;
                }
                if entry.sha256.as_deref() != Some(format!("{:x}", digest.finalize()).as_str()) {
                    return Err(invalid());
                }
                count += 1;
            }
            "directory" | "symlink_metadata" | "special_omitted" => {
                if entry.size != 0 || entry.sha256.is_some() {
                    return Err(invalid());
                }
                if entry.kind == "symlink_metadata" {
                    if entry.link_target.as_ref().is_none_or(|target| {
                        target.len() > 1024 || target.chars().any(char::is_control)
                    }) {
                        return Err(invalid());
                    }
                } else if entry.link_target.is_some() {
                    return Err(invalid());
                }
                entry.offset = 0;
                if entry.kind == "directory" {
                    directories.insert(entry.path.clone());
                }
            }
            _ => return Err(invalid()),
        }
    }
    for entry in &report.entries {
        if let Some((parent, _)) = entry.path.rsplit_once('/') {
            if !directories.contains(parent) {
                return Err(invalid());
            }
        }
    }
    if (report.version == 1 && count == 0)
        || count != report.file_count
        || directories.len() as u64 != report.directory_count
        || report.filesystem_roots != ["/"]
        || total != report.total_bytes
        || file.stream_position()? != file.metadata()?.len()
    {
        return Err(invalid());
    }
    report.created_at = if report.version == 2 { 0 } else { now() };
    Ok(report)
}
fn expected_isolation() -> serde_json::Value {
    serde_json::json!({"network":false,"hostDrives":false,"capabilities":"0000000000000000",
        "tmpfsBytes":268435456,"spaceReserveBytes":33554432,"rootReadOnly":true,"maxBytes":MAX_OUTPUT,"maxEntries":20000,"maxDepth":32})
}
fn require_publishable_job(tx: &rusqlite::Transaction<'_>, id: &str, hash: &str) -> Result<()> {
    let allowed: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM jobs j JOIN job_details d ON d.job_id=j.id WHERE j.id=?1 AND j.kind='firmware_extraction' AND j.status='running' AND d.cancel_requested=0 AND d.artifact_sha256=?2)", params![id,hash], |r| r.get(0))?;
    if !allowed {
        return Err(invalid());
    }
    Ok(())
}

impl LibraryService {
    pub fn extract(&self, hash: &str) -> Result<JobRecord> {
        self.ensure_open()?;
        let artifact = self.details(hash)?.artifact;
        if artifact.size_bytes == 0 || artifact.size_bytes > MAX_INPUT {
            return Err(LibraryError::new(
                "EXTRACTION_LIMIT",
                "Extraction supports images up to 512 MiB.",
            ));
        }
        let mut controls = lock(&self.inner.controls)?;
        self.ensure_open()?;
        if let Some(existing) = lock(&self.inner.repo)?
            .library_jobs()?
            .into_iter()
            .find(|j| {
                j.kind == "firmware_extraction"
                    && j.related_artifact.as_deref() == Some(hash)
                    && !j.status.terminal()
            })
        {
            return Ok(existing);
        }
        if controls.len() >= 16 {
            return Err(LibraryError::new("QUEUE_FULL", "Wait for an existing job."));
        }
        let id = nonce()?;
        lock(&self.inner.repo)?.create_library_job(
            &id,
            "firmware_extraction",
            artifact.size_bytes,
            JobInput {
                hash: Some(hash),
                filename: Some(&artifact.original_filename),
                ..Default::default()
            },
        )?;
        controls.insert(id.clone(), Arc::new(AtomicBool::new(false)));
        if self
            .inner
            .sender
            .try_send(Task::Extract {
                job: id.clone(),
                hash: hash.into(),
            })
            .is_err()
        {
            controls.remove(&id);
            lock(&self.inner.repo)?.library_transition(
                &id,
                JobStatus::Failed,
                Some(&LibraryError::new("QUEUE_FULL", "Extraction not queued.")),
            )?;
        }
        lock(&self.inner.repo)?.library_job(&id)
    }
    pub fn extraction_report(&self, hash: &str) -> Result<Option<ExtractionReport>> {
        self.ensure_open()?;
        self.details(hash)?;
        let report_hash:Option<String> = lock(&self.inner.repo)?.connection.query_row("SELECT blob_sha256 FROM artifact_occurrences WHERE provenance_kind='isolated_filesystem_extraction' AND artifact_class='metadata_report' AND source_reference=?1 ORDER BY collected_at DESC,occurrence_id DESC LIMIT 1",[format!("extraction:{hash}")], |r| r.get(0)).optional()?;
        let Some(report_hash) = report_hash else {
            return Ok(None);
        };
        let proof = self.inner.vault.verify_published(
            &report_hash,
            MAX_MANIFEST,
            &AtomicBool::new(false),
            |_| Ok(()),
        )?;
        let mut report: ExtractionReport =
            serde_json::from_reader(proof.reader()?).map_err(|_| invalid())?;
        if report.input_sha256 != hash
            || !matches!(report.version, 1 | 2)
            || !valid_hash(&report.bundle_sha256)
        {
            return Err(invalid());
        }
        // Derive eligibility for older immutable reports without rewriting them.
        for entry in &mut report.entries {
            entry.export_error = if entry.kind == "file" {
                tar_parts(&entry.path).err().map(|e| e.message)
            } else {
                None
            };
        }
        Ok(Some(report))
    }
    pub fn extraction_text(&self, hash: &str, path: &str) -> Result<String> {
        let report = self.extraction_report(hash)?.ok_or_else(invalid)?;
        let entry = report
            .entries
            .iter()
            .find(|e| e.path == path && e.kind == "file" && e.size <= 65536)
            .ok_or_else(|| {
                LibraryError::new(
                    "NOT_TEXT",
                    "Select an ordinary text file no larger than 64 KiB.",
                )
            })?;
        let proof = self.inner.vault.verify_published(
            &report.bundle_sha256,
            MAX_OUTPUT + MAX_MANIFEST + 10,
            &AtomicBool::new(false),
            |_| Ok(()),
        )?;
        let mut file = proof.reader()?;
        file.seek(SeekFrom::Start(entry.offset))?;
        let mut raw = vec![0; entry.size as usize];
        file.read_exact(&mut raw)?;
        if entry.sha256.as_deref() != Some(format!("{:x}", Sha256::digest(&raw)).as_str()) {
            return Err(invalid());
        }
        let text = String::from_utf8(raw)
            .map_err(|_| LibraryError::new("NOT_TEXT", "Binary file; text preview unavailable."))?;
        if text
            .chars()
            .any(|c| c.is_control() && !"\n\r\t".contains(c))
        {
            return Err(LibraryError::new(
                "NOT_TEXT",
                "Binary file; text preview unavailable.",
            ));
        }
        Ok(text)
    }
    pub fn export_extraction(&self, hash: &str, paths: &[String], owner: isize) -> Result<bool> {
        let report = self.extraction_report(hash)?.ok_or_else(invalid)?;
        validate_export_paths(&report, paths)?;
        if self.inner.dialog_open.swap(true, Ordering::AcqRel) {
            return Err(LibraryError::new(
                "DIALOG_BUSY",
                "Finish the current dialog.",
            ));
        }
        let result = (|| {
            let Some(path) = extraction_dialog(owner)? else {
                return Ok(false);
            };
            self.export_extraction_path(hash, paths, &path)?;
            Ok(true)
        })();
        self.inner.dialog_open.store(false, Ordering::Release);
        result
    }
    pub(crate) fn export_extraction_path(
        &self,
        hash: &str,
        paths: &[String],
        destination: &Path,
    ) -> Result<()> {
        self.ensure_open()?;
        safe_path(destination)?;
        if inside(destination, &self.inner.root)? || inside(destination, &self.inner.vault.root)? {
            return Err(LibraryError::new(
                "INTERNAL_DESTINATION",
                "Export outside application storage.",
            ));
        }
        let _guard = guard_directories(destination.parent().ok_or_else(invalid)?)?;
        let report = self.extraction_report(hash)?.ok_or_else(invalid)?;
        let entries = validate_export_paths(&report, paths)?;
        let proof = self.inner.vault.verify_published(
            &report.bundle_sha256,
            MAX_OUTPUT + MAX_MANIFEST + 10,
            &AtomicBool::new(false),
            |_| Ok(()),
        )?;
        let mut input = proof.reader()?;
        let tmp = destination
            .parent()
            .ok_or_else(invalid)?
            .join(format!(".obtainer-export-{}.tmp", nonce()?));
        let result = (|| {
            let mut out = new_file(&tmp)?;
            for entry in entries {
                write_tar_header(&mut out, &entry.path, entry.size)?;
                input.seek(SeekFrom::Start(entry.offset))?;
                let copied = std::io::copy(
                    &mut std::io::Read::by_ref(&mut input).take(entry.size),
                    &mut out,
                )?;
                if copied != entry.size {
                    return Err(invalid());
                }
                let padding = (512 - entry.size % 512) % 512;
                out.write_all(&vec![0; padding as usize])?;
            }
            out.write_all(&[0; 1024])?;
            out.sync_all()?;
            drop(out);
            fs::hard_link(&tmp, destination)?;
            Ok(())
        })();
        let cleanup = fs::remove_file(&tmp);
        result?;
        cleanup?;
        Ok(())
    }
}
fn validate_export_paths<'a>(
    report: &'a ExtractionReport,
    paths: &[String],
) -> Result<Vec<&'a Entry>> {
    if paths.is_empty() || paths.len() > 20000 {
        return Err(LibraryError::new(
            "SELECT_FILES",
            "Select ordinary files for export; nothing is selected automatically.",
        ));
    }
    let mut seen = HashSet::new();
    let mut entries = vec![];
    for path in paths {
        if !seen.insert(path) {
            return Err(invalid());
        }
        let entry = report
            .entries
            .iter()
            .find(|e| e.path == *path && e.kind == "file")
            .ok_or_else(invalid)?;
        tar_parts(path)?;
        entries.push(entry);
    }
    Ok(entries)
}
fn tar_parts(path: &str) -> Result<(&str, &str)> {
    let error = || {
        LibraryError::new("UNEXPORTABLE_FILENAME",&format!("Cannot export {:?}: USTAR requires a safe ASCII name of at most 100 bytes, or a prefix of at most 155 bytes and name of at most 100 bytes. Original filename is retained.",path.escape_debug().to_string()))
    };
    if !safe_name(path) || !path.is_ascii() {
        return Err(error());
    }
    if path.len() <= 100 {
        return Ok(("", path));
    }
    for (index, _) in path.rmatch_indices('/') {
        let (prefix, name) = (&path[..index], &path[index + 1..]);
        if prefix.len() <= 155 && name.len() <= 100 {
            return Ok((prefix, name));
        }
    }
    Err(error())
}
fn write_tar_header(out: &mut impl Write, path: &str, size: u64) -> Result<()> {
    let (prefix, name) = tar_parts(path)?;
    let mut h = [0u8; 512];
    h[..name.len()].copy_from_slice(name.as_bytes());
    h[345..345 + prefix.len()].copy_from_slice(prefix.as_bytes());
    h[100..108].copy_from_slice(b"0000600\0");
    h[108..116].copy_from_slice(b"0000000\0");
    h[116..124].copy_from_slice(b"0000000\0");
    h[124..136].copy_from_slice(format!("{size:011o}\0").as_bytes());
    h[136..148].copy_from_slice(b"00000000000\0");
    h[148..156].fill(b' ');
    h[156] = b'0';
    h[257..263].copy_from_slice(b"ustar\0");
    h[263..265].copy_from_slice(b"00");
    let checksum: u32 = h.iter().map(|v| *v as u32).sum();
    h[148..156].copy_from_slice(format!("{checksum:06o}\0 ").as_bytes());
    out.write_all(&h)?;
    Ok(())
}

impl Inner {
    pub(super) fn run_extraction(&self, id: &str, hash: &str, cancel: &AtomicBool) -> Result<()> {
        let artifact = lock(&self.repo)?.library_details(hash)?.artifact;
        self.progress(id, 0, artifact.size_bytes, "verifying_extraction_input")?;
        let input = self
            .vault
            .verify_published(hash, MAX_INPUT, cancel, |_| Ok(()))?;
        if input.size() != artifact.size_bytes {
            return Err(invalid());
        }
        free_space(
            &self.vault.root,
            input.size() + 2 * (MAX_OUTPUT + MAX_MANIFEST),
        )?;
        let parent = self.vault.staging.clone();
        let _parent_guard = guard_directories(&parent)?;
        let work = parent.join(format!("extract-{id}"));
        fs::create_dir(&work)?;
        let work_guard = guard_directories(&work)?;
        let result = (|| {
            let mut copy = new_file(&work.join("input.bin"))?;
            std::io::copy(&mut input.reader()?, &mut copy)?;
            copy.sync_all()?;
            drop(copy);
            let mut worker = new_file(&work.join("worker.py"))?;
            worker.write_all(WORKER.as_bytes())?;
            worker.sync_all()?;
            drop(worker);
            let mut engine = new_file(&work.join("extraction-engine-v2.py"))?;
            engine.write_all(ENGINE.as_bytes())?;
            engine.sync_all()?;
            drop(engine);
            fs::write(work.join("heartbeat"), b"alive")?;
            self.progress(id, 0, input.size(), "extracting")?;
            run_worker(&work, hash, input.size(), cancel)?;
            cancelled(cancel)?;
            self.progress(id, 0, input.size(), "validating_extracted_files")?;
            let mut output = read_file(&work.join("output.bin"))?;
            if output.metadata()?.len() > MAX_OUTPUT + MAX_MANIFEST + 10 {
                return Err(invalid());
            }
            let mut report = validate_bundle(&mut output, hash, input.size(), cancel)?;
            let (_, bundle_hash, bundle_size) = hash_file(
                &work.join("output.bin"),
                MAX_OUTPUT + MAX_MANIFEST + 10,
                cancel,
                |_| Ok(()),
            )?;
            report.bundle_sha256 = bundle_hash.clone();
            let raw = serde_json::to_vec(&report).map_err(|_| invalid())?;
            if raw.len() as u64 > MAX_MANIFEST {
                return Err(invalid());
            }
            let stage = self.vault.stage(id)?;
            let mut file = new_file(&stage)?;
            file.write_all(&raw)?;
            file.sync_all()?;
            drop(file);
            let (_, report_hash, _) = hash_file(&stage, MAX_MANIFEST, cancel, |_| Ok(()))?;
            // Input lease is still alive; verify again before transaction/publication.
            self.vault
                .verify_published(hash, MAX_INPUT, cancel, |_| Ok(()))?;
            let _fence = lock(&self.finalize)?;
            cancelled(cancel)?;
            let (bundle_path, _bundle_guards) = self.vault.blob(&bundle_hash)?;
            let (report_path, _report_guards) = self.vault.blob(&report_hash)?;
            for (source, dest) in [
                (&work.join("output.bin"), &bundle_path),
                (&stage, &report_path),
            ] {
                match fs::hard_link(source, dest) {
                    Ok(()) => (),
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
                    Err(e) => return Err(e.into()),
                }
            }
            let bundle = self.vault.verify_published(
                &bundle_hash,
                MAX_OUTPUT + MAX_MANIFEST + 10,
                cancel,
                |_| Ok(()),
            )?;
            let manifest =
                self.vault
                    .verify_published(&report_hash, MAX_MANIFEST, cancel, |_| Ok(()))?;
            if bundle.size() != bundle_size {
                return Err(invalid());
            }
            let mut repo = lock(&self.repo)?;
            let tx = repo.connection.transaction()?;
            require_publishable_job(&tx, id, hash)?;
            for proof in [&bundle, &manifest] {
                let size: Option<u64> = tx
                    .query_row(
                        "SELECT size_bytes FROM artifact_blobs WHERE sha256=?1",
                        [proof.hash()],
                        |r| r.get(0),
                    )
                    .optional()?;
                match size {
                    None => {
                        tx.execute(
                            "INSERT INTO artifact_blobs VALUES(?1,?2)",
                            params![proof.hash(), proof.size()],
                        )?;
                    }
                    Some(s) if s != proof.size() => return Err(invalid()),
                    _ => (),
                }
            }
            tx.execute("INSERT INTO artifact_occurrences(occurrence_id,blob_sha256,artifact_class,provenance_kind,source_reference,acquisition_occurrence_id,job_id,collected_at) VALUES(?1,?2,'metadata_report','isolated_filesystem_extraction',?3,?1,?4,?5)",params![format!("extraction:{id}"),manifest.hash(),format!("extraction:{hash}"),id,now()])?;
            tx.execute(
                "UPDATE jobs SET status='completed' WHERE id=?1 AND status='running'",
                [id],
            )?;
            let message = if report.file_count == 0 {
                "Inspection completed; no ordinary files extracted. See unsupported layers and unexplained regions."
            } else if report.completeness.as_deref() == Some("complete") {
                "Container contents extracted and independently hashed. No device acquisition or installation authority."
            } else {
                "Partial extraction retained. See layer limitations, omitted members and unexplained regions."
            };
            tx.execute("UPDATE job_details SET finished_at=?2,updated_at=?2,progress_bytes=total_bytes,stage='completed',message=?3 WHERE job_id=?1",params![id,now(),message])?;
            db::event(
                &tx,
                if report.file_count == 0 {
                    "firmware_inspected"
                } else {
                    "firmware_extracted"
                },
                message,
                Some(id),
                Some(hash),
                "info",
            )?;
            tx.commit()?;
            Ok(())
        })();
        // Delete only fixed files in the freshly created owned directory, never recurse.
        let cleanup = cleanup_work(&work, work_guard);
        if cleanup.is_err() {
            let _=lock(&self.repo)?.connection.execute("UPDATE job_details SET recovery_info='Extraction staging cleanup incomplete; retained for review.' WHERE job_id=?1",[id]);
        }
        result
    }
}
fn cleanup_work(work: &Path, guards: Vec<File>) -> std::io::Result<()> {
    // Keep the Windows directory lease through every child deletion. Unknown
    // entries cause remove_dir to fail; never follow or recursively remove them.
    for name in [
        "input.bin",
        "worker.py",
        "extraction-engine-v2.py",
        "heartbeat",
        "cancel",
        "error.txt",
        "output.bin",
    ] {
        match fs::remove_file(work.join(name)) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e),
        }
    }
    drop(guards);
    fs::remove_dir(work)
}
fn worker_command(work: &Path) -> Result<Command> {
    #[cfg(windows)]
    let command = {
        use std::os::windows::process::CommandExt;
        let path = work.to_str().ok_or_else(invalid)?.replace('\\', "/");
        if path.len() < 3 || path.as_bytes()[1] != b':' {
            return Err(invalid());
        }
        let guest = format!("/mnt/{}/{}", path[..1].to_ascii_lowercase(), &path[3..]);
        let mut cmd = Command::new("C:\\Windows\\System32\\wsl.exe");
        cmd.creation_flags(0x08000000).args([
            "-d",
            "Ubuntu",
            "--exec",
            PYTHON,
            "-I",
            "-B",
            &format!("{guest}/worker.py"),
            "--job",
            &guest,
        ]);
        cmd
    };
    #[cfg(not(windows))]
    let command = {
        let mut cmd = Command::new(PYTHON);
        cmd.args(["-I", "-B"])
            .arg(work.join("worker.py"))
            .arg("--job")
            .arg(work);
        cmd
    };
    Ok(command)
}
fn run_worker(work: &Path, hash: &str, size: u64, cancel: &AtomicBool) -> Result<()> {
    let child = worker_command(work)?.args(["--sha256",hash,"--size",&size.to_string()]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|_|LibraryError::new("WORKER_UNAVAILABLE","Install the qualified WSL Ubuntu extraction tools. No unsandboxed fallback is permitted."))?;
    let mut process = WorkerProcess {
        child,
        cancel: work.join("cancel"),
    };
    let child = &mut process.child;
    let deadline = Instant::now() + Duration::from_secs(165);
    let mut stopping = None;
    loop {
        if let Some(status) = child.try_wait()? {
            cancelled(cancel)?;
            if status.success() {
                return Ok(());
            }
            let code = fs::read_to_string(work.join("error.txt")).unwrap_or_default();
            let (code, message) = match code.as_str() {
                "" | "worker_unavailable" => ("WORKER_UNAVAILABLE", "Extraction needs WSL Ubuntu with qualified extraction tools: squashfs-tools 4.7.5 and Ubuntu bubblewrap 0.11.1-1ubuntu0.3. The worker could not start, or a required tool is missing, incompatible or fails its integrity check. Update/install compatible tools; your firmware stays preserved."),
                "resource_exhausted" => ("EXTRACTION_RESOURCES", "Extraction exceeded its combined temporary-space, memory or process resource budget. No new extraction was published; original firmware and existing results remain preserved."),
                "trx_worker_unavailable" => ("WORKER_UNAVAILABLE", "TRX extraction needs the separately qualified OpenWrt otrx parser. Install the pinned parser from the worker setup guide; no unsandboxed fallback is used."),
                "unsupported_container_version" => ("UNSUPPORTED_CONTAINER_VERSION", "This container version or its flags are not qualified for extraction."),
                "unsupported_nested_format" => ("UNSUPPORTED_NESTED_FORMAT", "This nested filesystem or container variant is not qualified. No new extraction was published."),
                "unsupported_format" => ("UNSUPPORTED_FORMAT", "No supported filesystem found within the bounded scan. Unknown or encrypted content remains unextracted."),
                "unsupported_compression" => ("UNSUPPORTED_COMPRESSION", "This filesystem compression is unsupported."),
                "corrupt_image" => ("CORRUPT_IMAGE", "Filesystem headers are invalid, truncated or ambiguous."),
                "output_limit" | "entry_limit" | "description_limit" | "manifest_limit" | "decompression_bomb" => ("EXTRACTION_LIMIT", "Extraction exceeded a shared byte, expansion-ratio, layer, metadata or file-count limit. No new result was published."),
                "source_changed" => ("SOURCE_CHANGED", "Input verification failed; no extracted output admitted."),
                "unsafe_path" | "unsafe_link" | "unsafe_mode" | "unsafe_special_file" | "colliding_paths" => ("UNSAFE_EXTRACTION", "Unsafe names, links, special files, privileged modes or colliding paths were rejected."),
                "corrupt_description" | "ambiguous_image" => ("CORRUPT_IMAGE", "The package description or container mapping is invalid or ambiguous; nothing was inferred or applied."),
                "timeout" | "owner_disconnected" => ("EXTRACTION_TIMEOUT", "Extraction timed out or its supervising application disconnected."),
                _ => ("EXTRACTION_FAILED", "Isolated extraction failed. Original firmware retained."),
            };
            return Err(LibraryError::new(code, message));
        }
        if cancel.load(Ordering::Acquire) || Instant::now() > deadline {
            if stopping.is_none() {
                fs::write(work.join("cancel"), b"cancel")?;
                stopping = Some(Instant::now());
            }
            if stopping.is_some_and(|start| start.elapsed() > Duration::from_secs(12)) {
                let _ = child.kill();
                let _ = child.wait();
                cancelled(cancel)?;
                return Err(LibraryError::new(
                    "EXTRACTION_TIMEOUT",
                    "Extraction exceeded its time limit.",
                ));
            }
        } else {
            fs::write(work.join("heartbeat"), b"alive")?;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extraction_worker_command_uses_the_pinned_absolute_python_path() {
        #[cfg(windows)]
        let work = Path::new(r"C:\isolated-profile\job");
        #[cfg(not(windows))]
        let work = Path::new("/isolated-profile/job");
        let command = worker_command(work).unwrap();
        #[cfg(windows)]
        {
            let args: Vec<_> = command.get_args().collect();
            let exec = args.iter().position(|arg| *arg == "--exec").unwrap();
            assert_eq!(args[exec + 1], "/usr/bin/python3.14");
        }
        #[cfg(not(windows))]
        assert_eq!(command.get_program(), "/usr/bin/python3.14");
    }
    fn synthetic_report() -> serde_json::Value {
        serde_json::json!({"unextractedRegions":[],"version":1,"inputSha256":"a".repeat(64),"inputBytes":1024,"container":"raw","expandedBytes":1024,
            "region":{"format":"squashfs","offset":0,"length":1024,"compression":4},
            "extractor":"unsquashfs 4.7.5","toolSha256":"003a7e9d70fbdcd05dfe132101c4b7bbb19a217976777f5393573eff21ddc202",
            "entries":[{"path":"etc","kind":"directory","size":0,"sha256":null,"linkTarget":null},
                {"path":"etc/synthetic","kind":"file","size":4,"sha256":format!("{:x}",Sha256::digest(b"test")),"linkTarget":null}],
            "totalBytes":4,"fileCount":1,"directoryCount":1,"filesystemRoots":["/"],"workerExit":0,"status":"extracted_verified","limitations":[],"isolation":expected_isolation()})
    }
    fn synthetic_bundle(
        value: &serde_json::Value,
        bytes: &[u8],
        cancel: bool,
    ) -> Result<ExtractionReport> {
        let raw = serde_json::to_vec(value).unwrap();
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(b"OBTX1\n").unwrap();
        file.write_all(&(raw.len() as u32).to_le_bytes()).unwrap();
        file.write_all(&raw).unwrap();
        file.write_all(bytes).unwrap();
        validate_bundle(&mut file, &"a".repeat(64), 1024, &AtomicBool::new(cancel))
    }
    #[test]
    fn ordinary_bytes_are_independently_hashed_and_indexed() {
        let report = synthetic_bundle(&synthetic_report(), b"test", false).unwrap();
        assert_eq!(report.total_bytes, 4);
        assert!(report.entries[1].offset > 10);
    }
    fn synthetic_v2() -> serde_json::Value {
        let engine_hash = format!(
            "{:x}",
            Sha256::digest(
                include_str!("../../../../../scripts/extraction-engine-v2.py").as_bytes()
            )
        );
        let mut value = synthetic_report();
        value["version"] = 2.into();
        value["container"] = "cpio_newc".into();
        value["region"]["format"] = "cpio_newc".into();
        value["region"]["compression"] = 0.into();
        value["extractor"] = "OBTAINER layered engine 2".into();
        value["toolSha256"] = engine_hash.clone().into();
        value["entries"][0]["mode"] = 493.into();
        value["entries"][1]["mode"] = 420.into();
        value["completeness"] = "complete".into();
        value["layers"] = serde_json::json!([{"id":"layer-0000","parentId":null,"space":"source","offset":0,"length":1024,"sourcePath":"<root>","inputSha256":"a".repeat(64),"handler":"cpio","handlerVersion":1,"format":"cpio_newc","confidence":"validated","decodedBytes":0,"completeness":"complete","reasons":[]}]);
        value["unexplainedRegions"] = serde_json::json!([]);
        value["entropyMap"] = serde_json::json!([]);
        value["scan"] = serde_json::json!({"bytes":1024,"allOffsetsScanned":true,"candidateLimit":4096,"candidatesRetained":1,"candidatesCapped":false,"entropySampled":true});
        value["swupdate"] = serde_json::json!([]);
        value["omittedMembers"] = serde_json::json!([]);
        value["provenance"] = serde_json::json!({"engineVersion":"2.0.0","engineSha256":engine_hash,"pythonVersion":"3.14.4","tools":[]});
        value["budgets"] = serde_json::json!({"maxLayerDepth":6,"maxLayers":256,"expandedBytes":0,"maxExpandedBytes":MAX_OUTPUT,"maxFileBytes":33554432,"ratioLimit":1024});
        value
    }
    #[test]
    fn v2_layers_and_content_hash_are_validated_without_wall_clock_metadata() {
        let value = synthetic_v2();
        let first = synthetic_bundle(&value, b"test", false).unwrap();
        let second = synthetic_bundle(&value, b"test", false).unwrap();
        assert_eq!(first.created_at, 0);
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
    }
    #[test]
    fn v2_rejects_false_completeness_bounds_forged_provenance_modes_and_topology() {
        for change in 0..7 {
            let mut value = synthetic_v2();
            match change {
                0 => {
                    value["unexplainedRegions"] = serde_json::json!([{"layerId":"layer-0000","offset":0,"length":1,"reason":"unknown"}])
                }
                1 => value["layers"][0]["length"] = 1025.into(),
                2 => value["layers"][0]["parentId"] = "layer-0000".into(),
                3 => value["provenance"]["engineSha256"] = "b".repeat(64).into(),
                4 => value["entries"][1]["mode"] = 0o4755.into(),
                5 => value["budgets"]["maxExpandedBytes"] = (MAX_OUTPUT + 1).into(),
                _ => value["scan"]["allOffsetsScanned"] = false.into(),
            }
            assert!(
                synthetic_bundle(&value, b"test", false).is_err(),
                "mutation {change}"
            );
        }
    }
    #[test]
    fn v2_unknown_is_a_metadata_inspection_and_never_an_extracted_success() {
        let mut value = synthetic_v2();
        value["entries"] = serde_json::json!([]);
        value["totalBytes"] = 0.into();
        value["fileCount"] = 0.into();
        value["directoryCount"] = 0.into();
        value["status"] = "encrypted-or-unknown".into();
        value["completeness"] = "encrypted-or-unknown".into();
        value["layers"][0]["completeness"] = "encrypted-or-unknown".into();
        value["layers"][0]["reasons"] = serde_json::json!(["unrecognized"]);
        value["unexplainedRegions"] = serde_json::json!([{"layerId":"layer-0000","offset":0,"length":1024,"reason":"unrecognized"}]);
        value["unextractedRegions"] = serde_json::json!([{"offset":0,"length":1024}]);
        assert!(synthetic_bundle(&value, b"", false).is_ok());
        value["status"] = "extracted_verified".into();
        assert!(synthetic_bundle(&value, b"", false).is_err());
    }
    #[test]
    fn omitted_regions_and_isolation_cannot_be_misrepresented() {
        for (field, value) in [
            ("status", serde_json::json!("partially_extracted")),
            (
                "unextractedRegions",
                serde_json::json!([{"offset":0,"length":1}]),
            ),
            ("isolation", serde_json::json!({})),
            ("fileCount", serde_json::json!(2)),
        ] {
            let mut report = synthetic_report();
            report[field] = value;
            assert!(
                synthetic_bundle(&report, b"test", false).is_err(),
                "{field}"
            );
        }
        let mut report = synthetic_report();
        report["region"]["offset"] = 10.into();
        report["region"]["length"] = 1014.into();
        assert!(synthetic_bundle(&report, b"test", false).is_err());
        report["unextractedRegions"] = serde_json::json!([{"offset":0,"length":10}]);
        report["status"] = "partially_extracted".into();
        assert!(synthetic_bundle(&report, b"test", false).is_ok());
    }
    #[test]
    fn publication_requires_current_uncancelled_extraction_and_rolls_back() {
        let mut db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE jobs(id TEXT,kind TEXT,status TEXT);CREATE TABLE job_details(job_id TEXT,cancel_requested INTEGER,artifact_sha256 TEXT);CREATE TABLE admitted(value TEXT);INSERT INTO jobs VALUES('j','firmware_extraction','running');INSERT INTO job_details VALUES('j',0,'h');").unwrap();
        for mutation in [
            "UPDATE jobs SET status='failed'",
            "UPDATE job_details SET cancel_requested=1",
            "UPDATE jobs SET kind='firmware_analysis'",
            "UPDATE job_details SET artifact_sha256='other'",
        ] {
            let tx = db.transaction().unwrap();
            tx.execute_batch(mutation).unwrap();
            assert!(require_publishable_job(&tx, "j", "h").is_err());
        }
        {
            let tx = db.transaction().unwrap();
            require_publishable_job(&tx, "j", "h").unwrap();
            tx.execute("INSERT INTO admitted VALUES('bundle')", [])
                .unwrap();
        }
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM admitted", [], |r| r.get::<_, u64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn cleanup_removes_only_owned_known_files_and_retains_unexpected_entries() {
        let temp = tempfile::tempdir().unwrap();
        let work = temp.path().join("owned");
        fs::create_dir(&work).unwrap();
        fs::write(work.join("input.bin"), b"synthetic").unwrap();
        fs::write(work.join("unknown"), b"retain").unwrap();
        assert!(cleanup_work(&work, guard_directories(&work).unwrap()).is_err());
        assert_eq!(fs::read(work.join("unknown")).unwrap(), b"retain");
        assert!(!work.join("input.bin").exists());
    }
    #[cfg(windows)]
    #[test]
    fn locked_output_cleanup_failure_does_not_delete_or_rename_the_directory() {
        let temp = tempfile::tempdir().unwrap();
        let work = temp.path().join("owned");
        fs::create_dir(&work).unwrap();
        fs::write(work.join("output.bin"), b"retained output").unwrap();
        let guards = guard_directories(&work).unwrap();
        assert!(fs::rename(&work, temp.path().join("swapped")).is_err());
        let lease = read_file(&work.join("output.bin")).unwrap();
        assert!(cleanup_work(&work, guards).is_err());
        assert_eq!(
            fs::read(work.join("output.bin")).unwrap(),
            b"retained output"
        );
        drop(lease);
        cleanup_work(&work, guard_directories(&work).unwrap()).unwrap();
    }
    #[test]
    fn source_substitution_and_bad_region_are_refused() {
        for (field, value) in [
            ("inputSha256", serde_json::json!("b".repeat(64))),
            ("inputBytes", serde_json::json!(999)),
            ("expandedBytes", serde_json::json!(999)),
            ("workerExit", serde_json::json!(1)),
            ("extractor", serde_json::json!("unknown")),
        ] {
            let mut report = synthetic_report();
            report[field] = value;
            assert!(
                synthetic_bundle(&report, b"test", false).is_err(),
                "{field}"
            );
        }
    }
    #[test]
    fn corrupt_missing_and_trailing_output_are_refused() {
        for bytes in [b"evil".as_slice(), b"tes", b"testextra", b""] {
            assert!(synthetic_bundle(&synthetic_report(), bytes, false).is_err());
        }
    }
    #[test]
    fn duplicate_paths_traversal_and_missing_parents_are_refused() {
        for path in [
            "../outside",
            "/absolute",
            "missing/file",
            "ETC",
            "etc/CON",
            "etc/a:b",
        ] {
            let mut report = synthetic_report();
            report["entries"][1]["path"] = path.into();
            assert!(synthetic_bundle(&report, b"test", false).is_err(), "{path}");
        }
    }
    #[test]
    fn worker_cannot_smuggle_a_device_or_excessive_file_size() {
        let mut report = synthetic_report();
        report["entries"][1]["kind"] = "device".into();
        assert!(synthetic_bundle(&report, b"test", false).is_err());
        report = synthetic_report();
        report["entries"][1]["size"] = (MAX_OUTPUT + 1).into();
        assert!(synthetic_bundle(&report, b"test", false).is_err());
    }
    #[test]
    fn cancellation_refuses_even_an_otherwise_valid_result() {
        assert_eq!(
            synthetic_bundle(&synthetic_report(), b"test", true)
                .unwrap_err()
                .code,
            "CANCELLED"
        );
    }
    #[test]
    fn traversal_and_windows_special_names_are_rejected() {
        for name in [
            "../escape",
            "/etc/passwd",
            "a//b",
            "a\\b",
            "a:b",
            "a/CON",
            "a/nul.txt",
            "a/COM1",
            "a/..",
            "a. ",
            "a\nb",
        ] {
            assert!(!safe_name(name), "{name}");
        }
        assert!(safe_name("etc/openwrt_release"));
    }
    #[test]
    fn worker_output_without_real_files_is_not_success() {
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(b"OBTX1\n\x02\0\0\0{}").unwrap();
        assert!(validate_bundle(&mut file, &"a".repeat(64), 1, &AtomicBool::new(false)).is_err());
    }
    #[test]
    fn export_header_never_grants_executable_permissions_or_links() {
        let mut bytes = vec![];
        write_tar_header(&mut bytes, "etc/config/network", 10).unwrap();
        assert_eq!(&bytes[100..108], b"0000600\0");
        assert_eq!(bytes[156], b'0');
        assert_eq!(bytes.len(), 512);
        assert!(write_tar_header(&mut vec![], "../bad", 1).is_err());
    }
    #[test]
    fn ustar_rejects_unicode_bidi_and_overlong_individual_names() {
        for path in ["café", "cafe\u{301}", "etc/a\u{202e}txt", &"a".repeat(101)] {
            assert!(write_tar_header(&mut vec![], path, 1).is_err(), "{path:?}");
        }
    }
    #[test]
    fn ustar_can_split_at_an_earlier_separator() {
        let path = format!("{}/{}/{}", "a".repeat(130), "b".repeat(50), "c".repeat(40));
        assert!(write_tar_header(&mut vec![], &path, 1).is_ok());
    }
    #[test]
    fn export_preflight_identifies_bad_files_and_refuses_links() {
        let mut report: ExtractionReport = serde_json::from_value(synthetic_report()).unwrap();
        let path = "x".repeat(101);
        report.entries[1].path = path.clone();
        let error = validate_export_paths(&report, std::slice::from_ref(&path)).unwrap_err();
        assert_eq!(error.code, "UNEXPORTABLE_FILENAME");
        assert!(error.message.contains(&path));
        report.entries[1].path = "safe".into();
        report.entries[1].kind = "symlink_metadata".into();
        assert!(validate_export_paths(&report, &["safe".into()]).is_err());
        assert!(validate_export_paths(&report, &[]).is_err());
    }
    #[test]
    fn trx_manifest_requires_pinned_parser_and_contained_root_partition() {
        let mut report = synthetic_report();
        report["container"] = "trx".into();
        report["region"]["offset"] = 28.into();
        report["region"]["length"] = 996.into();
        report["status"] = "partially_extracted".into();
        report["unextractedRegions"] = serde_json::json!([{"offset":0,"length":28}]);
        report["containerInfo"] = serde_json::json!({"format":"trx","version":1,"flags":0,"length":1024,"partitionOffsets":[28],"crc32":"12345678","parser":"OpenWrt otrx","sourceCommit":"e3853149fb18dfa55f10c10c9aaf99c329747365","sourceSha256":"e2ef4e7a08f84543945fec83c43bdebfdb1c252da1c440ebb90454913ec2dc3e","parserSha256":"66b870899eca6a12079668c23cdd95db069978afc9af5bcbc3681cd9c1e968e5"});
        assert!(synthetic_bundle(&report, b"test", false).is_ok());
        for (field, value) in [
            ("version", serde_json::json!(2)),
            ("flags", serde_json::json!(1)),
            ("length", serde_json::json!(999)),
            ("partitionOffsets", serde_json::json!([60, 28])),
            ("parserSha256", serde_json::json!("a".repeat(64))),
            ("crc32", serde_json::json!("not-hex")),
        ] {
            let mut invalid_report = report.clone();
            invalid_report["containerInfo"][field] = value;
            assert!(
                synthetic_bundle(&invalid_report, b"test", false).is_err(),
                "{field}"
            );
        }
        report["containerInfo"] = serde_json::Value::Null;
        assert!(synthetic_bundle(&report, b"test", false).is_err());
    }
}
