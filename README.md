# OBTAINER

OBTAINER is a Windows app intended to help owners keep unsupported routers and MiFi devices useful. It aims to help people understand what a device can do, then safely make supported changes such as custom DNS, Wi-Fi user limits, automatic or manual APN settings, and router-hosted USSD. Firmware storage, inspection, and builds for selected OpenWrt devices are later project goals.

## Current status

**Phase 2 discovery and connection foundation is complete.** The app finds relevant Windows device candidates, groups related USB interfaces, and explains what is known or still uncertain. You can add a router address, save it, and explicitly check whether its management page responds. Checks are read-only; the app does not try passwords or change router settings.

A USB-connected comFi MG11 was tested: its management address responded, unplug/reconnect restored the same device record, and saved details survived app restarts. Its model name remains user-supplied; a responding webpage does not prove model, chipset or firmware identity. See the [Phase 2 verification report](docs/PHASE_2_VERIFICATION.md).

The Windows desktop foundation, eight workspaces, device refresh, saved appearance, hot reload and restart recovery remain working. Device profiles and supported-feature decisions come next in Phase 3. Firmware extraction, customization and installation remain future work.

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

These are project goals, not features available in the current version. Changes to connected devices will require verified device support and a safe recovery path.

## Project structure

| Folder | Purpose |
| --- | --- |
| `apps/desktop/` | Windows application, screens, assets, and Tauri backend. |
| `device-profiles/` | Planned versioned device and capability profiles. |
| `router-packages/` | Planned optional software hosted on supported routers. |
| `workers/` | Notes for planned firmware analysis and build helpers. |
| `tests/` | Planned shared fixtures, simulators, and integration tests. |
| `scripts/` | Project development and validation helpers. |
| `docs/` | Product plan, roadmap, status, safety, and verification notes. |
| `.github/` | Automated project checks. |

Existing fixtures, project notes, scripts, tools, and build evidence are retained for reuse. Future folders will be added when their implementation phases begin.

Setup and project commands are in the [Windows development guide](docs/DEVELOPMENT.md).
