# Phase 7 firmware feasibility

Revision: 20 October 2026. Phase 7 supplies inspection metadata and conservative assessments. Actual test outcomes belong in [Phase 7 verification](PHASE_7_VERIFICATION.md). A possible method is not an implemented extraction, build, installation or recovery engine.

## Product objective and two pathways

The full product workflow remains detection, identification, firmware acquisition/extraction, analysis, customization, backup, rebuild or configuration apply, installation, verification and recovery.

Direct configuration customization uses a supported authenticated interface and a reviewed read/backup/validate/preview/approve/apply/verify/restore transaction. Firmware customization obtains an exact compatible image or supported extracted source, preserves it, analyzes and modifies supported components, rebuilds and validates a separate artifact, and later installs it through a qualified device-specific method with recovery requirements.

These pathways share device identity and compatibility evidence. Neither read access nor this report grants write authority. Physical configuration writes, partition reads, flashing and live USSD remain unavailable in Phase 7 while their dedicated engines and gates are pending.

## Implemented assessment boundary

`apps/desktop/src-tauri/src/openwrt/feasibility.rs` defines:

- `Status`: `REQUIRES_VERIFICATION`, `REQUIRES_TOOL`, `UNSUPPORTED`, `UNKNOWN`.
- `Assessment`: status, reason, evidence, missing requirements and `executable`.
- `CustomizationPath`: separate direct-configuration and firmware-rebuild assessments.
- `FirmwareFeasibilityReport`: authenticated board identity; acquisition, extraction, storage layout, image format, build, installation, recovery, four feature pathways, restrictions, observation time and confidence.
- `evaluate()`: derives bounded metadata-only results from an authenticated `Board` and optional parsed MTD metadata. It does not perform network acquisition, raw storage reading, image analysis, building or installation.

Every generated assessment has `executable: false`. Reports must never become firmware artifacts, original-firmware backups or enabled Apply/Extract/Install actions. Existing Library import, hashing, immutable storage, deduplication and backups retain their own provenance and behavior.

The current evidence source is narrow: exact OpenWrt `25.12.6` plus target `x86/64` yields a constant official release-directory candidate and matching ImageBuilder availability. Remote strings never become arbitrary download URLs. Model, hardware revision, BIOS/EFI mode and layout still require qualification. Another release/target or conflicting distribution produces no source candidate.

## Current assessments

| Dimension | Evidence accepted by current model | Current result and missing requirements |
| --- | --- | --- |
| Official acquisition | Authenticated distribution/version/target matches the researched release | `REQUIRES_VERIFICATION`; exact board/revision, artifact identity, published manifest, integrity/authenticity and safe acquisition engine remain required |
| Physical extraction | No qualified raw read method exists | `UNKNOWN`; exact device adapter, scope/coverage, authorized read, integrity, progress/cancel and secret-bearing partition policy remain required |
| MTD layout | Strictly parsed authenticated `/proc/mtd` metadata, when available | `REQUIRES_VERIFICATION`; completeness, UBI/block topology, dual-bank behavior and recovery coverage remain unproven |
| Missing MTD metadata | MTD unavailable on the target | `UNKNOWN`; does not mean the device has no storage or that extraction is impossible |
| Image format | Official selected x86 release distinguishes BIOS/EFI and ext4/squashfs variants | `REQUIRES_VERIFICATION`; rootfs type alone cannot select a safe image; signatures, encryption and size constraints remain unknown |
| Build route | Matching official ImageBuilder published for selected release/target | `REQUIRES_TOOL`; OBTAINER builder, exact profile, package/overlay requirements and output validation pending |
| Installation | No implemented qualified installation adapter | `UNKNOWN`; compatibility, protected backups, explicit approval, power/connection-loss handling and recovery qualification pending |
| Recovery | No physical recovery test | `UNKNOWN`; exact documented procedure, necessary tools/access and physical evidence pending |

