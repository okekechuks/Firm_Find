# OBTAINER GUI redesign verification

Date: 19 October 2026 (Africa/Lagos). Starting commit: `e71f676`, clean checkout. GUI redesign follows the supplied brief; the owner's later instruction removes purple and waives reference mockups. Existing device phases and configuration-first roadmap remain intact. No device adapter, extraction, flashing or proprietary rebuild was implemented.

**GUI desktop foundation: COMPLETE (PASS). Native development, optimized release, storage-copy restart and data-preservation acceptance passed.** This document distinguishes implemented controls from verified operations and unavailable device workflows.

## Repository audit and preservation

Reused the React shell/navigation, `Devices`/`ManualRouters`/`CompatibilityPanel`, `FirmwareLibrary`, appearance settings, `useDeviceInventory`, validated IPC, Windows discovery, profile engine, SQLite repository and bounded `LibraryService` worker. Removed the unreferenced planned-screen component and its styles; the roadmap remains documented separately. Legacy Analysis/Builder hashes still lead to the Library, and Compatibility remains accessible inside device details and through the retained internal route.

Normal data baseline: `.tools/gui-2026-10-19/normal-before.json`, read-only. Schema 4, one firmware artifact, 34 jobs, 20 import occurrences, 30 integrity checks, 141 device/metadata rows, one saved router, two profiles, 216 audit events. Settings fingerprint `70ca50b9ab62d27cbc6340ed90d7ebab7596b61ac8875a63e86219af831f4d97`; collective table fingerprint `a00a104f7fd022a1ca348cac652b4d4127dabc60b6d926ee2abd1a7c2c958c1d`. Tests use marked `OBTAINER-NativeTests` profiles or temporary Rust vaults, never real firmware or the normal database. The ordinary executable is compiled, not interactively driven against normal data.

## Implementation by GUI phase

| Phase | Status and evidence |
| --- | --- |
| A — Repository audit | Complete: clean Git baseline, actual modules/contracts inspected, read-only normal-data baseline retained. |
| B — Design system | Complete: navy/charcoal, blue accents, responsive panels/tables/forms/badges/switches, theme tokens, reduced motion, blue SVG and generated Windows icons. Native visual review found and corrected inherited statistic spacing, excessive default filter rows, light-theme blue contrast and short-window popover height. Final native dark/light/compact review and centered-confirmation geometry pass. |
| C — Sidebar/headers | Complete; verified in native development and optimized release: exactly Overview, Connected Devices, Library, Backup Manager, Logs, Settings; shared header and compact sidebar. |
| D — Page help | Complete; unit and native tested: one `PageHelp`, contextual instructions, explicit close, Escape/outside dismissal, focus return. Native keyboard/focus/viewport containment passes for all six contexts. |
| E — Overview | Complete: four genuine counts, shared active jobs, byte progress and cancel, empty state and Library/Backup links. No Recent Activity or second devices panel. |
| F — Devices | Existing discovery/refresh/manual connections/evidence reused. Conservative green/yellow/red classification; selection preserved; Extract blocked with reason because no acquisition adapter exists. |
| G — Library | Existing import/hash/integrity/manifest pipeline retained; global SQL type/sort filters, provenance tags, details, local backup, reversible Delete confirmation and capability-gated four-feature panel. |
| H — Backups | Real independently verified copies on the existing worker; confirmed checksum-verified restore, `_DEFAULT` names/conflict suffixes, preserved source, per-item reversible removal and Refresh. |
| I — Logs | Existing persisted events/jobs centralized in searchable/filterable timestamped table, structured outcome/error/recovery details; historical rows preserved. |
| J — Settings | Existing theme contract retained. Separate persistent refresh/notification preferences, managed backup directory, real native folder plan/copy/restart and actual backend concurrency display. |
| K — Integration | New commands explicitly registered in `build.rs`, handler and narrow Tauri capability permissions. One shared feed/provider drives jobs, notifications and views. Full native acceptance passes, including real IPC, shared state and restart. |
| L — Tests/visual QA | 84 frontend tests and 85 Rust tests pass, including schema-4 preservation and cancelling an active backup. 91 full native checkpoints and retained WebDriver smoke pass. Populated screenshots are visually inspected. |
| M — Build | TypeScript/lint and frontend production build pass. Final ordinary and dedicated optimized Windows builds pass (1m28s Rust compilation each); ordinary executable SHA-256 stays unchanged across the dedicated build. |

