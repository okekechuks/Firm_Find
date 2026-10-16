# Phase 0 final technical verification — OBTAINER

Audit date: **16 October 2026**, Africa/Lagos. Repository root (`R`): `C:/Users/chuka/OneDrive/Documents/Firm_Find`. Inspected baseline: `a8e51a92ad48bc40d2acae8b19a742aa6fb31bfe`, branch `main` tracking `origin/main`. This report supersedes the 15 October conclusions with freshly executed checks; earlier screenshots remain preserved.

## A. Executive summary

**Phase 0 verdict: PASS. Phase 1 readiness: READY.** The decision follows current source inspection, passing frontend/Rust checks, a fresh Tauri release build, a visually inspected native screenshot, actual IPC/discovery activity, and existing-settings/database checks before and after restart. Documentation alone is not the acceptance evidence.

The fresh release rendered React and navigated all eight destinations in the normal Windows session. Actual `get_snapshot` and `get_discovery` calls succeeded; clicking React's device refresh invoked `refresh_discovery` and advanced the native revision. Discovery returned `ready`, `watching: true`, no warnings, and 48 Windows inventory records. Those records are not 48 confirmed routers. Existing theme `system` and its settings fingerprint survived close/relaunch.

Two failed attempts are retained: restricted execution lost its renderer connection; the first normal-session attempt rendered and navigated successfully but the runner asserted while asynchronous discovery was still `starting`. Only the runner's readiness wait was corrected. The final normal-session run passed with the same release binary. No application feature, storage path, schema, identifier or lockfile changed.

Exact commands, exit codes and captured output: [execution transcript](evidence/phase-0-final-2026-10-16.txt). Final screenshot: [native release window](images/phase-0-final-2026-10-16.png). No Phase 1 work began.

## B. Requirement classifications

`VERIFIED` means the stated requirement has current evidence. `FAILED` means an attempted check failed and is retained as such. `BLOCKED` means required evidence could not be obtained. `NOT APPLICABLE` means outside this Phase 0 verification scope. Failed intermediate attempts do not become successful; a successful countercheck can resolve the corresponding baseline requirement.

| Phase 0 requirement | Classification | Evidence |
|---|---|---|
| Inspect current repository | VERIFIED | Clean initial `git status`; baseline/history; source/configuration/test reads, D/F. |
| Inventory frontend and Rust implementation | VERIFIED | Exact paths, functions and evidence limits in D/G. |
| Identify commands and frontend IPC | VERIFIED | Four registered commands and matching React wrappers/callers in D; native reads/refresh in H. |
| Inspect real Windows discovery | VERIFIED | Actual SetupAPI/Configuration Manager/IP Helper code, Windows capture test and native runtime response, D/H. |
| Identify database and persistent storage | VERIFIED | `Repository` methods, migration, actual directories and read-only SQLite checks, D/I. |
| Identify firmware functionality | VERIFIED | Reserved table/vault and planned screens distinguished from missing import/extraction/analysis/build/flash services, D/G. |
| Identify tests, mocks, placeholders and missing modules | VERIFIED | D; 10 frontend tests and 11 Rust tests pass, with fixture/native coverage separated. |
| Inspect Git status and protect existing work | VERIFIED | Worktree clean before edits; no reset, checkout, cleanup or migration. Final changes limited to reports/evidence and runner wait, C/I. |
| Verify Node/package manager and installed dependencies | VERIFIED | Node `22.17.0`, npm `10.9.2`, installed workspace tree; E/F. |
| Verify Rust/Cargo and target | VERIFIED | Rust/Cargo `1.99.0`, installed MSVC target, E/F. |
| Verify MSVC, SDK and WebView2 | VERIFIED | Complete C++ tool installation, actual versioned files, successful build and native render, E/H. |
| Frontend typecheck, lint, tests and production build | VERIFIED | All exit 0; 3 test files/10 tests; F. |
| Rust checks, tests and compilation | VERIFIED | Formatting, Clippy all targets with warnings denied, 11 tests, optimized native compilation; F. |
| Tauri build | VERIFIED | Root Tauri wrapper builds fresh optimized `target/release/obtainer.exe`; F. |
| Executable starts, native window opens and React renders | VERIFIED | Final normal-session WebDriver session, React DOM assertions and reviewed native screenshot; H. |
| Native navigation | VERIFIED | All eight destinations visited and expected headings checked; H. |
| Actual frontend-to-Rust IPC and discovery invocation | VERIFIED | React startup snapshot, independent real IPC reads, React device-refresh click and revision increase; H. |
| Settings survive restart; inspect data integrity | VERIFIED | Native pre/post-relaunch settings equality; read-only SQLite integrity and fingerprint before/after; I. |
| Evaluate source migration and naming | VERIFIED | `apps/desktop/` already committed; working root commands/relative paths; OBTAINER identity retained, J. |
| Update evidence/status and determine readiness | VERIFIED | Current report, [phase tracker](PHASE_STATUS.md), transcript and screenshot; L. |
| Source/data migration | NOT APPLICABLE | No move is necessary; existing structure works. |
| Firmware operations, physical router qualification, installers/signing, remote CI and Phase 1 development launch | NOT APPLICABLE | No such result is required or claimed for this baseline. Existing limitations remain explicit in D/K. |

