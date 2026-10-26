# Stage 3 design: artifact classes and schema 6

## Current integration, 26 October 2026

The owner accepted Stage 4 software at `f9e711510522ecf5b2e935640052efe51c7679fd`, including its CI run `40273674930`, and released this separate Stage 3 workstream. Branch `phase-7-5-stage-3-artifact-completion` starts at that accepted commit. It reuses the accepted amendments at `b8bed2c2f93de0148dff9f6e12ce17ef13c2eaf5` and the earlier isolated Stage 3 implementation; the original branches remain intact. Earlier branch/base descriptions below are historical. Stage 4 acquisition/binding/CI source remains identical to the accepted base.

Implementation hardening stays within the accepted schema contract:

- The per-database mutex normalizes its identity through the nearest existing canonical ancestor, including Windows verbatim-path aliases and not-yet-created parents. This changes the lock key only; it does not relocate storage or create directories before consent.
- New TEXT identity keys are explicitly non-null. Hash constraints check SQLite type, character length, byte length and lowercase hexadecimal content, rejecting BLOB values and embedded-NUL suffixes. Nullable receipt/coverage fields remain explicitly nullable.
- BEFORE INSERT conflict guards protect immutable identity/history against `INSERT OR REPLACE` even with SQLite's default nonrecursive triggers. User import checks for an existing equal-size blob before inserting; sharing bytes never shares occurrence class or authority. Migrations 1 through 5 are unchanged.
- Abrupt-process tests cover death before commit, after commit and after checkpoint. The pending marker continues to require explicit recovery consent and verified retained backup; an ordinary restart cannot silently finish or admit workers.

No additional producer consumer, authenticated client, manifest-2 exporter, physical request or real-data migration is released. See the current [verification](PHASE_7_5_STAGE_3_VERIFICATION.md) and [data-preservation report](PHASE_7_5_STAGE_3_DATABASE_PRESERVATION.md). The accepted decisions below remain the implementation contract.

## Accepted implementation amendments, 24 October

This section and the reviewer decisions at the end supersede the historical proposal, SQL class lists, quarantine policy and online-backup wording below. The implementation release is conditional on writing these additions before code; no additional approval is inferred. Branch `phase-7-5-stage-3-artifacts` starts at accepted code `def018836166151c0b781add711dcaa5d50efbdb`. Stage 4 is delivered separately at `5817cfe61e3a5a7740ade968fd63e0e2890b6343`, accepted by the reviewer; it is not merged into this independent branch.

### Existing Remove, backup and restore behavior

Source: `apps/desktop/src-tauri/src/library/desktop.rs::LibraryService::remove_item` (lines 288–331 at the accepted base) implements the GUI Delete/Remove action. It updates `firmware_backups.removed_at` or `artifact_details.removed_at`, records an event and retains bytes/history; it never deletes `firmware_artifacts`, occurrences or admissions. Keep this tombstone behavior unchanged. Immutable/no-delete history triggers therefore do not remove a working user feature. Reimport can clear the presentation tombstone without changing an earlier admission or occurrence. Existing removal, backup, restore and duplicate-restore tests stay mandatory.

The existing Create Backup feature copies already admitted firmware; it is not a router configuration export. Restoring that same verified backup reinstates its existing firmware membership and records the existing restore job/import history. It neither needs a new firmware admission nor reclassifies a legacy occurrence. The restore path must check the retained hash/size/admission and cannot admit a previously nonfirmware blob. It must not manufacture a new `legacy_unclassified` occurrence. New configuration/report producer consumers and manifest-2 export consumers remain under Q5's separate contract review.

### Approved class and producer contract

The six new classes are `official_image_download`, `device_firmware_export`, `physical_flash_dump`, `user_imported_unverified`, `configuration_backup` and `metadata_report`. Historical `legacy_unclassified` remains migration-only. Firmware admission permits the first four new classes, plus retained legacy backfill. A new local choose-file import uses only sealed `UserImportProducer`, with `provenance_kind='user_import'` and display label **User-provided, unverified**. Class/occurrence IDs are not derived from filenames, extensions, model labels or deduplication. This assertion never grants official/device-acquired/verified status or installation authority.

