# Phase 7 hardware qualification

Revision: 20 October 2026. **Physical-hardware verdict: PENDING.** The owner reports only a regular proprietary MiFi. No authorized compatible physical OpenWrt target is available for this phase.

This document separates physical qualification from genuine OpenWrt OS interoperability and controlled fixtures. Software/native acceptance is recorded in [Phase 7 verification](PHASE_7_VERIFICATION.md); this document does not predeclare it.

## Environment matrix

| Target | Identity/evidence category | Qualification status |
| --- | --- | --- |
| Disposable OpenWrt 25.12.6 x86/64 QEMU guest | Genuine OpenWrt OS; model `QEMU Ubuntu 26.04 PC (i440FX + PIIX, 1996)`; revision `r33311-981c978eff` | Controlled OS interoperability target; executed checks determine software acceptance |
| Controlled SSH/parser fixtures | Test-only peers and deterministic input | Security/error regression evidence only |
| Phase 6 Virtual Router Lab | Fictional simulator identity and authority | Simulator regression only; never physical-device evidence |
| Owner's regular proprietary MiFi / saved comFi MG11 | Proprietary target; owner-reported identity, revision/software evidence remains unverified | No new physical contact or OpenWrt qualification; four feature support states remain unknown |
| Physical compatible OpenWrt router | Not available | PENDING for every hardware-specific operation |

The compressed guest source image is `openwrt-25.12.6-x86-64-generic-ext4-combined.img.gz`, SHA-256 `f8538784d0288b4133a99ac2a68443e4922ffde6343e80df56a5882364b9d499`. Image provenance and virtual identity are not a physical device profile. [Official release directory](https://downloads.openwrt.org/releases/25.12.6/targets/x86/64/).

The earlier MG11 owner report included `LT800/` and software `MF69_AM802L_ZX_DD_N_241106_EU`. These remain user-reported, unverified values; the opaque software identifier is not parsed as a date or numeric release. No Phase 7 observation upgrades that evidence or grants DNS, Wi-Fi-limit, APN or USSD support.

## Physical support matrix

| Operation | Physical evidence | Status |
| --- | --- | --- |
| Discovery, selected endpoint/interface and arrival/removal correlation | Historical Windows inventory is retained; no new OpenWrt hardware qualification | PENDING |
| Independently verified SSH host identity and existing-key authentication | No compatible target | PENDING |
| Board, exact revision, firmware/build and feature-profile matching | No compatible target | PENDING |
| Configured upstream DNS, LAN-advertised DNS and runtime DNS inspection | No compatible target | PENDING |
| Wireless BSS/radio configuration, driver and station-count/limit evidence | Guest has no physical radio; no compatible target | PENDING |
| Cellular/APN and passive USSD capability inspection | Guest has no physical modem; no compatible target | PENDING |
| Storage/partition metadata and layout completeness | Virtual storage cannot qualify router flash/UBI topology | PENDING |
| Official compatible image acquisition | Candidate source is metadata, no device-qualified acquisition workflow | PENDING |
| Physical original-firmware extraction/backup | No method implemented or exercised | PENDING |
| Customized firmware build compatibility | Matching builder availability is not generated firmware | PENDING |
| Installation, restart verification and recovery | No installation or recovery attempted | PENDING |

No physical persistent configuration, firmware, calibration or modem state was intentionally changed by Phase 7 qualification because no physical target was contacted. This does not substitute for a future before/after physical inspection test.

## Smallest future hardware gate

An owner-authorized router already running the exact supported OpenWrt build is needed, with known model/hardware revision, selected management address/interface, independently verifiable SSH fingerprint, and an existing SSH public-key account. Qualification must not install packages, users or keys on the physical router as a testing prerequisite.

Once that target is available, record each result separately:

1. Exact model/revision/build, selected interface and management endpoint.
2. Independent fingerprint source, bounded authentication and identity-only handshake.
3. Separate approval for the fixed read-only queries; no broad network scan or shell surface.
4. Actual DNS/Wi-Fi/APN/passive-USSD fields, unavailable values and capability reasons.
5. Safe partition/storage metadata and source/build/installation/recovery prerequisites.
6. Negative credential, changed-host-key, wrong-identity, endpoint-substitution and stale-session results.
7. Cancellation, disconnect and process/session cleanup.
8. Before/after evidence for approved selected persistent settings, with secrets excluded.

A privileged SSH account can write independently of OBTAINER. Its use must be disclosed; the application still dispatches only reviewed inspection queries. Any extraction, configuration apply, service reload, package install, reboot, flashing or live USSD needs its later implemented gate and specific authorization.

## Progression rule

Gate A software acceptance and Gate B physical acceptance are independent. A successful genuine-guest transport/query test may support Gate A. It cannot close Gate B. With Gate A accepted and hardware still pending, Phase 8 may proceed against the simulator; real-device transaction/backup/recovery qualification remains pending.