No Phase 0 requirement is currently BLOCKED. Intermediate failures are individually classified FAILED in F/H.

## C. Changes made during verification

- Preserved the clean initial checkout and all existing user data. Updated this report and `docs/PHASE_STATUS.md`; added a separate transcript and screenshot without replacing older evidence.
- Corrected `scripts/native-smoke.mjs` at the initial discovery assertion: poll real `get_discovery` until `ready` or `degraded`, fail immediately on `failed`, and retain the existing bounded timeout. This resolves a demonstrated test timing error, rather than treating normal `starting` state as a failed discovery service.
- Audit mode still never invokes `save_settings`, including cleanup. The app binary and production source were not changed by this correction. Root lint and the full native audit pass afterward.
- Performed no package installation, destructive cleanup, source restructuring, data/schema migration, product rename, Git reset/commit/push, or physical router operation. Existing plans and roadmap were not recreated.

## D. Source inventory and evidence boundaries

Paths below are relative to `R`. “Native verified” refers to this run's real Windows application. “Test verified” identifies the actual test level; it does not imply hardware qualification.

| Area | Exact source and functions | Current evidence / limitation |
|---|---|---|
| Root workspace and wrappers | `package.json` scripts; `Cargo.toml` member `apps/desktop/src-tauri`; `scripts/desktop.ps1` and `scripts/rust.ps1` | VERIFIED: root commands resolve the desktop workspace/local toolchain and build root `target/` outputs. One root npm lockfile and Cargo lockfile retained. |
| Frontend entry and configuration | `apps/desktop/index.html` → `apps/desktop/src/main.tsx`; `apps/desktop/tsconfig.json`, `apps/desktop/vite.config.ts`; `eslint.config.js`; `apps/desktop/src/styles.css`, `public/mark.svg` | VERIFIED: TypeScript, lint, Vite build and native rendered React. |
| Desktop shell/navigation | `apps/desktop/src/app/App.tsx:25`, `App` with `refresh`, `persist` and `navigate`; `apps/desktop/src/app/navigation.ts`, `destinations`/`pageFromHash` | Native verified: Overview, Connected devices, Firmware library, Analysis, Backup manager, Firmware builder, Jobs & activity, Settings. Error/retry/browser-preview cases are component tests with mocks, not native fault injection. |
| Overview and audit history | `apps/desktop/src/features/Dashboard.tsx`, `Dashboard`; `apps/desktop/src/components/Activity.tsx`, `Activity`; `App.tsx` Jobs branch/`ListStatus` | Native verified: actual SQLite counts/events. Jobs table/view is infrastructure; no durable job coordinator/cancellation or firmware jobs. |
| Device UI and subscriptions | `apps/desktop/src/features/Devices.tsx`, `Devices`/`DeviceCard`/`DeviceDetails`; `apps/desktop/src/lib/useDeviceInventory.ts:5`, `useDeviceInventory`/`accept`/`refresh` | Native verified: inventory renders and Refresh works. Filtering/error details have component fixture tests. Live physical arrival/removal delivery was not exercised. |
| Settings UI | `apps/desktop/src/features/Preferences.tsx`, `Preferences`; `App.persist` | Native verified: existing saved theme read and retained across restart. Save behavior is component-tested with mocked IPC and storage-tested in a temporary DB. A live user's `save_settings` write was intentionally not executed. |
| IPC contracts/wrappers | `apps/desktop/src/lib/backend.ts:10`, `hasDesktopBackend`, `getSnapshot`, `saveSettings`, `getDiscovery`, `refreshDiscovery`, `onDiscoveryChanged`; `apps/desktop/src/types/contracts.ts`, Zod settings/snapshot/discovery schemas | Native verified: real reads and refresh. `onDiscoveryChanged` listens to `discovery-changed` revision events; hardware event delivery not qualified. Browser preview rejects native calls. |
| Native entry/setup | `apps/desktop/src-tauri/src/main.rs:3` → `obtainer_lib::run`; `apps/desktop/src-tauri/src/lib.rs:10`, `run` | Native verified: directories/logging, repository/startup, discovery worker, managed state, command registration, Tauri event loop and shutdown path. |
| Tauri commands | `apps/desktop/src-tauri/src/commands.rs:33` `get_snapshot`; `:45` `save_settings`; `:69` `get_discovery`; `:76` `refresh_discovery` | Read/refresh native verified. Settings write present and compiled; full native write not executed in this audit. DB operations use blocking tasks/mutex; refresh has a 20-second reply timeout. |
| Domain | `apps/desktop/src-tauri/src/domain.rs`, `Theme`/`Settings`/`Snapshot`/`OperationRisk::permitted_in_foundation` | Test verified: settings reject extra fields/invalid themes; device-write/destructive risk variants disallowed. No device-write command exists. This enum alone is not qualification of future operations. |
| SQLite | `apps/desktop/src-tauri/src/storage.rs:21`, `Repository::open`, `record_startup`, `save_discovery`, `settings`, `save_settings`, `snapshot`; `apps/desktop/src-tauri/migrations/001_foundation.sql` | Native verified reads/inventory persistence; temporary DB write/reopen and newer-schema rejection tests pass. WAL, 5-second busy timeout, foreign keys, transactional schema initialization. Real DB checks in I. |
| Windows capture | `apps/desktop/src-tauri/src/discovery/windows.rs`: `nodes` (`:218`), `add_interfaces` (`:285`), `adapters` (`:389`), `capture` (`:469`); helpers `instance_id`/`property`/`container_id`/`parent_id`/`registry_string` | Native verified and tested: `SetupDiGetClassDevsW`/enumeration, `CM_Get_Parent`/`CM_Get_Device_IDW`, read-only registry COM/adapter IDs, `GetAdaptersAddresses`. No PowerShell inventory fallback, router socket or serial-device open. |
| Notifications | Same `windows.rs`, `Notifications::register` (`:484`), `watching`, `Drop`, `pnp_changed`/`network_changed` | Runtime registration verified: `CM_Register_Notification` and `NotifyIpInterfaceChange` produce `watching: true`, warnings empty. Physical hot-plug/removal/reconnect remains untested. |
| Correlation/reconciliation | `apps/desktop/src-tauri/src/discovery/mod.rs`, `assemble` (`:245`), `identity`/`usb_root`/`physical_key`, `DiscoverySnapshot::reconcile` (`:358`), `replay_fixture` (`:409`) | Real capture plus fixture tests verified: composite correlation, deduplication, bounded cycles, disconnected/reconnected stable identities. Confidence/“possible MiFi” are heuristics, not confirmed chipset/model/firmware support. |
| Discovery worker | `apps/desktop/src-tauri/src/discovery/service.rs`, `DiscoveryService::start`/`snapshot`/`refresh_request`/`shutdown`; `run_worker`/`scan_and_reply`/`mark_failed` | Native initial capture/refresh verified. Bounded queue, debounce, 30-second rescan, transactional inventory persistence and revision-only event emission implemented. Failure/physical-notification paths not fully exercised natively. |
| Library/import/extraction | `apps/desktop/src/features/Planned.tsx`, `Planned`; `firmware_artifacts` table in migration; empty `vault/` | PLACEHOLDER/reserved storage only. No import/hash/dedup/extraction service, firmware file API or extraction IPC. No working firmware acquisition claim. |
| Analysis and OpenWrt builds | Same `Planned` component; `workers/README.md` | PLACEHOLDER/documentation only. No executable worker, firmware parser/extractor, analysis service, OpenWrt build service or firmware deployment. Tauri app compilation is not firmware compilation. |
| Backups/configuration | Same `Planned` component; empty `workspaces/` and `reports/` | PLACEHOLDER only. No configuration snapshot/backup/restore/rollback service or stored backup tree. No app-managed profiles/configuration store exists. |
| Adapters/simulator/customization | No connected production modules or registered commands | MISSING: router adapters/authentication, interactive simulator, exact-model profiles/evaluator, DNS/Wi-Fi/APN/USSD control and router-local USSD. Discovery fixtures are not a simulator. |
| ACL/CSP | `apps/desktop/src-tauri/build.rs`; `apps/desktop/src-tauri/capabilities/main.json`; `apps/desktop/src-tauri/permissions/autogenerated/{get_snapshot,save_settings,get_discovery,refresh_discovery}.toml`; `apps/desktop/src-tauri/tauri.conf.json` | Compiled baseline: four allowlisted commands, main Windows window, constrained CSP, typed settings and parameterized SQL. No comprehensive future-device security qualification claimed. |
| Frontend tests/mocks | `apps/desktop/src/test/App.test.tsx`, `apps/desktop/src/test/Devices.test.tsx`, `apps/desktop/src/test/backend.test.ts`, `apps/desktop/src/test/setup.ts` | VERIFIED: 3 files/10 tests. `vi.mock` supplies IPC/native fixtures; tests cover navigation, save UI, retry, preview, validation and device presentation/filtering. These alone do not prove native IPC. |
| Rust tests/fixtures | `apps/desktop/src-tauri/src/discovery/tests.rs`; test modules in `storage.rs`/`domain.rs`; `apps/desktop/src-tauri/fixtures/discovery-lifecycle.json`; `apps/desktop/src-tauri/examples/discovery-replay.rs` | VERIFIED: 11 tests (7 discovery, 2 storage, 2 domain). `native_inventory_is_available_without_opening_devices` actually calls Windows APIs. The other discovery lifecycle cases use synthetic fixture data. |
| Native runner/CI | `scripts/native-smoke.mjs`, `launch`/`invoke`/`preferences`/`close` and readiness loop; `.github/workflows/windows.yml` | Local release audit verified after readiness-wait correction. CI configuration inspected, remote CI not run. Default write-testing mode was not executed against user settings. |

