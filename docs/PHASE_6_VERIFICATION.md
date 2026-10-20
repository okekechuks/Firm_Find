# Phase 6 verification

## 20 October 2026 post-review concurrency correction

**Phase 6 correction verdict: PASS. Phase 7 readiness: READY; not started.** Both reproduced ownership defects and the confirmed integration defects are corrected. Final checks pass: **165 Rust tests in each profile, 113 frontend tests, 130 native checkpoints, both optimized Windows executable builds**, formatting, Clippy, TypeScript and lint. The original acceptance record below is retained as historical evidence. Existing settings/device UI and import-history changes were present at the start and were preserved. Commit `5b54ff6` appeared during verification and was left intact; verified simulator source hashes still match the final files.

### Reproduction before the fix

`powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib post_review_ --locked --offline` exited **101: 0 passed, 2 failed** against the original implementation. Evidence: `.tools/phase6-correction-red.log`.

- `post_review_cancel_idle_releases_busy_and_allows_reads` found `busy == true` after an idle Cancel.
- `post_review_scenario_reserves_busy_before_mutating_state` paused scenario selection immediately before state mutation and found that it did not own `busy`.

The second test uses a one-shot channel rendezvous, not a sleep to provoke a race. Test hooks compile only under `cfg(test)` and are absent from both Windows application binaries.

### Ownership and control behavior

`runtime.rs::OperationGuard` replaces the unmatched Cancel acquisition and `ScenarioReservation`. `Runtime::begin` acquires the shared slot with compare-exchange while holding the cancellation-token mutex. Only a constructed guard releases the slot. Its destructor clears its token and releases `busy` under that same mutex, then notifies condition-variable waiters. Errors, early returns and unwinding therefore cannot release another owner's slot or strand their own slot.

| Entry point | Behavior under contention |
| --- | --- |
| Requests, Scenario, Advance | Try the same exclusive guard. Return `BUSY` without changing state or releasing the owner when occupied. |
| Cancel | Signal the token registered at entry, then wait at most one second for exclusive ownership. Idle Cancel is a successful scoped no-op; repeated Cancel is safe. A wait timeout leaves the current owner's token/slot intact. It does not cancel a later owner that overtakes the waiter. |
| Reset | Cancel and drain the current request before restoring the fixture and advancing generation. If the bounded wait fails, no reset state is published. |
| Stop | Disable authority immediately, cancel/drain, shut down the session manager and clear access. A pending request cannot reinsert a valid session; restart owns a different endpoint. |

Short Scenario/Advance controls complete atomically under their guard; a concurrent interrupt waits for them. Scenario changes advance configuration revision and clear cached features, previews and outcomes, so queued old requests and previews are rejected. Identity/endpoint/disconnection changes also revoke session authority and advance generation. Successful snapshots are captured before the operation guard is released; responses carry revision/generation, and the frontend suppresses superseded replies.

### Additional correctness findings

- Advance now rejects during an operation. The GUI disables Advance while waiting. At idle it updates virtual service/USSD timers, clears cached feature observations and reconciles session expiry before returning. Reads succeed at 29,999 ms and are denied at the 30,000 ms boundary without requiring a separate status poll.
- Device summaries use the mutable router identity. Wrong Identity displays `Wrong Virtual Device`; the expected bootstrap profile remains unchanged and rejects it. The frontend accepts only the two explicit fictional model names, rejects model/scenario contradictions and refuses a supposedly connected/authorized wrong identity.
- Wi-Fi, APN and scripted USSD actions now call their corresponding Phase 5 read gate before touching virtual state. DNS-only approval cannot authorize these features. A grant for the correct feature does not need unrelated DNS approval. No physical write operation was added to Phase 5.
- Cancellation and timeout refresh actual backend status, restore controls and suppress late success. New frontend coverage checks cancellation followed by another valid request and timeout recovery.

An idle authenticated session intentionally retains one virtual connection between requests. Each operation still releases its busy slot and token. Cancellation/timeout that invalidates a session, disconnect, reset and stop return connection count to zero; tests distinguish retained authenticated sessions from leaked operation resources.

### Current executed checks

