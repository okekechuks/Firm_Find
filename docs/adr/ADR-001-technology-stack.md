# ADR 001 — Windows desktop stack

Status: selected from supplied guidelines for the user-authorized desktop foundation.

Use Tauri 2, React, TypeScript, Rust and SQLite. Build x64 Windows with MSVC and WebView2. Keep domain, persistence and commands in one Rust application until measured complexity warrants splitting crates. Use narrowly scoped IPC and local assets.

WSL2 is optional future tooling. It is not a prerequisite for launching the desktop shell. No background Windows service or default administrator elevation is introduced.
