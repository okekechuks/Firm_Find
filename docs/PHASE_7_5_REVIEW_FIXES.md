# Phase 7.5 review corrections R1-R5

23 October 2026, Africa/Lagos. Branch `phase-7-5-acquisition-engine`; start `1703d87ed340c07e67cdb5d11c3032849830ae72`. Original Sprint A review: PASS WITH LIMITATIONS. Reviewer independently verified 231 standard / 232 isolated-profile Rust tests and standard Clippy at 1703d87. Original frontend, GUI and physical results remain Codex-reported; the results in this report are also Codex-executed, not independent review.

## Corrections and evidence

All five findings were accepted. Before implementation, six assertions failed (R5 has two): captured fixture length, signal context, token-presence gate, virtual authorization time, BUSY/claim order and hostile confirmation name. Initial command exited 101; log `initial-tests.log`. R4's new clock helper initially retained the previous wall-elapsed behavior; R5's sanitizer initially retained raw text. They were deliberate nonfunctional test scaffolds before corrections, not claims that those functions existed in the original tree.

| Finding | Disposition | Files, functions and tests | Remaining boundary |
| --- | --- | --- | --- |
| R1 captured fixture | FIXED | `fixtures/asr-root-redacted.html` is the first 16,384 bytes of the actual supplied 18,914-byte capture, reviewed completely as text. `.gitattributes` disables line-ending normalization and text diffing for this exact byte-preserved fixture; review the file/hash directly. `acquisition::tests::r1_captured_fixture_and_positive_family_candidates` checks real prefix, prefix cut at 16,000 (before logo), ASR/Duster and OpenWrt candidate recognition. | Fixture is supplied capture evidence; separate physical reader retests are recorded below. No scripts executed. |
| R2 attribute context | FIXED | `acquisition/mod.rs::resource_attributes`, `recognize`: only complete src/href attributes on opening tags count. Comments and raw script/style bodies skipped; visible text, comments containing fake tags and JavaScript string decoys rejected by regression. Distinct script paths count once each. | Candidate evidence cannot verify identity or enable an API/export. The logo reference in the genuine page is commented out and correctly contributes nothing. |
| R3 claimed proof | FIXED | `Approvals::claim` returns `ClaimedApproval`, with private fields, no Clone/serde/default constructor. `discover_on_networks` consumes `Option<ClaimedApproval>`; branch validates binding and deadline. Both `connection::root_metadata` and `version_metadata` require a proof reference at the type boundary. Signature regression proves this requirement; forged token string without proof remains local-only. | Root and conditional static version remain the entire bounded request scope. No authentication or acquisition grant. |
| R4 simulator timing | FIXED in software | `adapters/lifecycle.rs::authorization_now`, `session_read_control`: simulated bindings use virtual time for evidence/authorization and virtual expiry. Real adapters retain elapsed wall-time checks and expiry-limited read deadlines. Both still use bounded real operation deadlines/cancellation. New deterministic tests inject five seconds of elapsed time and a one-millisecond virtual expiry margin; existing exact-boundary and scripted-USSD tests retained. Twenty consecutive default-parallel workspace runs passed; full evidence is recorded below. | No serial scheduling workaround. No physical adapter deadline relaxed. |
| R5 approval path | FIXED | `acquisition/ipc.rs::reserve_probe` obtains connection lease before claiming receipt. Dynamic two-held-leases test proves BUSY preserves receipt and retry can claim it. `approval::confirmation_value` removes controls/newlines and bidi formatting, caps device/interface/endpoint text at 80 Unicode characters. Hostile-name test and ordering regression pass. | Native confirmation remains default No; canonical endpoint binding remains complete, independently of display truncation. |

Fixture review: full SHA-256 `2cc338418cd6da2174ac8918b3d5015e184608a4af2974e39bb3251739b53fee`; retained prefix SHA-256 `298f2a5bb7daa44a582249fd4304eb39142c7780d6eda91bd4b3834faeb02e96`. No populated device-specific identifier or credential was found in the reviewed retained prefix, so no in-prefix replacement was necessary. Full-page static form defaults and blank password fields are beyond the retained cap. The root contains routines that could write if executed; it is only byte data in Rust tests, never rendered or executed. No referenced JavaScript is fetched by the fixture tests.

