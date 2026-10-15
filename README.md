# OBTAINER

OBTAINER is a Windows app for giving unsupported routers and MiFi devices a second life. Its new development plan focuses on four improvements: custom DNS, Wi-Fi user limits, automatic or manual APN settings, and a USSD page hosted on the router. Each feature depends on what the exact device and its software support. Firmware files, inspection, and supported OpenWrt builds are additional tools planned for later.

## Current status

**Rework planned — existing desktop foundation and passive discovery will be reused.**

Device Explorer is built to list USB devices, serial ports, and network adapters that Windows reports. It groups related interfaces when Windows provides matching identity information, updates its inventory as devices change, and lets users refresh it. The app also includes its original eight workspaces and saves light or dark appearance settings between sessions. Frontend and Rust checks pass, and the Windows debug build succeeds. Full desktop launch verification is pending because Windows reported that its embedded browser was already in use.

Device Explorer only reports what Windows can see. It cannot by itself identify a router's chipset or installed firmware, read its firmware, or prove that a device can be modified. OBTAINER does not yet extract firmware, analyze or customize images, create backups, or install firmware. These steps require verified methods for each device and hardware revision. The current app makes no changes to connected devices.

The [new master plan](RFS_MASTER_PLAN.md), [phased roadmap](docs/IMPLEMENTATION_ROADMAP.md), and [phase tracker](docs/PHASE_STATUS.md) follow the guideline supplied on 15 October 2026. Discovery is partial against the revised requirements. The new folder structure and customization workflows are planned; source files have not been relocated.

## What OBTAINER is for

OBTAINER aims to bring these steps together in one place:

- Discover candidate devices and explain the evidence for their identity and compatibility.
- Read settings through supported, authenticated management interfaces.
- Configure router DNS and DNS offered to connected clients separately.
- Change Wi-Fi user limits within verified device constraints.
- Suggest an APN from available operator information, with a manual option.
- Offer an authenticated router-local USSD page that can work while the desktop app is closed.
- Preview changes, preserve the previous configuration, verify results, and restore settings where supported.
- Keep firmware files and their history; later inspect proprietary images or build for explicitly supported OpenWrt targets.

These are project goals; they are not available in the current version.

## Project structure

| Folder | What it contains |
| --- | --- |
| `src/` | The screens and shared parts of the app. |
| `src-tauri/` | The Windows desktop app foundation and local settings. |
| `docs/` | Product plans, safety guidance, and project notes. |
| `workers/` | Notes for future firmware analysis helpers. |
| `public/` | App artwork and other visible assets. |
| `scripts/` | Helpers for working on the project. |
| `.github/` | Automated project checks. |

This table describes the current files. The proposed `apps/desktop/` layout is documented in the master plan and will be introduced during Phase 0.
