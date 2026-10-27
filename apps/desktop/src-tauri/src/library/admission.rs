//! The sole production firmware-membership writer. Caller names, labels, hashes,
//! extensions and content deduplication are never producer credentials.
use super::{files::VerifiedPublishedBlob, types::*};
use crate::connection::now;
use rusqlite::{params, OptionalExtension, Transaction};

mod sealed {
    pub trait Sealed {}
}
trait Producer: sealed::Sealed {
    fn class(&self) -> ArtifactClass;
}
struct UserImportProducer {
    _private: (),
}
impl sealed::Sealed for UserImportProducer {}
impl Producer for UserImportProducer {
    fn class(&self) -> ArtifactClass {
        ArtifactClass::UserImportedUnverified
    }
}
impl UserImportProducer {
    fn record(&self, tx: &Transaction<'_>, job: &str, proof: &VerifiedPublishedBlob) -> Result<()> {
        let hash = proof.hash();
        let size: Option<u64> = tx
            .query_row(
                "SELECT size_bytes FROM artifact_blobs WHERE sha256=?1",
                [hash],
                |r| r.get(0),
            )
            .optional()?;
        match size {
            Some(size) if size != proof.size() => return Err(conflict()),
            None => {
                tx.execute(
                    "INSERT INTO artifact_blobs(sha256,size_bytes) VALUES(?1,?2)",
                    params![hash, proof.size()],
                )?;
            }
            Some(_) => {}
        }
        let occurrence = format!("user-import:{job}");
        tx.execute("INSERT INTO artifact_occurrences(occurrence_id,blob_sha256,artifact_class,provenance_kind,acquisition_occurrence_id,device_id,job_id,collected_at) SELECT ?1,?2,?3,'user_import',?1,(SELECT id FROM devices WHERE id=d.device_id),job_id,?4 FROM job_details d WHERE job_id=?5",params![occurrence,hash,self.class().text(),now(),job])?;
        let member: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM firmware_artifacts WHERE sha256=?1)",
            [hash],
            |r| r.get(0),
        )?;
        if !member {
            tx.execute(
                "INSERT INTO firmware_admissions VALUES(?1,?2,?3)",
                params![hash, occurrence, self.class().text()],
            )?;
            tx.execute(
                "INSERT INTO firmware_artifacts(sha256,size_bytes) VALUES(?1,?2)",
                params![hash, proof.size()],
            )?;
        }
        retained_membership(tx, proof)
    }
}
fn conflict() -> LibraryError {
    LibraryError::new(
        "METADATA_CONFLICT",
        "Verified bytes disagree with retained admission. Existing records were preserved.",
    )
}
fn retained_membership(tx: &Transaction<'_>, proof: &VerifiedPublishedBlob) -> Result<()> {
    let valid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM firmware_artifacts a JOIN artifact_blobs b USING(sha256) JOIN firmware_admissions f USING(sha256) JOIN artifact_occurrences o ON o.occurrence_id=f.occurrence_id WHERE a.sha256=?1 AND a.size_bytes=?2 AND b.size_bytes=a.size_bytes AND o.blob_sha256=a.sha256 AND o.artifact_class=f.artifact_class)",params![proof.hash(),proof.size()],|r|r.get(0))?;
    if !valid {
        return Err(conflict());
    }
    Ok(())
}
pub(super) fn admit_import_or_restore(
    tx: &Transaction<'_>,
    job: &str,
    proof: &VerifiedPublishedBlob,
) -> Result<()> {
    let (kind,hash,size):(String,Option<String>,u64)=tx.query_row("SELECT j.kind,d.artifact_sha256,d.total_bytes FROM jobs j JOIN job_details d ON d.job_id=j.id WHERE j.id=?1 AND j.status='running' AND d.cancel_requested=0",[job],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
    if hash.as_deref() != Some(proof.hash()) || size != proof.size() {
        return Err(conflict());
    }
    match kind.as_str() {
        "firmware_import" => UserImportProducer { _private: () }.record(tx, job, proof),
        // Restoring a copy of already admitted firmware is not a new acquisition.
        "backup_restore" => retained_membership(tx, proof),
        _ => Err(LibraryError::new(
            "INVALID_PRODUCER",
            "This operation cannot admit firmware.",
        )),
    }
}

pub(super) fn admit_official(
    tx: &Transaction<'_>,
    job: &str,
    proof: &VerifiedPublishedBlob,
    source: &super::official::QualifiedSource,
) -> Result<()> {
    source.verify(proof)?;
    let entry = source.entry();
    let valid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM jobs j JOIN job_details d ON d.job_id=j.id WHERE j.id=?1 AND j.kind='official_image_download' AND j.status='running' AND d.cancel_requested=0 AND d.artifact_sha256=?2 AND d.total_bytes=?3 AND d.original_filename=?4 AND d.description=?5)",params![job,entry.sha256,entry.size_bytes,entry.filename,entry.id],|r|r.get(0))?;
    if !valid {
        return Err(conflict());
    }
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
        Some(size) if size != proof.size() => return Err(conflict()),
        _ => {}
    }
    let occurrence = format!("official:{job}");
    let reference = serde_json::to_string(entry).map_err(|_| conflict())?;
    tx.execute("INSERT INTO artifact_occurrences(occurrence_id,blob_sha256,artifact_class,provenance_kind,source_reference,acquisition_occurrence_id,device_id,job_id,collected_at) SELECT ?1,?2,'official_image_download','official_vendor_download',?3,?1,(SELECT id FROM devices WHERE id=d.device_id),?4,?5 FROM job_details d WHERE d.job_id=?4",params![occurrence,proof.hash(),reference,job,now()])?;
    let member: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM firmware_artifacts WHERE sha256=?1)",
        [proof.hash()],
        |r| r.get(0),
    )?;
    if !member {
        tx.execute(
            "INSERT INTO firmware_admissions VALUES(?1,?2,'official_image_download')",
            params![proof.hash(), occurrence],
        )?;
        tx.execute(
            "INSERT INTO firmware_artifacts VALUES(?1,?2)",
            params![proof.hash(), proof.size()],
        )?;
    }
    retained_membership(tx, proof)
}

