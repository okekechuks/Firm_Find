# Foundation security model

Current command surface: read workspace snapshot; save appearance preferences; read the local Windows device inventory; request a passive inventory refresh. No command connects to a router, opens a serial session, sends a network probe, executes an external process, accepts a caller-controlled file path, runs caller-supplied SQL, extracts an archive, or writes firmware.

Settings use a closed theme enum and reject unknown JSON fields. SQL values are bound parameters. Snapshot and settings responses are checked at the frontend boundary. Backend errors return an actionable code and message; internal database errors stay in local structured logs.

Application ACL permissions are generated from an explicit command manifest and granted only to the local main window. Production CSP loads local assets and permits only Tauri IPC connections. The development CSP additionally permits the fixed localhost Vite server. Remote application code is not loaded.

Data paths originate from Windows local application data, then a fixed `OBTAINER` directory. Foundation logs and database are ordinary local files, not encrypted storage. Do not store secrets here. Future credentials require Windows protected storage.

The domain risk policy allows passive, read-only and derived-file operations and rejects device-write/destructive classifications in tests. It is a policy baseline; firmware or router configuration APIs do not exist in the current app. Every future operation must check risk, exact identity, capability, and authorization inside Rust. Configuration writes require a preview, explicit consent, a verified before-state, verification, and rollback or a disclosed recovery route. Installation is a separate user-confirmed milestone and must be profile-specific, integrity-checked, and tested for recovery; no generic flashing command.

WSL2 is not a complete sandbox for hostile binaries. Future parsing/extraction must use bounded isolated workers and copies of originals. Signing, updater verification, parser fuzzing, and hardware acquisition are not implemented in this foundation.
