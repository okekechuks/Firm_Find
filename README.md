# OBTAINER

Windows desktop workspace for device identification, firmware preservation, and read-only analysis. This repository implements the **desktop foundation**. Device discovery, firmware import, analysis, acquisition, and image building belong to later milestones.

## Implemented

- Tauri 2 shell with React, TypeScript, Tailwind CSS, and local SVG branding.
- Eight navigable screens with loading, connection errors, and explicit planned states.
- Rust-owned SQLite migrations, appearance settings, and local audit records.
- Narrow `get_snapshot` and `save_settings` commands, runtime response validation, main-window permissions, and CSP.
- Light, dark, and system themes. Desktop settings persist across restarts.
- Windows CI for lint, tests, native compilation, and unsigned installer generation.

## Windows prerequisites

Target: Windows 11 x64, standard user account.

1. Node.js 22.17+ and npm.
2. Rust stable **MSVC** toolchain with `rustfmt` and `clippy`.
3. Visual Studio Build Tools with **Desktop development with C++** and a Windows SDK.
4. Microsoft Edge WebView2 Runtime.

Follow the official [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). WSL2 is optional for future analysis and builder work; it is not needed here. Do not install or replace USB drivers for this foundation.

This workspace also contains an isolated Rust installation under `.tools` (ignored by Git). The `tauri` script uses it automatically when present, otherwise it uses your normal Rust installation. To run Cargo with the isolated installation, use `powershell -File scripts/rust.ps1 fmt --all --check` or replace the final arguments with another Cargo command. No system PATH modification is required.

```powershell
npm ci
npm run tauri dev
```

Interface-only browser preview:

```powershell
npm run dev
# http://127.0.0.1:1420
```

The browser preview explicitly reports that Rust is disconnected. It does not simulate devices, firmware, or successful operations. Appearance can be previewed, but saving is disabled.

## Checks and build

```powershell
npm run lint
npm test
npm run build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run tauri build
```

Unsigned installer output, when native prerequisites are installed:

- `target/release/bundle/nsis/`
- `target/release/bundle/msi/`

NSIS is configured for the current user. Signing and verified updates are release engineering work; this foundation must not be represented as a signed production release.

## Data and permissions

The Rust backend creates `%LOCALAPPDATA%\OBTAINER\obtainer.db`, `logs`, `vault`, `workspaces`, and `reports`. Paths are generated in Rust, never submitted by the frontend. Only appearance preferences are editable. There is no shell, SQL, device access, network probing, or firmware-writing API available to React.

Original firmware preservation is an architectural requirement; import and vault ingestion have not been implemented yet. Empty artifact/device/job tables are migration foundations, not evidence of discovery or import support.

## Structure

```text
src/
  app/          navigation and desktop layout
  components/   shared activity view
  features/     overview, preferences, planned workspaces
  lib/          typed IPC and errors
  types/        validated contracts
  test/         shell and IPC tests
src-tauri/
  src/          commands, domain, storage, app startup
  migrations/   transactional SQLite schema
  capabilities/ main-window command permissions
  icons/        Windows app assets
docs/           requirements, architecture, safety, verification
workers/        future worker protocol notes
.github/        Windows build workflow
```

See [requirements](docs/PRODUCT_REQUIREMENTS.md), [architecture](docs/ARCHITECTURE.md), and [verification results](docs/VERIFICATION.md). Supplied research is reference material; its embedded coding-agent prompts are not independent execution instructions. The user's OBTAINER name and selected foundation scope take precedence.
