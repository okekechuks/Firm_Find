# Stage 3: classified artifacts and consent-gated schema 6

## Current completion candidate — 26 October 2026

Branch `phase-7-5-stage-3-artifact-completion`, based on independently accepted Stage 4 `f9e711510522ecf5b2e935640052efe51c7679fd`. The owner reports Stage 4 Windows CI `40273674930` passed. This work does not rerun or claim that CI result as its own. It reuses the accepted Stage 3 design amendment `b8bed2c2f93de0148dff9f6e12ce17ef13c2eaf5`, implementation `b4f3819be27c7a7e80590d5c1d9b70ee01a34248` and draft evidence `678b1999d177d40a85e7f1f1b238fc60013127f7`; those original worktrees and all existing modifications remain intact. No merge, push, tag or history rewrite. The source commit is pinned by the final repeat-run JSON rather than a self-referential hash in this file.

The dated draft report below is retained as historical evidence. Its test counts, unrun-GUI statement, earlier branch/base and next-work release are superseded by this section. Current owner boundaries control: no credentials, login, authenticated physical reads, real acquisition, writes, real-data migration or main release. MG11 challenge remains physically unqualified. Phase 7.5 is not complete.

### Implementation and new failing-first evidence

The accepted additive schema, six new artifact classes plus migration-only legacy class, independent occurrences/admissions, sealed unverified user-import consumer, default-No pre-startup consent, verified WAL-aware backup, persistent interrupted-upgrade recovery and derived-state repair are retained. Migrations 1–5, the accepted acquisition and endpoint-binding implementation, and Windows CI/build hash safeguard are unchanged from `f9e7115`.

All new logs below are under this worktree's ignored `.tools/stage3-completion/`:

| Regression | Initial result | Smallest correction and final evidence |
| --- | --- | --- |
| `stage3_windows_path_aliases_share_migration_admission_without_creating_data` | `alias-red.log`, exit 101: a verbatim Windows alias acquired a second migration mutex. | Canonicalize the nearest existing ancestor only for the lock identity; append missing suffix, do not change storage path or create parents. Tests cover existing and nonexistent DBs, ownership release and unchanged bytes. |
| `stage3_sql_contract_rejects_null_identity_and_noncanonical_hash_storage` | `sql-contract-red.log`, exit 101: all nine malformed cases were accepted, including NULL primary identities, BLOB hashes and NUL-suffixed hashes. | Explicit NOT NULL identity keys and text/character/byte/hex hash constraints in migration 6; nullable coverage/receipt values remain allowed. |
| `stage3_sql_contract_replace_cannot_reclassify_or_rewrite_immutable_history` | Same exit-101 run: six replacements bypassed immutable history with recursive triggers off. | BEFORE INSERT conflict guards reject replacement of blobs, occurrences, admissions, firmware identity and receipt regardless of recursive-trigger mode. Equal-size blob dedup is looked up before insert in the sealed import consumer; no class is reused from an equal hash. |
| Abrupt process loss | New crash parent test passes; this is added coverage, not claimed as a reproduced defect. | Owned child exits 86 without unwinding before commit, after commit and after checkpoint. Ordinary reopen stays blocked; declined recovery preserves every fixture file/directory; approved recovery validates backup and preserves old values. Ignored helper is actually executed three times by the parent. |
| Native dialog test helper | `native-upgrade.log`, exit 1 before any click: helper inspected default-button style before dialog initialization finished. | Bounded wait verifies both No's default style and dialog default ID 7 before clicking. `native-upgrade-second.log`, exit 0, confirms No/Yes/backup/restart/IPC. No application default or consent policy was relaxed. |

New source hardening: `src/storage/schema6/pending.rs`, `migrations/006_artifact_classes.sql`, `src/library/admission.rs`. New Rust regressions: `src/storage/schema6_contract_tests.rs`, `src/storage/schema6_crash_tests.rs`, additions to `src/storage/schema6_tests.rs`. The latter two child modules are included by the existing storage test module.

### Current commands and actual results

Commands ran from this worktree. Rust uses the existing shared Cargo/Rustup/target cache, `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=1`; test scheduling is default parallel, never `--test-threads=1`. Every build start had at least 15 GiB available on C:.

