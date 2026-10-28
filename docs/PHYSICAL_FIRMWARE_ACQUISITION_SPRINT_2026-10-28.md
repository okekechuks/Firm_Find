# Physical firmware acquisition sprint — October 28, 2026

## Outcome and source

**Hardware acquisition remains BLOCKED. No physical firmware bytes were acquired.** Public D-Link packages were downloaded for investigation; neither has been matched to a physical target. A working Library import, analyzer or extractor does not change these counters.

| Required metric | Original H01–H05 groups | Individual units (H01-A, H01-B, H02–H05) |
| --- | --- | --- |
| A: installed firmware physically acquired AND extracted | **0/5** | **0/6** |
| B: independently verified compatible manufacturer image acquired AND extracted | **0/5** | **0/6** |
| Metadata only / blocked | **5/5** | **6/6** |

H01-A and H01-B are separate units within **one** original target. There is no firmware artifact, file manifest, partition map, installed-image hash or extraction-completeness claim for either MG11.

Working branch: `sprint/physical-acquisition-2026-10-28`, in the existing isolated `.tools/beta2-hardening` worktree. Parent: `942195a5c7035f429ddf2bba4f0b30ec4c0f7cfa` (`feature/universal-firmware-workflow`). At start, `git ls-remote origin refs/heads/main refs/heads/feature/universal-firmware-workflow` returned main `d5d05413d30c01f189968526be2ac80979387bd5` and feature `942195a5c7035f429ddf2bba4f0b30ec4c0f7cfa`; the tracked trees were clean. Main, published tags and existing release assets are preserved. No merge or release is authorized by this report. Delivery code is `39898fc8e7926e78ee5cfbda8abff56690a10487` plus follow-up `fba2372baa58ed21ebcbb55cc573754bff98292a`; completed local verification is below.

## Actual physical observations and owner participation

The owner connected the Vitel MG11 and explained that PC Wi-Fi must stay on because this MiFi has no internet access. No programmatic Wi-Fi disconnect, route/metric edit, adapter reconfiguration or driver installation was performed.

`scripts/inventory-physical-device.ps1` performs **local Windows enumeration only**: CIM PnP/network records, PnP parent properties, IP addresses, DNS and route configuration. It does not open the normal application database or create a socket. Raw PnP IDs, GUIDs and parent identifiers are replaced with per-run salted SHA-256 references; salt is discarded. Arbitrary friendly names are omitted rather than relying on incomplete secret-pattern redaction. JSON output is restricted to ordinary directories under `.tools/`, protected against parent replacement, and created without overwrite.

Initial presence/privacy-corrected capture: `.tools/physical-sprint-inventory-final.json`, SHA-256 `7f7c166978022275053edc31aa514cb33cd8f7d5dcff4257d621b3e57ca02173`:

- Five present USB records share VID `1286` / PID `4E31` and observed parent relationships. These IDs do not establish model, board, chipset or firmware.
- One present, active correlated RNDIS network adapter: Windows interface 12, `192.168.100.100/24`, gateway `192.168.100.1`.
- PC Wi-Fi interface 19 is also active at `192.168.100.199/24`, reporting the same gateway. Both interface routes have route metric 256; Windows interface metrics were 55 for USB and 45 for Wi-Fi. This is a genuine overlap, not proof that both gateways are the same physical router.
- Composite/RNDIS records report problem code 0; two diagnostic interfaces and one AT interface report problem code 28. No diagnostic/AT commands, engineering mode, privileged storage reads or driver changes were attempted.
- Configured Windows DNS values are PC adapter observations, not independently read router settings. They are not fabricated device DNS or identity evidence.

Follow-up interface-class capture: `.tools/physical-sprint-inventory-interfaces.json`, SHA-256 `d24c242fb95b4108b88932567dcbb451860cc28b23755d3cc5db586817cca8e4`. Five records remain present, with one active USB-correlated network and one overlapping subnet. Strictly parsed Windows compatible IDs contain three full `FF/00/00` vendor-specific interface tuples and one `E0/01/03` RNDIS tuple. No DFU runtime (`FE/01/01`) or DFU-mode (`FE/01/02`) tuple was observed. These are Windows property observations, not raw descriptor interrogation. Vendor-specific interfaces do not identify a protocol or prove firmware-read access. No observed DFU advertisement does not rule out another, currently unexposed service mode. Upload support, authorized address ranges and firmware coverage remain unknown.