## Working backend extensions

`src-tauri/src/library/desktop.rs` extends the existing worker rather than creating a competing job system. New commands: `get_desktop_state`, `save_desktop_preferences`, `list_firmware_backups`, `create_firmware_backup`, `restore_firmware_backup`, `remove_firmware_item`, `open_managed_folder`, `select_library_directory`, `relocate_firmware_library`.

Additive migration `005_desktop_workspace.sql` introduces desktop preferences, backup metadata, presentation labels and a removal timestamp. Existing settings JSON, profile registry, identity evidence and import/job/audit records retain their structure. A dedicated nonempty schema-4 migration regression verifies old rows and theme across repeated opens.

Backups stream to a unique stage, check the source digest, synchronize and independently verify copied bytes before no-overwrite publication and transactional registration/completion. Restore rehashes the preserved backup before using the existing import pipeline. It never flashes a router. Restored names retain their extension, append `_DEFAULT` once where appropriate and use case-insensitive conflict suffixes. Content deduplication and all import occurrences remain preserved.

**Delete semantics:** remove only the selected item from its active inventory; preserve stored bytes and all history for recovery. Dialogs state this explicitly. No physical purge or automatic garbage collection was added. Backend refuses removal while library work is active. Reimport or a verified backup restore can recover a removed Library artifact.

Library directory changes use a native folder picker and an expiring opaque plan ticket, not arbitrary path IPC. Validate local/no-reparse paths, write access, free space, scope and destination conflict. Review count/bytes/source/destination before confirming. The existing worker copies and independently verifies registered blobs to a new dedicated directory; metadata stays in the original database. Only the final transaction saves the new location. Restart activates it; old bytes remain. A failed/cancelled copy never switches paths and retains uncertain output for review. No new imports enter after relocation is accepted until restart.

Automatic refresh is enforced by the existing discovery service: initial enumeration/manual Refresh still work; event/periodic scans respect the saved preference. Windows capture/correlation was not replaced. Completion/failure and candidate arrival/removal preferences control real notifications; historical completed jobs do not produce fictional startup toasts. The single worker remains enforced in Rust and is displayed read-only.

## Capability limits

Extract, APN/Wi-Fi limit/DNS/USSD changes and Rebuild remain unavailable. Four feature categories are visible with disabled switches and an explicit reason; no simulated progress, guessed limits or Apply path is added. Green means verified router identity, never extraction permission. Protected PC classes override a misleading candidate category. Generic Windows interfaces/USB IDs/user labels do not grant support.

On 19 October, read-only PnP inspection found no connected `1286:4E31` MG11. Device-specific physical qualification is therefore not claimed. Existing engine regressions retain UNKNOWN/unverified behavior. GUI device selection/screenshots use actual current Windows observations. The requested extraction screenshot shows the honest unavailable state, not invented extraction progress.

## Executed results and corrections

- Initial frontend build and Rust check passed. First regression run: 48/56 frontend passed; eight old assertions/missing new-command mocks corrected for the requested layout. A later 55/56 run exposed a split-text matcher; corrected. New GUI tests first brought the total to **78/78**. One later resource-stalled run timed out starting a worker and in the combined navigation test; split six-page help navigation into independent cases. Final serial and repository-default runs both pass **84/84 in nine files**. No unhandled worker errors remain in those successful runs.
- Initial Rust feature check exposed missing `Win32_UI_Shell_Common` feature and private worker method visibility; corrected. Initial feature tests **83/83** pass; final default/feature suites **85/85** pass. Default/feature Clippy, formatting and ESLint exit 0.
- Icon generation first used a root-relative path with the desktop wrapper (exit 1); corrected to `npm.cmd run tauri -- icon public/mark.svg --output ../../.tools/gui-2026-10-19/icons` (0). CLI reports an unrelated installed `mstmc.ttf` font warning; generated vector icons use no font.
- First native preview stopped on a test-script string syntax error before launch. Next preview `.tools/foundation/2026-10-19T11-11-02.076Z/` rendered all six destinations but exposed missing new-command capability permissions and a stale later heading assertion. Corrected explicit permissions and assertion; failed isolated run retained, normal before/after equal.
- Corrected preview `.tools/foundation/2026-10-19T11-13-54.885Z/` passes native dev GUI, new IPC, six destinations, three real discovery refreshes, same-value SQLite settings, empty console issue capture, process shutdown, cleanup refusal tests and guarded final cleanup. Run `3b0b522c985c1434c43c846fd3ad6888` removed; normal database/WAL/vault/settings unchanged. Actual screenshot review led to reducing inherited statistic-card spacing.

