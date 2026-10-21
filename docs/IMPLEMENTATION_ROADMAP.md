# OBTAINER implementation roadmap

Revision: 15 October 2026. This roadmap uses the new guideline's phase numbers. It replaces the earlier acquisition-first roadmap. See [master plan](../RFS_MASTER_PLAN.md) for product scope and the target structure, and [phase status](PHASE_STATUS.md) for actual progress.

Planning deliverables do not implement the features below. Each phase closes only after its relevant frontend, Rust backend, persistence, tests, and runtime evidence are connected.

The owner's Phase 7 firmware objective is preserved in the proposed [firmware pipeline amendment](FIRMWARE_PIPELINE_ROADMAP.md): exact compatible acquisition, supported physical extraction, analysis/customization, rebuilding, qualified installation, verification and recovery remain permanent product goals. Explicit acquisition/extraction and installation milestones are proposed subject to owner approval, without renumbering phases 0–19. Existing accepted evidence remains unchanged.

## Phase 0 — Repository and environment audit

**Deliver:** Evidence-backed source inventory, Windows environment versions, Git/user-change protection, executed checks, native launch/IPC/persistence evidence, data-location inventory, naming assessment, migration recommendation, and `PHASE_0_AUDIT.md`. Retain the existing root npm/Cargo workspaces and `apps/desktop/` layout unless a verified need justifies a further move.

**Exit gate:** Existing work and application data survive. Root commands, lint, tests, Rust checks, and desktop build pass. Native launch, React rendering, IPC, SQLite integrity, and settings across restart are verified, or the project owner explicitly accepts an environmental verification limitation. Merely isolating a launch failure does not close Phase 0. Do not advance to Phase 1 before this gate is satisfied. Use the acceptance checklist in `PHASE_STATUS.md`.

## Phase 1 — Desktop foundation closeout

**Depends on:** 0.

**Deliver:** Reuse existing Tauri/React shell, appearance settings, SQLite, and IPC. Add missing error containment and reconnect/loading behavior where the audit demonstrates a gap. Prepare navigation for Devices, Customization Studio, Library, Backups, Activity, Compatibility, and Settings as real features arrive.

**Exit gate:** Standard-user development and bundled application launch, navigate, invoke Rust, persist settings, and restart reliably. Native smoke verification passes in the relocated structure. Preserve the existing least-privilege command surface.

## Phase 2 — Discovery and candidate identity

**Depends on:** 1.

**Deliver:** Retain SetupAPI/Configuration Manager and IP Helper inventory, correlation, notifications, bounded refresh, and fixtures. Add validated manual router address entry, management-endpoint candidates, evidence sources, unknown fields, and actionable connection diagnostics. Separate Windows-observed facts from router facts that require later authenticated inspection.

**Exit gate:** Real Windows inventory and manual candidate entry work. Fixtures cover duplicate interfaces, disconnect/reconnect, unknown hardware, and ambiguous identity. No broad scanning or model-support claims result from a gateway/USB match. Physical arrival/removal behavior has recorded native evidence.

## Phase 3 — Profiles and capability decisions

**Depends on:** 2.

**Deliver:** Versioned JSON profile schema, validation, deterministic matching, evidence/confidence, firmware and hardware-revision constraints, and reasoned decisions for all four features. Introduce the seven decision states in Rust and TypeScript together. Build the support matrix and compatibility explanation UI. Place synthetic profiles in the simulator/fixture namespace.

**Exit gate:** Missing, ambiguous, conflicting, or stale evidence cannot enable unsupported writes. Matching and decision tests cover each case. Every UI status explains its reason and requirements. A model name or OpenWrt label alone cannot grant all four features.

## Phase 4 — Persistence, jobs, and firmware library

**Depends on:** 3; reuse existing SQLite foundation.

**Deliver:** Additive migrations for device evidence/history, profiles/capability results, jobs, activity, and artifact metadata. Implement controlled file selection, bounded streaming import, SHA-256, atomic immutable content-addressed storage, deduplication, integrity rechecks, search/filter, and manifest export. Connect Library and Activity to real backend data.

**Exit gate:** Imports survive restart; duplicates are recognized; corruption is detected. Progress, cancellation, interrupted imports, errors, and database/filesystem consistency are tested. Imported files and device-acquired artifacts have distinct provenance. Firmware contents are never executed.

## Phase 5 — Adapter framework

**Depends on:** 3–4.

**Deliver:** Independent traits/contracts for connection, authentication, identity, capability probes, feature reads, planning, backup, apply, verification, restoration, and USSD. Add registry, typed errors, deadlines/cancellation, and deterministic mock adapter contract tests. Real adapters begin read-only.

**Exit gate:** A new adapter integrates without rewriting feature UI. Unsupported operations fail explicitly. Credentials stay outside plaintext SQLite/logs. The backend enforces capability and authorization boundaries independently of button state.

## Phase 6 — Router and modem simulator

**Depends on:** 5.

