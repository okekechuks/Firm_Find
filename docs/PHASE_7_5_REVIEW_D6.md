# Phase 7.5 review corrections, D6 and merge readiness

23 October 2026, Africa/Lagos. Branch `phase-7-5-acquisition-engine`; clean start at `f029f05`. The owner accepted B1/B3/B6 with limitations and expressly approved D6. This delivery performs only local corrections and writes a B4 pre-step plan. No request was sent to the MiFi or Internet. Existing owned-loopback transport regressions remain in the Rust suites.

## Disposition and changed files

| Item | Status | Files and evidence |
| --- | --- | --- |
| F1 | DONE | `acquisition/strategy_api.rs`: default `AcquisitionStrategy::applicability` always returns `Unknown`. Test-only empty-requirements strategy with all five trusted identity fields cannot qualify itself. A future qualified implementation must override applicability and cite its qualification. |
| F2 | DONE | `acquisition/approval.rs`: proof fields private to approval module; `ClaimedApproval::strategy_probe` is the only production constructor and checks scope, strategy, nonempty request IDs and expiry. Bare-enum `from_scope` factory removed from `strategy_api.rs`. Test-only helper also goes through a claimed fixture. New `compile_fail,E0451` doctest rejects direct construction; metadata receipt regression retained. `mod.rs` re-exports proof type. |
| F3 | DONE | `acquisition/strategies.rs` derives candidate state from trait's documented `implementation_state`, which is `NotImplemented` for all five stubs, consistent with probes. Golden `fixtures/acquisition-catalogue-schema1.json` changes only five candidate state values from `unknown` to `not_implemented`; wire keys/IDs/schema remain identical. No executable path or authority added. |
| F4 | DONE | `acquisition/mod.rs::compatibility_stages` requires a recognized family candidate as well as HTTP response for management observation. Bare responses (200/404/500 tested) remain unknown, reason exactly `A web server answered; no management family recognized`. Recognized family stays observed/candidate, never verified. Tests in `acquisition/tests.rs` cover both outcomes. |
| F5 | DONE | `acquisition/request_registry.rs`: doctest pins `E0451`; both compile-fail doctests execute successfully. |
| F6 | DISPUTED; permitted fallback used | Trial extension to original substring scan failed against `ipc.rs`, `mod.rs`, `request_registry.rs`: `ASR` matches required `asr_duster_xml`, `AsrDusterXml`, `asr_signals`, `images/asr_logo.png`; `E-LINK` matches the existing `.contains("e-link")` title-to-evidence check. Excluding only `"Brand hint: e_link_oem"` cannot fix these collisions. Excluding identifiers/predicates or switching to token-only matching would weaken/change the existing scan. Per guideline's explicit fallback, original eight-name list and substring matching remain intact. `recognize()` now comments that this brand hint is evidence text, never identity/support selection. Scan assertion reports offending files. Reviewer decides whether a distinct branch-analysis test should be designed later. |
| D6 | DONE | `storage.rs::Repository::open` first opens existing file with `SQLITE_OPEN_READ_ONLY`, reads user_version, and refuses >5 before writable open/WAL. Read/open failures fall back to existing writable error path. Missing file creation and second version check remain. Temp schema-6 test proves identical bytes, delete journal, no WAL/SHM sidecars; schema-5 rows/settings remain and WAL is enabled; invalid file retains NotADatabase, exclusively locked file retains DatabaseBusy. Existing fresh/migration/restart regressions retained. |
| B4 pre-step | DONE, plan only | [PHASE_7_5_B4_PRESTEP_PLAN.md](PHASE_7_5_B4_PRESTEP_PLAN.md): exact static menu path, separate proposed scope/confirmation, authentication uncertainty, single-attempt stop rules, narrow parser/synthetic tests and supervised future read. No parser/spec/scope/transport code or live request implemented. |

All implementation paths above are relative to `apps/desktop/src-tauri/src/` except the golden fixture, relative to `apps/desktop/src-tauri/`. New behavior tests and existing expectation updates are in `acquisition/tests.rs`; storage regressions in `storage.rs`. Evidence/design documents changed: this report, `PHASE_STATUS.md`, `PHASE_7_5_B4_PRESTEP_PLAN.md`, and D6 policy wording in `FIRMWARE_ACQUISITION_ENGINE.md`. No GUI component or database migration changed.

## Test-first record and initial failures

`powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --locked --offline` exited 101 before fixes: 253 passed, 7 failed, 1 ignored. Failures were F1 (AvailableVerified vs Unknown), F2 (bare factory still present), F3 (candidate Unknown), F4 (observed without family), F5 (unpinned doctest), F6 (expanded list scan), D6 (wal vs delete). Log `.tools/review-d6/red.log`.

