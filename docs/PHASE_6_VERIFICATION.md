# Phase 6 verification

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