The official release directory provides the candidate-image and builder evidence; the report does not download or authenticate either. [OpenWrt 25.12.6 x86/64 artifacts](https://downloads.openwrt.org/releases/25.12.6/targets/x86/64/).

## Four feature assessments

For DNS, Wi-Fi client limits, APN and USSD, the current evaluator leaves both customization pathways `UNKNOWN`. This is intentional: observing configuration does not establish a qualified modification transaction, compatible package integration, hardware support or a safely installable rebuilt image.

| Feature | Direct configuration prerequisites | Firmware customization prerequisites |
| --- | --- | --- |
| DNS | Distinct upstream/LAN/runtime scope, exact mapping, backup/preview/apply/read-back/restore and behavior checks | Compatible target image, resolver/package configuration, validated build and installation/recovery route |
| Wi-Fi client limits | Exact radio/BSS, driver/service limits, management interruption handling, effective enforcement evidence | Compatible driver/firmware/package support; image change cannot create absent hardware capacity |
| APN | Exact modem/protocol, supported manual/automatic behavior, nonsecret IP-family/auth mapping and reconnection verification | Compatible modem drivers/packages and supported operator data; no SIM-unlock or universal-carrier claim |
| USSD | Exact passive modem/interface evidence, later bounded one-shot/interactive session engine | Compatible router-local authenticated service/page package and deployment; desktop must eventually be unnecessary for router-hosted use |

Guest observations without radio/modem hardware cannot satisfy physical Wi-Fi/APN/USSD requirements. The owner's proprietary MiFi remains outside the selected OpenWrt target; its saved software identifier remains user-reported and unverified.

## Artifacts are not interchangeable

| Item | What it represents | What it cannot establish alone |
| --- | --- | --- |
| Official compatible release image | Publisher's image for a documented target | Current device contents or full device recovery |
| Raw partition dump | Bytes from a specific storage range | Complete coverage, safe installation format or signature compatibility |
| Configuration archive | Selected settings/files | Bootloader/kernel/rootfs firmware backup |
| Inspection/feasibility JSON | Observed metadata and unresolved prerequisites | Firmware bytes, validated builder output or installation success |
| Rebuilt firmware artifact | Separately validated generated image | Safe installation or recovery on an unqualified device |

The release's upgrade implementation treats configuration archives, image validation and flashing as different operations. None is dispatched by the inspection allowlist. [OpenWrt sysupgrade source](https://raw.githubusercontent.com/openwrt/openwrt/v25.12.6/package/base-files/files/sbin/sysupgrade).

## Known model limits

The evaluator does not inspect an imported firmware binary, infer signing/encryption from a version string, validate hardware revision, recognize arbitrary vendor image containers, enumerate all block/UBI storage or establish a complete rollback image. It has no catalog of qualified physical installation/recovery routes. Its narrow constant OpenWrt source candidate is not universal compatibility.

Conflicting or missing identity, malformed/oversized metadata, uncertain image format, unavailable extraction method, and signed/encrypted proprietary images must retain explicit unknown/unavailable reasons. Numeric release comparisons must never be applied to opaque proprietary identifiers. Future extensions need typed evidence for their additional facts; they must not convert a device name, USB ID, endpoint success or profile hint into support.

The evaluator currently reports the presence of parsed MTD metadata separately from proof of completeness. If the observed table is empty, the UI must show the count and unresolved requirements; no extraction support follows from an empty or nonempty table.

## Future implementation gates

The [firmware pipeline roadmap amendment](FIRMWARE_PIPELINE_ROADMAP.md) proposes acquisition/extraction and qualified-installation milestones without renumbering existing phases. They require owner approval before adoption. Phase 15 remains supported OpenWrt builds, Phase 16 proprietary feasibility and Phase 17 hardware qualification. The Phase 7 boundary is temporary; firmware extraction, rebuilding, installation and recovery remain permanent goals.