## E. Observed environment

These are local tool/file observations from this run, not assumed versions.

| Prerequisite | Observed value | Classification / evidence |
|---|---|---|
| Windows | `10.0.26200.9457`, display `25H2` | VERIFIED: `cmd /c ver` and registry fields. |
| Node / npm | `v22.17.0` / `10.9.2` | VERIFIED: direct version commands. |
| Frontend tooling | TypeScript `5.7.3`, Vite `6.4.4`, Vitest `4.1.11`, React `19.3.0` | VERIFIED: installed workspace listing and executed checks. |
| Rust / Cargo | Both `1.99.0`; Rust commit `b940084d7`, Cargo `5f94df478`; LLVM `23.1.1` | VERIFIED: verbose version queries. |
| Rust target/channel | `stable-x86_64-pc-windows-msvc`; target `x86_64-pc-windows-msvc` installed | VERIFIED: `rustup` queries. `rust-toolchain.toml` uses mutable `stable`; no bit-identical-build guarantee. |
| VS Build Tools | 2022, product `17.14.41`, installation `17.14.37710.0` | VERIFIED: `vswhere` with required C++ tools reports complete installation, no reboot required. |
| MSVC | Tools directory `14.44.35207`; `cl.exe` product version `14.44.35229.0` | VERIFIED: actual compiler file found; optimized Rust/Tauri link succeeds. |
| Windows SDK | Include version `10.0.26100.0`; `rc.exe` product version `10.0.26100.7705` | VERIFIED: actual SDK trees/files and native build. |
| WebView2 / EdgeDriver | Both `155.0.4283.45` | VERIFIED in normal session: executable metadata, driver CLI and rendered native app. Restricted attempt separately FAILED. |
| Tauri CLI / API / driver | CLI `2.12.2`; API `2.12.3`; tauri-driver `2.1.0` | VERIFIED: CLI, npm workspace listing, read-only Cargo installed-package listing. |
| Session | Session 1, `AdministratorToken: false` in token probe | VERIFIED: no administrator-token requirement is established; final GUI audit runs outside the sandbox in the normal Windows session. |
| Optional WSL/Linux workers | Not required for current desktop baseline | NOT APPLICABLE: not rerun or used. No new WSL availability claim. |

