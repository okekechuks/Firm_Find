# Windows beta release sprint — executable milestone 1

26 October 2026. Integration branch `release/windows-beta-2026-10-28` starts at `5c303437d6538755c9013ef123277a55807de90f`, retaining Stage 3 and independently accepted Stage 4 `f9e7115` and all original branches. App version `0.2.0-beta.1`; MSI version mapping `0.2.0.1`. No main merge or normal-data migration was performed. The owner's emergency sprint releases official-image acquisition, bounded analysis and supported build work; it does not establish physical device support.

## Working milestone

- **Real official-image acquisition:** Library → Official Firmware → exact model/revision confirmation → bounded HTTPS download → full SHA-256 verification → immutable vault → typed official occurrence/admission → Open in Library. Three source entries: OpenWrt 25.12.6 `ath79/generic`, TP-Link Archer C7 v5/v2 and Ubiquiti UniFi AC Lite board profile. These are catalogue targets, not verified identities of connected hardware.
- **Source authenticity:** catalogue generated from actual OpenWrt `profiles.json` whose SHA-256 is in the signed release manifest. GPG verified signature subkey `92C561DE55AE6552F3C736B82B0151090606D1D9`, primary `8A8BC12F46B836C0F9CDB36F1D53D1877742E911`, against pinned fingerprints. Runtime uses compiled exact release hashes; it does not run GPG on each download. CI independently checks the retained public signature, key, manifest and profile data. Git attributes preserve their exact bytes on Windows.
- **No arbitrary source:** frontend supplies a catalogue ID and explicit confirmation, never a URL, hash, endpoint or producer class. Production transport is fixed to HTTPS `downloads.openwrt.org:443`; credentials, cookies, auth negotiation and redirects are disabled. Exact byte cap, three-minute overall budget, per-call timeouts, cancellation and interrupted-job recovery apply. Publication and verified-source admission share the existing immutable-vault and transaction safeguards. Equal bytes do not reclassify a prior admission.
- **Real bounded analysis:** full input hash checked through its read lease, then at most 32 MiB scanned for at most 64 format signatures; entropy samples at most 256 KiB; input cap 512 MiB. No recursive extraction, external decoder or image execution. A genuine JSON report is stored as a separate `metadata_report` blob/occurrence and remains outside firmware admissions. Reports survive restart.
- **Existing desktop retained:** six destinations, real React/Rust/SQLite IPC, discovery/refresh, settings, immutable inputs and existing backup/import behavior. New source and analysis views are popups; firmware cards retain their four fields. Exported manifest 1 gains a bounded `officialSources` extension with exact source/provenance facts; its unsigned/unknown-installability semantics remain explicit.

## Actual firmware and native GUI evidence

`npm.cmd run test:official-acquisition`, exit **0**, used dedicated optimized `obtainer-native-test.exe`, profile `6c506fd806cb328daa4997fc987504c5`. Evidence: `.tools/foundation/2026-10-26T10-44-50.262Z/` and `.tools/sprint/native-official-first.log`. This mode never launches the ordinary production executable against normal data.

The actual GUI selected `tplink_archer-c7-v5` and acquired:

```text
https://downloads.openwrt.org/releases/25.12.6/targets/ath79/generic/openwrt-25.12.6-ath79-generic-tplink_archer-c7-v5-squashfs-sysupgrade.bin
bytes: 7405862
SHA-256: 0ec9643bb9f43d65c689761d916a9e0c13bad5968656d74e516df010deb0be9d
```

Independent ordinary-file hash/length calculation matched the catalogue and stored artifact. The app recorded `official_image_download` and exact provenance. The GUI's Open in Library and Analyze Firmware actions succeeded. Report: U-Boot legacy at offset 0 and SquashFS observation at offset 2,751,134; all 7,405,862 bytes fit within the scan cap; entropy 7.99933090271732 over 262,144 sampled bytes. This identifies byte structure, not validated partitions or hardware support.

Restart retained the artifact, provenance, report, directory and settings. Six-page navigation and three local Windows discovery refreshes passed before and after restart; device feature support remained UNKNOWN. Captured console warnings/exceptions: none. Actual native screenshots were inspected and remain ignored local evidence. They are not committed. No MG11 login, challenge qualification or hardware acquisition is claimed.

A separate copy of the actual public image remains under `.tools/sprint/acquired/` on C:; no synthetic test payload is presented as firmware. The isolated profile was safely cleaned after all processes stopped. Normal Library was not populated or changed by this test.

## Commands, results and initial corrections

Rust commands use existing Cargo/Rustup tools and separate E: build output, one build job, `CARGO_INCREMENTAL=0`; local dev/test debug info is disabled. Runtime tests retain default parallel scheduling. Compiler temporary files were also moved to the owned E: build directory; normal app storage was not moved.