| Exact command | Actual result |
| --- | --- |
| `py scripts/probe-native-isolation.py .tools/phase6-correction-normal-before.json` | Exit 0; read-only normal-data baseline, schema 5, integrity `ok`, no foreign-key violations. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all` | Exit 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib simulator:: --locked --offline` | Exit 0; **35 simulator tests**, including the original 21 and 14 new correction regressions. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | Initial sandbox run: exit 101, 138 passed / 27 existing Windows directory-lease permission failures. Normal Windows-access rerun: exit 0, **165 passed**. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline --features native-test-profile` | Same initial 138/27 sandbox result; normal Windows-access rerun: exit 0, **165 passed**, including executable/doc-test targets. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all --check` | Exit 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | Exit 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | Exit 0. |
| `node node_modules/typescript/bin/tsc --project apps/desktop/tsconfig.json --noEmit` | Exit 0 after the frontend schema correction. |
| `npm.cmd run lint` | Exit 0. |
| `npm.cmd test` | Exit 0, **113 tests / 13 files** after the schema correction; original tests retained. |
| `npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline` | Final exit 0 after the frontend schema correction; frontend production build and optimized ordinary executable pass. Rust compilation 1m 47s. |
| `npm.cmd run build:native-tests` | Final exit 0; optimized native-test executable, Rust compilation 1m 52s; ordinary executable hash preserved. |
| `node .tools/phase6-correction-activation.mjs` | Exit 0; ordinary release with `--simulator`, without test/data overrides, refuses activation with actual exit 1 before opening data. |
| `node scripts/verify-foundation.mjs --phase5 --phase6 --gui-preview` | Exit 0 after the schema correction; real native development window/React/IPC, adapter regressions, GUI Cancel/reconnect/read, HMR and cleanup pass. Console issues `[]`; normal data unchanged. |
| `npm.cmd run test:simulator` | Final exit 0, **130 checkpoints** across native development, interrupted-launch recovery, optimized production and storage-copy restart. Earlier failed attempts remain recorded below. |
| `py scripts/probe-native-isolation.py .tools/phase6-correction-normal-after.json .tools/phase6-correction-normal-before.json` | Exit 0; complete normal database/WAL/rows/settings/saved-device/vault/other-file equality. |
| `node --check scripts/verify-simulator.mjs` | Exit 0. File-picker helper also passed PowerShell parser validation and actual native Open/Save tests. |
| `git diff --check` | Final exit 0; Windows line-ending conversion notices are separate from whitespace errors. |

Rust commands use `CARGO_INCREMENTAL=0`. Permission failures were `Directory lease failed at "C:\Users\chuka": Access is denied. (os error 5)` in existing safe-path tests. No application path protection was relaxed; those same tests passed with normal Windows directory access. Per-command logs/results are `.tools/phase6-correction-*`; initial failure logs remain retained.

The first native run (`.tools/foundation/2026-10-20T12-37-15.801Z/`) exposed a real integration defect after correcting the summary: frontend validation still required literal `Virtual Router Lab`. It rejected `Wrong Virtual Device`, leaving the prior connected view visible; the verifier then reported `Enabled simulation button missing: Approve virtual handshake`. The explicit virtual-model union and contradiction checks correct this. The failed run and its screenshot/UI/error evidence are retained. Its normal-data before/after comparison passed.

The next full run (`.tools/foundation/2026-10-20T12-47-03.032Z/`) passed the simulator, including real GUI Cancel/reconnect/read, then failed `Native file selection did not produce React confirmation`. Its captured UI remained in `Selecting…`; the test helper had treated a queued child-button click as completed selection. `scripts/drive-file-dialog.ps1` now waits for the owned button to become enabled, sends the owned dialog its default IDOK command, and requires actual dialog closure before reporting success. This changes verification automation only. Its PowerShell syntax check passes. Failed-run files and normal-data equality evidence remain retained.

One retry was not executed because automatic approval review reached its usage limit; this was an approval-service failure, not a test/application result. After the user resumed work, the same elevated verification path was approved and resumed. No approval bypass was used.

