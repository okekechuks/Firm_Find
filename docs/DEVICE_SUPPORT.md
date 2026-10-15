# Device support

Phase 2 is implemented. Windows Device Explorer enumerates present USB and serial device records and network adapter records, correlating them through Windows device ancestry and adapter identity. Change notifications trigger refreshed snapshots. This is passive host inventory only: OBTAINER opens no USB, serial, router, or firmware access session, and sends no network probes.

Windows-reported instance IDs and hardware IDs appear in local Device Explorer details. Stable SHA-256 identifiers are used as local inventory database keys. Raw hardware IDs and network addresses do not appear in audit records. An inventory match does not verify a router model, chipset, installed firmware, flash access, or feature support.

Router/MiFi detection labels are confidence-scored hints from observed USB/serial/network relationships. Gateway presence proves neither router identity nor ownership. No device-specific profile is currently approved. Firmware backup stays UNKNOWN; APN, Wi-Fi user limit, custom DNS, and USSD support all stay UNKNOWN. Installation is unavailable in the current app.

Later validation candidates from the guidelines: GL.iNet GL-MT3000, Huawei E8372, TP-Link Archer AX23. These are candidates only; model names alone never grant extraction, feature-write, backup, build, or installation capabilities. Record the exact hardware revision, chipset, firmware, control plane, recovery path, and evidence before assigning support.

Capabilities must independently report SUPPORTED, UNSUPPORTED, UNKNOWN, REQUIRES_AUTH, REQUIRES_TOOL or EXPERIMENTAL. UNKNOWN never means supported.