**Deliver:** Explicitly selected local simulator with identity, authentication, all four current configurations, and deterministic fault injection. Model disconnections, stale state, failed applies, delayed responses, verification failures, rollback failure, and recovery. Start with DNS as the first complete data path through UI, IPC, and adapter; add the other current-state reads to the same architecture.

**Exit gate:** Desktop connects to the simulator and displays all four current states. Tests reproduce success and failures without physical hardware. Simulation is visibly labelled and never presented as a detected physical device. Apply primitives are simulator-only until phase 8 supplies the shared transaction engine.

## Phase 7 — OpenWrt read-only connection

**Depends on:** 3, 5–6; requires an explicitly selected compatible device for hardware acceptance.

**Deliver:** Research current official interfaces; select a narrow firmware/version target. Implement authorized SSH or authenticated RPC, trusted endpoint identity, protected credentials, firmware/hardware identification, configuration reads, and per-feature probing. Read cellular/modem information only where the actual device exposes it. Produce evidence-based firmware acquisition/extraction, image/layout, build, installation and recovery feasibility; metadata must not masquerade as binary artifacts or executable support. See [firmware feasibility](PHASE_7_FIRMWARE_FEASIBILITY.md) and the proposed [pipeline gates](FIRMWARE_PIPELINE_ROADMAP.md).

**Exit gate:** Recorded inspection on the selected device, authentication/host-identity failure coverage, and evidence that no persistent configuration changed. Unavailable features remain disabled. If hardware is absent, report hardware acceptance pending and continue phase 8 against the simulator.

## Phase 8 — Backups and transaction engine

**Depends on:** 4–6; phase 7 is required to validate real-device backup/recovery adapters.

**Deliver:** Protected configuration snapshots with hashes and device/profile/firmware provenance, typed plans, semantic diffs, affected-service and connectivity previews, approval binding, durable transaction states, verification, bounded reconnect, rollback adapters, and recovery instructions. Separate plan creation from applying.

**Exit gate:** Simulator DNS traverses read → validate → preview → verified backup → approval → apply → verify → finish. Failure injection covers every stage, stale approval, mismatched identity, corrupt backup, crash/restart, and unsuccessful rollback. Outcomes never report false success. Physical writes remain unavailable without the required approval and verified adapter.

## Phase 9 — DNS customization MVP

**Depends on:** 8; real-device acceptance also requires 7 and a validated DNS profile.

**Deliver:** Complete DNS form and transaction flow. Separate router upstream DNS, LAN-advertised DNS, IPv4, IPv6, and resolver behavior. Implement only version-verified configuration mappings, affected-service reloads, read-back, functional checks, persistence, and restoration.

**Exit gate:** Full simulator success/failure workflow passes. On an authorized supported OpenWrt target, requested settings persist and verification/rollback evidence is recorded. Report the limits of what DNS tests establish. This is the first real configuration feature, not a prerequisite to start the simulator.

## Phase 10 — Wi-Fi client limits

**Depends on:** 9 and the shared transaction framework.

**Deliver:** Radio/SSID selection, effective limit/current clients where observable, evidenced valid ranges, version/driver-specific adapter, and Wi-Fi reload/reconnect handling.

**Exit gate:** Simulator handles invalid values and lost connectivity. A supported hardware/driver combination has evidence that the effective limit is enforced, not merely accepted as a stored setting. Preview and rollback account for the management connection being interrupted.

## Phase 11 — APN customization

**Depends on:** 9 and the shared transaction framework; live validation requires a suitable cellular target.

**Deliver:** Automatic suggestions from evidenced operator identity and a maintained, provenance-tracked APN source; manual override, supported authentication/IP-family fields, protected credentials, previous-setting preservation, reconnect, and verification.

**Exit gate:** Simulator covers missing/PIN-locked SIM, missing modem/operator data, invalid inputs, failed cellular connection, and restoration. Exact supported hardware has separate evidence for manual configuration and automatic selection. Automatic APN selection does not claim universal SIM compatibility or network unlocking.

## Phase 12 — USSD backend

**Depends on:** 5–6 and protected authentication/session infrastructure.

**Deliver:** Typed one-shot and interactive session state machine, simulated modem, verified modem adapter where available, validation, rate/size limits, cancellation, timeouts, and minimal sensitive-data retention. USSD session actions use their own authorization semantics rather than configuration rollback.

**Exit gate:** Simulator sessions, follow-ups, cancellations, timeout/error handling, concurrent-session rules, and unsupported-modem rejection pass. No arbitrary AT-command interface is exposed. Live requests that may incur charges require explicit authorization.

## Phase 13 — Router-local USSD administration

**Depends on:** 7–8, 12; deployment validation requires compatible OpenWrt hardware and modem.

**Deliver:** Optional `obtainer-agent` and LuCI package, authenticated router-local API/page, session controls, CSRF protections, management-interface restrictions, and desktop link. Installation is a separately approved device change.