Local Rust commands use the existing `scripts/rust.ps1` wrapper; Tauri uses `scripts/desktop.ps1` via npm. No prerequisite/dependency install was performed. The disk-space metadata field returned null in this restricted probe; it is not a verified free-space value. Successful builds are recorded independently.

## F. Commands executed and results

All commands ran from `R`. [The transcript](evidence/phase-0-final-2026-10-16.txt) contains the **exact command text, exit code and captured output for every row below**, including long MSVC/SDK/WebView2 and read-only SQLite probes. Source-reading commands/full orchestration evidence are additionally preserved under ignored `.tools/phase-0-final-2026-10-16/commands.json`. Composite metadata-probe exit codes are wrapper results; missing optional process matches are not compiler failures.

The principal verification commands were:

```powershell
node node_modules/typescript/bin/tsc --project apps/desktop/tsconfig.json --noEmit
npm.cmd run lint
npm.cmd test
npm.cmd run build
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all --check
$env:CARGO_INCREMENTAL='0'; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings
$env:CARGO_INCREMENTAL='0'; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline
$env:CARGO_INCREMENTAL='0'; npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline
npm.cmd run test:native -- target/release/obtainer.exe --audit
```

`CARGO_INCREMENTAL=0` applied only to the invoked process. Locked/offline Cargo avoids changing dependency selections or downloading packages. The Tauri build includes its configured frontend `beforeBuildCommand` and native optimized compilation; installer bundling/signing is NOT APPLICABLE to this audit.