pub(super) fn admit_analysis_report(
    tx: &Transaction<'_>,
    job: &str,
    input: &str,
    proof: &VerifiedPublishedBlob,
) -> Result<()> {
    let valid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM jobs j JOIN job_details d ON d.job_id=j.id JOIN firmware_artifacts a ON a.sha256=d.artifact_sha256 WHERE j.id=?1 AND j.kind='firmware_analysis' AND j.status='running' AND d.cancel_requested=0 AND a.sha256=?2)",params![job,input],|r|r.get(0))?;
    if !valid {
        return Err(conflict());
    }
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
        Some(size) if size != proof.size() => return Err(conflict()),
        _ => {}
    }
    let occurrence = format!("analysis:{job}");
    tx.execute("INSERT INTO artifact_occurrences(occurrence_id,blob_sha256,artifact_class,provenance_kind,source_reference,acquisition_occurrence_id,job_id,collected_at) VALUES(?1,?2,'metadata_report','bounded_format_analysis',?3,?1,?4,?5)",params![occurrence,proof.hash(),format!("analysis:{input}"),job,now()])?;
    Ok(())
}

/// Remaining reserved producers have no production consumers. Official images
/// and bounded reports use the constrained proof checks above; neither can
/// instantiate these device-export, dump or configuration contracts.
#[allow(dead_code)]
mod reserved {
    use super::*;
    pub(super) struct DeviceExportProducer {
        private: (),
    }
    pub(super) struct PhysicalDumpProducer {
        private: (),
    }
    pub(super) struct ConfigurationProducer {
        private: (),
    }
    pub(super) struct ReportProducer {
        private: (),
    }
    pub(super) struct FirmwareReceipt<P: Producer> {
        producer: P,
        blob: VerifiedPublishedBlob,
    }
    pub(super) struct NonFirmwareReceipt<P: sealed::Sealed> {
        producer: P,
        blob: VerifiedPublishedBlob,
    }
    macro_rules! producer {
        ($ty:ty,$class:expr) => {
            impl sealed::Sealed for $ty {}
            impl Producer for $ty {
                fn class(&self) -> ArtifactClass {
                    $class
                }
            }
        };
    }
    producer!(DeviceExportProducer, ArtifactClass::DeviceFirmwareExport);
    producer!(PhysicalDumpProducer, ArtifactClass::PhysicalFlashDump);
    impl sealed::Sealed for ConfigurationProducer {}
    impl sealed::Sealed for ReportProducer {}
}
