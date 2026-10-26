# OBTAINER 0.2.0 beta 1

Windows x64 beta for preserving and inspecting router firmware files. The application runs locally without an AI session. These application binaries are unsigned.

## Available

- Six-page Windows desktop, connected-device inventory, saved preferences and local Library.
- Official Firmware popup: acquire exact OpenWrt 25.12.6 images for Archer C7 v5, Archer C7 v2 or UniFi AC Lite board profile, with explicit model/revision confirmation, pinned signed-release provenance, bounded download, SHA-256 verification and immutable storage.
- Analyze Firmware popup: save a genuine bounded format/metadata report and reopen it after restart. It does not execute or recursively unpack images.
- Existing file import, integrity checks, provenance export, backups, job cancellation and interrupted-job recovery.
- Separate source-repository DNS build helper for the exact Archer C7 v5 OpenWrt profile. Requires the documented WSL tools; this is not the desktop Rebuild button. [Build guide](https://github.com/okekechuks/Firm_Find/blob/v0.2.0-beta.1/docs/DNS_BUILD_BETA.md).

## Limits

Downloading an official release image does not extract the firmware installed on a connected device and does not verify that device's identity. Model/revision and installability still require independent qualification. MG11 firmware extraction and authenticated physical qualification remain unavailable.

Generated DNS images are custom builds, not official releases; they are unqualified for physical installation. Physical DNS, Wi-Fi limits, APN, USSD, generic proprietary repacking and flashing are unavailable. The four customization categories remain visible with unsupported desktop actions disabled. Phase 7.5 is not declared complete.

## Start and preserve your files

Choose either the Windows x64 setup executable or MSI. The portable `obtainer.exe` is the same application without an installer and requires Microsoft's WebView2 runtime. A `SHA256SUMS.txt` accompanies published assets; compare your download before running it. Unsigned beta binaries may trigger Windows warnings.

The application keeps its existing data location and identity. If an older Library needs schema 6, startup requests explicit migration approval. A verified backup precedes the transaction; refusal performs no migration. Existing Library files are not moved. Keep your own backups when testing a beta.

No router settings are changed or firmware installed by these release workflows. Existing hardware approvals remain separate; this release does not authorize credentials or privileged device operations.

Source, verification and build instructions are retained in this repository and release assets independently of the AI conversation. [Actual results, commands and limitations](https://github.com/okekechuks/Firm_Find/blob/v0.2.0-beta.1/docs/RELEASE_SPRINT_VERIFICATION.md). The release checksum manifest identifies the exact source commit and CI run used to produce its binaries.