Focused native rerun `.tools/foundation/2026-10-20T12-41-29.917Z/` passed **34 checkpoints** with isolated ID `aafccbc16eec063aad03af05e22a8224`. Real CDP mouse input clicked Cancel during Slow Read; backend recorded `Cancelled`, `busy:false`, zero connections and no read grant. Scenario selection, fresh handshake/authentication/read approval and successful adapter feature reads followed. JSON records both cancellation and subsequent successful state. The actual cancellation screenshot was visually inspected. Owned processes stopped before marker-validated cleanup.

Final build logs/results are `.tools/phase6-correction-build-{production,native}-final.log` and `.tools/phase6-correction-build-final-results.json`. Ordinary executable SHA-256 is `3DB472E6FCF5F1B4CA6BF89AFB187723AFF946A9BD1FDA6BE5205231CF091C87`. Both release checks intentionally use executable builds; MSI/NSIS installer bundling is not part of this correction. The earlier WiX `light.exe` installer failure was not changed or represented as passing. Cargo progress on stderr appears as PowerShell `NativeCommandError` formatting in redirected logs; actual process exit codes are recorded separately and are zero.

### Final native qualification and data preservation

Final successful run: `.tools/foundation/2026-10-20T14-52-11.917Z/`, isolated ID `64f7f648ac5ed40c13c69c9ddca71d10`. Parent command exit is 0; `results.json` has **130 passing checkpoints**, `console-issues.json` is `[]`, and `cleanup.json` records status 0 after all owned application/WebView/Vite processes stopped. Full output and result are `.tools/phase6-correction-native-qualified.log` and `.tools/phase6-correction-native-qualified-result.json`. The reviewable [correction evidence transcript](evidence/phase-6-post-review-2026-10-20.txt) retains the successful checkpoints and command outcomes.

| Native child command | Actual exits |
| --- | --- |
| `npm.cmd run tauri -- dev --config src-tauri/tauri.native-test.conf.json --features native-test-profile -- --bin obtainer-native-test --locked --offline -- --simulator` | 0 initial development; 1 deliberately interrupted launch; 0 recovered development. The intentional termination is expected. |
| `.\target\release\obtainer-native-test.exe --simulator` | 0 optimized production; 0 production storage-copy restart. |

In **all three simulator sessions** (`dev-reopen`, `dev-recovery`, `production`), actual mouse input clicked the rendered Cancel button during Slow Read. Independent Rust status recorded cancellation, `busy:false`, zero connections and no read grant. Scenario selection, fresh authentication/read approval and another successful feature read then completed. Each `*-simulation.json` preserves `afterGuiCancel` and `recoveredAfterCancel` as well as final stopped state. The separate timeout path also recovered without stranding resources.

Retained adapter, six-destination GUI, actual IPC, HMR, unknown physical capability, disabled extraction, settings/restart and storage regressions passed. These include native generated-binary imports, identical queued 256 MiB sources, cancellation/interruption recovery, corruption/missing-blob detection, database/audit failures and retries, backup/restore, removal/recovery, reviewed storage copy/restart and integrity checks from the copied vault. No real MiFi firmware was selected. Native screenshots and DOM assertions establish rendering; terminal startup is not treated as proof of a working GUI.

There are **48 final actual captures**, 16 per simulator session, under [the final image directory](images/phase6-64f7f648ac5ed40c13c69c9ddca71d10/). Added captures show cancellation recovery and the actual Wrong Identity summary. Visually inspected [production cancellation recovery](images/phase6-64f7f648ac5ed40c13c69c9ddca71d10/production-cancel-recovered.png), [production feature state](images/phase6-64f7f648ac5ed40c13c69c9ddca71d10/production-feature-status.png), [production wrong identity](images/phase6-64f7f648ac5ed40c13c69c9ddca71d10/production-wrong-identity.png) and the development wrong-identity/cancellation views. They show the rendered six-page blue desktop, mode banner, correct model and appropriate disconnected/connected controls. Earlier-attempt captures remain under their own run IDs.

The independent final probe exactly equals `.tools/phase6-correction-normal-before.json`, including bytes/hashes of SQLite and WAL, all rows, preferences, saved router/profile IDs, firmware vault and logs/workspaces/reports files. The resumed run's baseline also equals the original correction baseline. Normal storage remains `%LOCALAPPDATA%\OBTAINER`, schema **5**, integrity **`ok`**, no foreign-key violations:

