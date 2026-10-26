# Qualified DNS image build helper

This beta includes a Windows developer helper for **OpenWrt 25.12.6, ath79/generic, TP-Link Archer C7 v5 only**. It builds a new image with one or two upstream DNS addresses. It does not patch the imported image, change a connected router, install firmware or establish hardware compatibility. The desktop's Rebuild button remains disabled; this helper is a separate source-repository command.

## Requirements

- Windows with Python 3.11 or newer and Git for Windows GPG.
- WSL distribution named `Ubuntu`, running as a normal, non-root user. The verified environment uses Python 3.14, `make`, GCC/G++, `gawk`, `rsync`, `file`, `unzip`, `zstd`, `bubblewrap`, `prlimit` and `unsquashfs`. User and network namespaces must work. Other environments are unqualified.
- At least 4 GiB free on the chosen local work volume and sufficient free space for WSL's Linux filesystem. Keep removable build storage connected throughout the command.
- The exact official Archer C7 v5 sysupgrade image downloaded through Library's Official Firmware popup, and the exact ImageBuilder tool archive below. Preserve the original image.

The helper checks the retained signed release manifest and pinned signer, verifies both input and tool archive hashes, and refuses any other target or input. It does not accept arbitrary scripts or package selections.

## Inputs

Obtain the tool archive from [the official release directory](https://downloads.openwrt.org/releases/25.12.6/targets/ath79/generic/):

```text
openwrt-imagebuilder-25.12.6-ath79-generic.Linux-x86_64.tar.zst
SHA-256: 6f40746139c833336e67b03d5bdb0501d3a80fb789bf7e5b7a9b9984b83a5e23

openwrt-25.12.6-ath79-generic-tplink_archer-c7-v5-squashfs-sysupgrade.bin
SHA-256: 0ec9643bb9f43d65c689761d916a9e0c13bad5968656d74e516df010deb0be9d
```

From a checkout of this release branch, substitute your local paths:

```powershell
py -B scripts/build-openwrt-dns.py `
  --input "C:/firmware/openwrt-25.12.6-ath79-generic-tplink_archer-c7-v5-squashfs-sysupgrade.bin" `
  --tool-archive "E:/tools/openwrt-imagebuilder-25.12.6-ath79-generic.Linux-x86_64.tar.zst" `
  --dns 1.1.1.1 9.9.9.9 `
  --work-root "E:/OBTAINER-DNS-Builds"
```

Only global unicast IP literals are accepted. The fixed first-boot UCI defaults disable automatic upstream DNS on `network.wan` and set its DNS list. LAN-advertised DNS, Wi-Fi, APN and USSD are unchanged. The helper does not execute that defaults script on this computer or a connected router.

## Output and cancellation

Each invocation owns a new randomly named workspace. A successful invocation writes a new sysupgrade image and `build-result.json`, including exact output hash, source provenance, tool hash and verified DNS defaults readback. A generated image is `custom_build_not_official`, with **unknown installability**. No physical test is implied. To keep it in Library, import it as an unverified file; do not represent it as an official vendor image.

The trusted ImageBuilder runs with only its temporary workspace writable, package signatures enabled and official release repositories. The normal Linux user is mapped to uid 0 only inside a private user namespace so packages retain root ownership; all capabilities are dropped before running the builder. No host root privilege is used. A recorded, narrowly checked tool change limits SquashFS compression to one worker and 256 MiB, while retaining its format options and root ownership. The report records the makefile hashes before and after this resource change.

The original input firmware is not mounted into the builder. A separate offline, resource-limited namespace reads only the one generated defaults file from the output image, with one decoding worker and bounded queues; it does not execute image contents or recursively unpack them.

Ctrl+C requests cancellation. The helper stops and reaps its own worker processes; it never kills an unrelated WSL process. Time, memory, file and log limits apply. Failure workspaces/logs are retained for inspection; a failure is not a usable build. Successful Linux temporary files are removed only after ownership checks; Windows output and input remain.

There is no flash command in this workflow. Physical installation, MG11/proprietary rebuilding and the three other customization categories remain unavailable. OpenWrt documents WSL as an unsupported ImageBuilder environment; this helper's verification applies only to the recorded software environment and output inspection, not router operation.

References: [OpenWrt ImageBuilder](https://openwrt.org/docs/guide-user/additional-software/imagebuilder), [release signatures](https://openwrt.org/docs/guide-user/security/release_signatures). Actual build results belong in [the sprint verification report](RELEASE_SPRINT_VERIFICATION.md).
