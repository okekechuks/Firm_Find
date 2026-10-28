# Physical firmware acquisition sprint — October 28, 2026

## Outcome and source

**Hardware acquisition remains BLOCKED. No physical firmware bytes were acquired.** Public D-Link packages were downloaded for investigation; neither has been matched to a physical target. A working Library import, analyzer or extractor does not change these counters.

| Required metric | Original H01–H05 groups | Individual units (H01-A, H01-B, H02–H05) |
| --- | --- | --- |
| A: installed firmware physically acquired AND extracted | **0/5** | **0/6** |
| B: independently verified compatible manufacturer image acquired AND extracted | **0/5** | **0/6** |
| Metadata only / blocked | **5/5** | **6/6** |

H01-A and H01-B are separate units within **one** original target. There is no firmware artifact, file manifest, partition map, installed-image hash or extraction-completeness claim for either MG11.

Working branch: `sprint/physical-acquisition-2026-10-28`, in the existing isolated `.tools/beta2-hardening` worktree. Parent: `942195a5c7035f429ddf2bba4f0b30ec4c0f7cfa` (`feature/universal-firmware-workflow`). At start, `git ls-remote origin refs/heads/main refs/heads/feature/universal-firmware-workflow` returned main `d5d05413d30c01f189968526be2ac80979387bd5` and feature `942195a5c7035f429ddf2bba4f0b30ec4c0f7cfa`; the tracked trees were clean. Main, published tags and existing release assets are preserved. No merge or release is authorized by this report. The delivery commit and completed verification are recorded below after execution.

## Actual physical observations and owner participation

The owner connected the Vitel MG11 and explained that PC Wi-Fi must stay on because this MiFi has no internet access. No programmatic Wi-Fi disconnect, route/metric edit, adapter reconfiguration or driver installation was performed.

`scripts/inventory-physical-device.ps1` performs **local Windows enumeration only**: CIM PnP/network records, PnP parent properties, IP addresses, DNS and route configuration. It does not open the normal application database or create a socket. Raw PnP IDs, GUIDs and parent identifiers are replaced with per-run salted SHA-256 references; salt is discarded. Arbitrary friendly names are omitted rather than relying on incomplete secret-pattern redaction. JSON output is restricted to ordinary directories under `.tools/`, protected against parent replacement, and created without overwrite.

Final actual capture: `.tools/physical-sprint-inventory-final.json`, SHA-256 `7f7c166978022275053edc31aa514cb33cd8f7d5dcff4257d621b3e57ca02173`:

- Five present USB records share VID `1286` / PID `4E31` and observed parent relationships. These IDs do not establish model, board, chipset or firmware.
- One present, active correlated RNDIS network adapter: Windows interface 12, `192.168.100.100/24`, gateway `192.168.100.1`.
- PC Wi-Fi interface 19 is also active at `192.168.100.199/24`, reporting the same gateway. Both interface routes have route metric 256; Windows interface metrics were 55 for USB and 45 for Wi-Fi. This is a genuine overlap, not proof that both gateways are the same physical router.
- Composite/RNDIS records report problem code 0; two diagnostic interfaces and one AT interface report problem code 28. No diagnostic/AT commands, engineering mode, privileged storage reads or driver changes were attempted.
- Configured Windows DNS values are PC adapter observations, not independently read router settings. They are not fabricated device DNS or identity evidence.

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

## Reproduced failures and corrections

- New CPIO/BSDIFF/extension regressions first failed against the existing analyzer: **5 passed / 3 failed**, exit **101** (`.tools/physical-sprint-analysis-red-all.log`). After correction, the same analysis module has **8 passed / 0 failed**, exit **0** (`physical-sprint-analysis-green.log`). Synthetic fixtures are labelled synthetic and never physical qualification. Truncated, near-miss and embedded magic remain unknown.
- Independent script review found that pattern-based name cleaning retained an unlabelled synthetic 16-hex identifier. A regression reproduced it with exit **1** (`inventory-review-redaction-before.log`). Arbitrary names were removed from queries/output; fixed labels and allowlisted class/status now replace them.
- Review also found historical records could inflate connection counts. Present-only correlation and explicit current status were added. Disconnect, absent and unknown-presence regressions are included in **30 passing synthetic checks**, exit **0**.
- Existing-output and junction-parent negatives exited **1**, preserving the previous report and creating no report through the junction. The original inventory report remains unchanged.

## Verification and preservation

Full command outcomes, native evidence, source commit and CI URL are appended below after execution. None of these software checks substitutes for metrics A or B.

Normal application data baseline was captured using `py scripts/audit-data-bytes.py .tools/physical-sprint-normal-before.json`: **12 ordinary files / 5,611,701 bytes**, exit **0**, without opening SQLite. All native work uses a marked dedicated isolated profile; no normal-data storage relocation, migration, cleanup or settings write is intended. Final byte comparison is recorded with the completed verification below.

**Physical firmware acquired and extracted: 0/5**  
**Manufacturer-compatible image acquired and extracted: 0/5**  
**Metadata-only/blocked: 5/5**

**Phase 7.5 and the full OBTAINER project remain incomplete.** Smallest acquisition step for the currently available MG11 is an exact manufacturer-authorized package or documented read-only export procedure. A reviewed interface-pinned transport would remove this particular Wi-Fi/USB routing obstacle, but would not itself supply firmware bytes or authorize authenticated reads.