Other producers are typed, sealed interfaces without production callers. Configuration/report occurrences cannot be changed into firmware, referenced by a firmware admission or used to mint a firmware receipt. Independent acquisitions may share verified content bytes, but require distinct immutable occurrences; hash reuse transfers no class or authority. New SQL CHECK/trigger lists must include the approved user-import class, require its user-import provenance, and retain the old `source_category` vocabulary in legacy tables. All production membership INSERTs move behind the sealed admission implementation; restore only checks existing membership. Legacy backfill runs before the migration-only insertion guard is installed.

### Derived-state drift check and repair

Source: `src/library/db.rs::commit_library_import`, `set_library_integrity` and `library_mark` (around lines 158–248) write legacy `artifact_details`. Preserve its original values. For a firmware blob with details, `artifact_blob_state` is a derived projection: same storage key, integrity value and verification time; storage maps `present` to `present`, `missing` to `missing`, everything else (including legacy `unreadable`) to `unknown`. No normalization rewrites the legacy row.

Check this projection before Library worker admission on every schema-6 startup and update it transactionally with firmware-state changes. Repair a missing/mismatched derived state from the retained `artifact_details` row only; never repair in the opposite direction or promote integrity. Missing details on a historical firmware row are allowed when both old and derived state are absent. Identity/admission/size conflicts, impossible references or derived firmware state without an authoritative details row block startup with a clear error; no guessed key, replacement blob or file read occurs. Tests deliberately drift derived state, prove exact repair and prove original rows remain unchanged.

### Invalid legacy hashes and refusal

Before backup or migration DDL, check every stored artifact SHA-256 and reference that will enter a new hash-constrained table. Values must be exactly 64 lowercase ASCII hexadecimal characters. Invalid values produce **Cannot upgrade library; existing data unchanged**. Do not lowercase, truncate, replace or discard them. Validate integrity, foreign keys and supported source schema as well. Temporary malformed schema-5 fixtures must retain original rows/version and show no schema-6 objects. No fixture is the owner's database or firmware.

### Native consent before any startup writes

Source: `src/lib.rs::run_profile` currently creates data subdirectories and a tracing writer before `Repository::open`, then installs profiles, starts Library workers, records startup and starts discovery. Move the upgrade decision before those filesystem/log/database writes and before creating the native WebView. Existing `startup::InstanceLock` uses a Windows named mutex, not a database/file write, and can still admit the process first.

For an existing supported pre-6 database, inspect the main header as ordinary bytes and show one native, default-No confirmation naming the unique backup destination under the existing application `backups` directory and explaining the one-way upgrade/old-version incompatibility. No means a quiet successful process exit: no SQLite open, directories/logging/profile installation, worker recovery, discovery or settings change. Yes authorizes only this local backup/migration. A genuinely new empty database may initialize schema 6 directly. Newer/unsupported schemas are refused. Temporary tests supply controlled Yes/No decisions and compare complete directory bytes for No; no native dialog is auto-answered for normal data.

`Repository::open` must not become a silent migration bypass: an existing older database requires the migration driver; ordinary open refuses it before SQLite. Fresh initialization and already-complete schema 6 remain supported. The production app must not be launched against normal data in verification, and schema-6 code cannot reach `main` until the owner explicitly accepts migrating their real database.

### Durable backup, transaction and incomplete recovery

Use `VACUUM INTO`, not the rusqlite backup feature, with a uniquely reserved candidate file. Verify integrity, foreign keys, source schema and a complete logical snapshot against the source; close handles, fsync the candidate, hash it and publish by a same-directory rename that cannot replace another file. Do not start migration DDL until the verified durable backup exists. Existing app writers are excluded by startup ownership and the pre-worker admission gate. After acquiring the SQLite migration transaction, compare the source again to the verified snapshot: a concurrent external write between backup and transaction aborts the upgrade rather than migrating unbacked data.

