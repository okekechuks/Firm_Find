# Firmware pipeline roadmap amendment

**Approved placement — 22 October 2026:** the owner has authorized Phase 7.5 between 7 and 8, with subphases A–F for identification, vendor recognition, acquisition strategy discovery, genuine-byte acquisition/Library integration, isolated analysis and physical qualification. This supersedes the earlier proposed placement below. No phase renumbering or Phase 8 implementation. [Architecture](PHASE_7_5_ARCHITECTURE.md) and [current results](PHASE_7_5_VERIFICATION.md). Sprint A does not yet qualify an executable acquisition route.

Proposed: 20 October 2026, from the owner's Phase 7 implementation requirements. **Proposed milestone placement is subject to project-owner approval.** Existing phases 0–19 keep their numbers and historical acceptance evidence. This document does not authorize physical extraction, configuration writes, firmware installation or live USSD during Phase 7.

The permanent product workflow is:

**Detect → Identify → Acquire/extract → Analyze → Customize → Back up → Rebuild or apply configuration → Install → Verify → Recover.**

Direct configuration transactions and firmware customization remain distinct supported pathways. The application must eventually expose both in Customization Studio when the exact device capabilities permit them. Metadata feasibility reports must never be presented as working acquisition, extraction, image generation or installation.

## Placement without renumbering

| Existing position | Proposed gate or retained phase | Dependencies |
| --- | --- | --- |
| After Phase 7 inspection/feasibility, before the relevant firmware build workflow | **Firmware acquisition/extraction milestone**, parallel to configuration-focused Phases 8–14 where independent | Phase 3 exact identity/profiles, Phase 4 vault/jobs/provenance, Phase 5 adapters, Phase 7 real inspection; device-specific authorization and evidence |
| Existing Phase 15 | **Supported OpenWrt firmware builds**, with explicit artifact/input qualification | Existing Phase 15 dependencies plus an accepted compatible-source acquisition route and target-specific build profile |
| After Phase 15 validated output, before advertising firmware installation support | **Qualified firmware installation milestone** | Accepted source/artifact validation, protected backups, exact target compatibility, dedicated installation/recovery adapter, explicit owner approval and physical qualification |
| Existing Phase 16 | **Proprietary firmware feasibility**, retaining device-specific acquisition/build/install restrictions | Existing safe analysis requirements and exact vendor evidence; proceeds independently where no unsupported claim results |
| Existing Phase 17 | **Hardware qualification**, accumulated from Phase 7 onward | Each relevant implemented configuration or firmware route; no simulator substitution |
| Existing Phases 18–19 | Security/reliability qualification and Windows release | Required accepted routes plus honest support matrix and release evidence |

These are dependency gates, not replacement phase numbers. Acquisition can be qualified for an official image before physical extraction is qualified. A later supported route must retain its own evidence category and cannot inherit success from a different route.

## Firmware acquisition/extraction milestone

Implement verified official image acquisition and exact compatible-image validation first where a trustworthy source exists. Add device-specific physical extraction only where a qualified authorized read method can establish useful coverage.

Required behavior:

- Resolve a constant approved publisher/source route from exact target/revision/release evidence. Check artifact format, integrity and authenticity; record manifests and source provenance.
- Store acquired/extracted originals immutably in the existing vault, with device/profile/build/source hashes and clear provenance. A downloaded image and a physical dump remain different artifact types.
- Use persistent jobs, bounded streaming, progress, cancellation, disk/resource limits, interrupted-operation recovery and device-specific errors.
- Define exact readable ranges and exclusions, bootloader/calibration/per-device-secret policy, coverage and completeness. Never expose an arbitrary raw-device command surface.
- Distinguish firmware bytes from configuration snapshots and metadata reports. A raw dump must carry installability unknown until a validated image conversion/installation route proves otherwise.
- Provide separate approvals for official network acquisition and physical device reads where required; reuse existing endpoint/identity/session authorization.

**Exit gate:** At least one genuine compatible acquisition route completes with validated binary bytes, integrity/authenticity/provenance, restart/cancellation/error evidence and preserved original storage. Physical extraction requires separate successful real-hardware evidence; mock or VM metadata cannot close that gate. No fake firmware file or copied metadata report qualifies.

