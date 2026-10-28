//! Bounded byte/format inspection only. No decompression, subprocess or image execution.
use super::*;
use serde::{Deserialize, Serialize};
use std::io::Read;

const ANALYSIS_COMPLETE_MESSAGE: &str = "Bounded analysis saved. Extraction is a separate sandboxed action; image contents are not executed.";
const MAX_INPUT: u64 = 512 * 1024 * 1024;
const MAX_SCAN: u64 = 32 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FormatRegion {
    pub offset: u64,
    pub format: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnalysisReport {
    pub report_version: u32,
    pub input_sha256: String,
    pub input_bytes: u64,
    pub analyzed_at: u64,
    pub scanned_bytes: u64,
    pub scan_truncated: bool,
    pub format: String,
    pub regions: Vec<FormatRegion>,
    pub sampled_entropy: f64,
    pub entropy_sample_bytes: u64,
    pub recursive_extraction: bool,
    pub executable_contents_run: bool,
    pub compatibility: String,
    pub limitations: Vec<String>,
}
fn inspect(mut file: File, hash: &str, size: u64, cancel: &AtomicBool) -> Result<AnalysisReport> {
    if size > MAX_INPUT {
        return Err(LibraryError::new(
            "ANALYSIS_LIMIT",
            "Bounded analysis supports inputs up to 512 MiB.",
        ));
    }
    let count = size.min(MAX_SCAN);
    let mut bytes = Vec::with_capacity(count as usize);
    while bytes.len() < (count as usize) {
        cancelled(cancel)?;
        let mut chunk = [0u8; 65536];
        let wanted = (count as usize - bytes.len()).min(chunk.len());
        let read = file.read(&mut chunk[..wanted])?;
        if read == 0 {
            return Err(LibraryError::new(
                "SOURCE_CHANGED",
                "Verified input ended unexpectedly.",
            ));
        }
        bytes.extend_from_slice(&chunk[..read]);
    }
    let signatures: [(&[u8], &str); 7] = [
        (b"\x27\x05\x19\x56", "U-Boot legacy"),
        (b"\xd0\x0d\xfe\xed", "Flattened Device Tree / FIT"),
        (b"hsqs", "SquashFS little endian"),
        (b"sqsh", "SquashFS big endian"),
        (b"UBI#", "UBI eraseblock"),
        (b"HDR0", "TRX container"),
        (b"\x7fELF", "ELF header"),
    ];
    let mut regions = Vec::new();
    for (offset, window) in bytes.windows(4).enumerate() {
        if offset % 65536 == 0 {
            cancelled(cancel)?;
        }
        if regions.len() >= 64 {
            break;
        }
        for (magic, name) in signatures {
            if window == magic {
                regions.push(FormatRegion {
                    offset: offset as u64,
                    format: name.into(),
                });
            }
        }
    }
    // Recognize only root signatures with their complete fixed-header size.
    // CPIO newc/crc: 6 magic bytes + thirteen 8-byte ASCII fields (110 bytes).
    // BSDIFF40: 8 magic bytes + three 8-byte integers (32 bytes).
    // This does not validate fields, archive members, checksums or patch streams.
    let format: String = if bytes.len() >= 110 && bytes.starts_with(b"070701") {
        "CPIO newc archive".into()
    } else if bytes.len() >= 110 && bytes.starts_with(b"070702") {
        "CPIO CRC archive".into()
    } else if bytes.len() >= 32 && bytes.starts_with(b"BSDIFF40") {
        "BSDIFF40 patch".into()
    } else if bytes.starts_with(&[0x1f, 0x8b]) {
        "gzip wrapper".into()
    } else if bytes.get(257..262) == Some(b"ustar") {
        "tar archive".into()
    } else {
        regions
            .iter()
            .find(|r| r.offset == 0)
            .map(|r| r.format.clone())
            .unwrap_or_else(|| "Unknown / vendor-specific".into())
    };
    let sample = &bytes[..bytes.len().min(256 * 1024)];
    let mut counts = [0u64; 256];
    for byte in sample {
        counts[*byte as usize] += 1;
    }
    let entropy = counts
        .iter()
        .filter(|&&n| n > 0)
        .map(|&n| {
            let p = n as f64 / sample.len() as f64;
            -p * p.log2()
        })
        .sum();
    let mut limitations = vec![
        "Magic matches are observations, not validated partitions or device support.".into(),
        "Only the first 32 MiB and at most 64 format signatures are scanned; entropy samples at most 256 KiB.".into(),
        "This analysis does not recursively extract or inspect packages. Extraction is a separate sandboxed action; analysis does not authorize firmware modification or installation.".into(),
    ];
    match format.as_str() {
        "CPIO newc archive" | "CPIO CRC archive" => {
            limitations.push("CPIO magic is an observation only; archive structure and checksums are not validated. No archive members are parsed or unpacked.".into());
            limitations.push("CPIO magic alone cannot determine whether the payload is a full firmware image or an incremental update. This analysis does not establish extraction support or device compatibility.".into());
        }
        "BSDIFF40 patch" => limitations.push("BSDIFF40 magic indicates a patch format, not a complete firmware image; patch structure and required base image are not validated. This analysis does not apply patches or establish extraction support or device compatibility.".into()),
        _ => {}
    }
    Ok(AnalysisReport {
        report_version: 1,
        input_sha256: hash.into(),
        input_bytes: size,
        analyzed_at: now(),
        scanned_bytes: count,
        scan_truncated: count < size,
        format,
        regions,
        sampled_entropy: entropy,
        entropy_sample_bytes: sample.len() as u64,
        recursive_extraction: false,
        executable_contents_run: false,
        compatibility: "unknown".into(),
        limitations,
    })
}
impl LibraryService {
    pub fn analyze(&self, hash: &str) -> Result<JobRecord> {
        self.ensure_open()?;
        let artifact = self.details(hash)?.artifact;
        if artifact.size_bytes > MAX_INPUT {
            return Err(LibraryError::new(
                "ANALYSIS_LIMIT",
                "Bounded analysis supports inputs up to 512 MiB.",
            ));
        }
        let mut controls = lock(&self.inner.controls)?;
        self.ensure_open()?;
        if controls.len() >= 16 {
            return Err(LibraryError::new(
                "QUEUE_FULL",
                "Wait for an existing Library job.",
            ));
        }
        let id = nonce()?;
        lock(&self.inner.repo)?.create_library_job(
            &id,
            "firmware_analysis",
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
            .try_send(Task::Analyze {
                job: id.clone(),
                hash: hash.into(),
            })
            .is_err()
        {
            controls.remove(&id);
            lock(&self.inner.repo)?.library_transition(
                &id,
                JobStatus::Failed,
                Some(&LibraryError::new("QUEUE_FULL", "Analysis was not queued.")),
            )?;
        }
        lock(&self.inner.repo)?.library_job(&id)
    }
    pub fn analysis_report(&self, hash: &str) -> Result<Option<AnalysisReport>> {
        self.ensure_open()?;
        self.details(hash)?;
        let report_hash: Option<String> = {
            use rusqlite::OptionalExtension;
            lock(&self.inner.repo)?.connection.query_row("SELECT blob_sha256 FROM artifact_occurrences WHERE artifact_class='metadata_report' AND provenance_kind='bounded_format_analysis' AND source_reference=?1 ORDER BY collected_at DESC,occurrence_id DESC LIMIT 1",[format!("analysis:{hash}")],|r|r.get(0)).optional()?
        };
        let Some(report_hash) = report_hash else {
            return Ok(None);
        };
        let proof = self.inner.vault.verify_published(
            &report_hash,
            1024 * 1024,
            &AtomicBool::new(false),
            |_| Ok(()),
        )?;
        let report: AnalysisReport = serde_json::from_reader(proof.reader()?).map_err(|_| {
            LibraryError::new(
                "INVALID_REPORT",
                "Stored analysis report could not be validated.",
            )
        })?;
        if report.input_sha256 != hash
            || report.report_version != 1
            || report.recursive_extraction
            || report.executable_contents_run
        {
            return Err(LibraryError::new(
                "INVALID_REPORT",
                "Stored report does not match this artifact or analyzer.",
            ));
        }
        Ok(Some(report))
    }
}
impl Inner {
    pub(super) fn run_analysis(&self, id: &str, hash: &str, cancel: &AtomicBool) -> Result<()> {
        let artifact = lock(&self.repo)?.library_details(hash)?.artifact;
        self.progress(id, 0, artifact.size_bytes, "verifying_analysis_input")?;
        let input = self
            .vault
            .verify_published(hash, MAX_INPUT, cancel, |_| Ok(()))?;
        if input.size() != artifact.size_bytes {
            return Err(LibraryError::new(
                "METADATA_CONFLICT",
                "Input size disagrees with retained artifact.",
            ));
        }
        self.progress(id, 0, artifact.size_bytes, "bounded_format_analysis")?;
        let report = inspect(input.reader()?, hash, input.size(), cancel)?;
        let raw = serde_json::to_vec_pretty(&report)
            .map_err(|_| LibraryError::new("REPORT_FAILED", "Report could not be serialized."))?;
        let stage = self.vault.stage(id)?;
        let mut output = new_file(&stage)?;
        output.write_all(&raw)?;
        output.sync_all()?;
        drop(output);
        let (file, report_hash, size) = hash_file(&stage, 1024 * 1024, cancel, |_| Ok(()))?;
        drop(file);
        let (blob, _guards) = self.vault.blob(&report_hash)?;
        let _fence = lock(&self.finalize)?;
        cancelled(cancel)?;
        match fs::hard_link(&stage, &blob) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        };
        let proof = self
            .vault
            .verify_published(&report_hash, 1024 * 1024, cancel, |_| Ok(()))?;
        if proof.size() != size {
            return Err(LibraryError::new(
                "REPORT_FAILED",
                "Published report size mismatch.",
            ));
        }
        cancelled(cancel)?;
        let mut repo = lock(&self.repo)?;
        let tx = repo.connection.transaction()?;
        super::admission::admit_analysis_report(&tx, id, hash, &proof)?;
        tx.execute(
            "UPDATE jobs SET status='completed' WHERE id=?1 AND status='running'",
            [id],
        )?;
        tx.execute("UPDATE job_details SET finished_at=?2,updated_at=?2,progress_bytes=total_bytes,stage='completed',message=?3 WHERE job_id=?1",rusqlite::params![id,now(),ANALYSIS_COMPLETE_MESSAGE])?;
        db::event(
            &tx,
            "firmware_analyzed",
            "Bounded analysis report saved; no image contents executed.",
            Some(id),
            Some(hash),
            "info",
        )?;
        tx.commit()?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn synthetic_header(magic: &[u8], header_bytes: usize) -> Vec<u8> {
        let mut bytes = vec![b'0'; header_bytes];
        bytes[..magic.len()].copy_from_slice(magic);
        bytes
    }
    fn inspect_bytes(bytes: &[u8]) -> AnalysisReport {
        use std::io::Seek;
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(bytes).unwrap();
        file.rewind().unwrap();
        inspect(
            file,
            &"c".repeat(64),
            bytes.len() as u64,
            &AtomicBool::new(false),
        )
        .unwrap()
    }
    #[test]
    fn analysis_describes_separate_extraction_without_claiming_execution() {
        let report = inspect_bytes(&synthetic_header(b"070702", 110));
        assert!(!report.recursive_extraction);
        assert!(!report.executable_contents_run);
        assert!(report
            .limitations
            .iter()
            .any(|text| text.contains("Extraction is a separate sandboxed action")));
        assert!(ANALYSIS_COMPLETE_MESSAGE.contains("separate sandboxed action"));
        assert!(ANALYSIS_COMPLETE_MESSAGE.contains("not executed"));
        assert!(!ANALYSIS_COMPLETE_MESSAGE.contains("extraction disabled"));
    }
    #[test]
    fn bounded_cpio_headers_are_observations_without_payload_or_compatibility_claims() {
        for (magic, expected) in [
            (b"070701", "CPIO newc archive"),
            (b"070702", "CPIO CRC archive"),
        ] {
            // Synthetic fixed header only: deliberately not a valid archive fixture.
            let report = inspect_bytes(&synthetic_header(magic, 110));
            assert_eq!(report.format, expected);
            assert_eq!(report.report_version, 1);
            assert_eq!(report.compatibility, "unknown");
            assert!(!report.recursive_extraction);
            assert!(!report.executable_contents_run);
            assert!(report
                .limitations
                .iter()
                .any(|text| text.contains("full firmware image or an incremental update")));
            assert!(report
                .limitations
                .iter()
                .any(|text| text.contains("archive structure and checksums are not validated")));
        }
    }
    #[test]
    fn bounded_bsdiff_header_does_not_claim_patch_validation_or_complete_firmware() {
        // Synthetic fixed header only: deliberately no compressed patch blocks.
        let report = inspect_bytes(&synthetic_header(b"BSDIFF40", 32));
        assert_eq!(report.format, "BSDIFF40 patch");
        assert_eq!(report.compatibility, "unknown");
        assert!(!report.recursive_extraction);
        assert!(!report.executable_contents_run);
        assert!(
            report
                .limitations
                .iter()
                .any(|text| text
                    .contains("patch structure and required base image are not validated"))
        );
        assert!(report
            .limitations
            .iter()
            .any(|text| text.contains("not a complete firmware image")));
    }
    #[test]
    fn cpio_bytes_not_zip_filename_determine_observed_root_format() {
        let bytes = synthetic_header(b"070702", 110);
        for suffix in [".zip", ".bin", ".delta", ".cpio"] {
            let mut named = tempfile::Builder::new().suffix(suffix).tempfile().unwrap();
            named.write_all(&bytes).unwrap();
            let file = File::open(named.path()).unwrap();
            let report = inspect(
                file,
                &"d".repeat(64),
                bytes.len() as u64,
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!(report.format, "CPIO CRC archive", "suffix {suffix}");
        }
    }
    #[test]
    fn truncated_cpio_and_bsdiff_headers_remain_unknown() {
        for (magic, header_bytes) in [
            (b"070701".as_slice(), 110),
            (b"070702".as_slice(), 110),
            (b"BSDIFF40".as_slice(), 32),
        ] {
            let bytes = synthetic_header(magic, header_bytes);
            for len in 0..header_bytes {
                assert_eq!(
                    inspect_bytes(&bytes[..len]).format,
                    "Unknown / vendor-specific",
                    "magic {magic:?}, length {len}"
                );
            }
        }
    }
    #[test]
    fn near_miss_cpio_and_bsdiff_magic_remains_unknown() {
        for magic in [
            b"070700".as_slice(),
            b"070703",
            b"070707",
            b"BSDIFF41",
            b"bsdiff40",
        ] {
            let bytes = synthetic_header(magic, 110);
            assert_eq!(inspect_bytes(&bytes).format, "Unknown / vendor-specific");
        }
    }
    #[test]
    fn embedded_cpio_or_bsdiff_magic_is_not_the_root_format() {
        for (magic, header_bytes) in [
            (b"070701".as_slice(), 110),
            (b"070702".as_slice(), 110),
            (b"BSDIFF40".as_slice(), 32),
        ] {
            let mut bytes = b"synthetic outer container ".to_vec();
            bytes.extend(synthetic_header(magic, header_bytes));
            let report = inspect_bytes(&bytes);
            assert_eq!(report.format, "Unknown / vendor-specific");
            assert!(report.regions.is_empty());
        }
    }
    #[test]
    fn bounded_magic_metadata_is_observed_without_extraction_or_execution() {
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(b"\x27\x05\x19\x56harmless synthetic hsqs bytes")
            .unwrap();
        use std::io::Seek;
        file.rewind().unwrap();
        let size = file.metadata().unwrap().len();
        let report = inspect(file, &"a".repeat(64), size, &AtomicBool::new(false)).unwrap();
        assert_eq!(report.format, "U-Boot legacy");
        assert_eq!(report.regions.len(), 2);
        assert!(!report.recursive_extraction);
        assert!(!report.executable_contents_run);
        assert_eq!(report.compatibility, "unknown");
    }
    #[test]
    fn oversized_or_cancelled_analysis_refuses_before_reading() {
        let file = tempfile::tempfile().unwrap();
        assert_eq!(
            inspect(
                file.try_clone().unwrap(),
                &"b".repeat(64),
                MAX_INPUT + 1,
                &AtomicBool::new(false)
            )
            .unwrap_err()
            .code,
            "ANALYSIS_LIMIT"
        );
        assert_eq!(
            inspect(file, &"b".repeat(64), 1, &AtomicBool::new(true))
                .unwrap_err()
                .code,
            "CANCELLED"
        );
    }
}
