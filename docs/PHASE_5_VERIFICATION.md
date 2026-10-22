# Phase 5 verification

21 October follow-up: [independent review corrections](POST_REVIEW_CORRECTIONS.md) document retained sessions after unavailable-feature reads, stable discovery revisions and the existing Phase 7 remote-endpoint route. Security failures still invalidate sessions. Original acceptance evidence below is historical.

Date: 19 October 2026, Africa/Lagos. **Phase 5 verdict: PASS. Phase 6 readiness: READY.** Phase 5 implementation and required qualification are complete. No Phase 6 implementation or physical router configuration operation was started.

## Post-review authentication bootstrap correction — 20 October 2026

The review identified a real bootstrap block. Previously `SessionManager::connect` called the established `authorize(...AuthorizedIdentityProbe...)` gate before the connector. That gate requires an exact VERIFIED profile and strong authenticated-management/supported-modem evidence for the bound endpoint/interface. The successful established-session fixtures already contain this evidence. A freshly discovered device with only Windows observations or user hints cannot pass; calling it an identity probe did not bypass the requirement.

`cold_start_reproduces_established_gate_block_without_seeded_authentication` demonstrates this explicitly: no authenticated-management or modem evidence, no selected profile, established connect returns `IDENTITY_UNVERIFIED`, no connector resource/session remains. The established policy and all 29 original adapter tests are retained.

### Corrected lifecycle

1. Discovery/profile evaluation remains passive and unverified. The backend selects the exact endpoint, live nonvirtual Windows interface, adapter/version and an expected authoritative profile/version/hash. An expected profile constrains the later identity check; it does not verify this device. A peer pin must be selected independently of an untrusted HTTP response.
2. An explicit user approval is represented by the opaque backend-only `HandshakeApproval::confirm_selected_target`. It binds the whole target, available evidence digest, discovery revision, expiry and simulation provenance. It is not serializable or cloneable, is single-use, lasts at most 30 seconds using a monotonic deadline and cannot carry arbitrary operations. Phase 7 must connect this constructor to its reviewed target/pin approval UI; Phase 5 adds no active IPC or approval screen.
3. `SessionManager::connect_initial` validates current evidence, target/interface availability, trusted expected profile, unique compatible adapter and implementation/provenance. Protected hardware, disconnected/stale/ambiguous/conflicting devices are denied. Only transport setup/authentication and one identity request are allowed. No capability/configuration read occurs in this path.
4. The connector contract requires exact interface/peer trust verification before credentials, bounded IO, and no redirect or interface fallback. After authenticated connect, the manager requires an exact `EndpointTrust` attestation before asking for identity. Missing attestation or substituted endpoint/interface/pin is rejected.
5. Actual returned manufacturer/model, hardware revision, firmware family/version and board/management identity feed the existing profile engine. Only a unique VERIFIED result matching the expected authoritative profile publishes a session. Missing/wrong identity is denied. The resulting evidence exists only in session memory; starting reports, discovery records and SQLite are unchanged. Simulated evidence is explicitly labeled and cannot enter a physical session.
6. `confirm_session_reads` is a separate explicit backend read-approval step. It derives its binding from private verified session state and invokes the retained established policy for every requested operation. Every read validates the starting device/evidence snapshot plus verified profile, endpoint/interface, adapter, revision, operation and expiry. A grant over the original unverified report cannot authorize an established read. Failed/stale/cancelled operations release the connection and concurrency slot.

The successful controlled test `cold_start_authenticates_then_requires_separate_established_read_approval` starts with Windows/user-hint evidence only, authenticates through the existing `cfg(test)` connector, verifies its peer and actual identity, obtains a separate established grant, then successfully reads identity and capabilities. `cold_start_also_works_without_user_identity_hints` repeats authentication with an empty starting identity-evidence list. Neither success seeds authenticated evidence; authenticated records are generated only from the returned handshake identity. Both leave the original report unverified and release all resources on disconnect.

### Files changed for this correction