| Command ID (exact command in transcript) | Exit code | Classification | Result / relevant error |
|---|---:|---|---|
| `git-baseline` | 0 | VERIFIED | Clean main worktree tracking origin/main before report edits; global ignore-file permission warning. |
| `git-identity` | 0 | VERIFIED | Baseline a8e51a92ad48bc40d2acae8b19a742aa6fb31bfe. |
| `git-migration` | 0 | VERIFIED | apps/desktop already existed in committed history (2c07c61). |
| `env-node` | 0 | VERIFIED | Node v22.17.0. |
| `env-npm` | 0 | VERIFIED | npm 10.9.2. |
| `env-rust` | 0 | VERIFIED | Cargo 1.99.0; x86_64-pc-windows-msvc. |
| `env-rustc` | 0 | VERIFIED | rustc 1.99.0; LLVM 23.1.1. |
| `env-native` | 0 | VERIFIED | Complete VS C++ installation, compiler/SDK/WebView2 executable versions, matching EdgeDriver. |
| `environment-extra` | 0 | VERIFIED | Installed MSVC Rust target; Git/Windows versions. Optional process lookup deliberately permits no matches. |
| `environment-details` | 0 | VERIFIED | Non-administrator token, session 1, Windows 25H2/26200.9457. FreeBytes was null: no disk-space measurement is claimed. |
| `env-driver` | 0 | VERIFIED | Read-only installed-package listing: tauri-driver 2.1.0. |
| `env-tauri` | 0 | VERIFIED | Tauri CLI 2.12.2. |
| `installed-deps` | 0 | VERIFIED | Workspace dependency tree resolves; actual installed versions recorded. |
| `typecheck` | 0 | VERIFIED | TypeScript noEmit check passes. |
| `lint` | 0 | VERIFIED | Initial root ESLint check passes. |
| `frontend-tests` | 0 | VERIFIED | 3 files, 10 tests passed. |
| `frontend-build` | 0 | VERIFIED | 1,601 modules; production dist generated. |
| `rust-format` | 0 | VERIFIED | rustfmt check passes. |
| `rust-clippy` | 0 | VERIFIED | All workspace targets; warnings denied; passes. |
| `rust-tests` | 0 | VERIFIED | 11 tests passed, including actual passive Windows capture and temporary SQLite persistence. |
| `tauri-release-build` | 0 | VERIFIED | Fresh optimized executable; beforeBuildCommand also reruns frontend build. Installer bundling intentionally excluded. |
| `native-restricted` | 1 | FAILED | FAILED before verified render: invalid session id; browser closed connection / not connected to DevTools. |
| `native-normal` | 1 | FAILED | FAILED test assertion after successful render/navigation: Unexpected discovery status: starting. Runner asserted before asynchronous initial capture completed. |
| `lint-runner-fix` | 0 | VERIFIED | Root ESLint passes after bounded native-runner readiness wait correction. |
| `native-normal-final` | 0 | VERIFIED | VERIFIED normal-session native render, eight destinations, actual IPC, inventory refresh, restart/settings. 48 records; ready; watching true; no warnings. |
| `data-before` | 0 | VERIFIED | Read-only: integrity ok, schema 1, preferences fingerprint recorded; 21 audit events, 54 cached devices. |
| `data-after` | 0 | VERIFIED | Read-only: integrity ok, schema 1, same preferences fingerprint; 26 audit events, 48 cached devices. |
| `artifact-hashes` | 0 | VERIFIED | Fresh release and unchanged npm/Cargo lockfile hashes. |
| `source-preservation-before-native` | 0 | VERIFIED | No application source, migration, configuration, lockfile or runner changes at this checkpoint. |
| `storage-paths` | 0 | VERIFIED | Actual data directories and absent backup/configuration/profile stores. |
| `native-log-review` | 0 | VERIFIED | Restricted driver renderer disconnect; final driver log empty; reviewed exact runner-only fix. |
| `preserve-screenshot` | 0 | VERIFIED | New screenshot copied without overwriting old evidence. |

