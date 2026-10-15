# OBTAINER foundation verification — 14 October 2026

This file preserves earlier foundation and discovery evidence. Latest results after the phase 0 workspace migration appear below and in the [phase tracker](PHASE_STATUS.md).

## Phase 0: workspace migration — 15 October 2026

**Result: structure migration and build checks passed.** The desktop app now lives in `apps/desktop/`; root npm commands, the shared dependency lockfile, Cargo membership, and desktop wrapper resolve the moved app. Generated outputs and temporary smoke files were cleaned after validation.

| Check | Result | Evidence |
| --- | --- | --- |
| Clean dependency install | Passed | Offline `npm ci --ignore-scripts`; 297 packages installed, zero reported vulnerabilities |
| Root frontend checks | Passed | ESLint; 3 files and 10 tests; production build of 1,601 modules |
| Rust checks | Passed | Format, Clippy with warnings denied, and 11 tests |
| Relocated Windows desktop build | Passed | Tauri produced `target/debug/obtainer.exe` through the root wrapper |
| Native UI smoke | Blocked by WebView2 | App failed before IPC: HRESULT `0x800700AA`, “The requested resource is in use”; WebDriver session closed. Phase 1 tracks this startup blocker. |

`git diff --check`, workspace dependency listing, and local Markdown links passed. These checks confirm project layout and build readiness; native window launch remains unverified.

## Phase 2: passive Windows discovery — 15 October 2026

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
| Native Windows UI and inventory flow | Blocked by host | WebView2 failed before app startup with `0x800700AA`; existing WebView2 processes were active. The smoke test did not reach Rust IPC or the UI. |

The Windows host inventory and fixture replay tests pass. Native window behavior remains unverified in this session due to the WebView2 resource lock. Physical router hot-plug and any device-specific firmware access have not been validated. Step 5 feature controls remain future work and require a verified device profile and safe control plane.

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

The native test temporarily changes appearance, records audit events, then restores the prior appearance and closes its own test windows/drivers.

## Next milestone

Under the revised plan, phase 0 structure migration and baseline checks are complete. Next, close phase 1 desktop launch and phase 2 discovery gaps, then implement phase 3 device profiles and capability decisions. The WebView2 launch failure is recorded under phase 1. Firmware import is now phase 4; authenticated OpenWrt reads are phase 7. The earlier acquisition-first ordering is superseded. Existing implementation limitations above remain unchanged by planning documents.