The previously flaky `post_review_virtual_expiry_boundary_revokes_reads_without_status_poll` advances to 29,999 ms of a 30,000 ms lease. The old shared read path imposed that final virtual millisecond as a real-time deadline. A second reported failure, `ussd_scripts_followup_busy_cancellation_and_timeout_use_virtual_time_only`, arose during connect/read authorization: wall elapsed time could date simulated identity evidence ahead of the fixed virtual clock, removing its exact match. The deterministic helper regression reproduces this time-domain mismatch without timing sleeps. Corrected simulated contexts no longer acquire elapsed wall milliseconds; physical contexts still do. The 29,999/1 boundary test is retained, not widened or skipped.

## Commands and actual results

Logs are in `.tools/phase7-5-review/`. Formatting of PowerShell stderr as NativeCommandError is not itself an exit failure; explicit exit codes and final Rust results are authoritative.

| Command | Actual result |
| --- | --- |
| `git status --short`; `git rev-parse HEAD` | 0; starting working tree clean, 1703d87ed340c07e67cdb5d11c3032849830ae72 |
| `py scripts/probe-native-isolation.py .tools/phase7-5-review/normal-before.json` | 0; read-only baseline |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --locked --offline r -- --nocapture` | 101; 231 pass / 6 initial regression failures / 1 ignored |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --locked --offline r` after first correction | 0; 237 pass / 1 ignored |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --locked --offline` | 0; 240 pass / 1 ignored; default parallel |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --features native-test-profile --locked --offline` | 0; 241 pass / 2 ignored; default parallel |
| `npm.cmd test` | 0; 145 tests, 17 files |
| `npm.cmd run lint` | 0 |
| `npm.cmd run build` | 0; TypeScript and Vite production build |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all -- --check` | 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | Initial 101: sanitizer after test module; moved helper before tests. Final 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | Same initial item-order warning; final 0. |

Initial full-workspace repetition series: runs 1-17 exited 0; run 18 exited 101 on retained `openwrt::transport::tests::blackhole_peer_cancellation_and_timeout_release_child_and_pipes` at transport.rs:613 (five-second listener-accept readiness wait). Both previously flaky simulator tests passed in that run. This occurred while optimized Windows builds were active. It is separate from the corrected simulator clock path; no OpenWrt production/test code was changed or failure hidden. Final repetition/native/data evidence is recorded below. Genuine OpenWrt guest tests are ignored unless explicitly started; none is counted as a pass. No physical OpenWrt qualification claimed.

## Supervised H01 re-test gate

These four physical results are separate from software fixtures and generic-interface native GUI checks:

| Run | Status | Actual evidence |
| --- | --- | --- |
| Wi-Fi on | PASS | Owner connected Wi-Fi to MG11 while USB remained connected, then clicked Yes. `subnet_conflict`, named Wi-Fi, HTTP status null; guard returns before TCP/HTTP. Evidence `2026-10-23T00-09-17.767Z`. |
| Wi-Fi off | PASS after parser correction | Owner clicked Yes with USB connected and Wi-Fi off. HTTP 200, `asr_duster_xml` candidate, endpoint-observed `REL_MRVL_UAPXC_v2.6.0` shown beside owner-reported `MF69_AM802L_ZX_DD_N_241106_EU`; both explicitly unverified. Evidence `2026-10-23T00-22-50.852Z`. Initial missing-version failure retained below. |
| Owner answers No | PASS | Actual MG11 native dialog rejected by owner; no network authorization or route report. Evidence `2026-10-23T00-16-25.870Z`. Initial mistaken Yes attempt is not counted. |
| Before/after preservation | PASS: app data VERIFIED; router display unchanged per owner | Each isolated run compared normal database/WAL/rows/settings/saved-device/vault fingerprints successfully. Final task-wide comparison exited 0. Owner confirmed unchanged router settings/firmware after Wi-Fi-on, initial USB-only and final corrected read, including the No case. Router internal database was not inspected. |

These are owner-supervised physical runs, separate from software fixtures and generic-interface native checks. Actual screenshots were visually inspected; terminal startup is not the GUI evidence. All physical feature assessments remain UNKNOWN and `canObtain` remains false. No authenticated vendor API, physical storage read, acquisition, firmware/configuration write, flash, reboot, reset or live USSD is enabled. Windows routing was never altered automatically.

## Sprint B disposition

| Item | Status / files / tests / commands | Left undone |
| --- | --- | --- |
| B1 strategy design gate | NOT STARTED; no code/design/test changes | Requires Part 1 re-review first. Then write trait signatures, evidence/error/approval flow in FIRMWARE_ACQUISITION_ENGINE.md and stop for design approval. |
| B2 artifact classes | NOT STARTED; none | No category/schema/Library migration. |
| B3 vendor registry | NOT STARTED; none | No new caller paths or family API. |
| B4 authenticated identity | NOT STARTED; none | No credential collection; exact GET-only names need device-JavaScript citations and owner supervision before client code. |
| B5 HTTP binding | NOT STARTED; none | No MAC identity or new session authority. |
| B6 eleven stages | NOT STARTED; none | Existing five-summary/collapsed UI retained. |
| B7 official-image registry | NOT STARTED; none | No real publisher URLs or downloads. |
| B8 analysis design/stub | NOT STARTED; none | Existing design retained; no worker or tool execution. |

H02 and H03 remain separate pending hardware rows; H04/H05 pending. No new label photo, exact revision, acquisition hash/provenance or analysis result exists. No merge to main and no Phase 8.

## Re-review handoff

Branch: `phase-7-5-acquisition-engine`. Start: `1703d87ed340c07e67cdb5d11c3032849830ae72`. End commit is supplied in delivery to avoid a self-referential hash. Review `git diff 1703d87..HEAD`, especially proof-only network entry, parser context, exact captured-prefix bytes, virtual/real clock separation and BUSY receipt ownership. Re-run default-parallel workspace tests and inspect the twenty-run evidence separately from native and physical evidence. Part 2 waits for reviewer response; B1 code also requires the subsequent design approval.


## Completed native, build and normal-data checks

| Command | Result |
| --- | --- |
| `npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline` | 0; ordinary optimized Windows executable |
| `npm.cmd run build:native-tests` | 0; dedicated isolated-profile optimized Windows executable; ordinary binary preserved |
| `node scripts/verify-foundation.mjs --device-gui --phase4 --phase4-1 --phase6` | 0; 107 native checkpoints, zero console issues, development/recovery/production windows and IPC; three actual GUI Cancel/reconnect/read checks |
| `npm.cmd run lint` after supervised-hardware harness changes | 0 |
| `py scripts/probe-native-isolation.py .tools/phase7-5-review/normal-after-software.json .tools/phase7-5-review/normal-before.json` | 0; normal application snapshot unchanged |

Retained native evidence: `.tools/foundation/2026-10-22T23-57-42.981Z/`. Tests used generated harmless binaries, isolated database/vault/WebView storage, retained job/settings/source-profile regressions, and safe owned-process cleanup. No router requests are enabled by this regression command. The subsequent supervised harness records owner Yes/No decisions separately, copies the exact owner-stated firmware string into the isolated profile as user-reported (never verified), and compares normal data before/after each run. It waits up to five minutes for the owner; no automatic Yes. This is test tooling, not a new adapter or device-write path.

Initial Windows observation found USB and Wi-Fi on different subnets; the owner then connected Wi-Fi to MG11 for the actual collision test. No route or router setting was changed by the agent.


## Final twenty-run parallel evidence

Command for every repetition: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline`. **All 20 consecutive final runs exited 0**, each with 240 passed / 1 ignored library tests, plus successful binary/doc-test targets. No `--test-threads=1` or serial scheduling was used. Both the exact virtual-expiry case and scripted-USSD case passed in each of the 20 logs. Repetition evidence: `quiet-parallel-runs.json` and `quiet-parallel-1.log` through `quiet-parallel-20.log` in the task directory. The earlier run-18 OpenWrt readiness timeout is retained separately; it was not relabelled or discarded.