The current HTTP implementation checks routes but does **not** own an interface-pinned socket. Its subnet guard therefore remains enabled. A numeric destination pin cannot resolve this interface ambiguity. No router HTTP request, root/version/menu/challenge approval, login or credential operation was dispatched in this sprint. The physical authentication-challenge gate remains **PENDING**; the earlier root-approval timeout remains a failed/incomplete attempt, not a successful challenge.

The owner manually inspected **H01-A** Configuration Management. Exact owner statements on October 28, 2026, Africa/Lagos:

> No Export Configuration or Backup button is visible.

> The .bin upload is for importing/updating configuration, not downloading installed firmware.

> I have not pressed Restore Factory Settings, Update or Save, and no settings have been changed.

The reported sections are Restore Factory Settings, Configuration File Update (Browse/Update, with reboot warning), and password-protected IMEI/TTL Modification (Save). Those controls were **not invoked**. No configuration backup was downloaded. A `.bin` upload control is not a firmware-export API. These statements cover the owner's visible page and actions; neither internal settings storage nor firmware bytes was independently inspected.

The owner subsequently reported the following identifiers from the Vitel Software Upgrade page and Dashboard:

- Hardware `MF69_AM802` (Dashboard).
- Software `MF69_AM802_ZX_DD_N_250725_EU`.
- Software date July 25, 2025.
- Web interface `UAPXC v2.6.0`.
- Management address `192.168.100.1`; Vitel Wireless variant.

Exact further statement:

> No Upgrade or Check Software Update buttons were pressed. No firmware installation or configuration changes were performed.

These remain **owner-reported, unverified identity fields**. They are not converted into authenticated evidence. Older H01-B observations and its different software identifier do not qualify H01-A.

## Target-by-target triage

| Target | Known identity and missing evidence | Genuine source / authorization | Actual result and next executable step |
| --- | --- | --- | --- |
| **H01-A Vitel MG11** | Owner-reported hardware `MF69_AM802`, software `MF69_AM802_ZX_DD_N_250725_EU`, July 25, 2025. Windows USB/RNDIS candidate observed; exact board/chipset/flash layout/region and authenticated identity unknown. | No documented installed-firmware export or exact approved image found. Configuration page has imports/destructive controls, no visible export. | No firmware bytes. Manual page inspection completed. Vendor request drafted separately; obtain exact approved package or official read-only export documentation. The independent challenge also needs unambiguous routing and its own native owner approval. |
| **H01-B universal MG11** | Earlier owner/screenshots: hardware `MF69`, software `MF69_AM802L_ZX_DD_N_241106_EU`, November 7, 2024. Historical USB/root/menu observations belong here only. | No exact approved image or installed-image export. Similar UAPXC footer does not establish compatibility. | No new physical run or firmware bytes. Separate vendor request; inspect its own configuration page only when this unit is selected and available. |
| **H02 older VIDA CPE2000B** | Exact revision, software, region and board unknown. | No qualified exact-revision manufacturer image/export located in the limited primary-source search. | No bytes. Owner offered Ethernet VIDA, then reported “nvm the device is dead, so proceed with mg11 vitel for now”. Which VIDA revision that refers to is unspecified; do not assign it to H02 or H03. Need a working identified unit before requests. |
| **H03 newer VIDA CPE2000B** | Independent revision/firmware unknown; cannot inherit H02 evidence. | Same source gap; no qualified physical method. | No bytes or current physical qualification. Obtain its own label/admin evidence and a working unit. |
| **H04 Airtel DWR-930-M** | Exact Airtel hardware, installed release and regional compatibility not supplied; no unit connected in this sprint. | Two actual public D-Link packages obtained below. Neither is qualified for this physical target. | SEA artifact is differential; ZA artifact is unauthenticated HTTP RAR/PAC. No install, patch application or compatible-image claim. Need target label/version and an exact manufacturer compatibility statement; request a full image where required. |
| **H05 MF968** | OEM, board, revision and installed firmware unknown. | No qualified OEM source. A model string shared by resellers is insufficient. | No unit/bytes. Obtain exact label/admin identity before selecting a source. |