- All rows SHA-256: `45d5a9f68b114512e6067327586198513af83d8403faea8cf29a3f399791e868`.
- Settings SHA-256: `b53a94def78900fd485d168459d6342e8ad0eab83c4164f7b07f74630e423b99`.
- Counts remain **181 device rows, one saved manual router, two profile versions, one firmware artifact, 20 import occurrences and 35 jobs**.
- No storage migration, normal-data cleanup, physical configuration change, extraction, flashing or live USSD occurred. MG11 was absent during native qualification; no new physical support claim is made, and its saved evidence is unchanged.

Native-test executable SHA-256 recorded after qualification: `68A8302DC5330690E92B62C4232B215010C9EC288CAD851802A3B71199434C68`. Ordinary release hash still matches the value verified by the build script. All eight recorded simulator/verification source hashes match the tested files; HMR instrumentation was restored. There is no unresolved critical correction blocker.

New Rust coverage uses controlled channel barriers for scenario/request/fault/interrupt ordering: idle and active cancellation, repeated cancellation, connection and DNS contention in both directions, competing scenarios, reset/stop during a delayed read, cancellation wait timeout without owner release, exact expiry, stale request/cache rejection, wrong summary, independent feature grants, unwind cleanup and physical namespace/action rejection. New frontend tests cover late cancellation success and the strict wrong-identity response contract.

Noncritical GUI improvement, deferred: a distinct “Cancelling…” label could make repeated clicks clearer. Repeated cancellation is handled safely by the backend; no redesign is required for this correction.

### Correction file inventory

```text
apps/desktop/src-tauri/src/simulator/runtime.rs
apps/desktop/src-tauri/src/simulator/adapter.rs       (test-only synchronization)
apps/desktop/src-tauri/src/simulator/tests.rs
apps/desktop/src/features/simulation/SimulationWorkspace.tsx
apps/desktop/src/types/simulator.ts
apps/desktop/src/test/SimulationWorkspace.test.tsx
apps/desktop/src/test/simulatorBackend.test.ts
scripts/verify-simulator.mjs
scripts/drive-file-dialog.ps1                       (native verification helper)
docs/PHASE_6_VERIFICATION.md
docs/PHASE_STATUS.md
docs/evidence/phase-6-post-review-2026-10-20.txt
docs/images/phase6-64f7f648ac5ed40c13c69c9ddca71d10/*.png
docs/images/phase6-8059fe5e77ad81398878daf255b13e7a/*.png  (first failed run)
docs/images/phase6-7365dd76bfe1cb5e97fe88474c3d6ac6/*.png  (file-picker failure)
docs/images/phase6-aafccbc16eec063aad03af05e22a8224/*.png  (focused native rerun)
```

No production transport, configuration write, firmware extraction, flashing or live USSD implementation was enabled. Phase 7 has not started.

## Original Phase 6 acceptance record

Date: 20 October 2026, Africa/Lagos. **Phase 6: COMPLETE (PASS). Phase 7 readiness: READY.** All mandatory simulator acceptance gates passed against actual source, both Windows build profiles and native React/Rust execution. Phase 7 implementation has not started.

Implemented source and boundaries are described in [implementation](PHASE_6_IMPLEMENTATION.md) and [architecture](SIMULATOR_ARCHITECTURE.md). The original 130 Rust and 99 frontend tests were retained. Before changes, normal data was captured read-only with `py scripts/probe-native-isolation.py .tools/phase6-normal-before.json` (exit 0). Starting Git worktree was clean; no reset, history rewrite, repository migration or normal-data cleanup occurred.

## Commands and current results

Rust commands use `CARGO_INCREMENTAL=0`. Existing storage regressions require approved normal Windows directory access for their safe directory leases; no path protections were removed. Final command logs/results are in `.tools/phase6-rust-final-*.log`, `phase6-rust-final-results.json`, `phase6-frontend-final-*.log` and `phase6-frontend-final-results.json`. Earlier successful checks and diagnostic attempts remain retained.

