# Phase 7.5 G1 header preflight and B4 plan decision

23 October 2026, Africa/Lagos. Branch `phase-7-5-acquisition-engine`; clean baseline `321d43fc32275ef0fdc2cbe965094901bc30f7e5`. F1–F5 and F6 fallback are accepted by review. D6 required G1 because SQLite itself may create sidecars even on a read-only open. B4 is approved as a plan only; implementation remains unauthorized.

## Changes and status

| Item | Status | Evidence |
| --- | --- | --- |
| G1 | Implemented; final checks recorded below | `apps/desktop/src-tauri/src/storage.rs::header_schema_version` uses standard-library read-only `File::open` and `read_exact` for 100 bytes, validates all 16 magic bytes, decodes the big-endian u32 at offset 60. `Repository::open` rejects a header version above shared `SUPPORTED_SCHEMA_VERSION` before any SQLite connection. Existing post-open check uses the same constant for a version present only in WAL. Short, invalid or unreadable headers fall through to the existing open/error path. No schema migration or storage-location change. |
| B4 plan edit 1 | DONE | Re-read capture and checked SHA-256 before citing `layout_manager.js:37–47`: `callXML` uses jQuery GET, supplied URL, XML response type, `async: false`, with no Authorization header or Digest helper. This supports an expectation only; real-device 401 still stops without retry. |
| B4 plan edit 2 | DONE | Re-read `layout_manager.js:53–75` and `dashboard.js:7`. Plan now follows Tab/Menu shapes and case-exact `implFunction="objdashboard"`, derived from static code and unconfirmed against a real menu body. Zero/multiple matches, missing/invalid single-token xmlName yield `no unique dashboard entry` and no extraction. Proposed tests updated, no parser/spec/scope/request implemented. |

Capture hashes match accepted design: `layout_manager.js` SHA-256 `9c8d5fd1fc47111180b80ceee1261cc38aa560fc1d53b8e03e286cae6fc3664c`; `dashboard.js` SHA-256 `f57e620d871ec877920d1a04cef4a7e41eb2db0af20c77bd457b967c40ae381d`. Only supplied local captures were read; no device request was made and no raw captures were added to Git.

Changed files: `storage.rs` (implementation and tests), `docs/PHASE_7_5_B4_PRESTEP_PLAN.md`, this report, `docs/PHASE_STATUS.md`, and current D6 policy wording in `docs/FIRMWARE_ACQUISITION_ENGINE.md`. Previous reports remain historical evidence.

## Test-first evidence

Added three tests before the fix, then ran `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --locked --offline g1_`: exit 101, 2 passed /1 failed. `g1_wal_newer_schema_refusal_preserves_entire_directory` reproduced exactly: before only `wal-six.db`; after refusal also `wal-six.db-shm` and `wal-six.db-wal`. Main bytes were equal. Log: `.tools/g1-preflight/red.log`.

The WAL regression creates and closes a temporary WAL-mode schema-6 writer first, verifies only the main file remains, compares full bytes and sorted directory listings across two refusal attempts, and checks header read/write format bytes remain `[2,2]` (WAL). It deliberately does not open SQLite again to inspect journal mode, because that inspection could itself create the files being tested.

`g1_short_and_wrong_magic_headers_keep_existing_fallback` checks a 16-byte magic-only file and a 100-byte wrong-magic file retain NotADatabase, while an empty file retains normal creation. `g1_post_open_check_rejects_version_bump_only_in_wal` leaves main header at version 5 with version 6 in an uncheckpointed WAL; post-open check refuses it. Existing delete-mode schema-6 preservation, schema-5 WAL/settings, migrations/restarts, invalid-file and exclusive-lock mapping tests remain unchanged.

## Resource failures and recovery

Disk exhaustion interrupted the first test-source write before compilation. Restored that file exactly from clean baseline before reapplying tests; no pre-existing user changes existed. Normal sandbox process setup also failed for lack of log space. Removed only verified, ignored generated artifacts: `obtainer*`/`libobtainer*` files inside `target/debug/deps` (5,236,557,245 bytes), followed by `release/deps` and `release/build` caches inside the old `target/connected-devices-info-build`, `target/obtainer-ui-refresh`, `target/sidebar-clean` builds (5,060,562,919 bytes). Absolute paths and workspace containment were verified before removal; top-level release executables and application data preserved.

Initial concurrent checks then hit memory/commit pressure: frontend test process failed to start (exit -1), lint/build exited 1, Rust compilation exited 101 with resource/compiler failures (including `0xc0000409`); no regression assertion failed in that attempt. Reran commands one at a time with `CARGO_INCREMENTAL=0` and `CARGO_BUILD_JOBS=1`. The latter limits compiler jobs only: test scheduling remains default-parallel, with no `--test-threads=1` or serial workaround.

## Executed verification

Rust checks below use `$env:CARGO_INCREMENTAL='0'; $env:CARGO_BUILD_JOBS='1'` to bound machine resources. Test databases are temporary. Normal Windows permissions are required for existing filesystem-lease regressions. Logs/computed records are under `.tools/g1-preflight/`; no script writes review or owner-confirmation prose.

| Command | Exit / result |
| --- | --- |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | 0; 266 passed /0 failed /1 ignored; two doctests pass (`standard-retry.log`) |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | 0; 267 passed /0 failed /2 ignored; two doctests pass (`isolated.log`) |
| `npm.cmd test` | 0; 146 passed /17 files (`frontend-retry.log`) |
| `npm.cmd run lint` | 0 |
| `npm.cmd run build` | 0; TypeScript project validation and Vite production bundle |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all` | 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all -- --check` | 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0 |
| `py scripts/probe-native-isolation.py .tools/g1-preflight/normal-after.json .tools/g1-preflight/normal-before.json` | 0; complete normal-data snapshot identical |
| `git rev-parse main` | 0; `369e74173c570009b47bfd4957a4852ed0856678` |
| `git merge-base --is-ancestor 369e741 HEAD` | 0 |

Required final-code repetition command: `py scripts/verify-parallel-rust.py --runs 20 --output .tools/g1-preflight/final-parallel`. Results and exact tested code revision are recorded after execution, followed by a documentation-only evidence commit.

Preservation baseline command: `py scripts/probe-native-isolation.py .tools/g1-preflight/normal-before.json`, exit 0. Matching after snapshot has identical DB/WAL bytes/hashes, row/settings hashes, saved routers/profiles, vault and log files; schema 5, integrity ok, no foreign-key errors. This explicit read-only audit is the only access to normal application data; Rust tests never open owner's database. A final comparison follows repeated verification.

Not run or implemented: native GUI launch, Windows release builds, physical OpenWrt qualification, any MiFi request, B4 menu read/parser/scope, credentials/authenticated CGI, B2 migration/artifact classes, B5 binding, B7 registry, B8 worker, all five hardware firmware acquisitions. No acquisition capability claimed. Phase 8 remains unstarted; no write, flashing, upgrade, reboot, reset, live USSD, push, merge or tag.

## Review handoff

Reviewer alone decides acceptance and performs a merge after independent verification. Main must remain `369e741`; ancestor check and clean final branch state are recorded at delivery. Review `git diff 321d43f..HEAD`. Final tested source commit and evidence-only end commit are distinguished explicitly; no implementation change follows repeated source verification.