| `final-protected-files` | 0 | VERIFIED | Exit 0: application source/configuration, lockfiles, wrappers, storage and roadmap match HEAD. |
| `final-whitespace` | 0 | VERIFIED | Exit 0: git diff --check; only normal LF-to-CRLF notices. |
| `final-status` | 0 | VERIFIED | Expected report/runner changes and new evidence only; no reset, move or commit. |
| `settings-integrity-comparison` | 0 | VERIFIED | Before/after settings hash and schema equal; integrity ok, zero foreign-key violations. |
| `final-evidence-validation` | 0 | VERIFIED | 9 Markdown links, 30 referenced source paths, 20 verified Phase 0 requirements, 2 not applicable, all 20 existing phase rows; executable hash and screenshot bytes match. |

Fresh release artifact: `target/release/obtainer.exe`, **11,109,376 bytes**, SHA-256 `65AD4BFF2B93B35AD524C00D3C26A6C51AF6761D83E16BDFBCF4CB12C7525DC1`. This is the binary used by the failed and successful native attempts; no application rebuild followed the runner-only fix.

## G. Actual working functionality

The shell loads Rust-owned workspace metadata/counts/audit history and offers eight navigable destinations. Device Explorer reads passive Windows inventory and requests a native rescan. Existing appearance settings are read from SQLite and remain selected after restart. SQLite migrations, isolated setting writes/reopen and malformed-setting rejection have passing tests. Real user preference writes were not executed in this audit.