| Exact command | Result |
| --- | --- |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all` | Exit 0; formatting applied. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all --check` | Exit 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | Exit 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | Exit 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | Exit 0, 151 tests; original 130 plus 21 simulator tests. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | Exit 0, 151 tests; executable/doc-test targets also pass. |
| `node node_modules/typescript/bin/tsc --project apps/desktop/tsconfig.json --noEmit` | Exit 0. |
| `npm.cmd run lint` | Exit 0. |
| `npm.cmd test` | Exit 0, 111 tests / 13 files; original 99 plus 12 simulator frontend tests. |
| `npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline` | Exit 0; frontend production build and optimized ordinary Windows executable. Final Rust compilation 1m 40s. |
| `npm.cmd run build:native-tests` | Exit 0; dedicated optimized native-test executable, final Rust compilation 1m 47s. Ordinary executable hash preserved. |
| `Start-Process -FilePath (Join-Path (Get-Location) 'target/release/obtainer.exe') -ArgumentList '--simulator' -WindowStyle Hidden -PassThru -Wait` | Expected exit 1 with normal-data activation refused; native-test/data override variables removed from the probe environment. |
| `node --check scripts/verify-simulator.mjs`, `node --check scripts/dev-simulator.mjs`, `node --check scripts/verify-foundation.mjs` | All exit 0. |
| `npm.cmd run test:simulator` | Final exit 0; 127 native checkpoints, including retained Phase 4/4.1/5 and six-destination GUI tests. |
| `py scripts/probe-native-isolation.py .tools/phase6-normal-after.json .tools/phase6-normal-before.json` | Exit 0; complete equality with the pre-change normal-data capture. |
| `git diff --check` | Exit 0; no whitespace errors. Git reports existing Windows LF/CRLF conversion notices separately from application checks. |

Both final release builds pass. Ordinary executable SHA-256: `29D814CBC1E827C9B898874A24A26FC6F30FB1BC23A2446165B54030753FC938`. Build evidence: `.tools/phase6-production-build-final.{log,json}` and `.tools/phase6-native-build-final.{log,json}`. PowerShell displays Cargo stderr progress as `NativeCommandError` records in the redirected dedicated-build log; its actual process exit is 0, the executable exists, and ordinary executable hash equality passes. GUI verification is assessed separately.

## Actual initial failures and corrections

- First Rust check failed because the new fixture `include_str!` used one extra parent directory. Corrected to the existing workspace-relative depth.
- First simulator test compilation failed with E0373: a scoped-thread barrier closure borrowed a local Arc. Moved the barrier into the closure while retaining an explicit shared service reference.
- Initial targeted runs had 1 pass / 18 failures from one shared fixture-load problem. The existing validator correctly rejected an invented device category and `simulation` as a physical modem protocol. Fixture now uses the existing unknown-device category and no physical modem protocol. Virtual transport belongs to the distinct simulator adapter. No profile validator rule was relaxed. A focused diagnostic run exposed the invalid protocol precisely.
- First TypeScript check rejected the mapped control-schema tuple (TS2345). The four control discriminants are now explicit strict Zod alternatives.
- Lint reported the async generation-ref cleanup dependency. An active flag and mounted ref now prevent post-unmount state updates without reading a changing ref in cleanup. No lint suppression was added.
- First functional targeted run passed 18 / 19 tests. Cancellation before adapter dispatch could leave an established virtual session allocated. Token registration and busy state are now synchronized, cancellation/timeout closes the virtual session even before dispatch, and cancellation control waits for bounded completion. All 21 final simulator tests pass, including reset/cancel races and zero resources.
- Final lifecycle review found that selecting a wrong-identity/peer scenario after a successful connection also needed immediate invalidation. Transitions now revoke sessions/approvals/read grants, clear cached features/previews/outcomes and advance generation. Scenario selection reserves the operation slot. Actual identity lives in configuration and is read through the adapter; snapshots capture it and scenario metadata. Added regressions reject stale requests/snapshots and fresh mismatched handshakes. Scheduled cellular recovery is independently tested at 2,999/3,000 virtual milliseconds, including truthful DNS verification while data connectivity is unavailable.
- A final guard probe initially used PowerShell's direct GUI invocation and sampled `$LASTEXITCODE` before the GUI process exited. The refusal message was correct, but that shell assertion was invalid. Replaced the probe with `Start-Process -PassThru -Wait`; actual exit 1 was observed. No application correction or normal-data access was involved.