- `apps/desktop/src-tauri/src/adapters/bootstrap.rs` — single-use handshake approval, initial policy, endpoint/identity verification and session-local evidence.
- `apps/desktop/src-tauri/src/adapters/lifecycle.rs` — separate initial connection/read approval, retained reconnect gate, current-evidence validation on reads and management-identity checks.
- `apps/desktop/src-tauri/src/adapters/types.rs` — internal endpoint attestation and returned management identity.
- `apps/desktop/src-tauri/src/adapters/contracts.rs` — endpoint attestation and precise bootstrap transport/authentication obligations.
- `apps/desktop/src-tauri/src/adapters/mock.rs` — controlled test-only attestation and missing/substituted-peer/board-identity faults. No physical IO.
- `apps/desktop/src-tauri/src/adapters/bootstrap_tests.rs` — 15 cold-start regression tests.
- `apps/desktop/src-tauri/src/adapters/mod.rs` — module exports; `policy.rs` — established-policy documentation only.
- `scripts/verify-foundation.mjs` — wait for restored fault-injection exports without a redundant forced navigation that races pending Tauri callbacks; retain the strict console gate.
- `scripts/drive-folder-dialog.ps1` — select only visible owned dialogs/controls, wait for the confirmation button to enable and require the native picker to close before reporting success.
- `docs/PHASE_5_IMPLEMENTATION.md`, `docs/PHASE_5_VERIFICATION.md`, `docs/PHASE_STATUS.md` — design and current evidence.

No original established-session tests, profile registry/engine, discovery implementation, GUI, IPC permissions, dependencies, database schema or data location were replaced.

### Correction checks

Rust compilation used `CARGO_INCREMENTAL=0`. Storage regressions ran with approved normal Windows directory access to retain existing safe path leases. Commands/results are also recorded in `.tools/phase5-correction-{rust,frontend}-results.json`; per-command logs have matching prefixes.

