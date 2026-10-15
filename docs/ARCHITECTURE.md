# Application architecture

This document describes existing code. The [master plan](../RFS_MASTER_PLAN.md) defines the proposed structure and configuration-first direction; planned modules are not implemented merely because they appear there.

Tauri 2 hosts locally bundled React/TypeScript in Windows WebView2. Tailwind's Vite integration provides styling support; local CSS defines the visual system. Rust owns all application persistence. A single Cargo workspace member keeps the foundation small; domain and repository modules provide extension points without premature service boundaries.

`get_snapshot` returns version, schema version, data directory, settings, table counts and the latest 20 audit records. `save_settings` accepts only the typed theme preference, rejects unknown fields, and commits settings plus audit event in one transaction. `get_discovery` returns the latest Windows inventory snapshot. `refresh_discovery` queues a bounded rescan. Rust emits only a snapshot revision in Tauri events. TypeScript validates native responses at the boundary.

Repository requests run in Tauri's blocking worker pool and share a mutex-protected SQLite connection. Database initialization enables foreign keys, WAL, a busy timeout, and an atomic schema migration. A newer schema is rejected. Startup happens before the main window is ready; setup failures abort rather than fabricating a healthy workspace.

Theme selection persists in SQLite. Browser preview has no database, exposes no simulated success and permits only temporary theme preview. Hash navigation supports history and direct screen links. System theme tracks the operating system preference.

Daily JSON logs go to the generated application data folder. The nonblocking writer guard is retained for the application lifetime. Credentials and raw hardware identifiers are not written to logs or audit records. Local Device Explorer may display raw Windows identifiers; its inventory database keys use stable SHA-256 digests.

Device counts come from reconciled Windows inventory. Firmware counters remain reserved and empty. A dedicated Rust worker reads SetupAPI/Configuration Manager and IP Helper snapshots. Plug-and-play and IP-interface callbacks enqueue bounded refresh signals; they do no device work. The worker registers notifications before initial enumeration, coalesces change events, and rescans every 30 seconds. A failed scan retains the last successful snapshot. Database records store hashed IDs, display names, and connection state; audit entries contain aggregate arrival/removal counts only. Optional Linux workers receive a future versioned JSON protocol, never frontend shell strings.

The revised roadmap starts with supported configuration interfaces and a simulator, followed by shared transactions and DNS, Wi-Fi client-limit, APN, and USSD workflows. USSD administration must run on the router. Supported OpenWrt image builds and conservative OEM feasibility follow later. Configuration writes require exact profile evidence, preview, preflight, a verified before-state, explicit consent, verification, and rollback or recovery. Firmware installation is outside the revised phases 0–19.

The application command manifest opts into Tauri ACL enforcement. Only the local `main` window receives permissions for the four registered Rust commands and the event listen/unlisten operations. No plugin shell, filesystem, network, or SQL permissions exist. See the official [capability documentation](https://v2.tauri.app/security/capabilities/) for the distinction between application command registration and ACL opt-in.