## Automated evidence

Rust tests cover initially empty authenticated evidence, endpoint attestation, separate read approval, missing/incorrect/replayed/expired approvals, actual session expiry, wrong identity and peer, physical namespace rejection, production registry exclusion, malformed IPC, forbidden physical operations, DNS address/source/list validation and canonicalization, exact previews, snapshots, apply/readback/verify/restore, stale/replayed changes, service/resolver distinction, failed rollback/reset requirement, deterministic fault counts/repeat/reset, two radio/BSS limits/clients/capacity, wireless disconnect/recovery, manual and synthetic-operator APN/IP family/authentication, SIM/PIN/operator/modem/registration/data failures, scripted USSD menu/follow-up/one-shot/busy/cancel/timeout/unsupported/network failure, shutdown and old-runtime invalidation.

Frontend tests cover strict simulated namespace/provenance, malformed/contradictory results, actual Tauri requirement, exact scoped request serialization, typed failures, explicit activation/handshake/read approval, backend-driven DNS preview/apply/readback, failed restoration/reset-required display, timeout recovery and late-result suppression after navigation. Test fixtures are explicitly test-only; production components do not contain fictional current-state responses.

## Executed DNS proof of concept

The native runner drives actual React controls and checks independent Rust state through Tauri IPC. It starts a fresh inactive laboratory, selects the fictional router, explicitly approves its endpoint, authenticates with `lab-only`, confirms that no read grant exists yet, and separately authorizes feature reads. The first DNS value is `1.1.1.1`.

It selects manual upstream IPv4, enters `9.9.9.9`, previews real source/address differences, applies the stored preview and observes configuration revision 2 with one snapshot. Adapter readback returns `9.9.9.9`; configuration, service and modeled resolution checks all pass. Independent Verify also passes. Restore returns `1.1.1.1` with rollback `succeeded` and a fresh adapter read.

The mismatch scenario applies a proposed `1.0.0.1` but actual readback is `203.0.113.53`; `applied:true`, `configurationMatches:false` and `verified:false` accurately coexist. Restoration succeeds. A separate failed-restore scenario reports rollback `failed` and `requiresReset:true`, shows the reset message, and recovers only after explicit reset/fresh authorization. This is a virtual configuration backup and modeled resolver check, not a firmware backup or internet lookup.

## Implementation and boundary evidence

| Acceptance area | Evidence |
| --- | --- |
| Explicit activation, distinct router, real mutable state | `SimulatorService::control`, owned `Router`, backend-private `Authority`; normal binary refuses activation. Native UI and IPC inspect the same state. |
| Phase 5 contracts and cold authentication | `SimulatorAdapter`, `HandshakeApproval`, `SessionManager::connect_initial`, peer/identity/profile validation and separate `confirm_session_reads`; no preseeded authenticated evidence. |
| All four reads and state transitions | DNS, multi-BSS Wi-Fi, APN/modem and USSD implement existing adapter read contracts. Native controls mutate/read Wi-Fi and APN and complete a scripted USSD interaction. |
| DNS vertical slice and truthful restoration | Revision-bound preview/snapshot/apply/readback/verification/restore; unit and native success/mismatch/failed-rollback paths. |
| Deterministic faults, cancellation and recovery | Typed stage/count/repeat, virtual clock, real bounded cancellation/deadlines, stale-generation rejection, scheduled cellular/Wi-Fi recovery and resource-count assertions. |
| Physical safety and provenance | Normal registry excludes simulator; scoped registry excludes physical connectors; private target/endpoint/interface/pin/generation authority; strict input parsing; physical write policy still denies. No sockets, shell, AT transport, physical firmware or carrier requests in simulator. |
| Data and inventory separation | No simulator calls to Repository; no virtual Windows records, profile rows, artifacts or backups; before/after probes and native inventory/library comparisons. |
| Frontend integration | Six existing destinations retained; laboratory only inside Connected Devices; visible banner, strict IPC, actual drafts/results, errors/recovery and contextual help. Retained native theme/help/layout regressions apply. |
| Builds and regressions | Final 151 Rust tests per profile, 111 frontend tests, formatting/Clippy/TypeScript/lint and both optimized builds pass; detailed native qualification below. |

