-- Preserve legacy artifact and job identities and status constraints.
CREATE TABLE artifact_details (
    sha256 TEXT PRIMARY KEY REFERENCES firmware_artifacts(sha256),
    storage_key TEXT NOT NULL,
    storage_status TEXT NOT NULL DEFAULT 'present',
    integrity_status TEXT NOT NULL DEFAULT 'not_checked',
    first_imported_at INTEGER NOT NULL,
    last_verified_at INTEGER,
    format TEXT NOT NULL DEFAULT 'UNKNOWN'
);
CREATE TABLE job_details (
    job_id TEXT PRIMARY KEY REFERENCES jobs(id),
    created_at INTEGER NOT NULL,
    started_at INTEGER,
    finished_at INTEGER,
    updated_at INTEGER NOT NULL,
    progress_bytes INTEGER NOT NULL DEFAULT 0,
    total_bytes INTEGER NOT NULL DEFAULT 0,
    stage TEXT NOT NULL DEFAULT 'queued',
    error_code TEXT,
    message TEXT NOT NULL DEFAULT '',
    artifact_sha256 TEXT,
    import_id TEXT,
    cancel_requested INTEGER NOT NULL DEFAULT 0,
    original_filename TEXT,
    description TEXT,
    device_id TEXT,
    duplicate INTEGER NOT NULL DEFAULT 0,
    recovery_info TEXT
);
CREATE TABLE artifact_imports (
    import_id TEXT PRIMARY KEY,
    artifact_sha256 TEXT NOT NULL REFERENCES firmware_artifacts(sha256),
    original_filename TEXT NOT NULL,
    imported_at INTEGER NOT NULL,
    source_category TEXT NOT NULL CHECK(source_category IN ('user_import','official_vendor_download','authorized_device_acquisition','generated_build','unknown')),
    provenance_status TEXT NOT NULL,
    description TEXT,
    associated_device_id TEXT,
    job_id TEXT NOT NULL UNIQUE REFERENCES jobs(id)
);
CREATE INDEX imports_by_artifact ON artifact_imports(artifact_sha256,imported_at DESC);
CREATE TABLE integrity_checks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    artifact_sha256 TEXT NOT NULL REFERENCES firmware_artifacts(sha256),
    job_id TEXT REFERENCES jobs(id),
    expected_size INTEGER NOT NULL,
    actual_size INTEGER,
    actual_sha256 TEXT,
    status TEXT NOT NULL,
    verified_at INTEGER NOT NULL,
    error_code TEXT
);
CREATE INDEX checks_by_artifact ON integrity_checks(artifact_sha256,id DESC);
CREATE TABLE activity_links (
    event_id INTEGER PRIMARY KEY REFERENCES audit_events(id),
    job_id TEXT REFERENCES jobs(id),
    artifact_sha256 TEXT,
    severity TEXT NOT NULL
);
CREATE INDEX jobs_by_created ON job_details(created_at DESC);
-- Historical minimal rows remain intact and explicitly lack a verified vault association.
INSERT INTO artifact_details SELECT sha256,'','unknown','not_checked',0,NULL,'UNKNOWN' FROM firmware_artifacts;
INSERT INTO job_details(job_id,created_at,updated_at,stage,message)
SELECT id,0,0,'legacy','Historical job retained; no Phase 4 execution metadata.' FROM jobs;