The single migration transaction adds the approved tables/triggers, backfills legacy envelopes and verifies every original table's values/IDs and associations before setting version 6. Retain a small additive migration receipt with source version and the canonical backup basename/hash, committed with the schema; it supports retry after a post-commit/pre-checkpoint crash without guessing or overwriting a backup. Initial fresh schema-6 creation requires no legacy backup receipt. Backup copies include committed WAL data; test that explicitly.

After commit, require non-busy `PRAGMA wal_checkpoint(TRUNCATE)` and raw main-header version 6 before any worker, profile installation or startup recording. A blocked checkpoint is **Library upgrade incomplete**, with data/backup retained and workers unavailable; never recommend an older binary. On a subsequent explicitly approved recovery, validate the schema and retained backup receipt, then retry the checkpoint before admitting workers. A raw main-header 6 is protected by the existing old-version preflight; the existing limitation for a newer version visible only in WAL remains documented. Do not add a speculative WAL parser.

Implementation detail for that admission barrier: a busy TRUNCATE can coexist with an already-checkpointed main header 6. Header equality alone must not allow a later ordinary open to bypass the incomplete state. Persist an application-owned pending marker before committing schema changes, serialize upgrades with a per-database Windows named mutex, and retain the marker across post-commit failure. Only the operation owning its generation may clear it after all checks succeed. No leaves an existing marker untouched. Recovery validates the retained receipt/backup; a pre-commit restart may create a fresh approved backup without deleting the earlier copy. A genuinely fresh initialization records an explicit no-legacy-backup receipt. These are local metadata only and create no firmware admission.

Failure injection covers candidate creation, VACUUM, verification, flush, publication, transaction start, DDL/backfill, validation, version update, commit and checkpoint. Test source-write races, duplicate bytes across classes, every CHECK/FK/trigger boundary, drift repair, declined consent, restart and old-version refusal using temporary profiles only. Source values/IDs and logical rollback are promised; physical file identity after committed SQLite activity is not.

Manifest-1 and its GUI consumer remain compatible. Define/test the versioned class/provenance/coverage manifest contract locally, without wiring a new export consumer before Q5 review. No new configuration/report consumer is released here. These boundaries satisfy the implementation release without silently overriding its separate contract-review gate.

23 October 2026, Africa/Lagos. PROPOSED; documentation only. No migration SQL was executed. Approval of this document alone releases no implementation or access to normal application data.

## Evidence and current state

`R` means confirmed by reading repository source at accepted base `ee3e53d`; `O` means confirmed by official documentation retrieved 23 October 2026; `P` means proposed policy; `U` means unknown pending qualification. No community claims are used.