## Final acceptance

**PASS for the implemented desktop foundation.** Final `npm.cmd run test:gui` exits **0**, with **91 checkpoints**, at `.tools/foundation/2026-10-19T12-35-41.796Z/`. Isolated run `995d66c51c7a8a0901843f0645034e26` was cleaned with exit 0 after all owned processes stopped. Portable results and the exact native child commands are in [the evidence transcript](evidence/gui-2026-10-19.txt).

- Real native development and optimized release render all six destinations, use typed React-to-Rust IPC/SQLite schema 5, refresh actual Windows observations three times without duplicate IDs, and retain settings/history across launches.
- Injected error/loading recovery, component edit/restoration with unchanged app/Vite PIDs, real streamed progress/cancel, concurrent deduplication, corruption/missing-file detection, transaction failures, graceful verification shutdown and forced import interruption/recovery pass.
- Actual backup, cancelled/confirmed restore, `_DEFAULT` naming, reversible removal/recovery and completion notifications pass in both dev and release. Four customization switches and Rebuild remain disabled; no fake operation was substituted.
- All help dialogs pass real Escape/focus/containment checks. A real 920 x 680 native window uses the compact sidebar without horizontal overflow. Dark/light/system appearance and persisted refresh preferences pass. The final copy confirmation is centered within the usable viewport, excluding the native scrollbar.
- Native folder selection and plan cancellation preserve the active path. Confirmed copy independently hashes three registered generated images totaling **270,532,608 bytes**. Restart activates the selected Library, retaining exact checksums/sizes/types/source counts/theme/refresh and the separate backup path. A further actual integrity job successfully reads the relocated storage after restart. Old Library bytes are retained.
- Captured native console warnings/errors/unhandled exceptions: **0**. Final census: **0** app/driver/workspace-Vite processes and **0** listeners on 1420/9223/4445/4446. Successful run root absent; failed diagnostic runs remain retained.
- Final normal-data probe exits **0** and exactly equals the original GUI baseline: database and WAL bytes/hashes, complete rows, settings, router/profile identities, vault and logs/workspaces/reports. Integrity is `ok`; foreign-key violations are empty. Normal schema remains 4 because the owner data was never launched/migrated by automated GUI acceptance. Schema-4-to-5 preservation is separately tested using nonempty isolated databases.

No critical desktop startup/runtime blocker remains in the tested flows. Extraction progress, device writes and proprietary rebuilding remain **UNAVAILABLE**, not passed operations. MG11 was absent today, so no fresh physical qualification is claimed. Separate Explorer-window inspection, installer/signing and additional Windows/filesystem/hardware combinations are outside this executed qualification. Next action: owner review of the blue UI; Phase 5 remains unstarted.

See [native testing](NATIVE_TESTING.md), [phase tracker](PHASE_STATUS.md), [library](FIRMWARE_LIBRARY.md) and [vault](FIRMWARE_VAULT.md).

## Exact executed check commands

Commands run from the repository root; `CARGO_INCREMENTAL=0` for Rust/native compilation.

