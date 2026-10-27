# Beta 2 firmware extraction verification

Date: 27 October 2026. Branch: `feature/firmware-extraction`, isolated from accepted Beta 1 source `7433c96b379dd70d1481f06d645ff7f49570d9fe`. Beta 1 tag and the owner's main worktree are preserved.

## Qualification result

**X1 real SquashFS extraction: PASS. X2 Library/persistence/cancellation/export: PASS. X3 additional gzip route: PASS. X4 physical acquisition: BLOCKED with evidence. X5 software release gates: PASS.** The actual Windows application has now produced and retained genuine extracted firmware contents. An initial desktop attempt failed at the new IPC permission boundary; that failure is retained below and does not substitute for the successful corrected run.

Implementation/release source: `e4524e80ac38ff0702e0917fc9a8f314d2f1ad82`. [Windows CI 40475441212](https://github.com/okekechuks/Firm_Find/actions/runs/40475441212) **passed**, including both Rust profiles, both native builds, the production-executable guard and Linux worker regressions. CI artifact `12720948242` belongs to that exact source; ZIP digest `cd156ca95d054ac5c4c0e5eb0085d2f4656e467a027c3c5fe9013188ff328bce`. Subsequent evidence/documentation commits do not change the qualified implementation.

Standalone isolated-worker qualification used the actual official Archer C7 v5 image, not a synthetic fixture: 7,405,862 bytes, SHA-256 `0ec9643bb9f43d65c689761d916a9e0c13bad5968656d74e516df010deb0be9d`. The worker discovered SquashFS at offset 2,751,134, declared length 4,629,526, XZ compression. Independent validation checked all 869 ordinary file hashes and 13,411,262 bytes, with 128 directories, 214 inert symlink records and one omitted special file. `etc/openwrt_release` is 214 bytes, SHA-256 `44da8c321b99cd986a02296b017ecdc57e96c72165bdf7144aa286900b10d97d`, and contains the actual OpenWrt 25.12.6 release text. The separate actual native acceptance run is recorded below.

## Published release

[v0.2.0-beta.2](https://github.com/okekechuks/Firm_Find/releases/tag/v0.2.0-beta.2) was published on 27 October 2026 after native verification and green Windows CI. Release source/tag is `e4524e80ac38ff0702e0917fc9a8f314d2f1ad82`; [the exact source-file diff](https://github.com/okekechuks/Firm_Find/commit/e4524e80ac38ff0702e0917fc9a8f314d2f1ad82) contains all 35 changed files. Evidence documentation follows in a separate commit. No merge or tag rewrite was performed.

| CI-built release asset | Bytes | SHA-256 |
|---|---:|---|
| `obtainer.exe` | 18,074,624 | `c5dbee6b4b23c0299239a1f6995015e75692f10554db8c24967a6d49edd4f876` |
| `OBTAINER_0.2.0-beta.2_x64-setup.exe` | 4,291,619 | `d9c3d0519904e915107637e2415a1b1a53564a0d09d13c94fc2c7cc6304b8d25` |
| `OBTAINER_0.2.0-beta.2_x64_en-US.msi` | 6,131,712 | `02a8c854d6ff6d93e36def7ff9c5f696315f43e1a919e5b45d343846d305b494` |

The actual 195,533-byte JSON extraction manifest and `SHA256SUMS.txt` are attached too. All five remote asset sizes and GitHub-provided SHA-256 digests matched the local files before publication. Installers/executable are unsigned. An initial combined upload was interrupted before publication; the draft was recreated, files uploaded individually and verified, then published. Beta 1 remains at `7433c96b379dd70d1481f06d645ff7f49570d9fe` as a fallback.

## Implementation

- `scripts/firmware-extract-worker.py`: version/hash-pinned, namespace-isolated SquashFS extraction and one bounded gzip-to-SquashFS route; manifest and ordinary-byte stream output. No firmware execution.
- `apps/desktop/src-tauri/src/library/extraction.rs`: immutable source proof, persistent jobs, scoped process supervision, cancellation/heartbeat, independent output validation, content-addressed publication, persisted browse/preview and explicit selected-file TAR export.
- `library/mod.rs`, `library/ipc.rs`, `lib.rs`, `build.rs`, `capabilities/main.json` and four generated command permissions: job and actual Tauri IPC registration.
- `library/desktop.rs`: relocation now retains derived blobs and reports, not just firmware records. `library/files.rs`: native selected-file export dialog.
- `FirmwareExtraction.tsx`, `FirmwareAnalysisDialog.tsx`, `types/extraction.ts`, `lib/backend.ts`, `styles.css`: existing Library popup, progress/cancellation, folder browsing, escaped text previews and owner-selected export. Rebuild remains disabled.
- Rust library/extraction tests, `FirmwareExtraction.test.tsx`, updated `OfficialFirmware.test.tsx`, `scripts/test-firmware-extraction.py`, `scripts/verify-extraction.mjs`, and existing native harness integration cover the new boundary.
- Version manifests identify the Beta 2 candidate. Windows CI includes Python extraction regressions. No new schema, sidebar page, physical adapter, credentials or device-write path.

Storage uses schema 6 without migration: binary bundle in `artifact_blobs`; JSON manifest in a `metadata_report` occurrence linked to the parent source and job. Neither is admitted as firmware. Publication requires the current running, uncancelled extraction job with the same source hash; SQLite transaction rollback leaves no successful relationship. Unexpected staging leftovers are retained for review. Directory leases remain held during known-file cleanup.

## Commands and results

Commands run from this worktree; Rust commands use `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1` as the repository's Cargo wrapper. `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0` reduce disposable build storage.

| Command | Actual result |
|---|---|
| `npm.cmd run lint` | Exit 0 |
| `npm.cmd test` | Exit 0; 167 tests, 20 files |
| `npm.cmd run build` | Exit 0; TypeScript and Vite production build |
| Rust wrapper `fmt --all --check` | Exit 0 |
| Rust wrapper `test --workspace --locked --offline` | 366 passed, 2 intentionally ignored; two doctests passed |
| Rust wrapper `test --workspace --features native-test-profile --locked --offline` | Exit 0; 367 passed, 3 intentionally ignored; two doctests passed |
| Rust wrapper `clippy --workspace --all-targets --locked --offline -- -D warnings` | Exit 0 |
| Rust wrapper `clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | Exit 0 |
| `npm.cmd run test:extraction-worker` | Exit 0; 13 Windows-applicable tests passed, 3 Linux-only skips |
| `wsl.exe -d Ubuntu --exec python3 -I -B /mnt/c/Users/chuka/OneDrive/Documents/Firm_Find/.tools/extraction-engine/scripts/test-firmware-extraction.py` | Exit 0; all 16 passed, including real Linux namespace and synthetic SquashFS fixture |
| `npm.cmd run test:catalogue` | Exit 0; pinned OpenWrt signer and exact three-entry catalogue verified |
| `npm.cmd run test:dns-build` | Exit 0; four Windows-applicable checks, nine explicit Linux skips |
| WSL Python `scripts/test-dns-build.py` | Exit 0; all 13 passed |
| `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/test-production-executable-guard.ps1` | Exit 0 |
| `py scripts/audit-data-bytes.py .tools/normal-pre-native.json .tools/normal-before.json` | Exit 0; all 12 normal files, 5,611,701 bytes unchanged |
| `npm.cmd run tauri -- build --ci -- --locked --offline` | Exit 0; production EXE, NSIS and MSI |
| `npm.cmd run build:native-tests` | Exit 0; separate isolated EXE; production hash guard passed |
| `npm.cmd run test:extraction-native` | Exit 0; actual native GUI, extraction, cancel/retry, restart, preview, export and gzip route |

The final source passed both Rust profiles, both Clippy profiles and both native builds. Existing ignored tests are explicit hardware/opt-in tests, not silently counted as passing. The local production EXE SHA-256 remained `f1b9e410cf9978b334f40900e90ade006c2b351a67c9e7bde117566b2090ecc0` during isolated-binary compilation. The owner's original main-worktree EXE independently remains `0278ddb4776a03458a4c24c0c219a7b0c32f74fae264d6d549883c94f4780c65`.

## Actual Windows acceptance run

Local evidence directory: `.tools/foundation/2026-10-27T08-50-36.549Z`; isolated run ID `daa67dee4bcb51d342c48bb4ebafec39`. The dedicated native executable uses an owned temporary database/vault/WebView profile. It runs the same production Library/worker/IPC code; no test-only extractor or generated firmware substitutes for the official image.

The native window and six React pages rendered, SQLite IPC succeeded, discovery refreshed three times with unique IDs and unknown feature support, and settings persisted. The actual official download was independently hashed and classified with retained source provenance. GUI Cancel stopped the first extraction without publishing a report; a second GUI request succeeded. The real tree and text preview rendered. After closing the native window and reopening the same isolated profile, the report, folders, file bytes and official provenance remained accessible. Explicit selection of `etc/openwrt_release`, the review checkbox and the native Save dialog produced a TAR. The additional route imported a locally generated gzip wrapper of the real image through the native picker and extracted the same files. No console warnings or unhandled exceptions were captured. Shutdown left no owned app/WebView/Vite processes, and guarded cleanup exited 0.

| Actual measurement | Result |
|---|---|
| Input image | 7,405,862 bytes; SHA-256 `0ec9643bb9f43d65c689761d916a9e0c13bad5968656d74e516df010deb0be9d` |
| SquashFS | Discovered offset 2,751,134; length 4,629,526; XZ |
| Ordinary files / directories | 869 / 128 |
| Ordinary bytes | 13,411,262 |
| Inert omissions | 214 symlinks, one special file; no links or device nodes exported |
| Unextracted regions | Prefix 2,751,134 bytes and suffix 25,202 bytes; status `partially_extracted` |
| Bundle SHA-256 | `6acba25084a32c35ae951b125923c9ea0e4f676de0594260816c20cceae76373` |
| Persisted JSON manifest SHA-256 | `54dbea906bf8055d26eccd86bb511c598f5fac0e44b6805051a7d4e70684ac2f` |
| Selected-file TAR | 2,048 bytes; SHA-256 `90a1ea762d7030b3b173d3b599f949b495348b89f26945361ea665daace5ef80` |
| Gzip wrapper | Local wrapper, 7,383,043 bytes; SHA-256 `7b9a3f5e89352ac509b0b0ef459965d5b809ac5d83db3441bba6c057e5c1bf25` |
| Gzip-route manifest SHA-256 | `a5ed98bd437bd74f2c87d3e52f74f6751fae9a7c6b46fea39f3f58b34ffa3000` |

Representative real files: `etc/openwrt_release`, `bin/busybox`, `sbin/procd`, `usr/lib/os-release`. The harness independently verified **every** ordinary entry against retained bundle bytes, not just the manifest's counts. A separate Python `hashlib`/`tarfile` check then revalidated every file hash, parsed the exported archive, confirmed its sole member is the selected regular file with mode 0600, and verified the release text/hash. Output is `.tools/extraction-independent-python.json`. The gzip result matches every raw-route path, kind, size and ordinary-file hash. Original source hash stayed unchanged.

Native screenshots `extraction-real-tree.png`, `extraction-reopened.png` and `gzip-real-firmware-tree.png` are local evidence; the rendered extraction tree was visually inspected. Preview text was also asserted through the real native WebView. These are not browser-only mocks or terminal-startup claims. The native before/after normal-data audits remained identical across all 12 files and 5,611,701 bytes, including SQLite/WAL/SHM when present, settings, firmware vault and saved-device state.

Negative coverage includes invalid offsets/signatures/superblocks/size/tables, truncated/ambiguous/opaque input, unsupported compression, gzip expansion limits, excessive count/bytes, unsafe names and aliases, path traversal, inert escaping symlinks, special files, changed/missing tools, startup/disk-full errors, crash, cancellation, lost owner/timeout and child reaping, empty or forged output, source-hash mismatch, duplicate jobs, interrupted-job recovery, cancelled/invalid publication, transaction rollback and failed/unknown-entry cleanup. Synthetic failures are labelled as such; no physical-device failure is inferred from them.

## Initial failures and corrections

1. Bubblewrap required explicit `--unshare-user` with `--disable-userns`; added it.
2. Strict unsquashfs could not create `dev/console` without privileges. Added sandboxed prelisting and explicit link/special exclusions; did not grant privileges. One diagnostic listing during diagnosis was outside the sandbox; it performed no extraction or execution and is not isolation evidence. Actual extraction is sandboxed.
3. Windows Python could not import Linux `resource`; moved that import inside the Linux inner worker.
4. The fixture test assumed `unsquashfs -version` exits zero. Corrected output/version inspection; it returns exit 1 on this tool.
5. Frontend regression still asserted that extraction did not exist; updated it and its IPC mock. Corrected one React cleanup lint warning.
6. First native run `.tools/foundation/2026-10-26T22-13-06.587Z` downloaded and analyzed real firmware, but `start_firmware_extraction` was rejected by Tauri ACL. Added command declarations and capability permissions. The test ultimately failed with `Extraction job missing`.
7. A relocation edit briefly used undefined `size` in an unrelated backup hash check; restored its existing limit and added derived-blob relocation regression.
8. An interrupted build never produced the updated isolated binary. A subsequent direct PowerShell wrapper invocation treated Cargo's progress stderr as a terminating error under inherited `Stop`; reran through the repository wrapper in a separate PowerShell process. Neither attempt counts as a successful build/test.

## Limits and separate physical gate

See [worker setup, isolation, formats and storage design](FIRMWARE_EXTRACTION.md). Only qualified WSL Ubuntu tools are accepted; they are not bundled or automatically installed. Unknown/encrypted and other unsupported formats remain unextracted. The gzip route is a locally produced wrapper around real OpenWrt bytes, not an OEM parser or another official source.

MG11 installed-firmware acquisition remains blocked: no reviewed firmware-export endpoint or partition-read interface has produced bytes. Existing physical acquisition strategies remain unavailable; authentication-challenge qualification remains pending. No credentials, login, reset, physical configuration change or flashing is performed. Phase 7.5 and universal physical acquisition are not complete.

Normal data was audited as bytes without opening the normal database. The owner's pre-existing `apps/desktop/src-tauri/src/adapters/mod.rs` modification on main remains untouched. Generated captures and full local logs remain outside Git. No merge into main or change to the Beta 1 tag occurred.