The expected virtual identity is a fixture; the authenticated identity comes from mutable router configuration through the adapter. Selecting an identity/peer substitution scenario after connection immediately invalidates grants, cached successes and old requests. Reset/stop/restart cannot reuse prior session authority. APN read failures retain the existing conservative Phase 5 behavior of closing the session; after modeled cellular recovery the user authenticates and authorizes reads again.

## Known limits

Only one fictional router and scripted modem are implemented. Virtual state, snapshots, grants and event history are in memory; application restart resets them. Simulation does not claim physical support or change the MG11's UNKNOWN features/user-reported firmware evidence. DNS resolution, wireless enforcement, APN registration and USSD are modeled; no real network/modem/carrier operations occur. Durable configuration journals, production recovery and hardware read/write adapters remain their later phases. Phase 7 work is not implemented by this change.

## Actual native desktop qualification

Final run: `.tools/foundation/2026-10-20T11-01-41.374Z/`, isolated ID `6569e7b84184434230cc74a12e516313`. `results.json` records **127 passing checkpoints**. `console-issues.json` is `[]`. `cleanup.json` records exit 0 and removal of only the matching marked run after owned app/WebView/Vite/listener shutdown. Full run output and exit are `.tools/phase6-native-verification-hardened.{log,json}`. A reviewable transcript is retained in [evidence](evidence/phase-6-2026-10-20.txt).

The following exact child commands were executed by the existing runner:

| Command | Observed exits and interpretation |
| --- | --- |
| `npm.cmd run tauri -- dev --config src-tauri/tauri.native-test.conf.json --features native-test-profile -- --bin obtainer-native-test --locked --offline -- --simulator` | 0 for initial native development run; 1 for deliberately interrupted launch; 0 for recovered native development run. The intentional interruption is not a startup defect. |
| `.\target\release\obtainer-native-test.exe --simulator` | 0 for optimized isolated release; 0 for subsequent storage-copy restart qualification. |

Development/recovery/optimized-release each verified actual window/React rendering, real SQLite/IPC, all six destinations, physical adapter safety, explicit simulator start, cold authentication, separate read grant, all four state-backed feature reads, DNS success/failure/restoration, Wi-Fi/APN mutations, interactive scripted USSD, cellular restart recovery, live identity conflict invalidation, bounded timeout, disconnect/reset/stop and unchanged physical inventory/artifact counts. Native assertions inspect rendered DOM and actual IPC state; build or terminal startup is not used as visual proof.

Retained checks also passed settings round trips, light/dark themes, help focus/Escape/geometry, narrow-window layout, HMR with stable process identities, native file/folder dialogs, generated-binary imports/hash/deduplication/integrity/cancellation, forced job interruption/recovery, safe backups/restores, library removal/recovery, reviewed storage copy/restart, queued identical imports and corruption/SQL/audit failure paths. Only harmless generated test binaries were used. No real MiFi firmware was selected.

Two earlier complete native runs passed 121 checkpoints before the final identity/cellular hardening: `.tools/foundation/2026-10-20T10-23-46.185Z/` and `2026-10-20T10-31-58.066Z/`. Their evidence remains retained. Initial screenshots partly obscured section headings behind the sticky simulation banner; capture framing was corrected without changing product behavior. Final run added the live-identity and cellular regressions in each of the three sessions, producing 127 checkpoints.

## Screenshot verification

Final actual native captures: [image directory](images/phase6-6569e7b84184434230cc74a12e516313/). There are **42 files**, 14 each for `dev-reopen`, `dev-recovery` and `production`: device-selection, connected-device, feature-status, dns-before, dns-preview, dns-apply-result, dns-restored, dns-verification-failure, dns-restore-failure, wifi-configuration, apn-modem, ussd-session, cellular-restart-recovered and disconnected. The visible SIMULATION MODE label distinguishes all captures from hardware.

Visually inspected final [development DNS success](images/phase6-6569e7b84184434230cc74a12e516313/dev-reopen-dns-apply-result.png), [production failed restore](images/phase6-6569e7b84184434230cc74a12e516313/production-dns-restore-failure.png) and [production cellular recovery](images/phase6-6569e7b84184434230cc74a12e516313/production-cellular-restart-recovered.png). They show the actual six-section blue desktop, readable mode banner, truthful result flags and current state. Prior-run captures remain under their own IDs.

