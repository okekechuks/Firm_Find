# Desktop foundation architecture

Tauri 2 hosts locally bundled React/TypeScript in Windows WebView2. Tailwind's Vite integration provides styling support; local CSS defines the visual system. Rust owns all application persistence. A single Cargo workspace member keeps the foundation small; domain and repository modules provide extension points without premature service boundaries.

`get_snapshot` returns version, schema version, data directory, settings, table counts and the latest 20 audit records. `save_settings` accepts only the typed theme preference, rejects unknown fields, and commits settings plus audit event in one transaction. TypeScript uses Zod to check both native responses and outgoing settings.

Repository requests run in Tauri's blocking worker pool and share a mutex-protected SQLite connection. Database initialization enables foreign keys, WAL, a busy timeout, and an atomic schema migration. A newer schema is rejected. Startup happens before the main window is ready; setup failures abort rather than fabricating a healthy workspace.

Theme selection persists in SQLite. Browser preview has no database, exposes no simulated success and permits only temporary theme preview. Hash navigation supports history and direct screen links. System theme tracks the operating system preference.

Daily JSON logs go to the generated application data folder. The nonblocking writer guard is retained for the application lifetime. Credentials and raw hardware identifiers are not collected.

Device and firmware counters come from reserved empty tables. Future milestones must extend migrations and add actual native workflows; counts do not establish support. Optional Linux workers receive a future versioned JSON protocol, never frontend shell strings.

The application command manifest opts into Tauri ACL enforcement. Only the local `main` window receives the two command permissions. No plugin shell, filesystem, network, or SQL permissions exist. See the official [capability documentation](https://v2.tauri.app/security/capabilities/) for the distinction between application command registration and ACL opt-in.
