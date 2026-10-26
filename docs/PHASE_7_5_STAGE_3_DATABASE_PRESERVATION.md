# Stage 3 database-preservation report

26 October 2026. Candidate branch `phase-7-5-stage-3-artifact-completion`; accepted base `f9e711510522ecf5b2e935640052efe51c7679fd`. Independent source review and real-data migration acceptance are separate gates. Neither is implied by isolated test success.

## Normal data: ordinary file bytes only

Before implementation, `py scripts/audit-data-bytes.py .tools/stage3-followup/before.json` ran from the preserved `.tools/stage3-artifacts` worktree. The script resolves Windows LocalAppData, refuses reparse traversal and hashes ordinary files. It does not import SQLite or open a database connection.

Baseline: **12 files, 5,611,701 bytes** under normal `%LOCALAPPDATA%\OBTAINER`: database, WAL, SHM, seven logs, vault lock and the retained original 1 MiB firmware file. Preferences and saved-device records remain inside those byte-identical database files. No record contents or credentials are exported by the audit.

Follow-ups `after-source-checks.json` and `after-native.json` in the same ignored evidence directory compare exactly with `before.json`, exit 0. Exact native follow-up command:

```powershell
py scripts/audit-data-bytes.py .tools/stage3-followup/after-native.json .tools/stage3-followup/before.json
```

The native harness additionally writes `normal-data-before.json` and `normal-data-after.json` and checks their complete file maps. Final delivery repeats this byte audit after the final committed-code repeat suite. This proves unchanged file bytes, not logical integrity, a fresh database snapshot or independently verified device identity. The normal database was never connected to, checkpointed, migrated or opened by the candidate app. The normal vault, Library location, backups, settings and saved entries were not changed.

The shared normal `obtainer.exe` SHA-256 remained `0278DDB4776A03458A4C24C0C219A7B0C32F74FAE264D6D549883C94F4780C65`. The dedicated test build verified that safeguard. No new production release build or normal production launch was performed for Stage 3.

## What the migration preserves and how it refuses

| Boundary | Implementation and evidence |
| --- | --- |
| Consent before writes | Native default No before opening old SQLite, tracing, profiles, Library workers, discovery or WebView. Decline does not create a missing backup directory or sidecars. Full temporary file/directory snapshots cover this, including live-WAL fixtures. Native Yes/No evidence below separately checks the actual dialog. |
| Unique verified backup | `storage/schema6/upgrade.rs::create_backup` reserves a unique candidate, uses `VACUUM main INTO`, compares the full typed logical snapshot including committed WAL, runs integrity/FK checks, synchronizes bytes, hashes, publishes without replacement using Windows write-through rename and revalidates. Existing files are never overwritten. |
| Original values and IDs | Additive migration 6 leaves migrations 1–5 unchanged. The immediate transaction compares all original SQL objects, typed cells and IDs before publishing version 6. Invalid old hashes or external source changes refuse migration; old rows are not silently repaired. |
| Crash/restart | Generation-owned persistent marker plus per-database mutex. Ordinary open cannot admit workers after interrupted upgrade. Approved recovery revalidates retained backup/receipt/schema, finishes a non-busy checkpoint and checks main-header version before clearing its marker. Decline retains all bytes. Abrupt child-process exits before commit, after commit and after checkpoint exercise actual loss without destructor cleanup. |
| Alias/concurrency | Ordinary and verbatim Windows paths resolve to one mutex identity, including nonexistent DB/parent suffixes. This normalization does not move the database or create directories. A competing opener cannot silently migrate or admit workers. |
| Class independence | Neutral immutable blob bytes do not confer class. Separate immutable occurrence, class, provenance and admission checks prevent configuration backups or metadata reports from becoming firmware by equal hash. Existing rows become migration-only `legacy_unclassified`; user imports become `user_imported_unverified`, never trusted firmware. |
| SQL identity/history | Explicit non-null TEXT keys, canonical text/byte-length hash checks and BEFORE INSERT conflict guards reject NULL/BLOB/NUL hash and REPLACE bypasses. Tests cover both recursive-trigger settings. |
| Repair and restore | Old `artifact_details` stays authoritative for derived blob state. Repair never changes original identity or promotes trust. Remove keeps tombstones/history/bytes. Restore validates retained membership instead of minting a new legacy class; duplicate restore preserves the original name. |
| Refusal coverage | Precommit and postcommit injection, corrupted/missing backup, backup-name collision, live WAL/source mutation, unsupported/newer version, malformed constraints/objects, inconsistent class and derived-state conflicts are tested with temporary fixtures. Nothing is tested on the normal database. |

SQLite contract references: [VACUUM INTO](https://www.sqlite.org/lang_vacuum.html), [checkpoint behavior](https://www.sqlite.org/pragma.html#pragma_wal_checkpoint), [transaction locking](https://www.sqlite.org/lang_transaction.html). Backup verification and the durable application marker are additional application requirements; a successful SQL commit alone is not treated as completed migration.

## Actual isolated Windows UI evidence

`npm.cmd run build:native-tests`, exit 0, builds only `obtainer-native-test.exe` with its independent profile and production-executable guard. `npm.cmd run test:library-upgrade`, exit 0, exercises the native application through its actual Windows dialog and WebView2 React/IPC interface.

First complete run: `b105a3e64268c042483aad6b2f50798a`, evidence `.tools/foundation/2026-10-26T09-23-38.683Z/`. The fixture is schema 5 with dark settings, a synthetic saved device/job/import and a **36-byte generated harmless file**, not firmware from a MiFi. Blob SHA-256: `edc2d517c776a0a880be1edda670ae4dfbaca89785ce4fb9cdb24dc764a3170f`.

- Owned native dialog showed the isolated backup path and one-way warning. No was both default button style 1 and dialog default ID 7. Explicit No exited 0; every data file and directory matched before/after.
- Explicit Yes completed schema 6. Preferences and original blob/size survived; admission remained `legacy_unclassified`. Backup was schema 5, integrity `ok`, no FK violations, SHA-256 matched receipt.
- Actual native window and React rendered; real Rust/SQLite IPC, six navigation pages and three local Windows discovery refreshes passed. Restart kept the same isolated directory and settings. Device features remained UNKNOWN; no device HTTP request or challenge was sent.
- Final run `9adebd5dd39a75868cd160ce20f3507e`, evidence `.tools/foundation/2026-10-26T09-24-53.923Z/`, exit 0, repeats these checks and additionally waits for the actual migrated Library card, opens details through Rust IPC, opens the customization modal, and checks all four switches and Rebuild remain disabled before and after restart.
- Console warnings/exceptions were absent. Marker mismatch, unknown staging and a cleanup junction were refused before the owned stopped profile was cleaned. A first unsuccessful helper-initialization run was retained rather than reported as success.

Dialog automation checks random run ID, active profile ownership, native-test child PID, exact dialog title and isolated backup path before responding. It cannot supply consent for normal app data or credential dialogs. No owner hardware confirmation is synthesized.

## Limits and release gate

Logical snapshots are bounded to one million rows and 128 MiB of stored cell payload; that is not a hard process-RAM cap. Existing header-preflight limits for a newer schema present only in WAL remain documented and tested; no new normal-data probe is used to evade them. Invalid historical identity requires a separately reviewed recovery, not silent normalization.

No additional producers, manifest-2 consumer/exporter, authenticated client, physical extraction/write or real-data migration is released. Normal migration and main merge remain blocked until explicit owner acceptance of the data upgrade. Stage 4 software acceptance is preserved; its physical MG11 challenge gate stays pending. Phase 7.5 is incomplete.
