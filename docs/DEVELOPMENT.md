# Windows development baseline

Run project commands from the repository root. The application lives in `apps/desktop/`; the root owns the npm lockfile, Cargo workspace, scripts, and CI entry points. See the [Phase 0 audit](PHASE_0_AUDIT.md) for executed commands and current evidence.

## Prerequisites

The audited host has Windows 11, Node 22.17.0, npm 10.9.2, Git 2.51.2, Rust/Cargo 1.99.0, Visual Studio Build Tools 2022 with the x64 C++ tools, Windows SDK 10.0.26100.0, and WebView2 155.0.4283.45. The Rust target is `x86_64-pc-windows-msvc`. WSL2/Ubuntu is available but is optional for the desktop baseline.

The PowerShell wrappers use `.tools/cargo` and `.tools/rustup` when a local Cargo installation exists. On a clean machine without those ignored directories, install the native prerequisites and put Rust/Cargo on PATH. The wrappers do not install system tools or download drivers automatically.

The npm and Cargo lockfiles fix dependency resolutions. `rust-toolchain.toml` currently selects the mutable `stable` channel, and CI selects Node major 22. Record exact compiler and Node versions when reproducing results; a byte-identical release is not claimed.

## Install and launch

For a clean checkout with prerequisites installed:

```powershell
npm ci
npm run tauri -- dev
```

`npm run dev` starts only the frontend browser preview. It does not provide native IPC, SQLite, or hardware inventory. `npm run tauri -- dev` starts the actual Windows application through the root wrapper, with React hot reload and the Rust watcher. For an already-cached toolchain, `npm run tauri -- dev -- --locked --offline` is the verified offline variant.

## Verify the baseline

```powershell
npm ls --workspace=@obtainer/desktop --depth=0
npm run lint
npm test
npm run build
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all --check
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked
npm run tauri -- build --debug --no-bundle --ci -- --locked
```

`npm run build` performs TypeScript checking and a frontend production build. Native debug and release builds share the root `target/` directory. Offline checks additionally require already-cached dependencies; the audit used `--offline` and disabled Rust incremental compilation per process to limit temporary disk use.

## Native audit

The native runner expects `.tools/cargo/bin/tauri-driver.exe` and `.tools/webdriver/msedgedriver.exe`. The audited Tauri driver is recorded in the audit; the installed Edge driver and WebView2 both report `155.0.4283.45`. These drivers are separate prerequisites, not npm dependencies.

Run from a normal Windows user session with an interactive desktop:

```powershell
npm run test:native -- target/debug/obtainer.exe --audit
```

Audit mode preserves saved preferences, checks real IPC and discovery, navigates the UI, and checks existing settings after restart. It writes new evidence below `.tools/native-audit/`. App startup still appends normal audit events and refreshes the passive inventory. Omitting `--audit` runs the existing broader smoke test, which temporarily changes appearance and restores it.

In restricted execution, WebView2 can fail with `0x800700AA` before IPC. Reproduce in a normal Windows session before diagnosing a host-runtime defect. Do not delete or reset the existing browser profile to work around this failure.

## Phase 1 native acceptance

With a fresh release executable available, close other OBTAINER development sessions and run in a normal interactive Windows session:

```powershell
npm run tauri -- build --no-bundle --ci -- --locked --offline
npm run test:foundation
```

This uses the existing Tauri development command and release executable, a temporary loopback WebView2 debugging port (9223), and Vite's existing port (1420). It verifies real IPC, eight destinations, repeated discovery refresh, shared storage/settings, native console health, React hot reload, shutdown/reopen and recovery from an interrupted test-owned launch. No WebDriver or extra npm package is needed.

The verifier temporarily instruments the frontend module for explicitly labelled failure/delay tests and changes a sentence to prove hot reload, then restores the exact source it read. Do not edit those files or run another build/test against them concurrently. It writes the existing appearance value through real settings IPC; it does not change the user's chosen appearance. Normal application audit/inventory writes still occur. Evidence is saved in unique `.tools/foundation/` directories. The [Phase 1 report](PHASE_1_VERIFICATION.md) records results and limits.

## Persistent data

Application state is under `%LOCALAPPDATA%/OBTAINER/`; WebView2 uses `%LOCALAPPDATA%/com.obtainer.desktop/EBWebView/`. Source moves do not change these paths. SQLite uses WAL, so copying `obtainer.db` alone while the application is active is not a valid backup. Keep the app closed before any owner-authorized migration or use a SQLite-aware backup procedure. Phase 0 performs no data migration, cleanup, profile reset, or physical router changes.

## Phase 3 verification

Build the current release first, close other OBTAINER sessions, and keep the owner-reported MG11 connected by USB:

```powershell
node scripts/generate-profile-schema.mjs
npm run tauri -- build --no-bundle --ci -- --locked --offline
npm run test:foundation -- --phase3
```

The optional `--phase3` suite verifies both the Compatibility workspace and connected-device details through real native WebView2 and IPC. It validates/rejects profile documents, saves the exact owner-provided identifier/software text as unverified local evidence, confirms four unknown MG11 feature assessments and checks persistence across dev/recovery/production launches. Repeated reports deduplicate. It makes no new router request and changes no router settings. The shared foundation suite checks nine destinations and the current database schema (4 after Phase 4). The interruption case deliberately terminates only its own launch; exit 1 for that subprocess is expected.

For manual inspection, run `npm run tauri -- dev -- --locked --offline`, open **Compatibility**, select the USB device candidate and expand feature evidence, candidate exclusions and identity evidence. Confirm unknown features stay unknown when there is no exact verified identity. Add a reported field and reopen the app to inspect persistence. The local profile validator previews schema validity only; it does not install a trusted profile. Native automation evidence and limits are in [Phase 3 verification](PHASE_3_VERIFICATION.md).

## Phase 4 native verification

Close other OBTAINER sessions and build the release first. Use a normal interactive Windows session; restrictive automation tokens can prevent the directory-sharing leases used by the vault.

```powershell
$env:CARGO_INCREMENTAL='0'
npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline
npm.cmd run test:foundation -- --phase3 --phase4
```

`--phase3` requires the previously qualified MG11 USB candidate. Omit that flag when no device is available and report hardware regression as unverified. `--phase4` generates its own clearly named harmless 1 MiB and 256 MiB files in the Windows temporary directory. It never selects real firmware. It drives native Open/Save dialogs, confirms import through React, checks persisted jobs and vault bytes, verifies duplicate imports and integrity, exports a manifest, cancels a real copy, forcibly terminates an owned active import, then verifies recovery and production persistence. A harmless generated artifact and its history remain in the real library; destructive corruption/SQL failure tests use isolated Rust temporary databases and vaults.

The test's temporary WebView2 debugging port is enabled only for its own processes. Native evidence, screenshots and captured failures are saved under `.tools/foundation/`. Do not edit the temporarily instrumented frontend files or run frontend checks concurrently with this suite. See [Phase 4 verification](PHASE_4_VERIFICATION.md), [library](FIRMWARE_LIBRARY.md), [jobs](JOB_ENGINE.md) and [vault](FIRMWARE_VAULT.md).