**Exit gate:** Router-hosted page works with the desktop closed. Unauthorized/CSRF requests fail; WAN exposure is disabled by default. A compatible modem completes an authorized session. A desktop-hosted page cannot satisfy this phase.

## Phase 14 — Integrated Customization Studio

**Depends on:** 9–13; shared UI patterns start with phase 9.

**Deliver:** Consistent capability cards, forms, previews, approvals, progress, verification summaries, backups, recovery guidance, contextual help, and loading/error/unsupported states across all four features.

**Exit gate:** Each simulator workflow passes end to end and persists relevant history. Unknown or unsupported capabilities cannot expose an active apply path. Hardware-specific claims remain limited to individually validated features. This phase consolidates working vertical slices rather than connecting disconnected mock screens.

## Phase 15 — Supported OpenWrt builds

**Depends on:** 4, 14, validated target profiles, and a suitable optional Linux worker.

**Deliver:** Bounded official Image Builder/build-system integration for exact targets, hardware revisions, release versions, packages, and overlays. Retain logs, tool/input provenance, manifests, hashes, and separate output artifacts in the library.

**Exit gate:** A known supported target produces a validated, traceable artifact. Unsupported revisions and incompatible packages are rejected. Cancellation/resource limits and secret handling are tested. Image generation does not establish installation safety; qualified installation remains a separate proposed [firmware pipeline milestone](FIRMWARE_PIPELINE_ROADMAP.md), requiring exact compatibility, protected backups, approval, post-restart verification and recovery evidence.

## Phase 16 — Proprietary firmware feasibility

**Depends on:** 3–5, safe library ingestion, and isolated analysis workers as needed.

**Deliver:** Bounded metadata/container inspection, documented vendor adapter candidates, signing/packaging restrictions, and evidence-based per-feature feasibility reports. Any device-acquisition extension requires an exact validated read method and provenance.

**Exit gate:** Untrusted images remain isolated and non-executable; reports distinguish documented configuration access from speculative firmware modification. Unknown/signed/incompatible images do not become safely rebuildable. Clear unsupported reasons are useful outcomes.

## Phase 17 — Hardware qualification

**Depends on:** Relevant implemented features; accumulate evidence from phase 7 onward.

**Deliver:** Reproducible matrix by model, revision, firmware, management protocol, modem/driver, and individual feature. Record reads, backup, authorized writes, persistence, interruption, verification, restoration, and recovery results. Include firmware acquisition/extraction coverage, build compatibility and qualified installation/recovery evidence from the proposed [pipeline amendment](FIRMWARE_PIPELINE_ROADMAP.md); no simulation result substitutes for physical qualification.

**Exit gate:** Every advertised supported combination has feature-specific evidence. Untested combinations stay unknown or restricted. Hardware absence blocks only its validation; simulator results do not count as physical-device proof. At least one real OpenWrt configuration path and a compatible router/modem USSD combination satisfy the guideline's full-project gate.

## Phase 18 — Security and reliability qualification

**Depends on:** Implemented surfaces; security controls and tests start in their owning phases.

**Deliver:** Threat review, dependency review, targeted fuzzing, parser/resource hardening, credential and log-redaction review, router-local API security, migration/corruption/crash tests, accessibility, and resource profiling.

**Exit gate:** Critical findings resolved; recoverability and truthful outcomes demonstrated; sensitive material absent from logs; full regression suite passes. Review actual implemented attack surfaces rather than relying on the foundation risk-policy unit test alone.

## Phase 19 — Windows release

**Depends on:** Required product features plus 17–18; prototype installers already exist.

**Deliver:** Qualified installers, clean Windows installation, upgrade/data migration, uninstall/data-retention tests, prerequisites, accurate support list, user guide, release notes, reproducible build evidence, and signing/release procedures.

**Exit gate:** Standard-user installed app launches without a development environment and completes supported simulator/hardware workflows. Installer generation, installer execution, signing, and CI results are reported separately. WSL2 is optional for desktop startup.

## Execution process

Work one phase at a time, with a complete UI/backend/test slice. Preserve source moves as a separate mechanical step from feature changes. Reuse existing dependencies until a demonstrated need justifies additions. Validate current primary documentation when a device-specific mapping or external tool is selected.

Run only relevant checks for the change. For application implementation, the baseline commands are root `npm run lint`, `npm test`, `npm run build`, Rust format/Clippy/tests, a native desktop build, and applicable runtime smoke/integration checks. Phase 0 must preserve those entry points after migration. CI configuration is not evidence that a CI run passed.

After each implementation phase, update the tracker with phase/name, status, implemented behavior, important changed files, exact test commands/results, exact build command/result, manual verification steps, limitations, and next work. Planning-only work records its documentation validation and does not claim new application tests.

Hardware gates are per feature. Continue simulator or isolated worker work when physical hardware is unavailable, while retaining partial status on hardware acceptance. Stop for specific authorization before physical-device changes; elapsed time or an earlier general request does not approve a particular device operation.