The F6 diagnostic rerun initially exposed one old test call missing the new family argument (E0061); corrected that call. The next exact targeted command `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --locked --offline acquisition_runtime_has_no_hardware_model_name_special_cases` exited 101 with the concrete scan collisions above (`f6-red-detail.log`). The permitted fallback preserves the original scan instead of broad exceptions. F2/F5 source assertions fail first, then actual compile-fail doctests pin private-field rejection.

An initial full-workspace build attempt failed with Windows error 112, disk full, while creating `target/debug/deps/obtainer_lib.lib`. Removed only the resolved, Git-ignored `target/debug/incremental` build cache (6.52 GiB), verified inside this workspace before deletion. Application data and executables were preserved. Subsequent Rust checks use `$env:CARGO_INCREMENTAL='0'` to avoid recreating that cache; parallel test scheduling is unchanged. No serial-test workaround.

## SQLite open-path audit and preservation

Only production `Connection::open` is `storage.rs::Repository::open`; `lib.rs` setup calls it for `obtainer.db`. `library/mod.rs::Library::start` receives the shared repository; library queries, job recovery and backup/restore use that connection (firmware backups copy artifact bytes, not another SQLite database). No separate application-database recovery/backup opener exists. Other Rust direct opens occur only in temporary-database test setup. Therefore the startup ordering defect had one application-database opener, now corrected. Read-only snapshot scripts do not migrate or request WAL.

`py scripts/probe-native-isolation.py .tools/review-d6/normal-before.json` and the matching after comparison exited 0. Normal schema remains 5, integrity `ok`, foreign-key check empty. DB/WAL hashes, all row/settings hashes, saved routers/profiles, vault files and logs match exactly. Unit tests use temp databases exclusively; only the explicitly read-only preservation probe accesses owner data.

## Executed checks

Rust commands below run with `$env:CARGO_INCREMENTAL='0'`; existing Windows path-lease tests require normal Windows filesystem permissions. Logs under `.tools/review-d6/`. No script writes review verdicts or owner-confirmation prose.

| Exact command | Exit / result |
| --- | --- |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | 0; 263 passed, 0 failed, 1 ignored; two doctests pass |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | 0; 264 passed, 0 failed, 2 ignored; two doctests pass |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all` | 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all -- --check` | 0 |
| `npm.cmd test` | 0; 146 passed /17 files |
| `npm.cmd run lint` | 0 |
| `npm.cmd run build` | 0; TypeScript project build and Vite production build |
| `py scripts/probe-native-isolation.py .tools/review-d6/normal-after.json .tools/review-d6/normal-before.json` | 0; identical preservation snapshot |
| `git rev-parse main` | 0; `369e74173c570009b47bfd4957a4852ed0856678` |
| `git merge-base --is-ancestor 369e741 HEAD` | 0; branch is a fast-forward candidate |

`py scripts/verify-parallel-rust.py --runs 20 --output .tools/review-d6/final-parallel` exited 0: twenty consecutive full-workspace default-parallel runs on final code commit `efc179a9d0d3b1c6f4cea75ed677aece8561f4cd`, each 263 passed /0 failed /1 ignored and two doctests passed. Every before/after revision matches; tracked tree clean throughout. All twenty log hashes match recorded bytes and new F1–F5/D6 regression names appear with `... ok` in every log. Computed records: `.tools/review-d6/final-parallel/results.json`. No implementation changes follow that revision; this follow-up edits documentation only.

`py scripts/probe-native-isolation.py .tools/review-d6/normal-final.json .tools/review-d6/normal-before.json` exited 0 after all twenty runs: same complete normal-data snapshot. Final code tree and normal data are verified independently of the evidence-only commit. Final branch HEAD (documentation included) is supplied in delivery; repository is committed and clean at handoff.

Not run: native GUI launch, Windows release builds, physical OpenWrt qualification or any physical router acquisition. B2 migration/artifact classes, B4 authenticated client, B5 binding, B7 official-image registry, B8 analysis worker remain unimplemented/blocked. All five hardware acquisition results H01–H05 remain pending; historical static metadata is not firmware acquisition. No acquisition claim until a real artifact has hash/provenance. Phase 8 unstarted. No hardware matrix change, credentials, write, flashing, upgrade, reset, live USSD, push, merge or tag.

## Review handoff

F1–F5 and D6 corrected; F6 fallback requires reviewer disposition. B4 plan awaits review before code or hardware requests. Phase 7/7.5 completion under owner's definition and merge are reviewer's decisions after acceptance. Branch/start/end revision and `git diff f029f05..HEAD` are supplied at handoff; exact tested source revision is recorded by the repeat runner. `main` remains `369e741`; reviewer alone performs any merge.
