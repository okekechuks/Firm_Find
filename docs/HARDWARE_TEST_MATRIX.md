# Phase 7.5 hardware matrix

## Web-management/recursive milestone — October 29, 2026

No new physical HTTP, credential, authentication, flash-read or write operation
has been executed. H01-A and H01-B remain distinct and BLOCKED: no qualified
read-only identity route or genuine firmware source/bytes. The owner has not yet
provided a sanitized HAR for this milestone. H02/H03 retain separate unknown
revision qualifications; the dead VIDA report does not identify either revision.
H04's exact physical revision/region remains unqualified; public packages cannot
be associated as compatible by filename or model label. H05 remains unqualified.

Existing genuine D-Link CPIO and RAR packages can now have bounded contents
extracted through the V2 feature engine, with partial/delta/omitted-member limits.
This updates file-handling capability only, not any physical-device success row.
Installed firmware acquired and extracted: **0/5 groups, 0/6 individual units**.
Verified-compatible physical-target manufacturer images: **0/5, 0/6**.
[Web-management evidence](WEB_MANAGEMENT_ACQUISITION_VERIFICATION.md) and
[structural extraction evidence](RECURSIVE_FIRMWARE_EXTRACTION_VERIFICATION.md).

## Physical sprint — October 28, 2026

Strict installed firmware acquired **and** extracted: **0/5 original groups, 0/6 individual units**. Verified-compatible manufacturer image acquired and extracted: **0/5, 0/6**. Metadata-only/blocked: **5/5 groups**. Public D-Link package downloads are not physical dumps or independently verified matches. [Current sprint evidence](PHYSICAL_FIRMWARE_ACQUISITION_SPRINT_2026-10-28.md).

| Unit | Identity evidence | Source and actual result |
| --- | --- | --- |
| H01-A, Vitel MG11 | Owner reports Dashboard hardware `MF69_AM802`, software `MF69_AM802_ZX_DD_N_250725_EU`, July 25, 2025. Current Windows USB/RNDIS candidate VID1286/PID4E31; unit association owner-reported, not independently verified. | Owner visually inspected Configuration Management: restore, configuration upload and locked IMEI/TTL controls; no visible export. No controls invoked. USB/Wi-Fi overlap remains; no new router request/challenge/login. No firmware source/bytes. |
| H01-B, universal MG11 | Earlier owner/screenshots: hardware `MF69`, software `MF69_AM802L_ZX_DD_N_241106_EU`, November 7, 2024. | Historical H01 root/version/menu evidence below belongs to this prior unit only. No new run or firmware bytes; cannot qualify H01-A. |
| H02, older VIDA | Exact revision/firmware still unknown. | Owner reported an offered VIDA dead, revision unspecified. No current physical test or source; do not assign the dead unit to a revision. |
| H03, newer VIDA | Separate unknown revision/firmware. | No inherited H02 qualification or exact source. |
| H04, Airtel DWR-930-M | No exact physical revision, installed release or region supplied. | Genuine official-linked SEA CPIO/delta and ZA HTTP RAR/PAC packages downloaded. Exact compatibility unknown; no full image reconstruction, payload extraction or install. |
| H05, MF968 | OEM/revision/firmware unknown. | No qualified source or physical test. |

Both MG11 units count within H01. Owner reports remain unverified; shared UAPXC UI/USB identifiers never imply interchangeable images. Separate [unsent vendor requests](MG11_VENDOR_FIRMWARE_REQUEST.md) preserve both identities. The physical challenge gate is pending; no physical firmware dumping, configuration write, reset or flash occurred in this sprint.

## Universal workflow integration — 27 October 2026

No new physical requests in this task. A synthetic identity plus a genuine public image tests software integration only; it does not qualify any row below.

| ID | Hardware identity | Firmware source / acquired bytes | Extraction | Customization | Rebuild | Installation / recovery |
|---|---|---|---|---|---|---|
| H01 | Existing USB/RNDIS and candidate management evidence; exact model/revision/installed firmware unverified | No qualified installed-image source/export/storage read; no acquired physical firmware bytes | Pending genuine compatible firmware | Unqualified | Unqualified | Both unqualified; no execution |
| H02 | Older CPE2000B revision, owner-reported; no physical qualification | Unknown / none | Pending | Unqualified | Unqualified | Unqualified |
| H03 | Newer CPE2000B revision, independent of H02; no physical qualification | Unknown / none; cannot inherit H02 route | Pending | Unqualified | Unqualified | Unqualified |
| H04 | DWR-930-M owner-reported; no physical qualification | Unknown / none | Pending | Unqualified | Unqualified | Unqualified |
| H05 | MF968 owner-reported; no OEM/hardware assumption | Unknown / none | Pending | Unqualified | Unqualified | Unqualified |

Existing dated physical observations below are preserved. Software-tested official-image paths remain distinct from installed-device acquisition.

Updated 23 October 2026. Observations and reports are not verified hardware identities. H02 and H03 are independent targets. No reset, flash, privileged maintenance, live USSD or device write is permitted by this phase.

