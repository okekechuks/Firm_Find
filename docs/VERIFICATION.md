# OBTAINER verification record

This file preserves earlier foundation/discovery evidence and the current corrective audit. The [Phase 0 audit](PHASE_0_AUDIT.md) contains exact commands, versions, classifications, data paths and limitations; the [phase tracker](PHASE_STATUS.md) contains the acceptance checklist.

## Corrective Phase 0 audit — 15 October 2026

**Result: COMPLETE; Phase 1 READY.** Fresh debug and optimized Windows builds pass. Native audits of the existing debug app and final release pass in a normal, non-administrator Windows session: React rendering, navigation, real Rust/SQLite reads, device refresh, and unchanged existing settings across restart.

| Check | Result | Evidence |
| --- | --- | --- |
| Dependencies and frontend | Passed | Offline `npm ci --dry-run --ignore-scripts`, workspace dependency listing, TypeScript, lint, 10 tests, frontend production build |
| Rust | Passed | Format, all-target Clippy with warnings denied, 11 tests including real Windows capture and temporary-database persistence |
| Current native builds | Passed | Debug and optimized release; both root Tauri commands used `--locked --offline --no-bundle` |
| Current normal-session native runtime | Passed | Real IPC/navigation/refresh/restart; release reports 54 records, ready status, notifications registered, no warnings |
| Existing data | Passed | Read-only SQLite integrity `ok`, no foreign-key violations, schema 1 and preference fingerprint unchanged |
| Restricted runtime | Failed in restricted context | WebView2 `0x800700AA` / WebDriver disconnect before IPC; normal-session counterchecks pass without profile reset |
| Visual evidence | Passed | [Fresh release screenshot](images/phase-0-audit-2026-10-15.png), reviewed after the status/tag corrections |

This run did not overwrite user preferences, move source/data, clean existing files, change schemas/names, operate on physical routers, generate new installers, or execute remote CI. `--audit` mode writes unique evidence under `.tools/native-audit/`; normal app startup/inventory records still occur. The earlier claim of an unresolved host-runtime blocker is superseded by the successful normal-session tests. Development launch and foundation gap review remain Phase 1 work.

## Historical workspace migration checks — earlier 15 October 2026

**Result: structure migration and build checks passed.** The desktop app now lives in `apps/desktop/`; root npm commands, the shared dependency lockfile, Cargo membership, and desktop wrapper resolve the moved app. Generated outputs and temporary smoke files were cleaned after validation.

| Check | Result | Evidence |
| --- | --- | --- |
| Clean dependency install | Passed | Offline `npm ci --ignore-scripts`; 297 packages installed, zero reported vulnerabilities |
| Root frontend checks | Passed | ESLint; 3 files and 10 tests; production build of 1,601 modules |
| Rust checks | Passed | Format, Clippy with warnings denied, and 11 tests |
| Relocated Windows desktop build | Passed | Tauri produced `target/debug/obtainer.exe` through the root wrapper |
| Restricted native UI smoke | Failed in that context | App failed before IPC: HRESULT `0x800700AA`, “The requested resource is in use”; WebDriver session closed. Current normal-session tests above now pass. |

`git diff --check`, workspace dependency listing, and local Markdown links passed in that run. Those checks alone did not verify native launch; the subsequent corrective audit above provides actual normal-session runtime evidence.

## Historical passive Windows discovery checks — earlier 15 October 2026

**Result: implemented and verified in host tests; native Windows debug build succeeds.** Device Explorer reads Windows USB, serial, and network-adapter inventory, shows correlation evidence and confidence, and supports a refresh. It performs no router sessions, firmware reads, configuration writes, network probes, or flashing. The native WebView smoke test could not start because Windows returned `0x800700AA` (“The requested resource is in use”) while creating WebView2.

| Check | Result | Evidence |
| --- | --- | --- |
| Rust format and Clippy | Passed | `fmt --all --check`; all targets, warnings denied |
| Rust tests | Passed | 11 tests, including passive Windows inventory, fixtures, identity correlation, safe unknown capabilities, and storage |
| Rust workspace check | Passed | Offline, locked dependencies |
| Frontend lint and TypeScript | Passed | ESLint with zero warnings; project typecheck |
| Frontend component tests | Passed | 3 files, 10 tests, including discovery UI, filtering, error handling, and browser-preview safety |
| Frontend production build | Passed | Vite 6.4.4; 1,601 modules |
| Native Windows debug build | Passed | Tauri produced `target/debug/obtainer.exe` |
| Native Windows UI and inventory flow | Failed restricted run | WebView2 failed before app startup with `0x800700AA`. That smoke test did not reach Rust IPC or the UI; it did not establish a host-runtime defect. |

The Windows host inventory and fixture replay tests passed. Native window behavior was unverified by that restricted run; normal-session native checks now pass in the corrective audit above. Physical router hot-plug and device-specific firmware access remain unvalidated. Feature controls remain future work and require a verified device profile and safe control plane.

## Historical foundation acceptance — 14 October 2026

**Result: desktop foundation verified on this Windows host.** Device discovery, firmware import, analysis, acquisition, and image generation remain later milestones.

User story: open OBTAINER, navigate eight workspaces, read Rust-owned SQLite state, change appearance, and preserve preferences across desktop restarts.

## Evidence

