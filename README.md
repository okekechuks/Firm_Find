# OBTAINER

OBTAINER is a Windows app being built to help people identify devices, keep copies of their firmware, and understand what is inside those files. It is intended to make firmware work easier to follow and safer to manage.

## Current status

**Early foundation — the desktop app is running, while its main device and firmware features are still being built.**

The app currently opens as a Windows desktop workspace. You can navigate its eight sections, use the overview and activity areas, and change the app's light or dark appearance. Your appearance choice is saved when you close the app. The other workspaces are placeholders for features still being built.

OBTAINER does not yet find or connect to devices, import or inspect firmware, create backups, or build firmware images. Those capabilities are planned. The current app does not change device settings or install firmware. Preserving original firmware remains a core project goal; the app cannot yet make or store firmware backups.

## What OBTAINER is for

The project aims to bring these tasks together in one place:

- Recognize supported routers and other devices.
- Keep original firmware files organized and preserved.
- Inspect firmware and present findings clearly.
- Manage backups and prepare firmware builds for supported devices.

These are project goals; most are not available in the current version.

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