There is no working firmware import, extraction, analysis, customization, build or flashing workflow. Firmware-related screens are explicitly planned UI; the artifact table and vault are empty. Chipset/firmware identification and feature qualification are not established by Windows inventory. All observed customization feature states remain `UNKNOWN` and flashing `UNSUPPORTED`.

Naming remains consistent: product/window `OBTAINER`, Rust package/executable `obtainer`/`obtainer.exe`, Rust library `obtainer_lib`, application ID `com.obtainer.desktop`, root npm package `obtainer-workspace`, desktop package `@obtainer/desktop`. RFS remains a reference architecture term, not a second product rename.

## H. Native Windows verification

| Attempt | Classification | Evidence |
|---|---|---|
| Fresh release, restricted execution | FAILED | Exit 1 before verified rendering: `invalid session id: session deleted as the browser has closed the connection` / `not connected to DevTools`. Driver log: `Unable to receive message from renderer`. No current HRESULT was captured; the older `0x800700AA` finding is historical, not a newly observed error code. |
| Same release, first normal-session run | FAILED (runner assertion) | Exit 1. Native React rendering/backend connection and navigation passed. Runner then asserted `Unexpected discovery status: starting` before initial asynchronous capture completed. This does not prove capture failed. |
| Same release, corrected readiness wait, normal session | VERIFIED | Exit 0. All native checks below passed. Ready inventory and no warning/error observed. |

Final run evidence directory: `.tools/native-audit/2026-10-16T10-29-38.032Z/`, containing `desktop-smoke.png` (111,031 bytes) and `native-driver.log` (0 bytes). Captured stdout and exact invocation are retained in the execution transcript. Failed-run logs remain in `2026-10-16T10-24-15.045Z/` and `2026-10-16T10-28-07.920Z/`.

Verified final sequence:

1. Tauri WebDriver starts the exact release executable and attaches to its native WebView2 window.
2. React renders “Desktop connected” after `App.refresh → getSnapshot → invoke("get_snapshot")` succeeds. This is rendered-state evidence, not a terminal startup message.
3. Navigation reaches all eight destination headings, including the deliberately planned screens.
4. Actual `get_discovery` returns `ready`; the script validates conservative feature states. Windows API results: **48 records**, USB 3, serial 1, network 46. Counts overlap because one record may have multiple transports.
5. Clicking the React Device Explorer Refresh button runs `useDeviceInventory.refresh → refreshDiscovery → invoke("refresh_discovery")`. A later native read has a greater revision.
6. Actual `get_snapshot` returns app version `0.1.0`, schema 1, data directory `C:\Users\chuka\AppData\Local\OBTAINER`, counts `devices: 48, artifacts: 0, runningJobs: 0`. The UI-selected theme agrees with stored settings.
7. A screenshot of the native Overview is saved and visually inspected: rendered sidebar/content, “Desktop connected,” 48 inventory records and zero artifacts/jobs.
8. The app closes and relaunches through a second native session. Selected theme and the complete returned settings object equal the first session; no `save_settings` call is made.

![Visually inspected native release window](images/phase-0-final-2026-10-16.png)

No manual confirmation is needed to support this verdict. Manual reproduction, if desired: in a normal Windows desktop session, run `.\target\release\obtainer.exe`; wait for “Desktop connected”; open each sidebar destination; open Connected devices and press Refresh devices; open Settings and note the selected appearance without changing it; close the window, launch the same executable again, then confirm Settings still shows that appearance. Browser preview or a running terminal alone does not satisfy this check.

## I. Persistent data protection

Initial Git status was clean before any write. `git diff --exit-code -- package-lock.json Cargo.lock Cargo.toml package.json apps/desktop scripts/rust.ps1 scripts/desktop.ps1 docs/IMPLEMENTATION_ROADMAP.md RFS_MASTER_PLAN.md docs/reference` exited 0. Existing application source, locks, storage configuration and schema are retained; no existing user modification was reset or overwritten. The only code edit is the bounded native test-runner wait described in C. Older evidence is preserved.

