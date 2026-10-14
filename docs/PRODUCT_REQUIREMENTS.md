# OBTAINER requirements and phase scope

The user selected **project setup + desktop shell** on 14 October 2026. OBTAINER replaces the RFS product name. The master roadmap governs milestone numbering; the broader Phase 1 coding prompt in the research report is deferred.

## Foundation acceptance

- Windows-only Tauri desktop entry point, standard user privileges.
- React navigation: overview, devices, firmware library, analysis, backup manager, firmware builder, jobs, settings.
- Rust IPC returns actual SQLite state and saves validated settings.
- Transactional migration, persistent preferences, and startup audit records.
- Clear loading and error states; truthful unimplemented workflows.
- Least privilege, local assets, no arbitrary process execution.
- Frontend lint, tests, build; native formatting, lint, tests and Windows installer verification.

Acceptance is **pending native validation** until a Windows desktop build and launch have actually passed. A frontend build does not prove desktop completion.

## Later milestones

2. Native USB/serial/network inventory, correlation, hot-plug events, fixture replay.
3. Streaming firmware import, SHA-256, atomic content-addressed vault, provenance and integrity verification.
4. Bounded native format analysis and report export (MVP).
5. Verified device profiles and explicitly authorized metadata interrogation.
6. Profile-gated, authenticated read-only acquisition.
7. Optional advanced Linux analysis with bounded extraction.
8. Supported OpenWrt image generation into separate artifacts.
9–11. Security, signing, hardware validation, beta and production operations.

Hardware write, driver replacement, bootloader unlocking, identity modification, and automatic flashing are excluded. No later milestone is started in this deliverable.