| Command | Executed outcome |
| --- | --- |
| `node node_modules/typescript/bin/tsc --project apps/desktop/tsconfig.json --noEmit` | 0 after initial test callback typing correction; production `tsc -b` also passes. |
| `npm.cmd run lint` | 0; no warnings. |
| `npm.cmd test` | 0 final; 84 tests / nine files. Earlier timed-out attempt retained in execution history. |
| `npm.cmd run test --workspace @obtainer/desktop -- --maxWorkers=1 --reporter=dot` | 0; 84 tests. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all --check` | 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | 0; 85 tests. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | 0; 85 tests. |
| `npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline` | 0; current frontend production assets and Windows executable. |
| `npm.cmd run build:native-tests` | 0; dedicated optimized test executable; ordinary executable SHA-256 preserved. |
| `npm.cmd run test:foundation -- --gui-preview --gui --phase4` | 0; populated development GUI, backup/restore/removal, real progress, theme/help/settings and preservation/cleanup. |
| `npm.cmd run test:gui` | 0 final; 91 native checkpoints, including storage-copy restart and new-location integrity. Earlier failed attempts documented below. |
| `npm.cmd run test:native -- target/release/obtainer-native-test.exe --audit` | 0; retained WebDriver smoke, same-value preferences and restart. |
| `py scripts/probe-native-isolation.py .tools/gui-2026-10-19/normal-after.json .tools/gui-2026-10-19/normal-before.json` | 0; exact full normal-data equality. |
| `git diff --check` | 0; LF/CRLF notices only. |

Final normal executable SHA-256 after current paired builds: `066FF4C83EFEFD4197F3DB31019550DBB22D087460D8182591B44AA0592795A9`.

Populated dev diagnostic evidence: `.tools/foundation/2026-10-19T11-31-19.854Z/`, run `e5c2f99a45985f0d3ef46fa47e678e36`, exit 0, successful guarded cleanup. Screenshots are actual native React views, not mockups. Final source adds the compact-filter/contrast/short-window refinements after that visual review.

The first full GUI run `.tools/foundation/2026-10-19T11-25-46.864Z/` passed Phase 4/4.1 streaming, duplicate/concurrency, integrity, cancellation and injected database/manifest failures, then timed out on component HMR. Isolated HMR reproduction `.tools/foundation/2026-10-19T11-29-28.381Z/` passed. The verifier now reloads the restored module graph between fault injection and its component-only HMR edit; the populated dev rerun proves that edit/restoration without another reload or changed app/Vite PIDs. Failed run `c2d407108f78093f6b3520878a2c5396` remains retained; normal data equality passes.

## Source inventory

Paths below are relative to `apps/desktop/` unless marked otherwise. Functional discovery, profile matching and bounded import/verification modules are retained.

| GUI phase | Relevant changed files and integration | Qualification and next step |
| --- | --- | --- |
| A | Root `README.md`; development, vault, library and job guides | Clean starting commit and read-only preservation snapshot; proceed to tokens. |
| B | `src/styles.css`, `public/mark.svg`, existing Windows files in `src-tauri/icons/` | Unit/build and real dark/light/compact screenshots; proceed to shell. |
| C | `src/app/App.tsx`, `src/app/navigation.ts` | Exactly six links in unit and native tests; proceed to instructions. |
| D | `src/components/PageHelp.tsx` | Six contexts, real keyboard focus/Escape, containment; proceed to Overview. |
| E | `src/features/Dashboard.tsx`, `src/components/WorkspaceProvider.tsx`, `src/lib/workspace-context.ts` | Actual backend statistics and active job lifecycle; proceed to Devices. |
| F | `src/features/Devices.tsx`, `src/lib/device-status.ts` | Existing discovery and conservative eligibility; extraction unavailable until a device adapter is qualified. Proceed to Library UI. |
| G | `src/features/FirmwareLibrary.tsx`, `src/components/CustomizationPanel.tsx`, `src/lib/format.ts` | Existing native import/integrity/manifest, tags, details, removal and disabled customization tested; rebuilding unavailable. Proceed to Backups. |
| H | `src/features/Backups.tsx`, `src/components/ConfirmDialog.tsx`, `src-tauri/src/library/desktop.rs` | Actual checksum-verified copies/restores and confirmation cancellation; proceed to Logs. |
| I | `src/features/Jobs.tsx`, `src/lib/useLibraryFeed.ts` | Persisted job/event filters and shared cancellation; proceed to Settings. |
| J | `src/features/Preferences.tsx`, `src-tauri/src/discovery/service.rs` | Real persisted appearance/refresh/notifications; native storage-copy/restart gate documented below. Proceed to integration. |
| K | `src/lib/backend.ts`, `src/types/library.ts`, `src-tauri/src/library/{mod,db,ipc,types}.rs`, `src-tauri/src/{storage,lib}.rs`, migration `005_desktop_workspace.sql`, `build.rs`, `capabilities/main.json`, nine generated command permissions | Typed IPC, additive schema 5 and one existing worker; proceed to full native tests. |
| L | `src/test/GUI.test.tsx`, existing App/Library fixtures/setup, `src-tauri/src/library/tests.rs`; root `scripts/verify-{foundation,firmware,profiles,gui}.mjs`, `native-smoke.mjs`, folder/resize helpers | Executed regression results and actual screenshots, not visual mockups; proceed to builds. |
| M | Root `package.json` adds `test:gui`; existing native build helpers reused | Ordinary and dedicated optimized Windows builds; final acceptance below. |

Open Folder dispatches the real managed-path Explorer command; automated acceptance does not inspect a separate Explorer window. No claim of general router support follows from the GUI checks. Backups display the latest 100 entries; Logs display the latest 100 jobs and 100 audit events without deleting older records. Library byte totals count active catalog images, not retained/orphan disk usage.

The populated full run `.tools/foundation/2026-10-19T12-13-59.839Z/` passed development and optimized-release navigation, real backup/restore/removal, narrow-window help, forced/graceful shutdown recovery and retained Phase 4/4.1 regressions, then failed in folder-picker automation. The helper sent a process-local string pointer in a custom window message across processes. Replaced that test-only mechanism with the actual visible folder edit control (`WM_SETTEXT`) and native OK button. No production code was changed for this automation correction. Failed run `819f6d25d74b66b7556ed9c85242f48a` is retained; normal-data before/after equality passed. Rerun evidence: `.tools/foundation/2026-10-19T12-20-34.480Z/`.

That rerun passed the same development/release regressions, then attempted Change folder before the preceding real preferences save had released its busy state. Corrected the native button helper to wait for an enabled control instead of treating asynchronous completion as immediate. Failed run `7789dcf9980ce98800567e2b869e46ac` is retained and normal-data equality passed. Final rerun evidence: `.tools/foundation/2026-10-19T12-23-45.382Z/`.

The corrected run at `12-23-45.382Z` exited 0 with **90 native checkpoints**: actual folder selection, plan cancellation, verified 258 MiB Library copy, optimized restart with identical artifact metadata/provenance/theme/refresh and unchanged backup path, clean console capture, preservation and guarded cleanup. Run `073bf84af971dd7c7cafd2b46a11e947` was removed only after owned processes stopped. Separate retained WebDriver audit `.tools/native-audit/2026-10-19T12-27-06.803Z/` also exits 0, including six-page navigation, real IPC, 51 observed Windows records, preference persistence and cleanup.

Visual review of the new storage-plan capture found the native HTML confirmation at the top-left because of the inherited margin reset. Added explicit `margin:auto` to `.confirm-dialog`; ordinary and dedicated optimized builds pass again. Native regression now asserts modal center/viewport bounds and performs a real integrity job after the relocated Library is reopened. Final rebuilt-source evidence is recorded in Final acceptance; this screenshot correction does not change storage or worker behavior.

Rebuilt-source run `.tools/foundation/2026-10-19T12-32-05.920Z/` passed retained development/release workflows and rendered the corrected centered dialog, but its new centering assertion measured `innerWidth`, including the 15 px Windows scrollbar. Corrected the assertion to use the usable `documentElement.clientWidth/clientHeight`; product code did not need another change. Failed run `f47591cb73e4fa1c04f41f0830d7d759` is retained with its actual centered-dialog screenshot and unchanged normal-data proof. Final native evidence: `.tools/foundation/2026-10-19T12-35-41.796Z/`.

## Actual native screenshots

All captures show the visibly marked isolated native Windows application, actual React views and generated harmless files. Device rows are current Windows observations. No reference images were provided or required after the owner's clarification.

| Requested view | Screenshot |
| --- | --- |
| Overview | [Release Overview](images/gui-production-overview.png) |
| Active job | [Real 256 MiB import/cancellation](images/gui-dev-reopen-overview-active-job.png) |
| Connected Devices | [Release devices](images/gui-production-connected-devices.png) |
| Selected device | [Release details](images/gui-production-selected-device-details.png) |
| Extraction | [Actual unavailable state](images/gui-production-extraction-unavailable.png). Extraction progress is unavailable because no acquisition adapter exists; no simulation is represented as extraction. |
| Library | [Release Library](images/gui-production-library.png) |
| Customization | [Four disabled categories and Rebuild](images/gui-production-customize-firmware.png) |
| Backup Manager | [Release backups](images/gui-production-backup-manager.png) |
| Logs | [Persisted release Logs](images/gui-production-logs.png) |
| Settings | [Release Settings](images/gui-production-settings.png) |
| Information | [Open contextual instructions](images/gui-production-information-popover.png) |
| Light theme | [Actual light appearance](images/gui-production-light-theme.png) |
| Compact layout | [Native 920 x 680 window](images/gui-production-compact-layout.png) |
| Storage review | [Centered copy/verify confirmation](images/gui-production-storage-copy-plan.png) |

Native captures were visually inspected for spacing, blue palette, legibility, clipping and popover containment. Automated geometry checks supplement visual review. The active-job image measures a real generated-file import, not extraction. The sidebar and main content can scroll independently in short windows.