| Data | Actual location | Observed/protected state |
|---|---|---|
| SQLite/settings | `C:/Users/chuka/AppData/Local/OBTAINER/obtainer.db` | Existing main file 4,096 bytes, schema 1, WAL. Preferences live in `settings` row `preferences`, theme `system`. |
| SQLite sidecars | Same directory: `obtainer.db-wal` and `obtainer.db-shm` | Initially WAL 350,232 bytes, shared-memory file 32,768 bytes; WAL 449,112 bytes after native runs. Do not treat a lone main-file copy as a complete active-WAL backup. No deletion/checkpoint/move performed. |
| Firmware | `C:/Users/chuka/AppData/Local/OBTAINER/vault/` | Exists, zero files; `firmware_artifacts` rows remain zero. |
| Settings/configuration | Settings table above; `workspaces/` and `reports/` under the same data root | Both directories empty. No implemented separate router-configuration/snapshot/profile store; `configurations/` and `profiles/` absent. |
| Backups | No existing app-managed `OBTAINER/backups/` directory or backup service found | Future placeholder only; no backup files were removed. |
| Logs | `C:/Users/chuka/AppData/Local/OBTAINER/logs/` | Two existing daily files before, three after normal startup logging. Preserved. |
| WebView2 profile | `C:/Users/chuka/AppData/Local/com.obtainer.desktop/EBWebView/` | Exists and preserved; no reset or reinstall. |
| Audit probes | `.tools/phase-0-audit/final-verification-2026-10-16-before.json` and `...-after.json` | Existing read-only helper uses `mode=ro` and `PRAGMA query_only=ON`. Both probes themselves leave DB/WAL fingerprints unchanged. |

Before and after: `PRAGMA integrity_check` = `ok`, foreign-key check = no rows, schema = 1, preference fingerprint = `70ca50b9ab62d27cbc6340ed90d7ebab7596b61ac8875a63e86219af831f4d97`. The native runner independently compares existing settings across restart.

Table counts before → after: settings **1 → 1**, audit events **21 → 26**, cached devices **54 → 48**, artifacts **0 → 0**, jobs **0 → 0**. Startup and changed Windows inventory normally write audit/cache rows through the existing repository implementation. These expected changes are not a claim that the entire database stayed byte-identical. No firmware/configuration/settings loss was observed.

`lib.rs::run` still uses `app.path().local_data_dir()?.join("OBTAINER")`. No application identifier, schema, local-data path or migration was changed.

## J. Migration evaluation

**KEEP EXISTING STRUCTURE.** The checkout already uses `apps/desktop/`; the relevant files are present in commit `2c07c61b0bf5a88d571d1e3bfaaf3c1db5a09d98`, before this audit baseline. Root npm/Cargo wrappers, Tauri `frontendDist: "../dist"` and output paths work as configured. There is no technical need for another move.

Do not duplicate the application at the root or relocate SQLite to match a diagram. No migration or rollback was performed. Existing roadmap and planning documents remain unchanged.

## K. Outstanding issues and limits

**No critical Phase 0 blocker remains.** Restricted GUI automation failed; normal-session evidence succeeds with the same release executable. The runner's startup-state timing failure was corrected and verified. The source and environment do not require restructuring or a WebView profile reset on this evidence.

Not qualified by this audit: physical router/MiFi hot-plug and authenticated management, exact chipset/firmware support, live native preference writes against the user's DB, firmware features, installation/upgrade/uninstall, signing, clean-machine or remote CI behavior. These are explicit future/isolated-test limits rather than passed functionality.

A global Git ignore-file permission warning appeared for `C:/Users/chuka/.config/git/ignore`; repository status/history/diff checks still work. This is not a native-build blocker. Mutable Rust `stable` and the unmeasured disk-space field remain recorded environment limits.

## L. Verdict and next boundary

**Phase 0 verdict: PASS. Phase 1 readiness: READY.** Required baseline evidence is current and directly inspectable in the transcript/screenshot, source inventory, command results, and data comparison above.

**Smallest next action:** Phase 1 development-mode launch verification, when requested. This run stops at Phase 0; no Phase 1 implementation or device customization began.
