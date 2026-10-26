# Windows beta release sprint — executable milestones

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

**E: disappeared during DNS build qualification and was reconnected by the owner on 26 October.** All four archived build files were rehashed after reconnection and matched. The installer, MSI and portable executable were then copied to `.tools/release-artifacts/0.2.0-beta.1/` in the original C: repository, with independent source/destination hash equality; its `artifacts.json` records sizes, hashes and source commit. No normal app data moved. The new `normal-after-restoration.json` audit again matches all 12 files / 5,611,701 bytes. A real acquired input copy also remains on C:.

## Published core checkpoint

Executable milestone **`cc2173e6fb611dfaf0fb87dfc77e54caf4c358d2`**, "Acquire pinned official firmware and persist bounded analysis", is pushed to the public release integration branch. [Windows Actions run 40313207839](https://github.com/okekechuks/Firm_Find/actions/runs/40313207839) completed **successfully**, including both Rust profiles, frontend, signature verification, both native builds and the production executable safeguard. Its actual NSIS/MSI artifact was also downloaded to `.tools/sprint/ci-core-installers/`. CI's installers are a separate build from the local binaries; their hashes are not assumed equal.

The committed computed-evidence runner executed **20 consecutive default-parallel standard suites at `cc2173e`**, each exit 0, **351 passed / 2 ignored plus 2 doctests**. The tracked tree and commit remained unchanged through every run. Evidence: `.tools/sprint/parallel-cc2173e/results.json` and each hashed command log. The first run rebuilt the application crate; no serial test workaround was used. This repeat result applies to that exact core commit, not an untested future commit. Subsequent DNS helper work does not change Rust/frontend runtime source.

## Real DNS build milestone

On 26 October, the complete source helper command below exited **0**. This uses actual signed-release tools and real firmware, not a simulated result:

```powershell
py -B scripts/build-openwrt-dns.py --input .tools/sprint/acquired/openwrt-25.12.6-ath79-generic-tplink_archer-c7-v5-squashfs-sysupgrade.bin --tool-archive E:/OBTAINER-Builds/openwrt-tools-25.12.6/openwrt-imagebuilder-25.12.6-ath79-generic.Linux-x86_64.tar.zst --dns 1.1.1.1 9.9.9.9 --work-root E:/OBTAINER-Builds/dns-image-builds
```

Owned run `d2f32850677a4dc0a16e474edc4c4212`; log `.tools/sprint/dns-live-readback-fixed.log`:

```text
output: openwrt-25.12.6-obtainer-dns-ath79-generic-tplink_archer-c7-v5-squashfs-sysupgrade.bin
bytes: 6947110
SHA-256: 51946949707eca1a71d4bc77511952b45f447bf8918933dfcdd7b0f15883f4b3
input SHA-256 before/after: 0ec9643bb9f43d65c689761d916a9e0c13bad5968656d74e516df010deb0be9d
tool archive SHA-256: 6f40746139c833336e67b03d5bdb0501d3a80fb789bf7e5b7a9b9984b83a5e23
worker SHA-256: 083a9c5936bacd607f5924a10dbdcff96b56fec46e9c42404da111f1fb16e663
DNS defaults bytes: 324
DNS defaults SHA-256: fd3cf8725e5e1d25dea33d8a544a3d9e665328cc6af99f9999ddd14f413d8109
verified SquashFS offset: 2751134
```

The output, report, defaults and logs were retained on E: and copied to `.tools/sprint/custom-dns-build/` on C: with independent hash equality. The successful Linux workspace was removed only after ownership checks; `pgrep -af openwrt-dns-worker` exited 1 (none remaining). `normal-after-dns-build.json` again matches all 12 files / 5,611,701 bytes.

The helper requires exact signature-qualified source and tool hashes. It builds from the trusted ImageBuilder/package set, rather than patching or executing the input. Package signatures remain enabled; all repositories are restricted to the official HTTPS release family. Only the fixed upstream DNS defaults script is accepted. The host's normal Linux user maps to namespace uid 0 for package ownership, with all capabilities dropped before the builder and no host root privilege. A narrowly checked compression resource change retains format options and root ownership, limiting compression to one worker / 256 MiB; the report records makefile hashes before and after. Overall time, memory, file, process and log limits remain enforced.

A separate offline namespace reads only the generated defaults file, with read-only image access, no host drives, a distinct network namespace, one decoder worker, bounded queues/output and a 512 MiB limit. Its exact 324-byte output matched the requested DNS defaults. Analysis/readback execute no image contents; only the trusted tool/package build runs code. Physical device requests: **zero**. The generated image is a **custom build with unknown installability**, not official firmware or physical qualification. GUI Rebuild remains disabled; the working path is the [source helper](DNS_BUILD_BETA.md). Imported output remains an unverified user input in Library. LAN-advertised DNS, Wi-Fi, APN and USSD are unchanged.

### Helper failures and regression evidence

- WSL path interpretation: fixed explicit `--exec` and forward-slash `wslpath` arguments.
- Safe TAR extraction initially rejected publisher-specific absolute links. Only 20 exact pinned SDK name/target pairs are omitted; no absolute link is created. Unknown links and traversal still fail.
- Missing distro `zstd`/`unzip` and private-namespace tool lookup were resolved normally. No `FORCE=1` or signature bypass.
- E: disappeared during an attempt; no output was admitted. Owner reconnected it and inputs/archive hashes were rechecked.
- APK produced 121 ownership errors and exited 2. A failing-first test now verifies scoped uid mapping, zero effective capabilities and host file ownership still belonging to the original non-root user.
- Compression defaulted to 12 workers and failed the enforced memory limit. A failing-first exact-invocation test covers the resource-only patch; memory limits were not raised.
- Decoder thread creation also failed under its cap. A real **synthetic** SquashFS readback test failed first, then passed with one worker/bounded queues. A fresh complete real-image run then passed as recorded above.
- Failing-first path validation rejects UNC, traversal and alternate streams; reparse ancestors and unavailable/remote drives are refused before input resolution/opening.

`npm.cmd run test:dns-build` exited **0**: **13 tests, 4 passed / 9 explicit Linux skips** on Windows. `wsl -d Ubuntu --exec python3 -B /mnt/c/Users/chuka/OneDrive/Documents/Firm_Find/.tools/release-sprint/scripts/test-dns-build.py` exited **0: 13 passed**. These include cancellation/timeout reaping and subsequent requests, archive traversal/absolute links, injection, script contract, real synthetic SquashFS readback, kernel-observed isolation, uid mapping and compression limits. Logs include `dns-ownership-before.log`, `dns-compressor-before.log`, `dns-readback-before.log`, `dns-tests-readback-fixed.log`, `dns-windows-final.log`. Fixtures are synthetic; live build evidence above is separate.

### Source changes

Core `cc2173e`: `library/{admission,analysis,db,files,ipc,mod,official,tests}.rs`, native `lib.rs`, `build.rs`, capability/command permissions; five public `assets/official/` files; frontend `FirmwareAnalysisDialog.tsx`, `OfficialImageDialog.tsx`, `FirmwareLibrary.tsx`, `backend.ts`, `types/{library,official}.ts`, `OfficialFirmware.test.tsx`; `refresh-official-catalogue.py`, `verify-foundation.mjs`, `verify-official-acquisition.mjs`; coordinated version/config, workflow, Git attributes and status/README. Exact 42-file inventory: `git show --name-only cc2173e`.

DNS milestone adds `scripts/build-openwrt-dns.py`, `scripts/openwrt-dns-worker.py`, `scripts/test-dns-build.py`, `docs/DNS_BUILD_BETA.md`, `docs/RELEASE_0_2_0_BETA_1.md`; updates `package.json`, `.github/workflows/windows.yml`, `README.md`, `docs/PHASE_STATUS.md` and this report. No Rust/frontend runtime, storage schema, normal database or device authorization changes occur in this milestone. The handoff/release manifest identifies the final source commit and CI rather than claiming a self-referential commit here.

## Published beta and final delivery

**Release source:** `7433c96b379dd70d1481f06d645ff7f49570d9fe`, "Build qualified OpenWrt DNS images in isolated bounded workers". [Final Actions run 40356416405](https://github.com/okekechuks/Firm_Find/actions/runs/40356416405) completed **successfully**, including Windows verification and the independent Ubuntu 22.04 job's 13 actual helper tests. Standard Rust: 351 passed / 2 ignored; isolated: 352 passed / 3 ignored; two doctests per profile. Frontend: 164 passed. Formatting, both Clippy profiles, lint, TypeScript/build, signed catalogue checks, production build/installers, dedicated native-test build and production hash safeguard all pass. Logs: `.tools/sprint/ci-final.log`, `final-ci-watch.log`. One nonblocking Actions annotation remains: older action majors target Node 20 and the runner forces Node 24. No application console error was observed in native verification.

**[Published unsigned prerelease v0.2.0-beta.1](https://github.com/okekechuks/Firm_Find/releases/tag/v0.2.0-beta.1).** The tag resolves exactly to `7433c96`; no main merge/history rewrite. Its CI-produced files are retained on C: at `C:\Users\chuka\OneDrive\Documents\Firm_Find\.tools\release-artifacts\0.2.0-beta.1-ci\` and uploaded as release assets:

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `obtainer.exe` | 17742848 | `93f5a27fd604724a86b8c95a34e2731ca3c889243eec69cc14530e42a05c1df7` |
| `OBTAINER_0.2.0-beta.1_x64-setup.exe` | 4236621 | `35c9128e530e88d746dd223820bd5dada51bc5f227a47389dd9686128bc0317f` |
| `OBTAINER_0.2.0-beta.1_x64_en-US.msi` | 6045696 | `d43a48325c12f8d3b407cc9477176a64b3c1174181d02343d46b4e45e62d3440` |

`Get-AuthenticodeSignature` returned **NotSigned** for all three. `SHA256SUMS.txt` and `release-manifest.json` accompany them, recording exact source, CI URL and computed lengths/hashes. All five server-side GitHub asset digests/sizes matched the retained files; every asset is uploaded, and the release is public/non-draft/prerelease. An anonymous HTTPS download of `SHA256SUMS.txt` matched local bytes. Source is retained in the public branch and immutable release tag; GitHub supplies source archives. Running the desktop does not depend on an AI session. The separate helper and guide are part of that tagged source, not a hidden temporary tool or an enabled desktop Rebuild action.

After publication, `normal-after-publish.json` still matches **12 files / 5,611,701 bytes**. Original normal data, storage location, original production executable and other worktrees remain preserved. This closing documentation update changes no executable/helper source; it records the already verified immutable release rather than restarting test cycles for documentation alone.

## Outstanding functionality

GUI Rebuild and physical firmware writes remain disabled. Physical Wi-Fi/APN/USSD, MG11 extraction and authenticated physical qualification remain unsupported/unqualified. The generated DNS image's installation/boot behavior has not been tested on hardware. Phase 7.5 is incomplete; no universal support is claimed. [Beta notes](RELEASE_0_2_0_BETA_1.md) distinguish desktop features from the separate helper. The next product gate is explicit physical qualification on supported hardware, not enabling unknown-device extraction.

References: [OpenWrt release signing](https://openwrt.org/docs/guide-user/security/release_signatures), [actual 25.12.6 target](https://downloads.openwrt.org/releases/25.12.6/targets/ath79/generic/), [Tauri WiX version override](https://v2.tauri.app/reference/config/#wixconfig).
