# OBTAINER requirements and phase scope

> Historical requirements and phase ordering. The configuration-first [master plan](../RFS_MASTER_PLAN.md), [implementation roadmap](IMPLEMENTATION_ROADMAP.md), and [phase status](PHASE_STATUS.md), dated 15 October 2026, supersede the future milestones below. Revised phase 3 is profiles and capability detection; the earlier acquisition-first sequence is retained here as project history.

OBTAINER is the selected product name for Router Firmware Studio. Its purpose is to extend the useful life of unsupported routers and MiFi devices. The app should identify device hardware and installed firmware where evidence supports it, preserve and analyze firmware, and help users enable useful features on verified devices.

For the requested APN, Wi-Fi client-limit, DNS, and USSD improvements, use the safest supported control plane first: existing configuration API, authenticated OpenWrt UCI/ubus, vendor API, and supported modem MBIM/AT interface. Consider an OpenWrt overlay only when a configuration route cannot supply the feature. OEM binary modification is research-only. Hardware reprogramming is recovery-only. Do not promise a feature based on chipset guesses or a device name alone. Reading a setting does not prove a write method is safe.

Every configuration change requires device/profile and firmware identity, validated values, a user-visible preview, preflight, a vaulted before-state snapshot, explicit confirmation for the exact change, read-back and functional verification, and verified rollback or a clear recovery path. Device firmware installation is a later phase, with a profile-specific method, explicit user confirmation, integrity checks, and tested recovery. Never silently modify or flash hardware.

The user selected **project setup + desktop shell** on 14 October 2026. The foundation is complete. On 15 October 2026, the user clarified the product goal and requested Phase 2. Passive Windows inventory is implemented; Rust host-inventory and frontend checks pass. The native WebView smoke test could not launch because Windows reported that a WebView2 resource was already in use. Phase 2 makes no router configuration or firmware changes.

## Foundation acceptance

- Windows-only Tauri desktop entry point, standard user privileges.
- React navigation: overview, devices, firmware library, analysis, backup manager, firmware builder, jobs, settings.
- Rust IPC returns actual SQLite state and saves validated settings.
- Transactional migration, persistent preferences, and startup audit records.
- Clear loading and error states; truthful unimplemented workflows.
- Least privilege, local assets, no arbitrary process execution.
- Frontend lint, tests, build; native formatting, lint, tests and Windows installer verification.

Foundation automated acceptance **passed on the local Windows host**: native builds, startup, actual IPC, restart persistence, navigation, unit/component checks, and unsigned installer generation. See [verification evidence](VERIFICATION.md).

## Remaining milestones

3. Firmware acquisition from a verified device-specific read method and optional local-file import. Preserve original bytes, hash during streaming, and record provenance and integrity.
4. Bounded firmware analysis for structure, components, chipset evidence, and limitations. Label evidence and uncertainty; do not claim unsupported firmware facts.
5. Profiles for APN, Wi-Fi client limit, DNS, and USSD. Start with safe configuration APIs and authenticated OpenWrt/vendor/modem interfaces. Back up and verify configuration; provide preview, consent, rollback, and recovery guidance. Binary firmware patching stays research-only.
6. Further profile-gated, authenticated read-only acquisition for approved devices.
7. Optional isolated Linux analysis with bounded extraction.
8. Focused customization engine and supported OpenWrt overlay builds, producing separate verified artifacts.
9. Device-profile-gated firmware installation, verification, rollback, and recovery for hardware with a tested recovery method. No generic or automatic flashing.
10–12. Security, signing, hardware validation, beta, and production operations.

Phase 2 completion means Windows-reported hardware can be browsed in the desktop app and refreshed. It does not establish device support, chipset identity, firmware access, or ownership. Step 5 remains a future profile-gated capability: APN mode, Wi-Fi client limit, custom DNS, and USSD each need device-specific evidence, preview, consent, verification, and rollback. See [verification](VERIFICATION.md), [device support](DEVICE_SUPPORT.md), and the [architecture](ARCHITECTURE.md).
