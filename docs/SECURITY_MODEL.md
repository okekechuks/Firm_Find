# Foundation security model

Current command surface: read workspace snapshot; save typed appearance preferences. No command accesses a router, executes an external process, accepts a file path, runs SQL, extracts an archive, or writes firmware.

Settings use a closed theme enum and reject unknown JSON fields. SQL values are bound parameters. Snapshot and settings responses are checked at the frontend boundary. Backend errors return an actionable code and message; internal database errors stay in local structured logs.

Application ACL permissions are generated from an explicit command manifest and granted only to the local main window. Production CSP loads local assets and permits only Tauri IPC connections. The development CSP additionally permits the fixed localhost Vite server. Remote application code is not loaded.

Data paths originate from Windows local application data, then a fixed `OBTAINER` directory. Foundation logs and database are ordinary local files, not encrypted storage. Do not store secrets here. Future credentials require Windows protected storage.

The domain risk policy allows passive, read-only and derived-file operations and rejects device-write/destructive classifications in tests. It is a policy baseline; there is currently no device-operation API to enforce or bypass. Every future operation must check risk, capability, identity and authorization inside Rust.

WSL2 is not a complete sandbox for hostile binaries. Future parsing/extraction must use bounded isolated workers and copies of originals. Signing, updater verification, parser fuzzing, and hardware acquisition are not implemented in this foundation.
