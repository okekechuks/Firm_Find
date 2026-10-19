use super::types::*;
use crate::{connection::now, storage::Repository};
use rusqlite::{params, Connection, OptionalExtension};

const JOB_SQL:&str="SELECT j.id,j.kind,j.status,d.created_at,d.started_at,d.finished_at,d.updated_at,d.progress_bytes,d.total_bytes,d.stage,d.error_code,d.message,d.artifact_sha256,d.import_id,d.cancel_requested,d.duplicate,d.recovery_info,d.original_filename,d.device_id FROM jobs j JOIN job_details d ON j.id=d.job_id";
fn job(r: &rusqlite::Row) -> rusqlite::Result<JobRecord> {
    let status: String = r.get(2)?;
    let status = match status.as_str() {
        "pending" => JobStatus::Pending,
        "running" => JobStatus::Running,
        "completed" => JobStatus::Completed,
        "cancelled" => JobStatus::Cancelled,
        _ => JobStatus::Failed,
    };
    Ok(JobRecord {
        id: r.get(0)?,
        kind: r.get(1)?,
        status,
        created_at: r.get(3)?,
        started_at: r.get(4)?,
        finished_at: r.get(5)?,
        updated_at: r.get(6)?,
        progress_bytes: r.get(7)?,
        total_bytes: r.get(8)?,
        stage: r.get(9)?,
        error_code: r.get(10)?,
        message: r.get(11)?,
        related_artifact: r.get(12)?,
        import_id: r.get(13)?,
        cancel_requested: r.get(14)?,
        duplicate: r.get(15)?,
        recovery_info: r.get(16)?,
        original_filename: r.get(17)?,
        device_id: r.get(18)?,
    })
}
pub(super) fn event(
    c: &Connection,
    kind: &str,
    message: &str,
    id: Option<&str>,
    hash: Option<&str>,
    severity: &str,
) -> rusqlite::Result<()> {
    c.execute(
        "INSERT INTO audit_events(kind,message) VALUES(?1,?2)",
        params![kind, message],
    )?;
    c.execute(
        "INSERT INTO activity_links VALUES(?1,?2,?3,?4)",
        params![c.last_insert_rowid(), id, hash, severity],
    )?;
    Ok(())
}
impl Repository {
    pub fn library_job(&self, id: &str) -> Result<JobRecord> {
        self.connection
            .query_row(&format!("{JOB_SQL} WHERE j.id=?1"), [id], job)
            .optional()?
            .ok_or_else(|| LibraryError::new("JOB_NOT_FOUND", "Job not found."))
    }
    pub fn library_jobs(&self) -> Result<Vec<JobRecord>> {
        let mut s = self.connection.prepare(&format!(
            "{JOB_SQL} ORDER BY d.created_at DESC,j.id DESC LIMIT 100"
        ))?;
        let rows = s
            .query_map([], job)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
    pub(crate) fn create_library_job(
        &mut self,
        id: &str,
        kind: &str,
        total: u64,
        input: JobInput<'_>,
    ) -> Result<()> {
        let JobInput {
            filename: name,
            description,
            device,
            hash,
        } = input;
        let tx = self.connection.transaction()?;
        tx.execute(
            "INSERT INTO jobs VALUES(?1,?2,'pending')",
            params![id, kind],
        )?;
        tx.execute("INSERT INTO job_details(job_id,created_at,updated_at,total_bytes,original_filename,description,device_id,artifact_sha256) VALUES(?1,?2,?2,?3,?4,?5,?6,?7)",params![id,now(),total,name,description,device,hash])?;
        event(
            &tx,
            "job_created",
            "Library operation queued.",
            Some(id),
            hash,
            "info",
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn library_transition(
        &mut self,
        id: &str,
        next: JobStatus,
        error: Option<&LibraryError>,
    ) -> Result<JobRecord> {
        let old = self.library_job(id)?;
        if old.status == next && old.status.terminal() {
            return Ok(old);
        }
        if !old.status.allows(next) {
            return Err(LibraryError::new(
                "INVALID_JOB_TRANSITION",
                "This job cannot return to an earlier state.",
            ));
        }
        let tx = self.connection.transaction()?;
        tx.execute(
            "UPDATE jobs SET status=?2 WHERE id=?1 AND status=?3",
            params![id, next.text(), old.status.text()],
        )?;
        tx.execute("UPDATE job_details SET started_at=CASE WHEN ?2='running' THEN ?3 ELSE started_at END,finished_at=CASE WHEN ?4 THEN ?3 ELSE finished_at END,updated_at=?3,stage=?2,error_code=?5,message=?6 WHERE job_id=?1",params![id,next.text(),now(),next.terminal(),error.map(|e|&e.code),error.map_or(if next==JobStatus::Running{"Operation running."}else{"Operation completed."},|e|e.message.as_str())])?;
        if next.terminal() {
            event(
                &tx,
                &format!("library_{}", next.text()),
                error.map_or("Library operation completed.", |e| e.message.as_str()),
                Some(id),
                old.related_artifact.as_deref(),
                if next == JobStatus::Completed {
                    "info"
                } else {
                    "warning"
                },
            )?;
        }
        tx.commit()?;
        self.library_job(id)
    }
    pub fn library_progress(
        &mut self,
        id: &str,
        bytes: u64,
        total: u64,
        stage: &str,
    ) -> Result<()> {
        self.connection.execute("UPDATE job_details SET progress_bytes=?2,total_bytes=?3,stage=?4,updated_at=?5 WHERE job_id=?1 AND EXISTS(SELECT 1 FROM jobs WHERE id=?1 AND status='running')",params![id,bytes,total,stage,now()])?;
        Ok(())
    }
    pub fn library_cancel_requested(&mut self, id: &str) -> Result<JobRecord> {
        self.connection.execute("UPDATE job_details SET cancel_requested=1,updated_at=?2 WHERE job_id=?1 AND EXISTS(SELECT 1 FROM jobs WHERE id=?1 AND status IN ('pending','running'))",params![id,now()])?;
        self.library_job(id)
    }
    pub fn library_candidate(&mut self, id: &str, hash: &str) -> Result<()> {
        self.connection.execute("UPDATE job_details SET artifact_sha256=?2,recovery_info='Published bytes may be retained if metadata commit is interrupted.' WHERE job_id=?1",params![id,hash])?;
        Ok(())
    }
    pub fn commit_library_import(
        &mut self,
        id: &str,
        hash: &str,
        size: u64,
        key: &str,
        duplicate: bool,
    ) -> Result<()> {
        let tx = self.connection.transaction()?;
        let running:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM jobs j JOIN job_details d ON d.job_id=j.id WHERE j.id=?1 AND j.status='running' AND d.cancel_requested=0)",[id],|r|r.get(0))?;
        if !running {
            return Err(LibraryError::new(
                "INVALID_JOB_TRANSITION",
                "Only an active, uncancelled import may commit.",
            ));
        }
        tx.execute(
            "INSERT OR IGNORE INTO firmware_artifacts VALUES(?1,?2)",
            params![hash, size],
        )?;
        let old: u64 = tx.query_row(
            "SELECT size_bytes FROM firmware_artifacts WHERE sha256=?1",
            [hash],
            |r| r.get(0),
        )?;
        if old != size {
            return Err(LibraryError::new("METADATA_CONFLICT","Stored artifact size disagrees with verified content. Existing metadata was preserved."));
        }
        tx.execute("INSERT INTO artifact_details(sha256,storage_key,integrity_status,first_imported_at,last_verified_at) VALUES(?1,?2,'verified',?3,?3) ON CONFLICT(sha256) DO UPDATE SET storage_key=excluded.storage_key,storage_status='present',integrity_status='verified',last_verified_at=excluded.last_verified_at",params![hash,key,now()])?;
        tx.execute("INSERT INTO artifact_imports SELECT job_id,?2,original_filename,?3,'user_import','unverified',description,device_id,job_id FROM job_details WHERE job_id=?1",params![id,hash,now()])?;
        tx.execute(
            "UPDATE artifact_details SET removed_at=NULL WHERE sha256=?1",
            [hash],
        )?;
        let restored: bool = tx.query_row(
            "SELECT kind='backup_restore' FROM jobs WHERE id=?1",
            [id],
            |r| r.get(0),
        )?;
        if restored {
            tx.execute("INSERT INTO artifact_presentation SELECT ?2,original_filename,'restored_default' FROM job_details WHERE job_id=?1 ON CONFLICT(sha256) DO UPDATE SET display_name=excluded.display_name,kind=excluded.kind",params![id,hash])?;
        }
        tx.execute("INSERT INTO integrity_checks(artifact_sha256,job_id,expected_size,actual_size,actual_sha256,status,verified_at) VALUES(?1,?2,?3,?3,?1,'verified',?4)",params![hash,id,size,now()])?;
        tx.execute("UPDATE job_details SET artifact_sha256=?2,import_id=?1,duplicate=?3,recovery_info=NULL WHERE job_id=?1",params![id,hash,duplicate])?;
        // Metadata and successful job publication share a single transaction.
        tx.execute(
            "UPDATE jobs SET status='completed' WHERE id=?1 AND status='running'",
            [id],
        )?;
        tx.execute("UPDATE job_details SET finished_at=?2,updated_at=?2,progress_bytes=total_bytes,stage='completed',message=?3 WHERE job_id=?1",params![id,now(),if duplicate{"Identical bytes already stored; import occurrence recorded."}else{"Import completed. Integrity verified; authenticity and device compatibility remain unverified."}])?;
        event(
            &tx,
            "firmware_imported",
            if duplicate {
                "Duplicate import recorded; existing verified blob reused."
            } else {
                "Firmware file imported; stored bytes verified."
            },
            Some(id),
            Some(hash),
            "info",
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn set_library_integrity(
        &mut self,
        id: Option<&str>,
        result: &IntegrityResult,
    ) -> Result<()> {
        let tx = self.connection.transaction()?;
        tx.execute("UPDATE artifact_details SET integrity_status=?2,last_verified_at=?3,storage_status=?4 WHERE sha256=?1",params![result.artifact_sha256,result.status,result.verified_at,match result.status.as_str(){"missing"=>"missing","unreadable"=>"unreadable",_=>"present"}])?;
        tx.execute("INSERT INTO integrity_checks(artifact_sha256,job_id,expected_size,actual_size,actual_sha256,status,verified_at,error_code) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![result.artifact_sha256,id,result.expected_size,result.actual_size,result.actual_sha256,result.status,result.verified_at,result.error_code])?;
        tx.commit()?;
        Ok(())
    }
    pub fn library_mark(&mut self, hash: &str, status: &str, storage: &str) -> Result<()> {
        self.connection.execute(
            "UPDATE artifact_details SET integrity_status=?2,storage_status=?3 WHERE sha256=?1",
            params![hash, status, storage],
        )?;
        Ok(())
    }
    pub fn library_artifacts(&self, q: &LibraryQuery, max: u64) -> Result<LibraryPage> {
        let search = q.search.as_deref().unwrap_or("");
        let page = q.page.unwrap_or(0);
        if search.len() > 256
            || page > 100000
            || q.from_date.zip(q.to_date).is_some_and(|(a, b)| a > b)
            || q.integrity.as_ref().is_some_and(|v| {
                ![
                    "not_checked",
                    "checking",
                    "verified",
                    "hash_mismatch",
                    "missing",
                    "unreadable",
                    "verification_failed",
                ]
                .contains(&v.as_str())
            })
            || q.source.as_ref().is_some_and(|v| {
                ![
                    "user_import",
                    "unknown",
                    "official_vendor_download",
                    "authorized_device_acquisition",
                    "generated_build",
                ]
                .contains(&v.as_str())
            })
        {
            return Err(LibraryError::new(
                "INVALID_FILTER",
                "Invalid library search, date range or filter.",
            ));
        }
        let kind_sql="COALESCE((SELECT kind FROM artifact_presentation p WHERE p.sha256=a.sha256),CASE (SELECT source_category FROM artifact_imports i WHERE i.artifact_sha256=a.sha256 ORDER BY imported_at,import_id LIMIT 1) WHEN 'user_import' THEN 'imported_original' WHEN 'official_vendor_download' THEN 'imported_original' WHEN 'authorized_device_acquisition' THEN 'extracted' WHEN 'generated_build' THEN 'build_output' ELSE 'unknown' END)";
        if q.kind.as_ref().is_some_and(|v| {
            ![
                "imported_original",
                "extracted",
                "customized",
                "restored_default",
                "build_output",
                "unknown",
            ]
            .contains(&v.as_str())
        }) {
            return Err(LibraryError::new(
                "INVALID_FILTER",
                "Unknown firmware type.",
            ));
        }
        let order=match q.sort.as_deref().unwrap_or("newest") {
            "newest"=>"d.first_imported_at DESC,a.sha256", "oldest"=>"d.first_imported_at ASC,a.sha256", "largest"=>"a.size_bytes DESC,a.sha256", "smallest"=>"a.size_bytes ASC,a.sha256",
            "name"=>"lower(COALESCE((SELECT display_name FROM artifact_presentation p WHERE p.sha256=a.sha256),(SELECT original_filename FROM artifact_imports i WHERE i.artifact_sha256=a.sha256 ORDER BY imported_at,import_id LIMIT 1),'Legacy artifact')) ASC,a.sha256",
            _=>return Err(LibraryError::new("INVALID_FILTER","Unknown sort order.")),
        };
        let condition=" WHERE d.removed_at IS NULL AND (instr(a.sha256,lower(?1))>0 OR EXISTS(SELECT 1 FROM artifact_imports i WHERE i.artifact_sha256=a.sha256 AND instr(lower(i.original_filename),lower(?1))>0)) AND (?2 IS NULL OR d.integrity_status=?2) AND (?3 IS NULL OR EXISTS(SELECT 1 FROM artifact_imports i WHERE i.artifact_sha256=a.sha256 AND i.source_category=?3)) AND (?4 IS NULL OR d.first_imported_at>=?4) AND (?5 IS NULL OR d.first_imported_at<=?5)";
        let condition = format!("{condition} AND (?6 IS NULL OR {kind_sql}=?6)");
        let values = params![
            search,
            q.integrity,
            q.source,
            q.from_date,
            q.to_date,
            q.kind
        ];
        let total=self.connection.query_row(&format!("SELECT COUNT(*) FROM firmware_artifacts a JOIN artifact_details d USING(sha256){condition}"),values,|r|r.get(0))?;
        let unique_bytes = self.connection.query_row(
            "SELECT COALESCE(SUM(a.size_bytes),0) FROM firmware_artifacts a JOIN artifact_details d USING(sha256) WHERE d.removed_at IS NULL",
            [],
            |r| r.get(0),
        )?;
        let sql=format!("SELECT a.sha256,a.size_bytes,d.storage_status,d.integrity_status,d.first_imported_at,d.last_verified_at,d.format,(SELECT COUNT(*) FROM artifact_imports i WHERE i.artifact_sha256=a.sha256),COALESCE((SELECT display_name FROM artifact_presentation p WHERE p.sha256=a.sha256),(SELECT original_filename FROM artifact_imports i WHERE i.artifact_sha256=a.sha256 ORDER BY imported_at,import_id LIMIT 1),'Legacy artifact'),COALESCE((SELECT source_category FROM artifact_imports i WHERE i.artifact_sha256=a.sha256 ORDER BY imported_at,import_id LIMIT 1),'unknown'),COALESCE((SELECT kind FROM artifact_presentation p WHERE p.sha256=a.sha256),CASE (SELECT source_category FROM artifact_imports i WHERE i.artifact_sha256=a.sha256 ORDER BY imported_at,import_id LIMIT 1) WHEN 'user_import' THEN 'imported_original' WHEN 'official_vendor_download' THEN 'imported_original' WHEN 'authorized_device_acquisition' THEN 'extracted' WHEN 'generated_build' THEN 'build_output' ELSE 'unknown' END) FROM firmware_artifacts a JOIN artifact_details d USING(sha256){condition} ORDER BY {order} LIMIT 25 OFFSET {}",page*25);
        let mut s = self.connection.prepare(&sql)?;
        let artifacts = s
            .query_map(values, |r| {
                Ok(FirmwareArtifact {
                    sha256: r.get(0)?,
                    size_bytes: r.get(1)?,
                    storage_status: r.get(2)?,
                    integrity_status: r.get(3)?,
                    first_imported_at: r.get(4)?,
                    last_verified_at: r.get(5)?,
                    format: r.get(6)?,
                    source_count: r.get(7)?,
                    original_filename: r.get(8)?,
                    source_category: r.get(9)?,
                    kind: r.get(10)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(LibraryPage {
            artifacts,
            total,
            unique_bytes,
            page,
            page_size: 25,
            max_file_bytes: max,
        })
    }
    pub fn library_details(&self, hash: &str) -> Result<ArtifactDetails> {
        if !valid_hash(hash) {
            return Err(LibraryError::new("INVALID_HASH", "Invalid SHA-256 digest."));
        }
        let page = self.library_artifacts(
            &LibraryQuery {
                search: Some(hash.into()),
                ..Default::default()
            },
            0,
        )?;
        let artifact = page
            .artifacts
            .into_iter()
            .next()
            .ok_or_else(|| LibraryError::new("ARTIFACT_NOT_FOUND", "Artifact not found."))?;
        let mut s=self.connection.prepare("SELECT import_id,artifact_sha256,original_filename,imported_at,source_category,provenance_status,description,associated_device_id,job_id FROM artifact_imports WHERE artifact_sha256=?1 ORDER BY imported_at DESC,import_id DESC LIMIT 50")?;
        let imports = s
            .query_map([hash], |r| {
                Ok(FirmwareImport {
                    import_id: r.get(0)?,
                    artifact_sha256: r.get(1)?,
                    original_filename: r.get(2)?,
                    imported_at: r.get(3)?,
                    source_category: r.get(4)?,
                    provenance_status: r.get(5)?,
                    description: r.get(6)?,
                    associated_device_id: r.get(7)?,
                    job_id: r.get(8)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut s=self.connection.prepare("SELECT expected_size,actual_size,actual_sha256,status,verified_at,error_code FROM integrity_checks WHERE artifact_sha256=?1 ORDER BY id DESC LIMIT 50")?;
        let checks = s
            .query_map([hash], |r| {
                Ok(IntegrityResult {
                    artifact_sha256: hash.into(),
                    expected_size: r.get(0)?,
                    actual_size: r.get(1)?,
                    actual_sha256: r.get(2)?,
                    status: r.get(3)?,
                    verified_at: r.get(4)?,
                    error_code: r.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(ArtifactDetails {
            artifact,
            imports,
            checks,
        })
    }
    pub fn library_activity(&self) -> Result<Vec<ActivityEvent>> {
        let mut s=self.connection.prepare("SELECT a.id,a.kind,a.message,a.created_at,l.job_id,l.artifact_sha256,COALESCE(l.severity,'info') FROM audit_events a LEFT JOIN activity_links l ON l.event_id=a.id ORDER BY a.id DESC LIMIT 100")?;
        let rows = s
            .query_map([], |r| {
                Ok(ActivityEvent {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                    message: r.get(2)?,
                    created_at: r.get(3)?,
                    job_id: r.get(4)?,
                    artifact_sha256: r.get(5)?,
                    severity: r.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
    pub fn library_exported(&mut self, hash: &str) -> Result<()> {
        let tx = self.connection.transaction()?;
        event(
            &tx,
            "manifest_exported",
            "Artifact metadata manifest exported; unsigned checksum record only.",
            None,
            Some(hash),
            "info",
        )?;
        tx.commit()?;
        Ok(())
    }
}
