# OBTAINER

OBTAINER is a Windows app for understanding connected routers and MiFi devices and preserving firmware files. It can download selected official OpenWrt images, verify them, save them in Library and inspect their basic structure. A separate helper can build a DNS-customized image for one OpenWrt profile. Generic device extraction and flashing are unavailable.

## Current status

**Windows beta release sprint — 0.2.0-beta.1 candidate.** Library's Official Firmware popup supports exact OpenWrt 25.12.6 images for TP-Link Archer C7 v5/v2 and Ubiquiti UniFi AC Lite. A real firmware download, verification, Library save, analysis and restart have passed through the Windows interface. This downloads a release image; it does not extract the firmware installed on your connected device. [Current results and limitations](docs/RELEASE_SPRINT_VERIFICATION.md).

The analyzer identifies basic formats and saves a report without running or recursively unpacking the image. Connected-device support remains separate: unknown hardware stays unknown, and no firmware flashing is enabled. A real Archer C7 v5 DNS image build and offline readback passed through the separate [source helper](docs/DNS_BUILD_BETA.md); physical installation is unqualified and desktop Rebuild stays disabled. Installers are built and core Windows CI is green. Phase 7.5 is not marked complete. [Beta download and usage notes](docs/RELEASE_0_2_0_BETA_1.md).

The app finds relevant Windows device candidates, groups related USB interfaces, and explains what is known or still uncertain. You can save a router address, check whether its management page responds, and review a separate support assessment for DNS, Wi-Fi user limits, APN and USSD.

Device details include a Compatibility view explaining each feature's evidence, missing information and prerequisites. Saved label or software details stay clearly marked as user-reported. Profiles distinguish exact hardware revisions and firmware versions; an uncertain match keeps feature support unknown. The app does not change router settings.

A USB-connected comFi MG11 was checked through the real desktop app. Its discovery record and saved details survive restarts, but its exact hardware identity and four feature capabilities remain unverified. The supplied `LT800/` identifier and software version are preserved as reports, not proof of support. See the [Phase 3 verification report](docs/PHASE_3_VERIFICATION.md).

**Phase 4 adds a local firmware library.** Choose a file, review it, then store a separate copy. The app checks its contents, avoids storing identical copies twice, keeps import history, and lets you check stored files again or export a small information file. Jobs show real progress, cancellation and interrupted-operation results. Your original file stays untouched. Verification status and test results are in the [Phase 4 report](docs/PHASE_4_VERIFICATION.md).

The redesigned desktop has six sections: **Overview, Connected Devices, Library, Backup Manager, Logs and Settings**. Overview shows current work; finished operations and history belong in Logs. The interface uses blue accents with light, dark and system themes, and each page has an information button for help.

You can preserve a verified backup of a Library image and restore a copy with `_DEFAULT` added to its name. Restoring never installs firmware on a router. Delete removes a selected item from its list while preserving stored bytes and history for recovery. Storage changes show a copy plan, verify the files and take effect after restart; previous files stay preserved.

Device refresh, saved appearance, hot reload and restart recovery work. GUI implementation and verification details are in the [GUI report](docs/GUI_REDESIGN_VERIFICATION.md). General firmware extraction, desktop customization and installation remain future work; only the separate exact-target DNS helper is qualified at the software level. Storing a file does not establish that it is safe or compatible with a router.

The [master plan](RFS_MASTER_PLAN.md), [phased roadmap](docs/IMPLEMENTATION_ROADMAP.md), and [phase tracker](docs/PHASE_STATUS.md) retain the supplied development sequence and existing project structure.

Phase 6 adds an explicitly isolated **Simulation Mode** for a fictional router. Run `npm.cmd run dev:simulator`, then use Connected Devices to start the virtual lab. You can inspect DNS, Wi-Fi, APN and USSD, preview and test virtual DNS changes, and practice recovery from modeled failures. Virtual changes never affect connected hardware. Simulator settings reset when the app restarts. See [simulator verification](docs/PHASE_6_VERIFICATION.md) for current test status.

**Phase 7 adds a secure OpenWrt identity check and an inspection window.** It requires a router already running OpenWrt, existing key access and an independently checked server fingerprint. Configuration inspection and firmware-feasibility screens passed tests against a genuine OpenWrt virtual machine. Physical router support is still pending: the normal app cannot read settings from an unqualified device, and a regular proprietary MiFi does not qualify. Feasibility results explain possible firmware routes and missing requirements; they do not extract, rebuild or install firmware. See [Phase 7 results](docs/PHASE_7_VERIFICATION.md).

## What OBTAINER aims to do

- Find connected devices and explain what is known about their identity and compatibility.
- Read settings through supported, authenticated management methods.
- Configure router DNS separately from DNS offered to connected devices.
- Set Wi-Fi user limits within verified device limits.
- Suggest a mobile operator APN, while allowing manual settings.
- Provide an authenticated USSD page hosted by supported routers.
- Preview changes, save the previous configuration, verify results, and restore settings where supported.
- Keep firmware files and their history; later inspect proprietary files or build for explicitly supported OpenWrt devices.

Discovery, compatibility review, local file storage and approved OpenWrt identity checks are available now. Physical settings access still needs a qualified device profile; changing settings, firmware extraction and installation remain future goals. Device changes will require verified support and a safe recovery path.

## Project structure

| Folder | Purpose |
| --- | --- |
| `apps/desktop/` | Windows application, screens, assets, and Tauri backend. |
| `device-profiles/` | Versioned compatibility profiles, schema and evidence references. |
| `router-packages/` | Planned optional software hosted on supported routers. |
| `workers/` | Notes for planned firmware analysis and build helpers. |
| `tests/` | Planned shared fixtures, simulators, and integration tests. |
| `scripts/` | Project development and validation helpers. |
| `docs/` | Product plan, roadmap, status, safety, and verification notes. |
| `.github/` | Automated project checks. |

Existing fixtures, project notes, scripts, tools, and build evidence are retained for reuse. Future folders will be added when their implementation phases begin.

Setup and project commands are in the [Windows development guide](docs/DEVELOPMENT.md).