| Exact command | Exit and result | Local evidence |
| --- | --- | --- |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | 0; 341 passed, 2 ignored; 2 doctests passed | `rust-hardened.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | 0; 342 passed, 3 ignored; 2 doctests passed | `rust-isolated-hardened.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | 0, clean | `clippy-hardened.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0, clean | `clippy-isolated-hardened.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all -- --check` | 0, clean | `fmt-hardened.log` |
| `npm.cmd test` | 0; 160 tests, 18 files | `frontend.log` |
| `npm.cmd run lint` | 0, no warnings, including final harness | `frontend-lint-complete.log` |
| `npm.cmd run build` | 0; TypeScript `tsc -b` and Vite production build | `frontend-build.log` |
| `npm.cmd run build:native-tests` | 0; dedicated optimized Windows executable, 8m14s Cargo build; production executable preserved | `native-build-hardened.log` |
| `npm.cmd run test:library-upgrade` | 0; native consent/backup/React/IPC/six pages/discovery/restart, loaded Library/modal checks and safe cleanup | `native-upgrade-final.log` |

The added `npm.cmd run test:library-upgrade` seeds schema 5 with a 36-byte harmless generated file in an owned `OBTAINER-NativeTests/<random-id>` profile. It answers only that owned profile's migration dialog. Default No and explicit No exit 0 and preserve all data-directory files and directories. Yes opens schema 6 with unchanged dark preferences and `legacy_unclassified` admission; the retained backup is schema 5, has integrity `ok`, no foreign-key violations and an actual SHA-256 matching its receipt. React renders in a native window, all six destinations work, real Rust/SQLite IPC succeeds, three Windows discovery refreshes retain unique IDs and all four device features remain UNKNOWN. Shutdown/reopen repeats those checks. Captured console issues are empty. Cleanup rejects marker mismatch, unknown staging and a junction before cleaning only its own stopped profile. Normal production executable is not launched by this new mode.

Computed native evidence: `.tools/foundation/2026-10-26T09-23-38.683Z/`, run `b105a3e64268c042483aad6b2f50798a`. Its 36-byte blob SHA-256 is `edc2d517c776a0a880be1edda670ae4dfbaca89785ce4fb9cdb24dc764a3170f`. The first failed run is retained for review; no failed run is counted as a successful native result. Screenshots are ignored local evidence, never committed.

Final native run: `.tools/foundation/2026-10-26T09-24-53.923Z/`, run `9adebd5dd39a75868cd160ce20f3507e`, exit 0. In addition to all checks above, the harness waits for the harmless migrated file's rendered Library card, opens its details through real IPC, then opens the actual modal and confirms all four switches and Rebuild are disabled, both after migration and after restart. The `*-loaded-library.png` and `*-disabled-customization.png` captures are local evidence; the rendered modal was visually inspected. There is no extraction or enabled Apply/Rebuild claim.

Before delivery, run `py scripts/verify-parallel-rust.py --runs 20 --output .tools/stage3-completion/final-20` on the final clean commit. Its computed `results.json` records exact commit before/after, exits, test counts and log hashes for each run. The final handoff must report the actual result; this document does not invent the future outcome. Any subsequent source change invalidates that final repeat gate.

### Preservation and limits

[Database-preservation report](PHASE_7_5_STAGE_3_DATABASE_PRESERVATION.md) records the byte-only baseline and native checks. All 12 normal files / 5,611,701 bytes remain identical through the native suite; no normal SQLite connection, storage move or normal application launch. Production executable remains SHA-256 `0278DDB4776A03458A4C24C0C219A7B0C32F74FAE264D6D549883C94F4780C65`. It was guarded rather than overwritten or rebuilt. No new production Windows build, Windows CI run or physical MG11 qualification is claimed for this branch.

Initial free C: space was 19.49 GiB, later build-start checks remained above 15 GiB; observed after native build: 16.48 GiB. No toolchain, user data, firmware, backup, evidence or build cache was deleted. One generated 5,404-byte Python bytecode file in this new worktree was removed, followed by its empty cache directory; the fixture loader now disables bytecode generation.

**Current Stage 3 disposition: software verification passed; independent source review pending.** Final committed-code repeat is a separate computed delivery gate. No normal-data migration acceptance is inferred. Stage 4 SOFTWARE PASS remains preserved; its physical gate remains PENDING. Later producer consumers, manifest-2 exporter, Stage 5, authenticated hardware and Phase 8 are not released.

### Current changed-file inventory

All paths are repository relative; this is the complete 35-file diff from accepted `f9e7115`:

- `apps/desktop/src-tauri/migrations/006_artifact_classes.sql`.
- `apps/desktop/src-tauri/src/{lib,startup,storage}.rs`.
- `apps/desktop/src-tauri/src/library/{admission,db,files,manifest_contract,mod,tests,types}.rs`.
- `apps/desktop/src-tauri/src/storage/{schema6,schema6_tests,schema6_contract_tests,schema6_crash_tests}.rs` and `storage/schema6/{pending,snapshot,upgrade}.rs`.
- `apps/desktop/src/{components/CustomizationPanel.tsx,features/FirmwareLibrary.tsx,test/FirmwareLibrary.test.tsx,test/artifactClasses.test.ts,types/library.ts}`.
- `docs/{FIRMWARE_ACQUISITION_ENGINE,PHASE_STATUS,PHASE_7_5_STAGE_3_ARTIFACT_SCHEMA_DESIGN,PHASE_7_5_STAGE_3_VERIFICATION,PHASE_7_5_STAGE_3_DATABASE_PRESERVATION}.md`.
- `package.json`; `scripts/{audit-data-bytes.py,stage3-native-fixture.py,answer-isolated-library-upgrade.ps1,native-test-profile.mjs,verify-firmware.mjs,verify-foundation.mjs}`.

No dependency/lockfile change, generated screenshot, binary, normal application data or acquisition/binding source change is included. Review command: `git diff f9e711510522ecf5b2e935640052efe51c7679fd..phase-7-5-stage-3-artifact-completion`.

## Historical isolated draft — 24 October 2026

24 October 2026, Africa/Lagos. Branch `phase-7-5-stage-3-artifacts`, accepted base `def018836166151c0b781add711dcaa5d50efbdb`; accepted design amendment `b8bed2c2f93de0148dff9f6e12ce17ef13c2eaf5`. Independent branch: accepted Stage 4 `5817cfe` remains separate. No merge, push, tag or history rewrite.

## Scope and implementation

| Item | State and evidence |
| --- | --- |
| Additive schema and original data | Implemented in `migrations/006_artifact_classes.sql` and `src/storage/schema6{.rs,/upgrade.rs,/snapshot.rs}`. Neutral immutable blobs, separate immutable occurrences, derived state, firmware admission, migration receipt and classified/nonfirmware views. Current schema-5 SQL objects, all original typed cell values and IDs are compared before version publication; earlier approved migrations retain their original columns/values. Legacy-only backfill precedes the legacy-insert guard. Invalid lowercase-hex hashes are refused, never corrected. |
| Explicit consent | `src/lib.rs::run_profile`, `src/startup.rs::confirm_library_upgrade`, `src/storage.rs::Repository::open` and `schema6::open_with_upgrade`. Default-No native confirmation names the fresh unique backup and any retained recovery backup. It runs before Builder/WebView, tracing, profile install, workers or discovery. No returns quietly before SQLite opens; temporary live-WAL directory bytes and missing backup directory are checked. Ordinary open cannot silently migrate old data. |
| Verified backup | `schema6/upgrade.rs::create_backup`: `VACUUM main INTO` a reserved unique empty candidate, integrity/FK/full typed logical comparison including committed WAL, explicit `sync_all`, SHA-256, non-replacing Windows write-through rename, rehash and revalidation. File/directory leases remain through migration. External source writes between backup and the write transaction abort migration. Partial files and published backups are retained, never offered as automatically usable firmware. |
| Transaction/restart admission | Immediate transaction; version 6 committed with migration receipt. `schema6/pending.rs::Lease` persists a generation-owned pending marker before commit under a per-database named mutex. Busy TRUNCATE or a post-commit failure retains the marker and blocks ordinary open/workers even if main header already says 6. Explicitly approved recovery verifies retained receipt and backup, validates/repairs schema, completes non-busy TRUNCATE and checks main header before clearing its own marker. No cannot clear pending state. |
| Constraint and derived-state validation | `schema6.rs::validate_constraints` compares additive object SQL to the compiled migration, including CHECKs, indexes, views and trigger bodies. `validate_and_repair` checks identity/admission/size/FKs and copies only authoritative old `artifact_details` into missing/drifted derived state. `unreadable` maps to derived `unknown`; it never promotes integrity or changes the original row. |
| Sealed local-import admission | `src/library/files.rs::VerifiedPublishedBlob` is constructed only by reading/hash-checking the published content-addressed vault file while retaining read/directory leases. `src/library/admission.rs::UserImportProducer` is private/sealed; the sole production `firmware_artifacts` INSERT lives here. It checks active uncancelled import job, candidate hash and size, creates a distinct `user_imported_unverified` / `user_import` occurrence and admits only that independently acquired occurrence. No filename/model/extension/MIME/hash string mints the proof. |
| Equal bytes and retained restore | Shared neutral blob does not transfer classification. Configuration/report occurrence IDs cannot satisfy firmware admission. Independent user assertion may acquire the same bytes through a new occurrence without relabelling the earlier configuration occurrence. Duplicate imports preserve earlier admission/class. Existing backup restore checks retained membership/hash/size and records existing restore history without minting another legacy occurrence or new admission. Remove stays a tombstone. Rollback tests include every new class table. |
| UI and manifest compatibility | `src/library/types.rs`, `db.rs`, frontend `types/library.ts`, `FirmwareLibrary.tsx`, `CustomizationPanel.tsx`: additive class field; new local import displays **User-provided, unverified** in import confirmation/customization. Four-field cards, six pages, popup behavior and disabled build actions retained. `library/manifest_contract.rs` defines/tests proposed manifest 2 with strict fields, class/provenance/coverage, privacy exclusions and unknown installability; it has no export/producer consumer. Existing manifest-1 exporter unchanged. Reserved other producer contracts have no production constructors/callers. Q5 consumer review remains mandatory. |
| Data-preserving evidence | `scripts/audit-data-bytes.py` records computed ordinary-file hashes only, including DB/WAL/SHM, logs and vault. No SQLite module/connection. Native harness uses this audit instead of the old SQLite probe; the old probe was not executed. Updated schema assertions in `verify-foundation.mjs` and `verify-firmware.mjs` are not a claim those GUI harnesses ran. |

Paths above are under `apps/desktop/src-tauri` unless explicitly frontend, docs or scripts. Added code does no device/network acquisition. Existing unit transport checks use owned loopback peers; no real credentials, router requests, extraction, writes, flashing or USSD.

SQLite documents [VACUUM INTO](https://www.sqlite.org/lang_vacuum.html) as a consistent snapshot and warns interrupted output can be incomplete. Its current documentation describes synchronization for NORMAL/FULL; this implementation additionally calls `sync_all` and verifies output. [Checkpoint documentation](https://www.sqlite.org/pragma.html#pragma_wal_checkpoint) defines the busy result; header equality alone does not prove TRUNCATE succeeded. The [transaction documentation](https://www.sqlite.org/lang_transaction.html) explains immediate write admission. VACUUM is outside the migration transaction; the verified source is re-compared inside it.

## Red tests and corrections

Evidence directory: `.tools/stage3/` (local computed logs; no owner statements or verdict-generating scripts).

| Log / command | Exit / actual finding | Correction |
| --- | --- | --- |
| `schema-red.log`: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace stage3_` | 101; fresh DB still version 5, raw unclassified firmware INSERT accepted | Added schema 6 and independent admission constraints. |
| `upgrade-api-red.log` | 101; consent-upgrade API did not exist | Added pre-write consent driver. This was a compile/API red, not a runtime result. |
| `upgrade-first.log` | 101; OS 112 while building (reviewer scratch had filled C:) | No data or code conclusion from disk failure; reviewer reclaimed their own scratch, build resumed. |
| `checkpoint-red.log`, `checkpoint-semantic-red.log` | First compile red: hook visibility; then runtime 101: busy TRUNCATE with main header 6 could bypass recovery by ordinary reopen | Added persistent generation-owned pending admission gate; focused five-test run exit 0. |
| `failure-matrix-first.log` | 101: accidental original-schema ALTER was accepted; four fixture checks also used nonexistent settings column `id` | Preserve original schema objects as well as values; correct fixture query to key `preferences`. |
| `failure-matrix-fixed.log` | 101: 12 passed; malformed-hash setup itself hit historical length CHECK | Explicit temporary corruption fixture disables only its old CHECK while seeding, then reenables it. All original invalid bytes remain unchanged on refusal. |
| `import-admission-red.log`: existing test executable, exact `library::tests::fresh_import_preserves_source_and_commits_metadata_and_progress` | 101: completed import instead failed DATABASE_UNAVAILABLE at new raw-insert guard | Replaced raw import INSERT with verified-file proof and sealed admission. |
| `full-first.log`: `... rust.ps1 test --workspace` | 101: 296 passed, 11 failed; older fixtures expected silent upgrade/version 5 and prior error mapping | Fixture upgrades explicitly approve only temporary DBs; current-version assertions 6, newer-version fixtures 7. Old schema-5 preflight refusal of 6 remains explicitly tested. Busy SQLite behavior is still tested below the new pre-SQLite consent boundary. |
| `admission-hardening-red.log`: `... rust.ps1 test --workspace stage3_` | 101: 17 passed; missing admission trigger still allowed startup | Compare required additive SQL objects to compiled contract before worker admission. |
| `frontend-class-red.log`, `frontend-ui-red.log`: focused Vitest commands | 1: unknown explicit class was stripped/accepted; expected unverified label absent | Add validated optional class and correct label. Focused green: 13 passed, exit 0. |
| First attempt invoking `npm` | PowerShell policy refused npm.ps1; no suite ran | Use installed `npm.cmd`; no policy change. |
| `full-second.log` | Interrupted turn while compiling; no final result captured | Not counted. Reran as `full-third.log`, exit 0: 316 tests and 2 doctests passed. The final gate explicitly includes the content-address proof regression. |

