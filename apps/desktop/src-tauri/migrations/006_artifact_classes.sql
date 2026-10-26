-- Additive classification. Existing rows, IDs and presentation values are retained.
CREATE TABLE artifact_blobs (
  sha256 TEXT PRIMARY KEY NOT NULL CHECK(typeof(sha256)='text' AND length(sha256)=64 AND length(CAST(sha256 AS BLOB))=64 AND sha256 NOT GLOB '*[^0-9a-f]*'),
  size_bytes INTEGER NOT NULL CHECK(size_bytes>=0)
);
CREATE TABLE artifact_occurrences (
  occurrence_id TEXT PRIMARY KEY NOT NULL,
  blob_sha256 TEXT NOT NULL REFERENCES artifact_blobs(sha256),
  artifact_class TEXT NOT NULL CHECK(artifact_class IN
    ('official_image_download','device_firmware_export','physical_flash_dump',
     'user_imported_unverified','configuration_backup','metadata_report','legacy_unclassified')),
  provenance_kind TEXT NOT NULL CHECK(length(provenance_kind) BETWEEN 1 AND 80),
  source_reference TEXT,
  acquisition_occurrence_id TEXT NOT NULL UNIQUE,
  device_id TEXT REFERENCES devices(id),
  job_id TEXT REFERENCES jobs(id),
  collected_at INTEGER NOT NULL CHECK(collected_at>=0),
  coverage_total_bytes INTEGER CHECK(coverage_total_bytes>=0),
  coverage_observed_bytes INTEGER CHECK(coverage_observed_bytes>=0),
  coverage_manifest_sha256 TEXT CHECK(coverage_manifest_sha256 IS NULL OR
    (typeof(coverage_manifest_sha256)='text' AND length(coverage_manifest_sha256)=64 AND length(CAST(coverage_manifest_sha256 AS BLOB))=64 AND coverage_manifest_sha256 NOT GLOB '*[^0-9a-f]*')),
  coverage_complete INTEGER NOT NULL DEFAULT 0 CHECK(coverage_complete IN (0,1)),
  installability TEXT NOT NULL DEFAULT 'unknown' CHECK(installability='unknown'),
  CHECK(artifact_class!='user_imported_unverified' OR provenance_kind='user_import'),
  CHECK(coverage_observed_bytes IS NULL OR
    (coverage_total_bytes IS NOT NULL AND coverage_observed_bytes<=coverage_total_bytes)),
  CHECK(artifact_class='physical_flash_dump' OR
    (coverage_total_bytes IS NULL AND coverage_observed_bytes IS NULL AND
     coverage_manifest_sha256 IS NULL AND coverage_complete=0)),
  CHECK(coverage_complete=0 OR
    (coverage_total_bytes IS NOT NULL AND coverage_total_bytes>0 AND
     coverage_observed_bytes IS NOT NULL AND coverage_observed_bytes=coverage_total_bytes AND
     coverage_manifest_sha256 IS NOT NULL))
);
CREATE INDEX occurrences_by_blob ON artifact_occurrences(blob_sha256,collected_at);
CREATE INDEX occurrences_by_class ON artifact_occurrences(artifact_class,collected_at);
CREATE INDEX occurrences_by_device ON artifact_occurrences(device_id,collected_at);
CREATE INDEX occurrences_by_job ON artifact_occurrences(job_id);
CREATE TABLE artifact_blob_state (
  sha256 TEXT PRIMARY KEY NOT NULL REFERENCES artifact_blobs(sha256),
  storage_key TEXT NOT NULL,
  storage_status TEXT NOT NULL CHECK(storage_status IN ('present','missing','unknown')),
  integrity_status TEXT NOT NULL,
  last_verified_at INTEGER
);
CREATE TABLE firmware_admissions (
  sha256 TEXT PRIMARY KEY NOT NULL REFERENCES artifact_blobs(sha256),
  occurrence_id TEXT NOT NULL UNIQUE REFERENCES artifact_occurrences(occurrence_id),
  artifact_class TEXT NOT NULL CHECK(artifact_class IN
    ('official_image_download','device_firmware_export','physical_flash_dump',
     'user_imported_unverified','legacy_unclassified'))
);
CREATE TABLE artifact_schema_receipt (
  schema_version INTEGER PRIMARY KEY CHECK(schema_version=6),
  source_version INTEGER CHECK(source_version BETWEEN 0 AND 5),
  backup_filename TEXT CHECK(length(backup_filename) BETWEEN 1 AND 100 AND
    instr(backup_filename,'/')=0 AND instr(backup_filename,'\')=0 AND instr(backup_filename,':')=0),
  backup_sha256 TEXT CHECK(backup_sha256 IS NULL OR (typeof(backup_sha256)='text' AND length(backup_sha256)=64 AND length(CAST(backup_sha256 AS BLOB))=64 AND backup_sha256 NOT GLOB '*[^0-9a-f]*')),
  backup_bytes INTEGER CHECK(backup_bytes>0),
  created_at INTEGER NOT NULL CHECK(created_at>=0),
  CHECK((source_version IS NULL AND backup_filename IS NULL AND backup_sha256 IS NULL AND backup_bytes IS NULL)
    OR (source_version IS NOT NULL AND backup_filename IS NOT NULL AND backup_sha256 IS NOT NULL AND backup_bytes IS NOT NULL))
);

-- CLASS BACKFILL
INSERT INTO artifact_blobs SELECT sha256,size_bytes FROM firmware_artifacts;
INSERT INTO artifact_blob_state
SELECT sha256,storage_key,
  CASE WHEN storage_status='present' THEN 'present'
       WHEN storage_status='missing' THEN 'missing' ELSE 'unknown' END,
  integrity_status,last_verified_at FROM artifact_details;
INSERT INTO artifact_occurrences
  (occurrence_id,blob_sha256,artifact_class,provenance_kind,source_reference,
   acquisition_occurrence_id,collected_at)
SELECT 'legacy-artifact:'||sha256,sha256,'legacy_unclassified','legacy_unclassified',
  NULL,'legacy-artifact:'||sha256,0 FROM firmware_artifacts;
INSERT INTO artifact_occurrences
  (occurrence_id,blob_sha256,artifact_class,provenance_kind,source_reference,
   acquisition_occurrence_id,job_id,collected_at)
SELECT 'legacy-import:'||import_id,artifact_sha256,'legacy_unclassified',
  source_category,'legacy-import:'||import_id,'legacy-import:'||import_id,job_id,imported_at
FROM artifact_imports;
INSERT INTO firmware_admissions
SELECT sha256,'legacy-artifact:'||sha256,'legacy_unclassified' FROM firmware_artifacts;

-- CLASS GUARDS
-- REPLACE may skip DELETE triggers when recursive_triggers is off. Reject
-- conflicting identities at INSERT too; application deduplication checks first.
CREATE TRIGGER blob_no_replace BEFORE INSERT ON artifact_blobs
WHEN EXISTS(SELECT 1 FROM artifact_blobs WHERE sha256=NEW.sha256) BEGIN
  SELECT RAISE(ABORT,'blob identity is already retained');
END;
CREATE TRIGGER occurrence_no_replace BEFORE INSERT ON artifact_occurrences
WHEN EXISTS(SELECT 1 FROM artifact_occurrences WHERE occurrence_id=NEW.occurrence_id
  OR acquisition_occurrence_id=NEW.acquisition_occurrence_id) BEGIN
  SELECT RAISE(ABORT,'occurrence identity is already retained');
END;
CREATE TRIGGER admission_no_replace BEFORE INSERT ON firmware_admissions
WHEN EXISTS(SELECT 1 FROM firmware_admissions WHERE sha256=NEW.sha256 OR occurrence_id=NEW.occurrence_id) BEGIN
  SELECT RAISE(ABORT,'firmware admission is already retained');
END;
CREATE TRIGGER firmware_no_replace BEFORE INSERT ON firmware_artifacts
WHEN EXISTS(SELECT 1 FROM firmware_artifacts WHERE sha256=NEW.sha256) BEGIN
  SELECT RAISE(ABORT,'firmware identity is already retained');
END;
CREATE TRIGGER schema_receipt_no_replace BEFORE INSERT ON artifact_schema_receipt
WHEN EXISTS(SELECT 1 FROM artifact_schema_receipt WHERE schema_version=NEW.schema_version) BEGIN
  SELECT RAISE(ABORT,'migration receipt is already retained');
END;
CREATE TRIGGER occurrence_no_legacy_insert BEFORE INSERT ON artifact_occurrences
WHEN NEW.artifact_class='legacy_unclassified' BEGIN
  SELECT RAISE(ABORT,'legacy classification is migration-only');
END;
CREATE TRIGGER occurrence_immutable BEFORE UPDATE ON artifact_occurrences BEGIN
  SELECT RAISE(ABORT,'occurrence is immutable');
END;
CREATE TRIGGER occurrence_no_delete BEFORE DELETE ON artifact_occurrences BEGIN
  SELECT RAISE(ABORT,'occurrence history is retained');
END;
CREATE TRIGGER blob_immutable BEFORE UPDATE ON artifact_blobs BEGIN
  SELECT RAISE(ABORT,'blob metadata is immutable');
END;
CREATE TRIGGER blob_no_delete BEFORE DELETE ON artifact_blobs BEGIN
  SELECT RAISE(ABORT,'blob identity history is retained');
END;
CREATE TRIGGER admission_validate BEFORE INSERT ON firmware_admissions BEGIN
  SELECT CASE WHEN NEW.artifact_class='legacy_unclassified' OR NOT EXISTS
    (SELECT 1 FROM artifact_occurrences o WHERE o.occurrence_id=NEW.occurrence_id
     AND o.blob_sha256=NEW.sha256 AND o.artifact_class=NEW.artifact_class
     AND o.artifact_class IN ('official_image_download','device_firmware_export','physical_flash_dump','user_imported_unverified'))
    THEN RAISE(ABORT,'firmware requires a separate firmware occurrence') END;
END;
CREATE TRIGGER admission_immutable BEFORE UPDATE ON firmware_admissions BEGIN
  SELECT RAISE(ABORT,'firmware admission is immutable');
END;
CREATE TRIGGER admission_no_delete BEFORE DELETE ON firmware_admissions BEGIN
  SELECT RAISE(ABORT,'firmware admission history is retained');
END;
CREATE TRIGGER firmware_requires_admission BEFORE INSERT ON firmware_artifacts BEGIN
  SELECT CASE WHEN NOT EXISTS
    (SELECT 1 FROM firmware_admissions f JOIN artifact_blobs b USING(sha256)
     WHERE f.sha256=NEW.sha256 AND b.size_bytes=NEW.size_bytes
     AND f.artifact_class IN ('official_image_download','device_firmware_export','physical_flash_dump','user_imported_unverified'))
    THEN RAISE(ABORT,'firmware admission missing') END;
END;
CREATE TRIGGER firmware_identity_immutable BEFORE UPDATE ON firmware_artifacts BEGIN
  SELECT RAISE(ABORT,'firmware blob identity is immutable');
END;
CREATE TRIGGER firmware_identity_no_delete BEFORE DELETE ON firmware_artifacts BEGIN
  SELECT RAISE(ABORT,'remove uses a presentation tombstone; history is retained');
END;
CREATE TRIGGER schema_receipt_immutable BEFORE UPDATE ON artifact_schema_receipt BEGIN
  SELECT RAISE(ABORT,'migration receipt is immutable');
END;
CREATE TRIGGER schema_receipt_no_delete BEFORE DELETE ON artifact_schema_receipt BEGIN
  SELECT RAISE(ABORT,'migration receipt is retained');
END;

-- Original firmware state remains authoritative. Keep the new projection in the same transaction.
CREATE TRIGGER firmware_state_insert AFTER INSERT ON artifact_details BEGIN
  INSERT INTO artifact_blob_state VALUES(NEW.sha256,NEW.storage_key,
    CASE WHEN NEW.storage_status='present' THEN 'present' WHEN NEW.storage_status='missing' THEN 'missing' ELSE 'unknown' END,
    NEW.integrity_status,NEW.last_verified_at)
  ON CONFLICT(sha256) DO UPDATE SET storage_key=excluded.storage_key,storage_status=excluded.storage_status,
    integrity_status=excluded.integrity_status,last_verified_at=excluded.last_verified_at;
END;
CREATE TRIGGER firmware_state_update AFTER UPDATE OF storage_key,storage_status,integrity_status,last_verified_at ON artifact_details BEGIN
  INSERT INTO artifact_blob_state VALUES(NEW.sha256,NEW.storage_key,
    CASE WHEN NEW.storage_status='present' THEN 'present' WHEN NEW.storage_status='missing' THEN 'missing' ELSE 'unknown' END,
    NEW.integrity_status,NEW.last_verified_at)
  ON CONFLICT(sha256) DO UPDATE SET storage_key=excluded.storage_key,storage_status=excluded.storage_status,
    integrity_status=excluded.integrity_status,last_verified_at=excluded.last_verified_at;
END;
CREATE VIEW firmware_classified AS
SELECT a.sha256,a.size_bytes,f.artifact_class,f.occurrence_id
FROM firmware_artifacts a JOIN firmware_admissions f USING(sha256);
CREATE VIEW nonfirmware_occurrences AS
SELECT * FROM artifact_occurrences WHERE artifact_class IN ('configuration_backup','metadata_report');
