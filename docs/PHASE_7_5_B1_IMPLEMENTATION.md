# Phase 7.5 B1/B3/B6 implementation report

**Date:** 23 October 2026 (Africa/Lagos)

**Branch:** `phase-7-5-acquisition-engine`
**Start commit:** `c03c1a3bb7947a5ba874740cb96f1f592ff337d5`

The owner accepted the B1 design and authorized only local B1, B3, B6, model-name scanning and D6a characterization. No router, MiFi or Internet request was sent. No credential or device configuration was changed. The normal database, Library, vault and saved connections were not opened by these checks.

## Results

| Item | Result | Evidence |
| --- | --- | --- |
| B1 strategy interface | DONE | Five compiled modules and stable IDs are registered. Each probe returns `not_implemented`; `ProbeContext` exposes no I/O client. Plans list missing requirements and always remain non-executable. `canObtain` remains false. A schema-1 candidate golden fixture verifies the existing candidate output and IDs. |
| Approval scope | DONE | Existing receipt path still issues only `EndpointMetadata`. A claimed metadata proof cannot convert to `StrategyProbeApproval`; the strategy API accepts only that separate proof type. No production path issues a strategy-probe scope. Existing 30-second endpoint approval and binding remain in place. |
| Evidence and plan invalidation | DONE | Tests cover equal user-reported/observed text failing verified requirements, missing exact revision, stale evidence, mixed endpoints, changed discovery generation and conflicting identity. User labels do not affect applicability. |
| B3 request table | DONE | Closed `asr_duster_xml` table contains exactly `GET /` (16 KiB) and `GET /version_num.txt` (4 KiB). Validator accepts only HTTP/HTTPS origins and rejects non-GET, bodies, caller paths/queries, percent encoding, traversal, userinfo, fragments, alternate hosts, redirects and all eleven denied action words. Compile-fail doctest verifies `RequestSpec` fields cannot be constructed by callers. Login, logout and `xml_action.cgi` are not registered. No dispatch or new Tauri command exists. |
| B6 compatibility stages | DONE | Schema-1 report adds eleven fixed, ordered stages; original report fields remain present. UI keeps first five summary rows, then reveals six more on demand. States are limited to `verified`, `observed`, `unknown`, `not_checked`, `not_supported` and `not_implemented`. No stage grants another stage authority; a verified identity leaves all later stages unchanged and Obtain remains disabled. |
| Model-name scan | DONE | Test scans runtime Rust under `acquisition/`, case-insensitively, excluding test modules and fixtures. Current hardware-matrix names live in `acquisition/fixtures/hardware_model_names.txt`. Matching evidence under different user labels gives identical applicability. |
| D6a characterization | DONE; storage fix NOT IMPLEMENTED | On a temporary schema-6 database, real `Repository::open` returns `NewerSchema`, while `journal_mode` changes from `delete` to `wal` and database bytes change. `user_version` remains 6. This confirms the existing preflight defect. Proposed fix remains read-only version check before requesting WAL and awaits the owner's separate approval. No storage behavior or migration changed. |

## Test-first evidence

The new acceptance tests were run before implementation and failed as expected:

- B1 targeted Rust tests exited 1 because the strategy API, proof scope and registry methods were absent.
- B3 targeted Rust tests exited 1 because the request registry did not exist.
- Model-name scan targeted Rust test exited 1 because the scanner was not implemented.
- `npm.cmd test -- src/test/Devices.test.tsx` exited 1 with 2 failures: the eleven-stage summary and independent neutral status rendering were absent.

D6a was characterization-only and passed while observing the existing WAL mutation. During implementation, Rust formatting caught import order and Clippy caught a needless indexed test loop; both were corrected. An initial un-escalated full Rust run reached 223 passing tests but 32 existing filesystem lease tests failed because sandbox access to `C:\Users\chuka` was denied. The same command with normal Windows test access passed completely.

## Final verification commands

| Command | Exit / result |
| --- | --- |
| `npm.cmd test` | 0; 146 tests passed across 17 files |
| `npm.cmd run lint` | 0; no warnings |
| `npm.cmd run build` | 0; TypeScript and Vite production build passed |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all -- --check` | 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | 0; 255 passed, 0 failed, 1 ignored; compile-fail doctest 1 passed. Run with normal Windows access because existing path-lease tests traverse temp-directory ancestors. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | 0; 256 passed, 0 failed, 2 ignored; compile-fail doctest 1 passed. Run with normal Windows access for existing path-lease tests. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0 |

The required 20-run standard Rust check is executed after the implementation commit. `scripts/verify-parallel-rust.py` records only computed logs/results under `.tools/phase7-5-b1-implementation/final-parallel/`; it does not write verdict prose. Its result JSON contains the exact commit hash and each run's test counts and exit code.

## Scope and handoff

No native GUI launch was needed: the only `Devices.tsx` change renders the B6 stage report. No release build or hardware retest was run. No firmware, schema-6 artifact classes, authenticated client, transport binding, official-image registry, analysis worker, Phase 8 or physical OpenWrt support was started. Firmware extraction, configuration writes, flashing and live USSD remain disabled.

Changed implementation files are listed in the final review diff from this branch's start commit. Review current design and status at [FIRMWARE_ACQUISITION_ENGINE.md](FIRMWARE_ACQUISITION_ENGINE.md), [PHASE_STATUS.md](PHASE_STATUS.md) and [PHASE_7_5_DESIGN_REVIEW.md](PHASE_7_5_DESIGN_REVIEW.md).