Additional deterministic regressions: all twelve precommit fault boundaries, three postcommit faults and approved/declined recovery; backup corruption and destination collision; external committed write; invalid hashes; missing trigger; derived-state loss/drift; barrier-controlled competing open and worker rejection; lost/corrupt retained backup; class/FK/coverage/installability/immutability; legacy restore/reimport; independent same-byte acquisition; import rollback; content-address proof rejection; old-version refusal and existing G1 WAL-only coverage. All inputs are harmless synthetic bytes or temporary DBs.

## Verification commands and results

Environment for Rust commands: absolute existing workspace `.tools/cargo` and `.tools/rustup`, root `target` cache via `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=1`. Unit tests retain default parallel scheduling, with no `--test-threads=1`. Windows file-lease tests require the existing unsandboxed directory-sharing access; no normal data is opened.

| Exact command | Exit / result |
| --- | --- |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | 0; 316 passed, 1 ignored; 2 doctests passed; `rust-standard.log`. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | 0; 317 passed, 2 ignored; 2 doctests passed; `rust-isolated.log`. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | 0; `clippy-first.log`. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0; clean; `clippy-isolated.log`. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all -- --check` | 0; clean; `rust-fmt.log`. |
| `npm.cmd test` | 0; 158 tests, 18 files; `frontend-tests.log`. |
| `npm.cmd run lint` | 0; `frontend-lint.log`. |
| `npm.cmd run build` | 0; TypeScript `tsc -b` and Vite production build; `frontend-build.log`. |
| `py scripts/audit-data-bytes.py .tools/stage3/normal-mid-files.json .tools/stage3/normal-before-files.json` | 0; 12 files / 5,611,701 bytes unchanged; zero SQLite connections. |
| `git diff --check` | 0. Generated permission files have only worktree EOL metadata, no source-content difference. |

Committed code `b4f3819be27c7a7e80590d5c1d9b70ee01a34248` passed `py scripts/verify-parallel-rust.py --runs 20 --output .tools/stage3/code-20`, exit 0: twenty consecutive full-workspace default-parallel runs, each 316 standard tests /0 failed /1 ignored plus two doctests. Every run recorded the same clean commit before and after. Independently recomputed all twenty log SHA-256 values and checked all computed exits/counts. Results: `.tools/stage3/code-20/results.json`; this is a code-commit result, not an invented future delivery-commit result.

After documentation close-out, require one more `py scripts/verify-parallel-rust.py --runs 20 --output .tools/stage3/final-20` on the actual delivered HEAD. Its computed `results.json` records that exact delivery commit and all exits; the final handoff reports that gate separately. No source behavior changes in this close-out.

Byte audits `normal-before-commit-files.json`, `normal-after-code-files.json` and `normal-after-code20-files.json` exactly match `normal-before-files.json`: all 12 files /5,611,701 bytes, including retained DB/WAL/SHM, settings/device rows inside those untouched files, logs and vault bytes. This is byte preservation evidence, not a logical SQLite integrity claim. No SQLite connection to normal data occurred.

## Limits and disk

Free-space checks precede every build; all observed Stage 3 starts exceed the reviewer's 15 GiB threshold (initial 28,299,055,104 bytes; later 24,947,404,800 bytes and 25,377,333,248 bytes). No build cleanup/deletion was needed in Stage 3. Earlier Stage 4 cleanup was separately reported and is not claimed again; the reviewer's reclaimed scratch is not ours.

Not run: normal app launch, native consent GUI, GUI harness, optimized Windows release builds or physical MG11 run. No normal-data SQLite connection, real migration, vault relocation or file replacement. Native-dialog behavior is source-checked; consent effects are tested with controlled decisions on temporary DBs, not presented as GUI evidence. Physical challenge still waits for a new owner-at-machine / USB-connected / Wi-Fi-off confirmation. Schema 6 cannot reach main without owner acceptance of real-data migration. Other producer consumers, manifest-2 consumer, Stage 5, real authenticated client/hardware, sandbox tools and Phase 8 remain held by their separate gates.

**Stage 3 software verdict: PASS on committed code; reviewer acceptance pending.** Final delivery repeat is independently recorded as specified above. Real-data migration is not released. **Phase 7/7.5 remains incomplete.**

## Changed-file inventory

- `apps/desktop/src-tauri/migrations/006_artifact_classes.sql`.
- `apps/desktop/src-tauri/src/storage.rs`, `storage/schema6.rs`, `storage/schema6/{upgrade,pending,snapshot}.rs`, `storage/schema6_tests.rs`.
- `apps/desktop/src-tauri/src/{lib,startup}.rs`.
- `apps/desktop/src-tauri/src/library/{admission,manifest_contract,db,files,mod,types,tests}.rs`.
- `apps/desktop/src/{components/CustomizationPanel.tsx,features/FirmwareLibrary.tsx,types/library.ts,test/FirmwareLibrary.test.tsx,test/artifactClasses.test.ts}`.
- `scripts/{audit-data-bytes.py,native-test-profile.mjs,verify-foundation.mjs,verify-firmware.mjs}`.
- `docs/{FIRMWARE_ACQUISITION_ENGINE,PHASE_7_5_STAGE_3_ARTIFACT_SCHEMA_DESIGN,PHASE_7_5_STAGE_3_VERIFICATION,PHASE_STATUS}.md`.

No dependency/lockfile change. No generated screenshot, binary, test artifact or normal application data is committed.

## Review handoff

Branch `phase-7-5-stage-3-artifacts`; start `def018836166151c0b781add711dcaa5d50efbdb`; implementation commit `b4f3819be27c7a7e80590d5c1d9b70ee01a34248`. Clean committed tree at the twenty-run code gate. Review diff: `git diff def018836166151c0b781add711dcaa5d50efbdb..phase-7-5-stage-3-artifacts`; local computed patch `.tools/stage3/review.diff`. Delivery commit is recorded in the final repeat JSON rather than attempting a self-referential hash in this document.

The next reviewer-released implementation is a separate branch from accepted Stage 4: authenticated identity-read code tested only on owned loopback fixtures, with hardware constant false. Check for newer watcher instructions after every commit. Its dependency-fetch gate is separate from this completed Stage 3 code gate. No real-device challenge occurs before a new owner presence/USB/Wi-Fi-off confirmation.