## Hardware findings and corrective follow-up

1. First No-labelled attempt: owner chose Yes while changing Wi-Fi networks. The UI invalidated its result during discovery revision change; no report was published. It does not qualify either a No decision or a completed metadata read. Actual UI text was captured locally, then only the identified owned test window was closed. Harness exited 1 on the resulting CDP disconnect/timeout, preserved failed profile/evidence and confirmed normal data unchanged. No write-capable endpoint exists in this flow.
2. Fresh Wi-Fi-on collision case: owner connected Wi-Fi to MG11 (Wi-Fi 192.168.100.200/24; USB 192.168.100.100/24), then clicked Yes. Command `node scripts/verify-foundation.mjs --release-only --device-gui --acquisition-mg11` exited 0. Actual report: subnet_conflict, HTTP status null, named Wi-Fi in the plain UI message. Source guard returns before TCP preflight and WinHTTP dispatch. Screenshot visually inspected at `.tools/foundation/2026-10-23T00-09-17.767Z/production-h01-acquisition.png`. Owner confirmed settings/firmware unchanged; normal-data comparison passed.
3. Initial USB-only case: owner disconnected Wi-Fi, kept USB and clicked Yes. Same command exited 0 for generic native checks; HTTP 200 and candidate asr_duster_xml were real. However, no endpoint-observed firmware appeared, so the requested static-version subcheck FAILED. Evidence `.tools/foundation/2026-10-23T00-11-26.569Z/`. This exit 0 is not presented as a pass of the missing-version requirement. Owner confirmed unchanged settings/firmware; normal data preserved.
4. Bounded diagnostic of the already authorized exact static path: unauthenticated USB-only `GET http://192.168.100.1/version_num.txt` returned HTTP 200 / 115 bytes: XML declaration, RGW wrapper and firmware_version REL_MRVL_UAPXC_v2.6.0. No proxy, redirects, cookies or default credentials; three-second cancellation, 4 KiB read bound. This was an independent diagnostic, not claimed as an application proof-token test. Initial PowerShell attempt failed before dispatch because System.Net.Http was not loaded; Add-Type corrected the diagnostic. Raw non-secret static response became `fixtures/asr-version-redacted.xml`.
5. New genuine-wrapper regression failed with exit 101 before correction. `version_evidence` now accepts a bounded XML declaration and optional exact RGW wrapper, retains bare-tag compatibility, does not evaluate XML, and rejects entity values/DOCTYPE, duplicates, nested/unbalanced wrappers and unsafe value characters. Firmware evidence remains endpoint_response/observed/observed, never verified. New negative regression covers entities/duplicate tags/malformed wrappers.
6. Repeated actual MG11 No case: owner clicked No, with USB connected and Wi-Fi off. `node scripts/verify-foundation.mjs --release-only --device-gui --acquisition-mg11 --h01-no` exited 0; no network approval/report and normal data unchanged. Evidence `.tools/foundation/2026-10-23T00-16-25.870Z/`. This is the qualified No result, separate from the interrupted first attempt.

