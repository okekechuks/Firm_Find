# OBTAINER foundation verification — 14 October 2026

User story: open OBTAINER, navigate eight workspaces, read Rust-owned SQLite state, change appearance, and preserve preferences across desktop restarts.

## Evidence

| Check | Result | Evidence |
| --- | --- | --- |
| Frontend production build | Passed | TypeScript check and Vite 6.4.4; 1,598 modules; generated `dist/` |
| Frontend lint | Passed | `npm run lint`, zero warnings |
| Shell / IPC tests | Passed | 2 files, 7 tests, Vitest 4.1.11 |
| Development server | Passed | Vite listens on `http://127.0.0.1:1420`; HTML and transformed entry module return HTTP 200 |
| Dependency audit | Passed | npm install reports zero vulnerabilities after patched Vitest upgrade |
| Rust format | Passed | `cargo fmt --all --check` using isolated MSVC toolchain |
| Rust dependency resolution | Passed | Cargo.lock generated; HTTP multiplexing disabled for local network retry |
| Native unit tests | Blocked | `cargo test --workspace --locked` reaches compilation, then `error: linker link.exe not found` |
| Native installer build | Blocked | `npm run tauri build` completes frontend build, then the same missing MSVC linker error |
| Native clippy | Pending | Requires the same missing compiler/linker prerequisite |
| Actual desktop IPC / persistence | Pending | Desktop executable could not be built or launched; frontend tests use explicit native mocks |
| Browser visual verification | Unavailable | Browser tool fails before connection with `missing field sandboxPolicy`; no screenshot or visual pass claimed |

No Windows executable or installer has been produced. Planned output locations remain `target/release/bundle/nsis/` and `target/release/bundle/msi/`.

## Prerequisites detected

- Windows x64 host.
- Node 22.17.0, npm 10.9.2, Git available.
- WebView2 155.0.4283.45 detected by `tauri info`.
- Rust/Cargo 1.99.0 and rustup 1.29.1 installed under the workspace's ignored `.tools` directory; no global PATH change.
- Tauri Rust/API 2.12.3 and CLI 2.12.2 resolved in lockfiles.
- Visual Studio C++ tools / Windows SDK components not detected by Tauri or `vswhere`; MSVC linker absent.

## Corrections during verification

npm 10 crashed while resolving the patched test dependencies. A temporary npm 11 invocation rebuilt the generated dependency tree and produced a working package-lock. No global npm upgrade occurred. Vitest includes only `src/test/**/*.test.{ts,tsx}` and Tailwind source scanning is limited to `src`, preventing ignored local tool caches from entering checks. The Settings decoration is hidden from assistive technology so its link has a clear accessible name.

## Native acceptance still required

Install Visual Studio Build Tools with Desktop development with C++ and a Windows SDK, then run:

```powershell
powershell -File scripts/rust.ps1 clippy --workspace --all-targets --locked -- -D warnings
powershell -File scripts/rust.ps1 test --workspace --locked
npm run tauri dev
# Save a theme, close, reopen, and verify persistence plus audit history.
npm run tauri build
```

Resolve actual compiler or runtime failures before marking the foundation accepted. Test unsigned current-user installation separately. This is source implementation with partial verification, not a validated desktop release.

## Next milestone after foundation acceptance

Windows device discovery using SetupAPI / Configuration Manager, serial enumeration, IP Helper network inventory, interface correlation, and arrival/removal fixture tests. There are currently **no real or simulated device discovery providers**, firmware imports, backups or flashing operations.