| Check | Result | Evidence |
| --- | --- | --- |
| Frontend production build | Passed | TypeScript and Vite 6.4.4; 1,598 modules; generated `dist/` |
| Frontend lint | Passed | Zero warnings, including native test harness |
| Shell / IPC component tests | Passed | 2 files, 7 tests, Vitest 4.1.11 |
| Development server | Passed | Localhost HTML and transformed entry return HTTP 200 |
| Dependency audit | Passed | npm reports zero vulnerabilities after patched Vitest upgrade |
| Rust format | Passed | `cargo fmt --all --check` |
| Rust unit / repository tests | Passed | 4 tests: risk policy, settings validation, migration/reopening persistence, newer-schema rejection |
| Rust clippy | Passed | `cargo clippy --workspace --all-targets --locked -- -D warnings`, zero warnings |
| Bundled debug desktop build | Passed | Actual `target/debug/obtainer.exe` generated |
| Native debug UI flow | Passed | Navigation, real state, saved preferences, invalid-input rejection, restart persistence |
| Native production UI flow | Passed | Same flow against final release executable; original theme restored |
| Normal development launch | Passed | `npm run tauri -- dev --no-watch` starts Vite and launches an OBTAINER window |
| Native development UI flow | Passed | Actual IPC and restart persistence using the development binary and live Vite server |
| Release / installer build | Passed | Optimized Windows x64 application plus NSIS and MSI bundles |
| Visual review | Passed | Actual native WebView screenshot reviewed; see [desktop screenshot](images/desktop-smoke.png) |

Native tests use actual commands and SQLite, without IPC mocks. Component tests use explicit test fixtures. The risk test verifies the foundation policy; no hardware-operation API exists.

## Generated artifacts

| Artifact | File | Bytes |
| --- | --- | ---: |
| NSIS, current user | `target/release/bundle/nsis/OBTAINER_0.1.0_x64-setup.exe` | 2,801,667 |
| MSI | `target/release/bundle/msi/OBTAINER_0.1.0_x64_en-US.msi` | 3,989,504 |
| Standalone executable | `target/release/obtainer.exe` | See local file |

SHA-256:

```text
E569DA5B05424ACAE3CCB121557EBE7E7AC71EF972AC29EBB6A4BDF9C4A29DF8  OBTAINER_0.1.0_x64-setup.exe
F6AA3CA16C5C5B808CD6521CE32FFC38B6E31F77C4BCABA11856A76D1D0EDB2F  OBTAINER_0.1.0_x64_en-US.msi
```

Installers are **unsigned development artifacts**. Clean-machine installer execution, upgrades/uninstall, release signing and verified updates are not yet validated. No production-release claim is made.

## Prerequisites detected and installed

- Windows x64 host; native tests run in the normal user session.
- Node 22.17.0, npm 10.9.2 and Git available.
- WebView2 / Edge 155.0.4283.45.
- Isolated Rust/Cargo 1.99.0 and rustup 1.29.1 under ignored `.tools`; no global Rust PATH change.
- User-approved Visual Studio Build Tools 2022 installed through winget: 17.14.41, installation version 17.14.37710.0, C++ workload and Windows SDK 10.0.26100.0. Installer exited successfully.
- Tauri Rust/API 2.12.3 and CLI 2.12.2, recorded in lockfiles.
- Tauri driver 2.1.0 installed in the isolated toolchain. Matching Microsoft Edge driver downloaded locally; Microsoft Corporation Authenticode signature verified.

## Corrections during verification

The missing MSVC linker blocker was resolved by the approved compiler/SDK installation. Native tests and builds then passed.

npm 10 crashed while resolving patched test dependencies. A temporary npm 11 invocation rebuilt the generated dependency tree and produced a working package-lock. No global npm upgrade occurred. Tests include only `src/test/**/*.test.{ts,tsx}`; Tailwind scans only `src`. Vite ignores local tools, native sources and target output so generated files do not trigger frontend reloads.

PowerShell wrappers forward raw script arguments, preserving Cargo lint flags. The Settings decoration is hidden from assistive technology so its link has a clear accessible name.

The Codex in-app browser tool failed before connecting with `missing field sandboxPolicy`. Native Tauri WebDriver validated the Windows application directly, including visual capture; no in-app browser verification is claimed. See the official [native Tauri driver setup](https://v2.tauri.app/develop/tests/webdriver/manual-setup/).

## Reproduction commands

```powershell
npm run lint
npm test
npm run build
powershell -File scripts/rust.ps1 fmt --all --check
powershell -File scripts/rust.ps1 test --workspace --locked
powershell -File scripts/rust.ps1 clippy --workspace --all-targets --locked -- -D warnings
npm run tauri -- build --debug --no-bundle --ci
npm run test:native -- target/debug/obtainer.exe
npm run tauri -- build --ci -- --locked
npm run test:native
npm run tauri -- dev --no-watch
# While the development server runs, test the development binary:
npm run test:native -- target/debug/obtainer.exe
```

The default native test temporarily changes appearance, records audit events, restores the prior appearance, and closes its own test windows/drivers. The new `--audit` option checks existing settings without calling `save_settings`; use it for a data-preserving baseline audit.

## Next milestone

The corrective Phase 0 gate is satisfied. Next, verify Phase 1 development-mode launch and shell loading/error/reconnect behavior, then close revised Phase 2 discovery gaps before implementing Phase 3 profiles/capability decisions. Firmware import is Phase 4; authenticated OpenWrt reads are Phase 7. Restricted runtime failures remain recorded as execution-context limitations, not an unresolved normal-session launch blocker.