| ID | Owner-reported target | Detection / identity evidence | Management | Acquisition / analysis | Gate |
| --- | --- | --- | --- | --- | --- |
| H01 | comFi MG11 4G MiFi | Native generic USB/RNDIS observation, VID 1286/PID 4E31; reported LT800 and MF69_AM802L_ZX_DD_N_241106_EU remain unverified | 23 October owner-approved USB-only root HTTP 200, candidate `asr_duster_xml`; static version `REL_MRVL_UAPXC_v2.6.0` endpoint-observed/unverified. Exact identity and authenticated API unknown | Five unknown, nonexecutable acquisition routes; no qualified source/export/read; no firmware artifact | Native Yes/read, Wi-Fi collision guard and owner No VERIFIED; normal app data preserved. Owner confirmed router settings/firmware display unchanged. Exact identity/acquisition pending |
| H02 | Airtel VIDA CPE2000B, older revision | No new observation | Unknown | Unknown | Physical qualification pending |
| H03 | Airtel VIDA CPE2000B, newer revision | No new observation; must not inherit H02 qualification | Unknown | Unknown | Physical qualification pending |
| H04 | Airtel DWR-930-M 4G MiFi | No new observation | Unknown | Unknown | Physical qualification pending |
| H05 | MF968 4G LTE Mobile WiFi | No new observation; OEM not assumed | Unknown | Unknown | Physical qualification pending |

Owner check: connect and power on one target; open Connected Devices, More Info; confirm Windows observations and selected local management address/interface. Optionally approve one read-only root-page check and run Check Compatibility. Export Diagnostics to a new file. Record model/revision/firmware separately as user reports until a qualified identity response is available. A candidate family or HTTP success never proves firmware export. Obtain Firmware must stay disabled without a qualified executable route.

H01 history: first native owner-approved check returned `no_route`, HTTP status absent, because Windows preferred Wi-Fi while both interfaces shared 192.168.100.0/24. No wrong-interface fallback was attempted. After the owner disconnected Wi-Fi and confirmed USB ready, the same native release check passed 21 checkpoints with zero console issues. The bounded root GET returned HTTP 200 / `identity_unknown`, candidate `e_link_oem`, with metadata capped/truncated. Redacted diagnostics and an actual GUI screenshot were captured; normal data remained unchanged. The owner was told Wi-Fi can be reconnected. [Latest report](evidence/phase-7-5-h01-redacted-2026-10-22.json) and [earlier route block](evidence/phase-7-5-h01-routing-blocked-2026-10-22.json) remain separate evidence. No firmware support follows from an HTTP response alone; `canObtain: false` and all four physical features remain UNKNOWN.


Current evidence boundary: the fixture is the first 16,384 bytes of the reviewed genuine root capture. Native approval, `asr_duster_xml` recognition and the corrected static-version reader were physically rerun on 23 October as recorded below. The saved owner-reported `MF69_AM802L_ZX_DD_N_241106_EU` and endpoint-observed `REL_MRVL_UAPXC_v2.6.0` remain separate and unverified; no relationship between the identifiers is inferred. Label photos and exact hardware revisions for H01-H05 have not been newly supplied. H02 and H03 remain independent pending rows. First authenticated read remains future owner-supervised Sprint B work after design approval.


23 October 2026 review correction: the actual owner-supplied MG11 root capture (18,914 bytes) was reviewed locally and its first 16,384 bytes retained as a fixture. This is captured-file/positive-parser evidence, not a new physical HTTP run. The logo is in an HTML comment and correctly excluded; two real script references qualify candidate ASR/Duster XML. The four supervised H01 outcomes are now recorded in PHASE_7_5_REVIEW_FIXES.md; the captured fixture itself is not physical qualification. H02 and H03 remain independent; no evidence or support is inherited across devices. No hardware matrix row is promoted from software fixture tests.


23 October supervised update: Wi-Fi-on/USB overlap returned subnet_conflict naming Wi-Fi, without TCP/HTTP dispatch. Owner disconnected Wi-Fi, approved the final corrected USB-only check and the native GUI displayed observed `REL_MRVL_UAPXC_v2.6.0` beside owner-reported `MF69_AM802L_ZX_DD_N_241106_EU`, both unverified. Initial USB read missed the XML-wrapped version; that failure, new failing regression, correction and successful retest are retained. Actual owner-No repeat produced no authorization or route report. Owner confirmed unchanged settings/firmware after Wi-Fi-on, initial USB read and final corrected read, including the No case. Normal application database/WAL/rows/settings/saved-device/vault fingerprints equal the task baseline. Router internal database was not inspected. No firmware acquired and all four features remain UNKNOWN. [Current report](PHASE_7_5_REVIEW_FIXES.md), [redacted summary](evidence/phase-7-5-review-2026-10-23.json). Label photos and exact revisions remain pending; H02-H05 have no new evidence.

Owner statements, 23 October 2026 (Africa/Lagos), in this Codex conversation's hardware-check replies: after Wi-Fi-on, "Unchanged; Wi-Fi off; USB ready"; after the initial USB read, "Settings and firmware unchanged"; after the final USB read and earlier No test, "Settings and firmware unchanged". These statements cover only the router's settings/firmware web display. They do not verify its internal storage or firmware bytes. Normal Windows application data preservation is a separate computed comparison.

R8 follow-up, 23 October: local tests show the genuine captured page produces identical candidate evidence before and after the stray-`<` scanner correction. No approval code changed and no additional physical run was required or performed. Part A R6-R10 and B1 design review are tracked in [follow-up](PHASE_7_5_R6_R10.md).