Post-parser-fix commands: full default-parallel workspace test exited 0 (242 passed / 1 ignored), isolated-profile library suite exited 0 (243 passed / 2 ignored), both Clippy profiles and final formatting check exited 0. Same exact commands as above; final logs `version-final-standard.log`, `version-final-isolated.log`, `version-clippy-standard.log`, `version-clippy-isolated.log`, `fmt-final.log`. The 20-run clock qualification preceded this independent static-parser follow-up (240 tests each); clock/expiry code did not change afterward. Frontend tests remain 145, lint 0 and frontend build 0; no frontend production source changed during this parser fix. Both corrected Windows release builds exited 0. Final supervised USB result is recorded below.

7. Final corrected USB-only case: owner clicked Yes in the native dialog with Wi-Fi off and USB connected. The same supervised command exited 0. Actual GUI shows HTTP 200, candidate `asr_duster_xml`, observed `REL_MRVL_UAPXC_v2.6.0` beside the owner-reported `MF69_AM802L_ZX_DD_N_241106_EU`, both unverified. Exact model/revision and authenticated identity remain unknown; acquisition remains disabled. Summary, redacted diagnostic, screenshot and zero-console capture are in `.tools/foundation/2026-10-23T00-22-50.852Z/`. Screenshot was visually inspected. Normal data comparison passed and owned test processes/profile were cleaned by the existing guarded harness.

## Final checks, files and decision

