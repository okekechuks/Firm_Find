# OBTAINER

OBTAINER is a Windows app intended to help owners keep unsupported routers and MiFi devices useful. It aims to help people understand what a device can do, then safely make supported changes such as custom DNS, Wi-Fi user limits, automatic or manual APN settings, and router-hosted USSD. Firmware storage, inspection, and builds for selected OpenWrt devices are later project goals.

## Current status

**Phase 0 is complete:** the existing desktop app is in the new project layout, root commands work, and the app has passed frontend, Rust, and Windows build checks. A clean dependency install from the shared lockfile also passed.

The app can list USB devices, serial ports, and network adapters that Windows reports. It groups related interfaces when Windows provides matching identity information, updates the inventory as devices change, and lets users refresh it. It also has eight workspaces and remembers light or dark appearance settings. This discovery is passive: it does not identify every router's chipset or firmware, change router settings, extract firmware, or install anything.

**Phase 1 is still in progress:** the native app window closes during startup because WebView2 reports `0x800700AA` (“The requested resource is in use”). The failure happens before the app connects to its Rust backend. This is recorded for Phase 1; a successful build alone does not verify a working window.

The [master plan](RFS_MASTER_PLAN.md), [phased roadmap](docs/IMPLEMENTATION_ROADMAP.md), and [phase tracker](docs/PHASE_STATUS.md) follow the supplied development guideline. Discovery remains partial against its requirements.

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
| `workers/` | Firmware analysis and build helpers. |
| `tests/` | Planned shared fixtures, simulators, and integration tests. |
| `scripts/` | Project development and validation helpers. |
| `docs/` | Product plan, roadmap, status, safety, and verification notes. |
| `.github/` | Automated project checks. |

Existing fixtures, project notes, scripts, tools, and build evidence are retained for reuse. Future folders will be added when their implementation phases begin.