No test fixture, generic USB match, Windows gateway, marketing specification or public package counts as a physically verified target. The three-of-five stretch goal cannot be met with the currently available unqualified MG11 route and unavailable/unidentified remaining targets.

## Public-source research and actual bytes

### MG11 / IX240

The [Movingcomm MG11 product page](https://www.movingcomm.com/products/mg11-4g-wifi-6-mifi.html) gives product-family specifications and contact information. Its ASR1803 claim is a research lead, **not** evidence for either owner's board. No exact `AM802` or `AM802L` firmware package or installed-image export contract was found there. Public contact `marketing@movingcomm.com` is a vendor lead, not proof of the particular unit's manufacturer.

Page 18 of the [different IX240 manual](https://27648563.s21i.faiusr.com/61/ABUIABA9GAAghMyCxAYoyPCHqwE.pdf) visually shows “Export configuration file” and directs users to FAE for firmware packages. It does not document MG11 support. PDF: 1,839,833 bytes, SHA-256 `9ee02779849e253a64a93513f8e2648cd27c570df247d6e36f844622301d7a14`.

[Two exact, UNSENT firmware requests](MG11_VENDOR_FIRMWARE_REQUEST.md) preserve the variant distinction and ask for signed full packages, compatibility/base-image restrictions, documented read-only export coverage and recovery procedures. No email, identifying device information or credentials was sent externally.

Vitel's [official support page](https://www.vitelwireless.com/vitel-wireless-support) publishes `support.love@vitelwireless.com`; this is the primary lead for the branded unit. Exact-build web searches for both reported identifiers found no qualified public package. This is a limited search result, not a claim that no package exists. The [MG11 family specification](https://27648563.s21i.faiusr.com/61/ABUIABA9GAAgvZequgYopczXLw.pdf) lists management features and region variants but supplies no installed-image export procedure or exact-build compatibility for either owner's unit.

[Airtel's official support contact](https://www.airtel.com.ng/ng/about/contact-us) is an independent lead for the VIDA revisions. No unsupported assertion of an exact firmware source is made. MF968's OEM remains unknown.

### D-Link SEA differential package

Official linking page: [DWR-930M downloads](https://support.dlink.com.sg/consumer/5g-4g-mobile-hotspot-mifi/dwr-930m/downloads). It links a public GitBook asset named `1.02SEA--OTA--1.03SEA-diff.zip`, advertised as 1.03SEA. The original download is retained unchanged under `.tools/physical-sprint-artifacts/`.

- Size: **10,735,104 bytes**.
- SHA-256: **`a592a30abac46942dc8c8b9fdc4254882f4be1b3fa1261e140d6ca5bddfa5743`**.
- HTTP 200, `application/x-zip-compressed`; bytes actually begin `070702` (CRC-newc CPIO).
- Read-only header/offset/hash investigation found `sw-description` (4,342 bytes, SHA-256 `46f3e75cee3a17838037d802ef2c34cd095fc9da5dea01fac8b2d9c89fd09cc1`), its signature, `boot.img.delta`, `system.img.delta`, other deltas, NV material and update code. All twelve CPIO record checksums matched, including trailer; trailing padding was zero. This weak archive checksum is not publisher authentication.
- SWUpdate metadata declares differential image handling and required base partitions. A complete installed firmware filesystem cannot be reconstructed without the exact base bytes.
- Publisher verification key and exact Airtel target compatibility are unavailable. Signature verification is **not claimed**.
- No CPIO payload was unpacked, embedded updater executed, delta applied or filesystem manifest published. Container listing/hashes are **metadata analysis only**, not verified firmware extraction.

The local/native test imports this unchanged package through the existing immutable Library as **user-imported/unverified**, with no official-catalogue producer credential and no verified target association. Public-source research provenance above does not bypass Library admission rules.

### D-Link Southern Africa service package lead

The [official HTTPS product page](https://www.d-link.co.za/product/dwr-930m/) links [FW 2.07ME](http://downloads.d-link.co.za/DWR/dwr930m/firmware/FW%202.07ME/). The working binary route uses **plain HTTP**, so its transport authenticity is not cryptographically verified. No downgrade or TLS bypass was substituted for a trusted acquisition.

- Original filename: `3 uis8310_csm18-01_weijia_m1c_pl_cc_64mb_lte_4m-cs_halo-stable_user_20230216211708.rar`; retained local name `dwr930m-207me.rar`.
- Size: **54,728,019 bytes**.
- SHA-256: **`4996408e418225b41478bd79ff2e98f3889911b4b75de9ca4abdcd61d81ddcd9`**.
- Initial public download timed out after 47,902,144 bytes (curl exit 28). Explicit resume returned HTTP 206 and completed (exit 0). No incomplete file was classified as a valid image.
- Read-only RAR5 header investigation found 1,001 header records / 999 entries with matching header CRCs. A PAC member is declared as 34,460,798 bytes. These are header declarations, not decoded payload validation.
- Declared combined unpacked size: **318,315,552 bytes**, larger than the existing 256 MiB sandbox; unpack-all was not attempted. No new unqualified RAR/PAC decoder, service tool or flashing driver was executed.
- Exact model revision/region/base and publisher authenticity remain unknown. This is a research artifact, not a verified-compatible image or physical dump.

The local source metadata/response-facts files record exact resolved URLs, headers, sizes and hashes. Public CDN query parameters, raw downloads and generated screenshots remain local/uncommitted. Neither package is silently added to the trusted official-source catalogue.

## Focused implementation

1. `scripts/inventory-physical-device.ps1`: reusable local Windows USB/network inventory, redacted topology, present/active association, overlap observations, connected-candidate/absent/unknown states and protected no-overwrite output. A successful capture alone never means a router was verified.
2. `apps/desktop/src-tauri/src/library/analysis.rs`: root-only CPIO newc/CRC and BSDIFF40 **signature observations**, requiring complete fixed-header lengths. The misleading `.zip` extension no longer determines format. No archive parser, payload unpacking, patch application, device authority, schema or transport is introduced. Complete fixed-header length is not proof of valid structure; report limitations say so explicitly.
3. `scripts/verify-manufacturer-package.mjs` and the `--manufacturer-package` branch in `scripts/verify-foundation.mjs`: genuine public package through real native file selection, immutable Library, Rust analysis IPC, honest unsupported extraction, shutdown/reopen and source hash checks. Windows discovery is genuine; no synthetic identity or router approval is used.
4. `.github/workflows/windows.yml`: runs the inventory's synthetic privacy/containment/presence tests, while retaining both Rust profiles, both native builds and the production-executable safeguard.

Existing adapters, vault, schema 6, jobs, six-page GUI, customization/build gates and extraction sandbox are preserved. No speculative acquisition adapter was enabled: neither a documented MG11 export nor a verified target image exists. Configuration-only data cannot be sent through firmware Import as though it were an installed dump.

### Request for a universal acquisition “backdoor”

No backdoor, credential bypass or hidden persistent access was implemented. A shared acquisition engine can support protocol families; it cannot assume every device exposes the same read operation. Existing access is required even to install a new hidden component, so that would not solve first acquisition.

The [USB-IF DFU 1.1 specification](https://www.usb.org/sites/default/files/DFU_1.1.pdf), section 6.2, defines firmware retrieval through DFU_UPLOAD. Its class/interface descriptors and **separate** upload-capability flag must be established first; [U-Boot DFU support](https://docs.u-boot.org/en/latest/usage/dfu.html) also needs configured backends and target-specific alternate ranges. The current MG11 does not advertise this interface in Windows. No DFU request, detach, reset or boot-mode change was sent.

Other genuine protocol families require their own evidence: an existing authorized shell and exact readable partition map; a known-voltage UART/bootloader console with a qualified read/export command; or supported, electrically isolated external flash equipment. None is established for either MG11. A [chipset vendor's ASR1803 product page](https://www.asrmicro.com/en/goods/proinfo/44.html) lists NOR/NAND boot and different software environments but supplies no MG11 readback contract. It cannot verify this unit's chipset, imply a Linux root shell or authorize guessed service commands. Missing diagnostic drivers and an `FF` interface are recorded as limitations, not treated as a universal dump route.

## Reproduced failures and corrections

- New CPIO/BSDIFF/extension regressions first failed against the existing analyzer: **5 passed / 3 failed**, exit **101** (`.tools/physical-sprint-analysis-red-all.log`). After correction, the same analysis module has **8 passed / 0 failed**, exit **0** (`physical-sprint-analysis-green.log`). Synthetic fixtures are labelled synthetic and never physical qualification. Truncated, near-miss and embedded magic remain unknown.
- Independent script review found that pattern-based name cleaning retained an unlabelled synthetic 16-hex identifier. A regression reproduced it with exit **1** (`inventory-review-redaction-before.log`). Arbitrary names were removed from queries/output; fixed labels and allowlisted class/status now replace them.
- Review also found historical records could inflate connection counts. Present-only correlation and explicit current status were added. Disconnect, absent and unknown-presence regressions are included in **30 passing synthetic checks**, exit **0**.
- Existing-output and junction-parent negatives exited **1**, preserving the previous report and creating no report through the junction. The original inventory report remains unchanged.
- Interface-class regressions first failed with `dfu_runtime_tuple_observed`, exit **1** (`inventory-interface-before.log`). Strict tuple parsing/classification then passed **51 synthetic checks**. DFU advertisement never becomes upload authority, including in synthetic positives; injected paths, identifiers, partial tuples and malformed values are rejected.
- The first extended real capture exposed a PowerShell `$Matches` automatic-variable collision: five present records were reported alongside an incorrect `unknown` connection status. The raw collection was renamed `$usbRecords`; the incorrect capture is retained as `inventory-interfaces-before-match-scope-fix.json`. The corrected actual capture reports five present records and `connected_candidate`. No capture sent a device request.

## Completed local verification

Application runtime source: `39898fc8e7926e78ee5cfbda8abff56690a10487`. Follow-up inventory/native-harness source: **`fba2372baa58ed21ebcbb55cc573754bff98292a`**. The latter does not change compiled application behavior or storage. No independent reviewer acceptance of this sprint is claimed.

Commands below run from `.tools/beta2-hardening`. `RUST` means exactly `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1`. Rust invocations use `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`. Cargo's ordinary stderr was captured through a child PowerShell; wrapper `NativeCommandError` annotations are not counted as failures when the child exits 0 and the test/build summary succeeds.

| Exact command | Exit | Actual result / local log |
| --- | --- | --- |
| `RUST test --workspace --locked --offline library::analysis::tests` before implementation | 101 | 5 passed / 3 failed; `physical-sprint-analysis-red-all.log`. |
| Same targeted command after implementation | 0 | 8 passed / 0 failed; `physical-sprint-analysis-green.log`. |
| `RUST test --workspace --locked --offline` | 0 | **385 passed / 2 ignored**, plus **2 doctests**; `physical-sprint-rust-standard.log`. |
| `RUST test --workspace --features native-test-profile --locked --offline` | 0 | **386 passed / 3 ignored**, plus **2 doctests**; `physical-sprint-rust-isolated.log`. Ignored VM/subprocess entrypoints are not new physical qualification. |
| `RUST fmt --all --check` | 0 | `physical-sprint-fmt.log`. |
| `RUST clippy --workspace --all-targets --locked --offline -- -D warnings` | 0 | `physical-sprint-clippy-standard.log`. |
| `RUST clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0 | `physical-sprint-clippy-isolated.log`. |
| `npm.cmd test` | 0 | **176 tests / 22 files**; `physical-sprint-frontend-tests.log`. |
| `npm.cmd run lint` | 0 | `physical-sprint-lint.log`. |
| `npm.cmd run build` | 0 | TypeScript and Vite production build; `physical-sprint-frontend-build.log`. |
| `npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline` | 0 | Optimized Windows application; `physical-sprint-production-build.log`. |
| `npm.cmd run build:native-tests` | 0 | Dedicated isolated executable; production executable hash guard passed; `physical-sprint-native-test-build.log`. |
| `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/inventory-physical-device.ps1 -SelfTest` | 0 | **51** synthetic checks after interface follow-up. |
| `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/inventory-physical-device.ps1 -OutputPath .tools/physical-sprint-inventory-interfaces.json` | 0 | Genuine local Windows observation, redacted; five present records / one active USB network / one overlap / no DFU advertisement. |
| Repeat inventory to existing output; inventory with a junction output parent | 1, 1 | Expected refusals; prior output preserved, no junction traversal. |
| `node --check scripts/verify-manufacturer-package.mjs` | 0 | Syntax check. |
| `node --check scripts/verify-foundation.mjs` | 0 | Syntax check. |
| `node scripts/verify-foundation.mjs --manufacturer-package` | 0 | Actual native import/analysis/unsupported extraction/restart; `physical-sprint-native-package.log`. |
| `npm.cmd run test:universal-workflow-native`, initial run | 1 | Actual extraction completed, but shutdown did not settle in 30 seconds. Failure retained below. |
| Same command, first retry | 1 | Harness invoked Rust before Tauri injected `window.__TAURI_INTERNALS__`; readiness race reproduced. |
| Same command after readiness correction | 0 | Full GUI Cancel/retry extraction/browsing/restart/cleanup; `physical-sprint-workflow-native-ready.log`. |
| Same command, confirmation on unchanged corrected source | 0 | Full second pass; `physical-sprint-workflow-native-confirm.log`. |
| `py scripts/audit-data-bytes.py .tools/physical-sprint-normal-after-confirm.json .tools/physical-sprint-normal-before.json` | 0 | **12 files / 5,611,701 bytes unchanged**, including DB/WAL/SHM where present, settings, saved-device data, logs and vault. No normal SQLite connection. |
| `git diff --check` after report whitespace correction | 0 | No whitespace errors. |

No development/HMR or new physical OpenWrt test was run in this sprint. Existing unit adapter/simulator regressions passed within both full Rust profiles. The local native runs cover actual Windows release executables; they do not constitute new live DNS, APN, Wi-Fi, USSD, flash or recovery qualification.

Local executable SHA-256 values:

- `target/release/obtainer.exe`: `881e429179c0ef544cee25907b76ef424c8449bf5327bc787f0e3e83c90813d8`.
- `target/release/obtainer-native-test.exe`: `36506d1bd61a1f45ad3e599d5fb2ec69c7949ac7883d7f3639f42fb4d3b1a6cc`.

### Actual native evidence and observed failures

**Genuine D-Link package:** `.tools/foundation/2026-10-28T09-44-47.236Z`, owned profile `0adbd4d282704963463c960ca5aea155`. Actual HWND/React, six navigation pages, SQLite IPC, discovery refreshes without duplicate IDs, import confirmation, immutable source hash, CPIO report and extraction failure `UNSUPPORTED_FORMAT` all verified. Reopening retained the package, report, failed job and unverified classification; extraction report remains null. The actual screenshot was opened and visually inspected: it shows “CPIO CRC archive”, “Hardware compatibility: unverified” and the unsupported filesystem message. No captured console warnings/unhandled exceptions. The one genuine current USB candidate remained unverified with `probe.attempted=false`, `not_approved`, null HTTP status and `canObtain=false`; this local assessment does not exercise the transport collision guard or verify H01-A. Normal data unchanged; owned profile cleaned after shutdown.

**Initial broader regression failure:** `.tools/foundation/2026-10-28T09-49-35.448Z`, retained profile `94e84079f872d8cf11ac002c17b25cd3`. Real official image download, analysis, Cancel/retry extraction, tree/hash verification and GUI preview completed. Window shutdown then timed out (`Native session or Vite listener did not stop`), exit 1. The runner stopped its owned process tree and preserved the failed profile. This timeout's root cause was not established, and is not relabelled as a successful restart. Diagnostic capture was added for any future shutdown timeout; no production shutdown behavior was changed merely to hide it.

The next attempt failed immediately because a CDP page existed before Tauri's invoke bridge. The workflow branch now waits for actual “Desktop connected” React/Rust readiness on both launches, matching the established foundation helper. Two subsequent complete runs passed shutdown and restart with the same 30-second limit. The original shutdown timeout has not recurred in these runs; monitor it in review rather than asserting an unproven cause.

**Corrected software-pipeline regression:** `.tools/foundation/2026-10-28T09-54-38.500Z` and confirmation `.tools/foundation/2026-10-28T09-58-29.424Z`. Synthetic identity is explicitly marked as a software fixture; the downloaded OpenWrt image is genuine. Both actual native sequences passed GUI Cancel, retry, 869 file hashes, browsing `etc/openwrt_release`, background integrity refresh without losing preview, restart, stage jobs and the direct selected Library link. This remains **zero physical device successes**.

OpenWrt regression input: 7,405,862 bytes, SHA-256 `0ec9643bb9f43d65c689761d916a9e0c13bad5968656d74e516df010deb0be9d`; bundle SHA-256 `3e4c62186d7eed588d42edf1c98faf8e0e5c749c8cd436e20a535dd2385aae02`; 869 ordinary files / 128 directories / 13,411,262 file bytes, partial filesystem-region extraction. Corrected-run manifest SHA-256 `22a62340a8f1d582aa9d819b9d5b7aba9071ae6bd42487b147d4316766d1b2f1`; confirmation manifest `d6a1bccb0d58a8be281d26990008ed2a5cb9d653708e64d13c906f0292c6befb`. Timestamped manifests differ; input and bundle bytes agree. These are existing pipeline regressions, not H01–H05 firmware acquisition or compatibility evidence.

All generated screenshots, firmware downloads, failed-run profiles, local logs and full native evidence remain uncommitted. No release assets or published tags were replaced.

## Verification and preservation

Full local command outcomes, native evidence and source commits are recorded above. [CI 40666456683](https://github.com/okekechuks/Firm_Find/actions/runs/40666456683), exact application source `39898fc8e7926e78ee5cfbda8abff56690a10487`, is **green for all three jobs**, including both Windows builds and the production-executable safeguard. Final helper/native-harness source **`fba2372baa58ed21ebcbb55cc573754bff98292a`** also has **all three jobs green** in [CI 40668887195](https://github.com/okekechuks/Firm_Find/actions/runs/40668887195): Windows verification (both Rust profiles, frontend, inventory self-test, both native builds and production hash safeguard), DNS helper and qualified extraction worker. The final evidence-only documentation commit changes no application, worker or harness behavior. None of these software checks substitutes for metrics A or B.

Normal application data baseline was captured using `py scripts/audit-data-bytes.py .tools/physical-sprint-normal-before.json`: **12 ordinary files / 5,611,701 bytes**, exit **0**, without opening SQLite. All native work used marked dedicated isolated profiles. Final `physical-sprint-normal-after-confirm.json` matches the baseline exactly: no normal-data storage relocation, migration, cleanup or settings write occurred. Main stays `d5d05413d30c01f189968526be2ac80979387bd5`; Beta 1/Beta 2 tags stay `7433c96b379dd70d1481f06d645ff7f49570d9fe` / `e4524e80ac38ff0702e0917fc9a8f314d2f1ad82`. The old reviewer scratchpad is unavailable, and no new independent acceptance is inferred from committing or green CI.

**Physical firmware acquired and extracted: 0/5**

**Manufacturer-compatible image acquired and extracted: 0/5**

**Metadata-only/blocked: 5/5**

**Phase 7.5 and the full OBTAINER project remain incomplete.** Smallest acquisition step for the currently available MG11 is an exact manufacturer-authorized package or documented read-only export procedure. A reviewed interface-pinned transport would remove this particular Wi-Fi/USB routing obstacle, but would not itself supply firmware bytes or authorize authenticated reads.