## Existing Phase 15 — Supported OpenWrt firmware builds

Extend the existing exact-target ImageBuilder/build-system phase with:

- Accepted compatible inputs, exact release/target/subtarget/profile/hardware-revision and image type.
- Versioned package and overlay selections; credentials excluded from logs and public manifests.
- Bounded worker execution, reproducibility requirements, deterministic configuration where feasible, resource limits, progress and cancellation.
- Target-specific flash/image size constraints and format/metadata/integrity checks.
- Build logs, builder/tool hashes, input/source manifests, package provenance and separate immutable output artifacts.
- Feature-specific validation: a router-local USSD page/package, for example, still needs supported modem interfaces and authenticated LAN service behavior.

**Exit gate:** Produce and validate an actual compatible firmware image for at least one selected supported target. Repacking an unknown binary, an available builder download or a successful configuration read does not qualify. Image generation alone does not enable installation.

The selected Phase 7 source publishes a matching ImageBuilder, profile manifest and multiple image variants. This is a researched candidate, not proof that OBTAINER has built a compatible image. [OpenWrt 25.12.6 x86/64 release tools](https://downloads.openwrt.org/releases/25.12.6/targets/x86/64/).

## Qualified firmware installation milestone

Implement a dedicated supported device-specific installation adapter only after its prerequisites are met:

- Exact board, revision, current firmware, output format, image metadata and size eligibility; reject stale/conflicting identity and changed endpoint/host trust.
- Protected original/configuration backups and verified integrity, with honest completeness and recovery limitations.
- A concrete reviewed installation/recovery plan, power and connectivity prerequisites, irreversible-step disclosures and explicit user approval bound to the exact image/device/operation/expiry.
- Bounded transfer and validated image staging through a documented supported method. No forced compatibility bypass or generic unsupported-device flashing.
- Durable states for disconnect, interruption, restart and unknown outcome. A lost connection cannot be reported as successful installation.
- Fresh post-restart host/firmware identity checks, feature behavior verification and qualified restoration/recovery guidance.

**Exit gate:** Authorized installation and post-restart verification succeed on a qualified physical device, with documented recovery requirements and negative/interruption evidence. Official image availability, a VM boot or an upgrade dry-run cannot establish physical installation/recovery support.

## Existing Phase 16 — Proprietary firmware feasibility

Retain evidence-based per-device research: documented vendor acquisition/configuration/recovery routes, bounded image analysis, container/partition formats, signing/encryption constraints and feature feasibility. Unknown encrypted/signed images do not become rebuildable merely because their contents can be listed.

No arbitrary proprietary firmware unlock/repack/sign/install claim is permitted. The regular MiFi owned by the user needs its own exact model/revision/software/chipset/interface evidence. OpenWrt support cannot be inferred from USB IDs, labels, HTTP success, a commercial model alias or an opaque firmware identifier.

**Exit gate:** Accurate useful per-device evidence and explicit unsupported/unknown reasons; any additional executable route must pass its own acquisition/build/installation/hardware gate.

## Existing Phase 17 — Hardware qualification

Extend the support matrix to include acquisition source, extraction method/coverage, input/output compatibility, actual builds, installation, post-restart verification and recovery, alongside DNS, Wi-Fi limits, APN and USSD.

Each claim records exact model, hardware revision, firmware/build, management transport, host-key policy, profile, storage, wireless driver and modem where relevant. A configuration route can pass while extraction remains unknown. Read-only inspection, configuration writes, firmware installation and recovery each have separate approvals and evidence.

**Exit gate:** Every advertised combination has actual feature-specific evidence. At least one qualified physical route must satisfy any firmware operation advertised as supported. Hardware absence leaves its gate pending while simulator and independent worker implementation may proceed.

## Status and future decisions

This amendment preserves extraction, rebuilding, installation and recovery as product goals. Current Phase 7 remote dispatch remains inspection-only. Placement and acceptance gates above are proposals for owner adoption; implementation does not silently schedule or execute them. The [master plan](../RFS_MASTER_PLAN.md), [implementation roadmap](IMPLEMENTATION_ROADMAP.md) and [phase tracker](PHASE_STATUS.md) reference this amendment so the firmware pipeline remains visible.
