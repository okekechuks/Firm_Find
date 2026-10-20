# Phase 6 implementation — Router and Modem Simulator

Date: 20 October 2026, Africa/Lagos. **COMPLETE (PASS).** Final qualification: 151 Rust tests in each profile, 111 frontend tests, both optimized Windows builds and 127 actual native checkpoints. Acceptance, failures, screenshots and data-preservation evidence are recorded in [verification](PHASE_6_VERIFICATION.md). Phase 7 is READY; it has not started.

## Repository audit and retained foundation

Work started at HEAD `0c579fafe790fc154c79fd72097dd63f3b08640a` with a clean Git worktree. README, master plan, roadmap, phase tracker, Phase 5 implementation/verification and native-isolation documentation were reviewed. Existing `apps/desktop/`, npm/Cargo workspaces, discovery, profiles, SQLite schema 5, library/jobs, six destinations and native runners are retained. No roadmap recreation, data migration, dependency addition, physical router access or firmware operation was needed.

The existing adapter extension points are `DeviceAdapter`, `ReadOnlyConnectionAdapter`, `AdapterConnection`, DNS/Wi-Fi/APN/USSD readers, `HandshakeApproval`, `SessionManager` and profile evaluation. Physical discovery is `discovery/mod.rs`, Windows APIs remain in `discovery/windows.rs`, and `DiscoveryService` continues owning only observed Windows inventory. Existing application persistence remains `storage.rs` and `library/`; the simulator does not write through either module.

## Implementation map

| Source | Working responsibility |
| --- | --- |
| `src-tauri/src/simulator/mod.rs` | Private backend authority and exact virtual target binding. |
| `simulator/model.rs` | Stateful configuration, two radios/BSS records, modem/SIM/operator, services, revision, virtual clock, bounded snapshots/events and typed deterministic faults. |
| `simulator/adapter.rs` | Phase 5 adapter implementation, peer trust/authentication, actual virtual identity, capability probe, all feature reads, close/cancel/timeout and counted resources. No physical transport. |
| `simulator/runtime.rs` | Explicit start/stop/reset, serialized operations, single-use handshake/read approval, DNS preview/apply/verify/restore, Wi-Fi/APN changes, scripted USSD and fault/lifecycle recovery. |
| `simulator/types.rs`, `ipc.rs` | Strict tagged request/control types, structured errors/results and three scoped Tauri commands. |
| `simulator/tests.rs` | 21 multi-case regressions for cold auth, boundaries, all features, faults, revisions, cancellation, live identity changes, cellular restart and cleanup. |
| `adapters/{types,registry,bootstrap,policy,lifecycle,contracts}.rs` | Distinct simulator implementation/transport and sealed registry path; physical checks retained. Backward-compatible multi-radio read method defaults to the original single-radio reader. |
| `src/types/simulator.ts`, `src/lib/simulator.ts` | Strict request/response/error Zod schemas and actual IPC wrappers, including contradictory-outcome rejection. |
| `src/features/simulation/{SimulationWorkspace,DnsSimulator}.tsx`, `simulation.css` | Explicit laboratory controls inside Connected Devices, all current feature states, DNS editor/preview/results/restore and separate virtual Wi-Fi/APN/USSD controls. |
| `src/app/App.tsx`, `components/PageHelp.tsx`, `types/contracts.ts` | Mode visibility from isolated backend profile and contextual help. Sidebar and physical device presentation retained. |
| `src-tauri/{build.rs,tauri.native-test.conf.json,capabilities/simulator.json}` and generated permissions | Native-test-only simulation permissions. Normal `main.json` capability unchanged. |
| `src-tauri/src/{lib,domain}.rs` | Dedicated `--simulator` argument, normal executable rejection, service ownership/shutdown and explicit profile availability metadata. |
| `tests/fixtures/simulator/virtual-router.json` | Fictional profile, opaque firmware identifier, simulator-local provenance; never installed into normal profile storage. |
| `scripts/{dev-simulator,verify-simulator,verify-foundation}.mjs`, `package.json` | Explicit isolated launch and additive native end-to-end tests using existing runner/process/data guards. |

Paths beginning `src-tauri/` or `src/` above are relative to `apps/desktop/`.

## User workflow

Run `npm.cmd run dev:simulator`, open Connected Devices, and select **Start simulation**. Approve the displayed virtual endpoint's bounded handshake, enter the fictional password `lab-only`, connect, then explicitly authorize feature reads and select **Read all features**. The help popover explains the credential and controls; no actual credential is needed.

Change DNS source/addresses, select Preview, inspect exact differences and select Apply to Simulator. Rust captures a virtual configuration snapshot and reports accepted/applied/configuration/service/resolution results separately. The frontend reads the resulting state again through the adapter. Independent Verify and Restore Previous Virtual State use actual current state and snapshot ownership; modeled failures cannot turn into unconditional success.

Scenario selection changes only virtual state. One-shot faults are consumed; repeat mode is available in the typed control API. The GUI offers named scenarios and explicit virtual time. DNS verification and rollback failure, offline router, identity/endpoint/authentication errors, bounded slow reads/authentication, SIM/PIN/operator/modem/data problems, scheduled cellular restart, Wi-Fi management interruption and USSD timeout/network failure are modeled. Identity or endpoint fault transitions invalidate sessions, approvals and cached results immediately. Failed rollback requires a simulator reset.

## Boundaries and limitations

The [architecture](SIMULATOR_ARCHITECTURE.md) details provenance, cancellation, profile scope, request binding and Phase 8 reuse. Physical writes, extraction, flashing, firmware execution, restore and live USSD remain denied by the existing policy. Simulator mutations reference the concrete owned router state and cannot dispatch a generic physical adapter operation.

All simulator state is in memory and isolated. Normal device inventory/counts, firmware artifacts, backups, capability evidence, saved settings and production logs are not used for virtual records. Application restart resets simulated configuration and approval/session state. There is no production transaction journal, real network resolver, physical recovery proof, OpenWrt connector, SIM unlocking or router-hosted USSD page. Those owning phases remain separate.