| Class | Finding |
| --- | --- |
| R | `apps/desktop/src-tauri/migrations/001_foundation.sql:17` defines firmware_artifacts(sha256 PK, nonnegative size); `:21` jobs and its status CHECK. |
| R | `migrations/004_firmware_library.sql:2`, `:11`, `:31` define artifact_details, job_details and artifact_imports; imports reference firmware_artifacts and jobs, with a unique job_id and source_category CHECK restricted to user_import, official_vendor_download, authorized_device_acquisition, generated_build, unknown. `:43` integrity_checks and `:55` activity_links retain firmware hashes. |
| R | `migrations/005_desktop_workspace.sql:3` artifact_presentation and `:8` firmware_backups reference firmware hashes; presentation kind is display metadata, not a content class. |
| R | `src/library/db.rs:158` commit_library_import inserts every admitted import into firmware_artifacts and records user_import. `:249` library_artifacts_matching, `:288` kind_sql, `:323` counts, `:329` list and `:358` exact details treat that table as the firmware universe. `src/storage.rs:367` dashboard counts do too. `src/library/mod.rs:493` manifest schema 1 has no class. These are the concrete every-artifact-is-firmware assumptions. Paths in this table are under apps/desktop/src-tauri. |
| O | SQLite [Online Backup API](https://sqlite.org/backup.html) supports a consistent destination snapshot; [VACUUM INTO](https://www.sqlite.org/lang_vacuum.html) produces a vacuumed snapshot whose interrupted output can be incomplete. Proposed choice: online backup with explicit completion and verification, not a raw live-file copy. |

## Classes, producers and admission

P: five new classes are official_image_download, device_firmware_export, physical_flash_dump, configuration_backup and metadata_report. Only the first three may create new firmware_artifacts rows or enter firmware counts. Existing rows retain all original values/IDs and receive an additive legacy_unclassified envelope; that exception preserves legacy counts without guessing content. authorized_device_acquisition does not prove a dump/export class. Raw local file import remains legacy_unclassified only for historical records; new unclassified imports need a separately approved quarantine UX before implementation, never an automatic firmware promotion.

P: a sealed Rust producer returns a class-specific receipt containing the verified blob/hash and acquisition occurrence. Caller labels, MIME, extension, source_category, presentation kind and SHA deduplication cannot mint that receipt. Configuration/report producers have no firmware-admission method. Database constraints and immutable association triggers independently reject cross-class linking. Equal bytes may share artifact_blobs, but each acquisition has its own immutable occurrence; a configuration occurrence cannot be used as firmware admission. An independently qualified firmware producer may separately acquire identical bytes and create its own firmware occurrence: deduplication is storage reuse, never authority.

## Proposed complete additive SQL

P: execute inside the single migration transaction described below. Existing tables/columns/indexes are not renamed, rebuilt, dropped or rewritten. Existing source_category CHECK stays unchanged. Old field values and IDs remain byte-for-byte equal as values; SQLite page/file identity is not promised after a committed migration.

```sql
PRAGMA foreign_keys=ON;
PRAGMA synchronous=FULL;
BEGIN IMMEDIATE;
CREATE TABLE artifact_blobs (
  sha256 TEXT PRIMARY KEY CHECK(length(sha256)=64 AND sha256 NOT GLOB '*[^0-9a-f]*'),
  size_bytes INTEGER NOT NULL CHECK(size_bytes>=0)
);
CREATE TABLE artifact_occurrences (
  occurrence_id TEXT PRIMARY KEY,
  blob_sha256 TEXT NOT NULL REFERENCES artifact_blobs(sha256),
  artifact_class TEXT NOT NULL CHECK(artifact_class IN
    ('official_image_download','device_firmware_export','physical_flash_dump',
     'configuration_backup','metadata_report','legacy_unclassified')),
  provenance_kind TEXT NOT NULL CHECK(length(provenance_kind) BETWEEN 1 AND 80),
  source_reference TEXT,
  acquisition_occurrence_id TEXT NOT NULL UNIQUE,
  device_id TEXT REFERENCES devices(id),
  job_id TEXT REFERENCES jobs(id),
  collected_at INTEGER NOT NULL CHECK(collected_at>=0),
  coverage_total_bytes INTEGER CHECK(coverage_total_bytes>=0),
  coverage_observed_bytes INTEGER CHECK(coverage_observed_bytes>=0),
  coverage_manifest_sha256 TEXT CHECK(coverage_manifest_sha256 IS NULL OR
    (length(coverage_manifest_sha256)=64 AND coverage_manifest_sha256 NOT GLOB '*[^0-9a-f]*')),
  coverage_complete INTEGER NOT NULL DEFAULT 0 CHECK(coverage_complete IN (0,1)),
  installability TEXT NOT NULL DEFAULT 'unknown' CHECK(installability='unknown'),
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
  sha256 TEXT PRIMARY KEY REFERENCES artifact_blobs(sha256),
  storage_key TEXT NOT NULL,
  storage_status TEXT NOT NULL CHECK(storage_status IN ('present','missing','unknown')),
  integrity_status TEXT NOT NULL,
  last_verified_at INTEGER
);
CREATE TABLE firmware_admissions (
  sha256 TEXT PRIMARY KEY REFERENCES artifact_blobs(sha256),
  occurrence_id TEXT NOT NULL UNIQUE REFERENCES artifact_occurrences(occurrence_id),
  artifact_class TEXT NOT NULL CHECK(artifact_class IN
    ('official_image_download','device_firmware_export','physical_flash_dump','legacy_unclassified'))
);
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
CREATE TRIGGER admission_validate BEFORE INSERT ON firmware_admissions BEGIN
  SELECT CASE WHEN NEW.artifact_class='legacy_unclassified' OR NOT EXISTS
    (SELECT 1 FROM artifact_occurrences o WHERE o.occurrence_id=NEW.occurrence_id
     AND o.blob_sha256=NEW.sha256 AND o.artifact_class=NEW.artifact_class
     AND o.artifact_class IN ('official_image_download','device_firmware_export','physical_flash_dump'))
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
     AND f.artifact_class IN ('official_image_download','device_firmware_export','physical_flash_dump'))
    THEN RAISE(ABORT,'firmware admission missing') END;
END;
CREATE TRIGGER firmware_identity_immutable BEFORE UPDATE ON firmware_artifacts BEGIN
  SELECT RAISE(ABORT,'firmware blob identity is immutable');
END;
CREATE VIEW firmware_classified AS
SELECT a.sha256,a.size_bytes,f.artifact_class,f.occurrence_id
FROM firmware_artifacts a JOIN firmware_admissions f USING(sha256);
CREATE VIEW nonfirmware_occurrences AS
SELECT * FROM artifact_occurrences
WHERE artifact_class IN ('configuration_backup','metadata_report');
-- Migration driver must inspect these result rows and the application-level
-- legacy-value comparisons, rolling back on any mismatch before proceeding.
PRAGMA integrity_check;
PRAGMA foreign_key_check;
-- Execute only after all comparison/check assertions succeed:
PRAGMA user_version=6;
COMMIT;
```

P: before schema bump, compare every legacy table row/value/ID using canonical ordered value serialization, old hash/size pairs, counts and foreign-key results. No automatic FK to associated_device_id is backfilled because historical references can be absent; legacy source_reference preserves the import id without modifying that row. New device references must exist or be null, never guessed. Provenance authenticity is separate from class and installability. Neutral blob state supports nonfirmware availability without inserting into firmware-only artifact_details; unknown legacy storage states are normalized only in the new derived state, never in original rows. Synchronize derived blob state and legacy firmware state transactionally for firmware operations.

P: future firmware producers record the existing legacy source_category vocabulary (for example official_vendor_download) separately from the new class; do not put a new class string into that old CHECK. On a duplicate existing firmware hash, skip membership INSERT entirely rather than relying on INSERT OR IGNORE, whose BEFORE trigger still runs. Record the separately qualified occurrence without rewriting the original admission/class/presentation. Additional occurrence classes can be displayed beside the retained primary membership class; they never reclassify the historical row. New nonfirmware producers create only neutral blob/state/occurrence rows. Retaining working arbitrary local imports requires a separately approved quarantine/import policy before migrating a production app; migration is not releasable while that consumer contract remains unresolved.

## Admission, backup, transaction and crashes

P: migration occurs before Library::start/worker recovery, normal startup recording or discovery writes. Acquire existing single-instance/workspace ownership and a new shared admission gate used by every Library/storage worker; drain active workers, prevent new jobs and obtain SQLite write admission. Reject unknown schema, invalid integrity/FKs, illegal legacy hashes or missing backup space without starting migration. A schema-5 fixture violating the new hash check is refused, not corrected. No test opens the owner's database.

P: with application writers quiesced, open a uniquely named backup destination in the existing backup directory; use SQLite online backup until SQLITE_DONE, finish, close, flush destination, reopen read-only and check integrity/FKs and logical snapshot equality. Hash/record the backup; interrupted/failed candidates are never marked usable. No transaction starts until a verified durable backup exists. Current rusqlite backup-feature availability must be checked at implementation; no dependency change is authorized now.

P: BEGIN IMMEDIATE with foreign_keys ON and synchronous FULL, execute SQL, verify old rows and new associations, integrity_check and foreign_key_check, then set user_version=6 inside that transaction and commit. Verify after reopen before permitting workers. A WAL checkpoint to durable main-header schema 6 is a mandatory downgrade-protection step before declaring success or starting other writes. If checkpoint is blocked, stay unavailable with all data retained and do not offer an old binary. Unexpected crash before checkpoint is governed by the existing WAL-aware post-open check, whose raw-header limitations are stated below.

P: crash before backup completion leaves original unmodified and an unusable candidate; after verified backup/before transaction leaves schema 5; crash during transaction recovers the old logical state; commit is atomic, so crash after commit yields schema 6 plus retained backup. A failed SQL transaction rolls back all added objects/data. SQLite journal/WAL bytes and lock artifacts can differ even when logical state is restored; promising identical file bytes after arbitrary SQLite writes would be false. If literal physical-byte rollback is required, reviewer must approve an offline verified backup restoration procedure, retaining the failed database and sidecars rather than overwriting them in place.

## Downgrade and Library impact

R: `src/storage.rs:11` header_schema_version reads raw main-header offset 60 without SQLite; Repository::open `:37` refuses a newer main-header version before SQLite open. Tests at `:563`, `:639`, `:698` cover schema-6 refusal, WAL and WAL-only newer version. A version visible only in WAL is detected after open; do not claim arbitrary WAL-only recovery never creates sidecars. Successful migration must checkpoint main schema 6, and downgrade qualification must exercise both paths.

P: downgrade means close the app, preserve schema-6 DB/WAL/SHM plus vault, and restore the verified schema-5 backup through a separately reviewed offline recovery flow. New post-migration records do not exist in that snapshot; retain them for later schema-6 reopening/export. No class coercion or lossy reverse SQL migration. Existing Library firmware queries/counts keep their legacy membership, adding class through firmware_classified. Configuration/report views use nonfirmware_occurrences and never firmware_artifacts. Existing import source counts remain; a distinct occurrence count avoids double-counting envelope backfill. Schema-1 manifest stays unchanged for existing callers; a separately versioned manifest-2 exports typed occurrence/provenance/class/coverage with privacy redaction and no installation claim. Metadata files/configuration use the same managed storage infrastructure, never a second database or vault.

## Planned tests (not executed)

| Test on temporary fixture storage only | Required assertion |
| --- | --- |
| Populated schema 5, all CHECK/FK cases and orphan historical labels | Every original ordered value/ID/hash/count unchanged; new class legacy_unclassified; no inference. |
| Fail backup allocation/step/finish/flush/verification | Original schema/data unchanged; no migration; partial backup not offered. |
| Inject failure/cancel/crash before/after every transaction stage and checkpoint | Old or complete new logical schema only; no mixed objects; recovery blocks worker admission until verified. |
| Competing import/relocation/second process | Exclusive gate prevents migration/work overlap; no normal path accessed. |
| Equal configuration/metadata/firmware bytes | Shared blob allowed; nonfirmware occurrence cannot create/relabel admission or affect firmware counts. Direct SQL trigger checks reject bypasses. |
| Invalid coverage, missing device/job, malformed hashes | CHECK/FK rejection with complete transaction rollback. |
| Old binary main-header 6 and WAL-only 6 | Refusal behavior matches G1 tests; successful main-header case leaves complete directory bytes unchanged. |
| Restore verified backup after new schema-6 records | Old logical rows/settings exact; later records retained in archived schema-6 dataset; vault not silently deleted. |
| Restart/cancel during producer commit | No successful occurrence without verified blob; no duplicate occurrence or stale success. |

## Open questions

| ID | Proposed answer for reviewer |
| --- | --- |
| Q1 | Accept additive blob/occurrence/admission tables and legacy-only backfill instead of rebuilding legacy tables? Yes. |
| Q2 | New arbitrary local imports lack a trustworthy class. Release quarantine UX separately; do not classify by filename or force into one of five classes. |
| Q3 | Does preservation mean original field bytes/IDs and logical rollback, rather than impossible universal physical-file identity after SQLite activity? Proposed yes; backup restoration is separate. |
| Q4 | Accept synchronous FULL plus verified online backup and mandatory main-header checkpoint before worker admission? Proposed yes; WAL-only refusal limitation remains explicit. |
| Q5 | New backup/config/report producer APIs and manifest-2 need a follow-up contract review before consumers change. Proposed yes. |

Scope: design only. Files changed: this document and its PHASE_STATUS.md entry only. Checks: source/citation/section coverage and `git diff --check`; no SQL execution, application suites, native launch, network/device request or owner-data access for this document. Still blocked after design approval: all Stage 3 code until separate release, Stages 4–6 code, authenticated flow/hardware and Phase 8. Handoff: `git diff phase-7-5-stage-2a-auth-design..HEAD -- docs/PHASE_7_5_STAGE_3_ARTIFACT_SCHEMA_DESIGN.md docs/PHASE_STATUS.md` on branch phase-7-5-design-track.


## Reviewer decisions, 24 October

Design accepted. These decisions supersede the proposals and open questions above.

- **Q1 yes**: additive blob, occurrence and admission tables with legacy-only backfill.
- **Q2 DECIDED — no quarantine UX; add a sixth firmware-admissible class `user_imported_unverified`.** New local imports through the Library's existing "choose a file" workflow are a deliberate user assertion that the file is firmware. They get class `user_imported_unverified`, `provenance_kind='user_import'`, shown as "User-provided, unverified". It is admissible to `firmware_admissions` like the other firmware classes, but it can never be treated as official, device-acquired or verified, and a configuration backup or metadata report still cannot become any firmware class by any path. `legacy_unclassified` stays migration-only for historical rows. The old import code path is replaced by a sealed `UserImportProducer`; no code may insert into `firmware_artifacts` directly.
- **Q3 yes**: preservation means original values and IDs plus logical rollback, not physical file identity.
- **Q4 yes**, with an addition: after commit run `PRAGMA wal_checkpoint(TRUNCATE)` and require a non-busy result and a main-file header version of 6 before any worker is admitted; if blocked, the app stays in a clear "library upgrade incomplete" state and never offers an old binary.
- **Q5 yes**: producer APIs for backup, report and manifest-2 need their own contract review before consumers change.
- **Backup method DECIDED:** use `VACUUM INTO` a uniquely named file, then verify (integrity check, foreign-key check, logical comparison), fsync, and publish by rename. Do not enable the rusqlite `backup` feature unless source reading shows it is already enabled. Confirm `VACUUM INTO` behaves correctly on a WAL-mode database in a test.
- **Additions required in the design before code:** (1) cite whether the Library has any delete or remove-artifact feature; the immutability and no-delete triggers must not break an existing user feature, and if one exists the design states how it behaves. (2) A startup drift check that `artifact_blob_state` and `artifact_details` agree for firmware blobs, with a repair rule. (3) A pre-migration check for any stored hash that violates the new lowercase-hex CHECK; the result is a clear "cannot upgrade, data unchanged" message, never an automatic correction. (4) **Owner consent gate:** the migration must not run silently. At startup, a native confirmation dialog names the backup location and the one-way nature of the upgrade; Yes performs backup then migration; No exits the app with data untouched, so the old binary still works. Tests cover both answers.
- **Release condition:** schema-6 code never touches the owner's real database in any test. It must not reach `main` before the owner has said they accept migrating their real database; the reviewer holds the merge for that decision.

## Implementation contract boundary, 24 October

`library/manifest_contract.rs` defines and tests the proposed manifest-2 field contract (version, hash/size, class, separate class/provenance/coverage occurrences, omission count, unknown installability). It rejects extra fields, private paths/device identifiers/credentials, unsupported versions and invalid complete-dump coverage. It has no export or producer consumer. The Library still exports manifest 1 through its existing unchanged path. Reserved sealed producer contracts likewise expose no new production acquisition entry point. Q5 contract review remains required before either kind of consumer is wired. See [implementation evidence](PHASE_7_5_STAGE_3_VERIFICATION.md); historical proposal sections retain their original review context.