## Final normal-data preservation

An independent final read-only probe compared `.tools/phase6-normal-after.json` with `.tools/phase6-normal-before.json` and exited 0. Complete JSON equality includes database/WAL byte hashes, all table rows/counts, settings, saved router/profile IDs, vault blobs and logs/workspaces/reports files. Normal root remains `%LOCALAPPDATA%\OBTAINER`, schema 5, integrity `ok`, no foreign-key violations.

- All rows SHA-256: `6c4c4640c029fc1d0bb91e24f07bac4e1f321dcf7222bddce886db5f42a27e91`.
- Settings SHA-256: `70ca50b9ab62d27cbc6340ed90d7ebab7596b61ac8875a63e86219af831f4d97`.
- Counts remain 160 inventory rows, one saved manual router, two profile versions, one firmware artifact and 34 jobs. Existing history was preserved, not cleaned.

MG11 was not physically present in final qualification (`mg11Connected:false`). No new MG11 hardware claim is made; retained unknown-capability/disabled-operation regressions pass and saved user-reported evidence is unchanged. Simulator operations never made a router configuration, firmware or carrier request. No unresolved critical Phase 6 blocker remains. Phase 7 is READY for separately scoped read-only OpenWrt work.

## Exact changed-file inventory

Paths below are relative to the repository root. Screenshot paths are listed individually in the evidence transcript. No lockfile, normal Tauri capability/configuration, database schema or physical discovery implementation changed.

```text
README.md
apps/desktop/src-tauri/build.rs
apps/desktop/src-tauri/capabilities/simulator.json
apps/desktop/src-tauri/permissions/autogenerated/simulation_control.toml
apps/desktop/src-tauri/permissions/autogenerated/simulation_request.toml
apps/desktop/src-tauri/permissions/autogenerated/simulation_status.toml
apps/desktop/src-tauri/src/adapters/bootstrap.rs
apps/desktop/src-tauri/src/adapters/contracts.rs
apps/desktop/src-tauri/src/adapters/lifecycle.rs
apps/desktop/src-tauri/src/adapters/policy.rs
apps/desktop/src-tauri/src/adapters/registry.rs
apps/desktop/src-tauri/src/adapters/types.rs
apps/desktop/src-tauri/src/domain.rs
apps/desktop/src-tauri/src/lib.rs
apps/desktop/src-tauri/src/simulator/adapter.rs
apps/desktop/src-tauri/src/simulator/ipc.rs
apps/desktop/src-tauri/src/simulator/mod.rs
apps/desktop/src-tauri/src/simulator/model.rs
apps/desktop/src-tauri/src/simulator/runtime.rs
apps/desktop/src-tauri/src/simulator/tests.rs
apps/desktop/src-tauri/src/simulator/types.rs
apps/desktop/src-tauri/tauri.native-test.conf.json
apps/desktop/src/app/App.tsx
apps/desktop/src/components/PageHelp.tsx
apps/desktop/src/features/simulation/DnsSimulator.tsx
apps/desktop/src/features/simulation/SimulationWorkspace.tsx
apps/desktop/src/features/simulation/simulation.css
apps/desktop/src/lib/simulator.ts
apps/desktop/src/test/SimulationWorkspace.test.tsx
apps/desktop/src/test/simulatorBackend.test.ts
apps/desktop/src/test/simulatorFixtures.ts
apps/desktop/src/types/adapters.ts
apps/desktop/src/types/contracts.ts
apps/desktop/src/types/simulator.ts
docs/NATIVE_TESTING.md
docs/PHASE_6_IMPLEMENTATION.md
docs/PHASE_6_VERIFICATION.md
docs/PHASE_STATUS.md
docs/SIMULATOR_ARCHITECTURE.md
docs/evidence/phase-6-2026-10-20.txt
package.json
scripts/dev-simulator.mjs
scripts/verify-foundation.mjs
scripts/verify-simulator.mjs
tests/fixtures/simulator/virtual-router.json
```