| Command | Result / local log |
| --- | --- |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | 0; **351 tests + 2 doctests**, 2 ignored; `.tools/sprint/rust-boxed-standard.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | 0; **352 tests + 2 doctests**, 3 ignored; `rust-boxed-isolated.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | 0; `clippy-standard-final.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0; `clippy-isolated-final.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all -- --check` | 0; final source format check |
| `npm.cmd test` | 0; **164 tests / 19 files**; `frontend-first.log` |
| `npm.cmd run lint` | 0; `lint-final.log` |
| `npm.cmd run build` | 0; TypeScript/Vite; `frontend-build-first.log`, repeated by native packaging |
| `npm.cmd run test:catalogue` | 0; full signature/pinned-source check; `catalogue-check.log` |
| `npm.cmd run tauri -- build --ci -- --locked --offline` | 0; NSIS **4,226,494 bytes**, MSI **6,037,504 bytes**; `production-build-final.log` |
| `npm.cmd run build:native-tests` | 0; optimized native-test build and production-executable hash guard; `native-build.log` |
| `npm.cmd run test:official-acquisition` | 0; actual public firmware/GUI/IPC/Library/analysis/restart; `native-official-first.log` |
| `py scripts/audit-data-bytes.py .tools/sprint/normal-after-core.json .tools/sprint/normal-before.json` | 0; 12 files / 5,611,701 bytes identical, no SQLite connection |

Initial genuine failures were corrected: Git's MSYS GPG required POSIX drive paths for its temporary home; a loopback fixture inherited nonblocking mode (one standard suite failed with WSAEWOULDBLOCK 10035, fixed by setting the accepted socket blocking); Clippy rejected a 440-byte queue variant (source payload now boxed); MSI rejected `beta.1` as an installer prerelease number (explicit WiX numeric version mapping fixes this without renaming the product or storage). Their initial logs are retained. No warning suppression or serial test workaround was used.

New Rust regressions cover catalogue IDs/confirmation, wrong revisions/URLs/labels, size/hash mismatch, cancelled-before-network, redirect/auth status refusal, truncated/oversized responses, active cancellation and fresh subsequent request, deadline expiry, report/firmware class separation, report restart persistence, input preservation, denied source promotion and interrupted download/analysis recovery. Frontend tests cover approval, error/success navigation and rejection of false execution/extraction/installability claims.

## Data and storage

Normal `%LOCALAPPDATA%\OBTAINER` was audited as ordinary file bytes only: database/WAL/SHM, seven logs, vault lock and original 1 MiB firmware. All **12 files / 5,611,701 bytes** remain identical after native verification and after the build-drive loss. No normal SQLite connection, checkpoint, migration, storage switch or normal production launch occurred. This is byte-preservation evidence, not a fresh logical integrity claim. Original C: production executable remains SHA-256 `0278DDB4776A03458A4C24C0C219A7B0C32F74FAE264D6D549883C94F4780C65`.

Four generated Stage 3 debug library/symbol files were moved to an owned E: archive with before/after SHA-256 verification, not deleted. Exact computed inventory: `.tools/sprint/generated-symbols-archive.json`. Existing release dependency cache was copied to E: without copying production/test executables or old installer bundles. Source/history, original production executable and user data were not replaced. Compiler output/temp moved to E: to retain 15 GiB on their actual output volume; C: OS headroom was separately checked.

**E: subsequently disappeared during DNS build qualification.** Its installers, cache, archived symbols and tools are temporarily inaccessible; they are not claimed as currently accessible retained artifacts. Source, verification evidence, real acquired image copy and normal data remain on C:. Owner was asked to reconnect E:. The next independent checkpoint is remote CI artifact retention plus resumed build qualification. Do not treat a local build log as proof of artifact availability after drive loss.

## Remaining release gates

Push this executable milestone and run Windows CI to completion; record its actual URL/outcome in the handoff. A prior green Stage 4 run is not this branch's result. Retain installable artifacts somewhere accessible after the AI subscription ends. Supported DNS customization/build qualification continues separately; no custom image has yet passed. GUI Rebuild and all physical firmware writes remain disabled. Wi-Fi/APN/USSD physical support and MG11 firmware extraction remain unsupported/unqualified. Phase 7.5 is not complete, and no universal-router support is claimed.

References: [OpenWrt release signing](https://openwrt.org/docs/guide-user/security/release_signatures), [actual 25.12.6 target](https://downloads.openwrt.org/releases/25.12.6/targets/ath79/generic/), [Tauri WiX version override](https://v2.tauri.app/reference/config/#wixconfig).