| Exact command | Exit / result |
| --- | --- |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | 0; final 242 passed / 1 ignored, default parallel; `version-final-standard.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --features native-test-profile --locked --offline` | 0; final 243 passed / 2 ignored; `version-final-isolated.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all -- --check` | 0; `fmt-final.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | 0; `version-clippy-standard.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0; `version-clippy-isolated.log` |
| `npm.cmd test` | 0; 145 frontend tests / 17 files; `frontend.log` |
| `npm.cmd run lint` | 0; repeated after final harness edits; `lint-final.log` |
| `npm.cmd run build` | 0; TypeScript plus Vite; also rerun by both release scripts |
| `npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline` | 0 after parser correction; `version-build-production.log` |
| `npm.cmd run build:native-tests` | 0 after parser correction; ordinary executable preserved; `version-build-isolated.log` |
| `node scripts/verify-foundation.mjs --device-gui --phase4 --phase4-1 --phase6` | 0; 107 checkpoints, zero console issues, including dev/recovery/release GUI Cancel/read. Before independent static-parser follow-up; focused corrected hardware test followed. |
| `node scripts/verify-foundation.mjs --release-only --device-gui --acquisition-mg11` | 0 for qualified Wi-Fi-on and final corrected USB-only cases; summaries above. Initial USB command exited 0 but missing-version subcheck failed, explicitly retained. |
| `node scripts/verify-foundation.mjs --release-only --device-gui --acquisition-mg11 --h01-no` | First interrupted attempt 1; qualified owner-No repeat 0. |
| `py scripts/probe-native-isolation.py .tools/phase7-5-review/normal-final.json .tools/phase7-5-review/normal-before.json` | 0; full normal application snapshot equals initial baseline, integrity ok, no FK violations. |
| `git diff --check` | 0; captured HTML intentionally byte-preserved with binary diff attribute. |

Changed files:

- `.gitattributes`; `apps/desktop/src-tauri/fixtures/asr-root-redacted.html`; `apps/desktop/src-tauri/fixtures/asr-version-redacted.xml`: genuine bounded captures and exact-byte preservation.
- `apps/desktop/src-tauri/src/acquisition/approval.rs`, `ipc.rs`, `mod.rs`, `tests.rs`; `apps/desktop/src-tauri/src/connection.rs`: claimed proof, BUSY order, sanitized confirmation, contextual family recognition, safe static-version parser and regressions.
- `apps/desktop/src-tauri/src/adapters/lifecycle.rs`: virtual/physical time separation and deterministic regressions.
- `scripts/verify-device-gui.mjs`: supervised physical Yes/No evidence; isolated user-reported firmware only, no automatic Yes.
- `docs/PHASE_7_5_REVIEW_FIXES.md`, `docs/PHASE_7_5_VERIFICATION.md`, `docs/PHASE_7_5_SPRINT_A_CLOSEOUT.md`, `docs/PHASE_STATUS.md`, `docs/HARDWARE_TEST_MATRIX.md`, `docs/evidence/phase-7-5-review-2026-10-23.json`: actual results, limitations and review gate.

Machine-readable evidence: [review summary](evidence/phase-7-5-review-2026-10-23.json). Full raw logs/screenshots remain under `.tools/phase7-5-review/` and the dated `.tools/foundation/` directories. Generated simulator screenshots under `docs/images/phase6-ab9b7d2675ca6ce25660adbaffed50ba/` are retained locally, excluded from this correction commit. Ordinary executable: `target/release/obtainer.exe`, SHA-256 `034f3b572b5853db02ab5a2658090dcbf8b5a62e2103fc04f76f4cf52416739e`.

R1-R5 software and supervised reader checks PASS. Owner confirmed final router settings and firmware display unchanged. Part 1 re-review remains pending. Sprint A retains PASS WITH LIMITATIONS. No firmware file was acquired, so no acquisition capability is claimed. B1-B8 and Phase 8 remain NOT STARTED. Next step: re-review `git diff 1703d87..HEAD`; only acceptance allows B1 design, followed by its separate design approval gate.

Final owner acknowledgement (23 October): "Settings and firmware unchanged" after the corrected USB read and earlier No test. This is owner-observed device preservation, not an independent dump of its internal database.
