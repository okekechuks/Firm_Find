# OBTAINER

OBTAINER is a Windows app intended to help owners keep unsupported routers and MiFi devices useful. It aims to help people understand what a device can do, then safely make supported changes such as custom DNS, Wi-Fi user limits, automatic or manual APN settings, and router-hosted USSD. Firmware storage, inspection, and builds for selected OpenWrt devices are later project goals.

## Current status

**Phase 3 device profiles and compatibility review are complete.** The app finds relevant Windows device candidates, groups related USB interfaces, and explains what is known or still uncertain. You can save a router address, check whether its management page responds, and review a separate support assessment for DNS, Wi-Fi user limits, APN and USSD.

A new Compatibility workspace explains each feature's evidence, missing information and prerequisites. Saved label or software details stay clearly marked as user-reported. Profiles distinguish exact hardware revisions and firmware versions; an uncertain match keeps feature support unknown. The app does not change router settings.

A USB-connected comFi MG11 was checked through the real desktop app. Its discovery record and saved details survive restarts, but its exact hardware identity and four feature capabilities remain unverified. The supplied `LT800/` identifier and software version are preserved as reports, not proof of support. See the [Phase 3 verification report](docs/PHASE_3_VERIFICATION.md).

The desktop foundation, nine destinations, device refresh, saved appearance, hot reload and restart recovery work. Firmware storage/import workflows come next in Phase 4. Firmware extraction, customization and installation remain future work.

The [master plan](RFS_MASTER_PLAN.md), [phased roadmap](docs/IMPLEMENTATION_ROADMAP.md), and [phase tracker](docs/PHASE_STATUS.md) retain the supplied development sequence and existing project structure.

## What OBTAINER aims to do

- Find connected devices and explain what is known about their identity and compatibility.
- Read settings through supported, authenticated management methods.
- Configure router DNS separately from DNS offered to connected devices.
- Set Wi-Fi user limits within verified device limits.
- Suggest a mobile operator APN, while allowing manual settings.
- Provide an authenticated USSD page hosted by supported routers.
- Preview changes, save the previous configuration, verify results, and restore settings where supported.
- Keep firmware files and their history; later inspect proprietary files or build for explicitly supported OpenWrt devices.

Discovery and compatibility review are available now. Reading and changing device settings, firmware extraction and installation remain future goals. Device changes will require verified support and a safe recovery path.

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
