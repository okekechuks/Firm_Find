# Beta 2 firmware extraction verification

Date: 27 October 2026. Branch: `feature/firmware-extraction`, isolated from accepted Beta 1 source `7433c96b379dd70d1481f06d645ff7f49570d9fe`. Beta 1 tag and the owner's main worktree are preserved.

## Qualification status at initial source submission

Implementation is ready for native qualification. **Beta 2 publication is held until the corrected native end-to-end test and Windows CI pass.** An initial desktop attempt reached the real download and analysis stages but failed at the new extraction IPC permission boundary. Its failure is retained; it is not counted as successful extraction through OBTAINER.

Standalone isolated-worker qualification used the actual official Archer C7 v5 image, not a synthetic fixture: 7,405,862 bytes, SHA-256 `0ec9643bb9f43d65c689761d916a9e0c13bad5968656d74e516df010deb0be9d`. The worker discovered SquashFS at offset 2,751,134, declared length 4,629,526, XZ compression. Independent validation checked all 869 ordinary file hashes and 13,411,262 bytes, with 128 directories, 214 inert symlink records and one omitted special file. `etc/openwrt_release` is 214 bytes, SHA-256 `44da8c321b99cd986a02296b017ecdc57e96c72165bdf7144aa286900b10d97d`, and contains the actual OpenWrt 25.12.6 release text. This standalone result does not substitute for the pending native acceptance run.

## Implementation

- `scripts/firmware-extract-worker.py`: version/hash-pinned, namespace-isolated SquashFS extraction and one bounded gzip-to-SquashFS route; manifest and ordinary-byte stream output. No firmware execution.
- `apps/desktop/src-tauri/src/library/extraction.rs`: immutable source proof, persistent jobs, scoped process supervision, cancellation/heartbeat, independent output validation, content-addressed publication, persisted browse/preview and explicit selected-file TAR export.
- `library/mod.rs`, `library/ipc.rs`, `lib.rs`, `build.rs`, `capabilities/main.json` and four generated command permissions: job and actual Tauri IPC registration.
- `library/desktop.rs`: relocation now retains derived blobs and reports, not just firmware records. `library/files.rs`: native selected-file export dialog.
- `FirmwareExtraction.tsx`, `FirmwareAnalysisDialog.tsx`, `types/extraction.ts`, `lib/backend.ts`, `styles.css`: existing Library popup, progress/cancellation, folder browsing, escaped text previews and owner-selected export. Rebuild remains disabled.
- Rust library/extraction tests, `FirmwareExtraction.test.tsx`, updated `OfficialFirmware.test.tsx`, `scripts/test-firmware-extraction.py`, `scripts/verify-extraction.mjs`, and existing native harness integration cover the new boundary.
- Version manifests identify the Beta 2 candidate. Windows CI includes Python extraction regressions. No new schema, sidebar page, physical adapter, credentials or device-write path.

Storage uses schema 6 without migration: binary bundle in `artifact_blobs`; JSON manifest in a `metadata_report` occurrence linked to the parent source and job. Neither is admitted as firmware. Publication requires the current running, uncancelled extraction job with the same source hash; SQLite transaction rollback leaves no successful relationship. Unexpected staging leftovers are retained for review. Directory leases remain held during known-file cleanup.

## Checks completed before native qualification

Commands run from this worktree; Rust commands use `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1` as the repository's Cargo wrapper. `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0` reduce disposable build storage.

| Command | Actual result |
|---|---|
| `npm.cmd run lint` | Exit 0 |
| `npm.cmd test` | Exit 0; 167 tests, 20 files |
| `npm.cmd run build` | Exit 0; TypeScript and Vite production build |
| Rust wrapper `fmt --all --check` | Exit 0 |
| Rust wrapper `test --workspace --locked --offline` | 366 passed, 2 intentionally ignored; two doctests passed |
| `npm.cmd run test:extraction-worker` | Exit 0; 13 Windows-applicable tests passed, 3 Linux-only skips |
| `wsl.exe -d Ubuntu --exec python3 -I -B /mnt/c/Users/chuka/OneDrive/Documents/Firm_Find/.tools/extraction-engine/scripts/test-firmware-extraction.py` | Exit 0; all 16 passed, including real Linux namespace and synthetic SquashFS fixture |
| `npm.cmd run test:catalogue` | Exit 0; pinned OpenWrt signer and exact three-entry catalogue verified |
| `npm.cmd run test:dns-build` | Exit 0; four Windows-applicable checks, nine explicit Linux skips |
| WSL Python `scripts/test-dns-build.py` | Exit 0; all 13 passed |
| `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/test-production-executable-guard.ps1` | Exit 0 |
| `py scripts/audit-data-bytes.py .tools/normal-pre-native.json .tools/normal-before.json` | Exit 0; all 12 normal files, 5,611,701 bytes unchanged |

The final isolated Rust profile, both final Clippy profiles, final native builds and end-to-end run are being verified. Earlier successful builds do not establish that the final source has passed.

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

Normal data was audited as bytes without opening the normal database. The owner's pre-existing `apps/desktop/src-tauri/src/adapters/mod.rs` modification on main remains untouched. Generated captures and full local logs remain outside Git; final native evidence, manifest hash, CI URL and release links will be added after actual qualification.