| Exact command | Actual result |
| --- | --- |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all` | Exit 0; formatting applied. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline adapters:: -- --test-threads=4` | Exit 0, initial 42 adapter tests; two additional tests and missing-attestation coverage were added before final full qualification. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all --check` | Exit 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | Exit 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | Exit 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | Exit 0, 130 tests: all previous 115 plus 15 cold-start tests. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | Exit 0, 130 tests; both executable test targets and doc tests pass. |
| `node node_modules/typescript/bin/tsc --project apps/desktop/tsconfig.json --noEmit` | Exit 0. |
| `npm.cmd run lint` | Exit 0. |
| `npm.cmd test` | Exit 0, 99 tests / 11 files. |
| `npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline` | Exit 0; production frontend and optimized ordinary Windows executable built. Rust release compilation 1m 52s. |
| `npm.cmd run build:native-tests` | Exit 0; dedicated optimized test executable built, Rust release compilation 2m 04s. Ordinary executable SHA-256 preserved. |
| `node --check scripts/verify-foundation.mjs` | Exit 0 after the native verifier correction; lint rerun also exits 0. |
| PowerShell `Parser::ParseFile` on `scripts/drive-folder-dialog.ps1` | Exit 0, no parse errors after readiness/closure/visibility changes. |
| `npm.cmd run test:gui -- --phase5` | Attempts exit 1, 1, then **0**; failures retained below. Final run **103 checkpoints**, zero console issues. |
| `py scripts/probe-native-isolation.py .tools/phase5-correction-normal-before.json` | Exit 0; normal data captured read-only before changes. |
| `py scripts/probe-native-isolation.py .tools/phase5-correction-normal-after.json .tools/phase5-correction-normal-before.json` | Exit 0; exact full byte/row/settings/vault equality after all attempts. |
| `git diff --check` | Exit 0 after final source/documentation updates. |

The 15 tests cover first authentication with/without hints; the old bootstrap denial; separate read approval and approval replay; bad credentials; endpoint/interface/pin/adapter/profile/evidence/revision substitution; absent/substituted trust attestation; wrong firmware family and board identity; cancellation before/during authentication; bounded connection/authentication/identity timeouts; wall-clock and monotonic approval expiry; invalid/down/virtual interface and physical/simulated provenance; protected/disconnected/ambiguous/conflicting devices; stale discovery, changed evidence, session expiry, shutdown/race cleanup; and rejection of configuration writes, firmware acquisition/build/installation and active USSD.

The first native attempt exited 1 at the strict console gate: `.tools/foundation/2026-10-20T02-24-20.986Z/`. Its GUI/IPC, development/release restart, storage and normal-data equality checks passed, but two development warnings were captured: `IPC custom protocol failed, Tauri will now use the postMessage interface instead` and a missing callback after reload. Their timestamps align with the verifier's `Page.reload` immediately following restoration of temporary backend fault injection; the first stack is an in-flight Library read, followed by a callback in the replaced execution context. The verifier now waits for restored module exports and retains the page context before the component HMR proof. No warnings are filtered or ignored, and no product IPC fallback code was changed. Failed isolated run `9fa662d21f3ecd456736ad40df67bba9` and its diagnostic evidence are retained; normal data was byte/row-identical after that attempt.

The second attempt (`.tools/foundation/2026-10-20T02-30-04.679Z/`, exit 1) captured **zero** console warnings/errors/exceptions in `console.json`, confirming the reload correction. It later timed out waiting for the second reviewed storage-copy plan in the retained GUI suite (`Reviewed storage plan missing`). The folder-picker driver had reported success merely when a native click was posted; it did not require an enabled confirmation button or dialog dismissal. It now scopes to visible owned dialogs/controls, waits for readiness and requires actual closure. Product storage code and its copy/restart assertions are unchanged. Failed isolated run `6bf7857977f6782cbd7f838d840a0e5c` is retained; normal-data equality passed again.

Final native run: **exit 0, 103 checkpoints** in `.tools/foundation/2026-10-20T02-35-07.136Z/`. `console-issues.json` is empty; all 39 captured console entries are informational/debug, with no warnings/errors/unhandled exceptions. Actual development/recovery/optimized-release windows, React/Rust IPC, six destinations, passive adapter queries, disabled extraction/customization, HMR, interrupted startup, SQLite/settings persistence and retained library/backup/storage workflows pass. The folder picker now passes both cancellation and reviewed-copy interactions. Actual development and production adapter-status captures were visually inspected. These native runs verify retained production behavior; the new active cold-start connection is demonstrated only by the controlled unit-test connector, which is not compiled into either native executable.

Exact native child commands recorded in final `results.json`:

- `npm.cmd run tauri -- dev --config src-tauri/tauri.native-test.conf.json --features native-test-profile -- --bin obtainer-native-test --locked --offline`: exits **0, 1, 0**. The middle launch is intentionally terminated to verify interruption recovery.
- `.\\target\\release\\obtainer-native-test.exe`: two launches, exits **0, 0**.

Every owned app/WebView/Vite process and development listener stopped. Guarded cleanup exited 0 and removed only successful marked run `10782addffb4846a1f932138357ae87e`; diagnostic failed runs remain retained. Temporary backend/Preferences edits were restored, and the final Git inventory contains no product frontend changes. Current ordinary executable SHA-256: `EA1DC5AF55C408B17270E119066C7D066B6F814BF49DEEB21929FCB6F364B2C2`.

Final independent normal-data comparison is exactly equal to `.tools/phase5-correction-normal-before.json`: schema 5, integrity `ok`, no FK violations, every DB/WAL byte/hash and table row, settings, saved router/profile IDs, vault files and logs/workspaces/reports unchanged. Entire-row SHA-256 remains `6c4c4640c029fc1d0bb91e24f07bac4e1f321dcf7222bddce886db5f42a27e91`; settings SHA-256 remains `70ca50b9ab62d27cbc6340ed90d7ebab7596b61ac8875a63e86219af831f4d97`. A read-only SQLite query also confirms `LT800/` and `MF69_AM802L_ZX_DD_N_241106_EU` remain `user_reported` / `user_supplied`. MG11 was absent in native discovery, so no new physical qualification is claimed. No router authentication/configuration request was made; native firmware regressions used generated harmless test binaries only.

**Correction verdict: PASS. Phase 5: PASS. Phase 6: READY.** First authentication is now proven without preseeded authenticated evidence; established-session policy and all previous regressions pass. Firmware extraction/build/installation, configuration writes, restore and active USSD remain disabled. No full simulator, physical OpenWrt adapter, active IPC, GUI redesign, database migration or data-directory change was introduced.

## Implementation and source audit

The [implementation report](PHASE_5_IMPLEMENTATION.md) contains the audit map, architecture, matching/policy/authentication/lifecycle design, threat model, extension guide and limitations. Work remains in the existing repository, branch `main`, starting HEAD `f7a450a0b48647e0cad01d1af24f297917ba9080`. No commit, history reset or repository relocation was requested or performed. The initial continuation inherited uncommitted Phase 5 changes and hardened them in place.

Changed/new files:

- `apps/desktop/src-tauri/src/adapters/{mod,types,contracts,errors,registry,policy,auth,lifecycle,mock,tests}.rs`.
- `apps/desktop/src-tauri/src/{lib,commands}.rs`; `discovery/{identity,service,tests}.rs`; `profiles/mod.rs`.
- `apps/desktop/src-tauri/build.rs`, `capabilities/main.json`, `permissions/autogenerated/{get_device_adapter_matches,list_available_adapters}.toml`.
- `apps/desktop/src/types/adapters.ts`, `src/lib/backend.ts`, `src/features/{AdapterStatus,Compatibility,ManualRouters}.tsx`, `src/styles.css`.
- `apps/desktop/src/test/AdapterStatus.test.tsx`, `adapterBackend.test.ts`, `Compatibility.test.tsx`, `adapterFixtures.ts`.
- `scripts/verify-adapters.mjs`, `verify-foundation.mjs`, `verify-gui.mjs`.
- `docs/PHASE_5_IMPLEMENTATION.md`, this report, `PHASE_STATUS.md`.

The existing frontend/Rust, discovery, profile registry, firmware library, SQLite schema 5, six-page GUI and root scripts are retained. Adapter queries are passive. No new dependency or database migration was added. Tests use generated harmless binaries and isolated temporary storage.

## Executed commands

Commands run from repository root. `CARGO_INCREMENTAL=0` was set for final Rust/native qualification. Rust tests need Windows directory leases outside the restricted sandbox; approved execution preserves those checks rather than bypassing them.

| Exact command | Result |
| --- | --- |
| `node node_modules/typescript/bin/tsc --project apps/desktop/tsconfig.json --noEmit` | Exit 0. |
| `npm.cmd run lint` | Exit 0, no warnings. |
| `npm.cmd test` | Exit 0, **99 tests / 11 files**. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all` | Exit 0; formatting applied before final check. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all --check` | Exit 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | Exit 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | Exit 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | Exit 0, **115 tests**, including retained regressions. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | Exit 0, **115 tests**; both application test targets and doc tests also exit 0. |
| `npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline` | Exit 0. Production `tsc -b` / Vite and optimized Windows executable built; Rust compilation 1m 55s. |
| `npm.cmd run build:native-tests` | Exit 0. Dedicated optimized test executable built in 1m 33s; normal executable SHA-256 preserved. |
| `npm.cmd run test:gui -- --phase5` | Final exit **0**, **103 checkpoints**. Actual development, recovery and optimized isolated release; all retained Phase 4/4.1 and six-page GUI regressions plus new adapter checks pass. Evidence: `.tools/foundation/2026-10-19T16-16-55.145Z/`. Earlier failed attempts are retained below. |
| `node --check scripts/verify-adapters.mjs` | Exit 0. |
| `py scripts/probe-native-isolation.py .tools/phase5-normal-before.json` | Exit 0, normal database opened read-only; schema 5, integrity `ok`, no FK violations. |
| `py scripts/probe-native-isolation.py .tools/phase5-normal-after.json .tools/phase5-normal-before.json` | Exit 0, exact full equality with the continuation's original fingerprint. |
| `git diff --check` | Final exit 0; platform LF/CRLF notices are informational. |

Native qualification used the dedicated test executable and separate fixed-root test data/WebView profile, not the owner's normal data. The ordinary release was built and its test-activation guard verified. Actual rendered native screenshots, DOM interaction, React/Zod-to-Rust responses and native window handles provide GUI evidence; terminal startup alone is not used as proof.

Native child commands recorded in `results.json`:

- `npm.cmd run tauri -- dev --config src-tauri/tauri.native-test.conf.json --features native-test-profile -- --bin obtainer-native-test --locked --offline`: three launches, exits **0, 1, 0**. The middle exit 1 is the deliberately interrupted owned launch/import; the subsequent development launch verifies recovery. It is not an unexplained passing command.
- `.\\target\\release\\obtainer-native-test.exe`: two optimized launches, each exit **0**, including restart after the verified Library copy.

Final read-only Windows census exits 0: **0** workspace application processes, **0** workspace Vite servers and **0** listeners on 1420/9223/4445/4446. Captured console has 32 informational/debug entries and **0** warnings/errors/unhandled exceptions. Successful marked run `ecf26b2df762d31d1bec8509481b2415` was removed by the existing guarded cleanup after all owned processes stopped. Earlier failed diagnostic runs remain retained; no cleanup targeted the normal database/vault.

Current ordinary release executable SHA-256: `4DF50209918EE32B867ABE6350D453EC86319022AF5B0B431BF3229F1EE4B6EA`. The dedicated build helper independently checks this remains unchanged after compiling the separate test target.

## Actual native results

The final run verifies all six destinations, actual React/Rust/SQLite IPC, three duplicate-free Windows refreshes per session, new registry/match queries, malformed inputs and absent write/execution endpoints. Host NIC observations remain generic/protected; no test adapter or connected management state is fabricated. Saved endpoint creation uses the real Add Router form; selecting adapter status issues only a passive query, and the saved row retains no HTTP observation. Extract Firmware and all customization controls stay disabled.

The retained suite also verifies real native Open/Save/folder dialogs; generated harmless 1 MiB/256 MiB file import; SHA-256 vault bytes; duplicate occurrences; streaming progress/cancel; concurrent identical imports; corrupt/missing blob recovery; database and manifest failure handling; real backups and `_DEFAULT` restoration; confirmation cancellation; reversible list removal; logs; appearance/refresh persistence; actual component hot reload/restoration with unchanged app/Vite PIDs; graceful/forced interruption recovery; compact 920×680 layout; six keyboard-accessible page-help dialogs; and reviewed storage copying with restart and integrity checking at the copied location.

Actual [development adapter status](images/phase5-dev-adapter-status-20261019.png) and [optimized isolated-release adapter status](images/phase5-release-adapter-status-20261019.png) were visually inspected. The panel renders protected hardware, rejected adapter details and four UNKNOWN features without overflow. These are native WebView captures, not mockups. Historical GUI screenshots were not overwritten.

## Actual failures and corrections

- Initial scaffolding omitted command permission registration, filtered all test adapters from matching, accepted caller-controlled authentication flags and matched an incorrect hardcoded profile ID. These were corrected before final qualification: immutable validated registry, family/profile constraints consumed from actual profile results, scoped backend-only grants, real fixture handshake/identity checks, explicit permissions and isolated test registration.
- Initial Rust formatting check exited 1; formatting was applied with the repository helper and final formatting exits 0.
- First restricted Rust test execution exited 1 with **83 passed / 27 failed**. Existing storage/native-isolation tests failed while leasing `C:\\Users\\chuka`: `Access is denied. (os error 5)`. No path-safety code was weakened. Approved rerun passed all **110** tests at that stage; final expanded suites pass **115** in each profile.
- Rust compilation initially warned that `AtomicUsize::fetch_update` was renamed; changed to the installed toolchain's `try_update`. Clippy then rejected an authentication `map_err` used only for a side effect; corrected to `inspect_err`. Final Clippy exits 0 under `-D warnings` for both profiles.
- First full frontend run exited 1 with **93 passed / 1 failed**: the new adapter assertion ran while the independent request was still loading. The test now awaits the actual response text. Final expanded frontend run passes **99** tests.
- Intermediate `git diff --check` found a new blank line at the end of `contracts.ts`; removed it. Final whitespace result will be recorded after documentation is complete.
- Native GUI helper still clicked the former whole-device card. It now invokes the existing **Check compatibility** and **More info** buttons and closes the actual popup. Assertions remain on real selection, disabled extraction and existing workflows. Phase 5 captures go to a new evidence folder, preserving historical GUI screenshots.
- First native attempt (`.tools/foundation/2026-10-19T16-11-29.074Z/`, exit 1) passed actual development rendering, registry/direct-IPC rejection, host-hardware policy and React adapter status, then failed `Saved endpoint UI missing`. The verifier inserted a row through IPC after the view had loaded its list. Corrected the verifier to save through the actual **Add Router** form, exercising the real user workflow. No product list-refresh behavior was substituted or fabricated. Normal-data equality passed; failed isolated run `244f38bd1219badab2c1bf320b239b8c` is retained.
- Second native attempt (`.tools/foundation/2026-10-19T16-14-17.836Z/`, exit 1) passed adapter/saved-endpoint GUI checks and generated-file integrity/cancellation/concurrency/failure regressions, then failed `HMR proof anchor missing`. The helper referenced filler copy removed in an earlier user-requested cleanup. Changed the temporary test anchor to the existing `<h2>Appearance</h2>`, preserving the actual hot-reload/PID/source-restoration assertions. Normal-data equality passed; failed isolated run `a1830d029f0c79a8dbdefc1c5652e83f` is retained.

## Security and data evidence

Automated Rust coverage includes exact/partial/ambiguous matches, unique exact precedence, unknown/wrong/reported identity, protected PCI/storage/HID hardware, scoped operation/endpoint/interface/adapter/profile/evidence grants, endpoint pin mismatch, missing/invalid credentials, redacted errors, explicit authentication transitions, handshake identity mismatch, genuine deadline expiry, cancellation, expiry/revision invalidation, unsupported DNS/APN/Wi-Fi/USSD/backup, forbidden physical operations, two-session limit and cleanup after faults/shutdown. Mock is `cfg(test)` only and never appears in either real executable's inventory.

Frontend coverage includes protected/missing/ambiguous adapter states, authentication requirements, disabled device actions, malformed/writable/simulated/connected passive responses, typed errors/retry, expiry, late-response suppression and saved-endpoint requests. Existing navigation, help, library/backups/settings and compatibility tests remain passing.

Normal baseline `.tools/phase5-normal-before.json` records full DB/WAL hashes and rows, settings, saved router/profile IDs, vault and other data directories. Baseline: schema 5; 160 discovery rows; one saved endpoint; two profile versions; two user-reported identity observations; one generated 1 MiB artifact; 34 historical jobs; 220 audit rows. No normal application launch or migration is required by this work.

Final `.tools/phase5-normal-after.json` is exactly equal: every original table row, saved setting, router/profile ID, artifact/job row, DB/WAL byte/hash, vault file and logs/workspaces/reports fingerprint remains unchanged. Integrity is `ok` and FK violations are empty. Entire-row fingerprint: `6c4c4640c029fc1d0bb91e24f07bac4e1f321dcf7222bddce886db5f42a27e91`; settings fingerprint: `70ca50b9ab62d27cbc6340ed90d7ebab7596b61ac8875a63e86219af831f4d97`. No real MiFi firmware was imported, overwritten or modified.

An additional read-only SQLite query confirms the saved `LT800/` identifier and `MF69_AM802L_ZX_DD_N_241106_EU` firmware value retain `user_reported` verification and `user_supplied` source. Actual native Windows observations report MG11 absent during these runs; no fresh physical qualification is claimed. Existing profile/opaque-version and four-feature UNKNOWN regressions remain passing.

## Acceptance gates

| Gate | Current classification and evidence |
| --- | --- |
| Adapter interfaces, validated registry/descriptors and strongly typed capabilities | VERIFIED by compiled implementation and Rust contract tests. |
| Existing profile/discovery evidence, exact precedence and conservative exclusions | VERIFIED by real profile-engine fixtures and classification regressions. |
| Backend policy, secret handling and forbidden-operation denial | VERIFIED by Rust/frontend tests and actual native malformed/direct-IPC rejection. |
| Authentication, deadline/cancellation, expiry and cleanup | VERIFIED in isolated deterministic sessions. Physical protocol qualification remains outside Phase 5. |
| Typed commands and permissions | VERIFIED by strict Zod/request tests and real commands in development/recovery/optimized isolated release. |
| Existing six-page navigation, page-help and extraction/write restrictions | VERIFIED by retained unit and final 103-checkpoint native run. |
| Normal user-data preservation | VERIFIED by final exact DB/WAL/rows/settings/vault equality and guarded isolated cleanup. |
| Production/dedicated native builds and Windows dev/release rendering | VERIFIED: both builds exit 0; real native window/React/IPC and inspected screenshots from isolated development/release. |
| Implementation report / phase tracker | VERIFIED: implementation, verification and phase tracker updated; historical evidence retained. |

Real OpenWrt reads/authentication remain Phase 7; full simulator Phase 6; durable approvals/backups/recovery Phase 8; DNS/Wi-Fi/APN/USSD implementation belongs to later feature phases. No Phase 6 implementation has begun.

No critical Phase 5 blocker remains. Physical protocol qualification, clean-machine installers/signing, additional hardware/filesystems and future secret-bearing persistence are not qualified by this framework phase. Next action: begin Phase 6's explicitly simulated router/modem environment using these contracts when requested.

## Step closeout

| Step | Status | Executed result / files |
| --- | --- | --- |
| 5.1 baseline/audit | Complete, timing limitation disclosed | Existing-source map and data fingerprint in implementation report; retained baseline tests run in final suites. A full pre-change suite was not run before the initial scaffold. |
| 5.2–5.4 types/traits/errors | Complete | Compiled `adapters/{types,contracts,errors}.rs`; strict frontend schemas and typed errors. |
| 5.5–5.7 registry/evidence/policy | Complete | `registry.rs`, `policy.rs`; exact/weak/ambiguous/protected/stale/spoofed cases and direct-IPC denial pass. |
| 5.8–5.9 auth/lifecycle/mock | Complete | `auth.rs`, `lifecycle.rs`, cfg-test-only `mock.rs`; authentication, trust, real deadlines/cancel, resource and cleanup tests pass. |
| 5.10–5.11 IPC/UI | Complete | Commands/ACLs, `backend.ts`, strict adapter schemas, reusable status panel; real native dev/recovery/release requests and saved endpoint workflow pass. |
| 5.12 classification | Complete | `discovery/identity.rs` and tests distinguish actual host PCIe NICs from external USB network candidates; Overview/native display remain conservative. |
| 5.13 automated verification | Complete | 99 frontend tests, 115 Rust tests in each profile; TypeScript/lint/fmt/Clippy pass; actual failures/corrections retained above. |
| 5.14 native Windows | Complete | Both release builds, 103 native checkpoints, inspected native captures, console/process cleanup and exact normal-data equality. |
| 5.15 documentation | Complete | Implementation report, this verification report and phase tracker reflect actual results. |

Safety status for every step: no unauthorized physical-device request/configuration/firmware operation; native firmware tests use only generated harmless files in marked isolated storage. No later phase was implemented as a shortcut.

## Exact change inventory

Paths are repository-relative. Build outputs and local `.tools/` evidence are ignored by Git and are not source changes.

```text
apps/desktop/src-tauri/build.rs
apps/desktop/src-tauri/capabilities/main.json
apps/desktop/src-tauri/permissions/autogenerated/get_device_adapter_matches.toml
apps/desktop/src-tauri/permissions/autogenerated/list_available_adapters.toml
apps/desktop/src-tauri/src/adapters/auth.rs
apps/desktop/src-tauri/src/adapters/contracts.rs
apps/desktop/src-tauri/src/adapters/errors.rs
apps/desktop/src-tauri/src/adapters/lifecycle.rs
apps/desktop/src-tauri/src/adapters/mock.rs
apps/desktop/src-tauri/src/adapters/mod.rs
apps/desktop/src-tauri/src/adapters/policy.rs
apps/desktop/src-tauri/src/adapters/registry.rs
apps/desktop/src-tauri/src/adapters/tests.rs
apps/desktop/src-tauri/src/adapters/types.rs
apps/desktop/src-tauri/src/commands.rs
apps/desktop/src-tauri/src/discovery/identity.rs
apps/desktop/src-tauri/src/discovery/service.rs
apps/desktop/src-tauri/src/discovery/tests.rs
apps/desktop/src-tauri/src/lib.rs
apps/desktop/src-tauri/src/profiles/mod.rs
apps/desktop/src/features/AdapterStatus.tsx
apps/desktop/src/features/Compatibility.tsx
apps/desktop/src/features/ManualRouters.tsx
apps/desktop/src/lib/backend.ts
apps/desktop/src/styles.css
apps/desktop/src/test/AdapterStatus.test.tsx
apps/desktop/src/test/Compatibility.test.tsx
apps/desktop/src/test/adapterBackend.test.ts
apps/desktop/src/test/adapterFixtures.ts
apps/desktop/src/types/adapters.ts
docs/NATIVE_TESTING.md
docs/PHASE_5_IMPLEMENTATION.md
docs/PHASE_5_VERIFICATION.md
docs/PHASE_STATUS.md
docs/images/phase5-dev-adapter-status-20261019.png
docs/images/phase5-release-adapter-status-20261019.png
scripts/verify-adapters.mjs
scripts/verify-foundation.mjs
scripts/verify-gui.mjs
```
