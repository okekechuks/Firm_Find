# Capability decisions

`profiles::gather_evidence` reuses Phase 2 logical-device grouping and saved-router associations. It collects Windows USB/product/manufacturer observations, physical identity, gateway evidence, saved labels and unauthenticated HTTP status without opening device handles, authenticating or making new network requests. Saved user observations are merged by logical device ID. The IPC command `get_device_compatibility` evaluates on a worker and rejects a result if discovery changed before publication.

Each evidence record has its own source, collection method/time, reliability, verification status, endpoint/interface association, hardware/firmware applicability, conflicting observation IDs and stale flag. Windows manufacturer strings describe driver evidence; they do not become verified device manufacturers. A successful HTTP response says nothing about an accessible configuration API.

The matching engine normalizes safe identity fields and USB hex formatting without collapsing hardware revisions. Firmware identifiers retain literal case/whitespace for conflict detection. Numeric firmware constraints use the known three-part scheme; explicit opaque constraints compare exact strings and never infer numeric ordering or dates. Every candidate reports matched, missing and conflicting/excluded fields with reasons. A unique exact match needs fresh, strong, independently verified manufacturer/model, explicit revision, firmware family/version constraints and required identity tokens anchored to one trusted management endpoint. Existing Windows and user data cannot meet that standard by themselves. No command accepts caller-supplied verified evidence. Trusted-response fixtures are test-only; production authenticated identity adapters are later phases.

| Confidence | Meaning |
| --- | --- |
| VERIFIED | One authoritative curated profile matches all exact identity/revision/firmware constraints. |
| PROBABLE | Strong model evidence exists, but exact applicability is incomplete or provenance is untrusted. |
| CANDIDATE | Hints or incomplete identity suggest a record; no authoritative selection. |
| AMBIGUOUS | Two or more authoritative VERIFIED profiles match, or multiple weaker candidates remain without a verified winner; none is selected. |
| NO_MATCH | No identifying evidence, invalid evidence, or exact constraints exclude candidates. |
| CONFLICTING | Fresh strong identity claims contradict each other; both remain visible. |

Phase 3.1 ranks one authoritative VERIFIED match ahead of weaker candidates, regardless of ID or input ordering. Weaker entries and USB hints remain visible with their original reasons, but cannot invalidate or replace that winner. Unmatched optional hint/corroboration rules do not demote otherwise verified identity. Fresh verified firmware takes precedence over weak firmware reports, which remain in the evidence list; without verified firmware, reports alone cannot verify applicability. Multiple VERIFIED profiles remain ambiguous even if their declared capabilities differ. Conflicting verified identity evidence still blocks selection. With no VERIFIED match, all capability assessments remain unknown; the existing probable/candidate/ambiguous fallback is retained.

Evidence older than five minutes, over 30 seconds in the future, or belonging to a disconnected device cannot verify a match. A report also expires in the frontend; changing device/discovery generation or receiving an error withdraws the corresponding result. Stale asynchronous responses cannot replace a newer device's matrix. Profile matches never establish ownership or permission.

Each feature has a separate hardware/software assessment (`SUPPORTED`, `UNSUPPORTED`, `UNKNOWN`) and readiness list. Readiness can require authentication, packages, a firmware build, manual verification, or indicate experimental/unavailable status. All Phase 3 decisions include `NOT_IMPLEMENTED`, `adapterImplemented: false` and `canApply: false`. Missing adapters never imply hardware is unsupported. No Apply control, device write, modem command, configuration adapter, firmware modification or flashing is implemented.

| Feature | Independently evaluated details |
| --- | --- |
| DNS | Upstream resolvers, LAN IPv4, LAN IPv6, forwarding service, persistence. |
| Wi-Fi user limit | AP capability, managed BSS, actual limit, verified numeric range, persistence, reload and management connectivity risk. No invented range or enforcement claim. |
| APN | Modem presence/interface, reading current APN, manual settings, automatic selection, operator identity and reconnect. Manual support never implies automatic support. |
| USSD | Modem identity/interface, one-shot and interactive requests, response handling/cancellation, router service and router-hosted web UI. Modem capability never implies a ready router page. |

Without a unique verified profile, all four features and every sub-capability remain unknown. With an exact match, only documented feature-specific claims apply; prerequisites remain separate, and unknown sub-capabilities remain unknown. Curated DNS behavior does not unlock Wi-Fi/APN/USSD or claim runtime packages have been verified.

The Compatibility page and connected-device details show the same Rust result: assessment/readiness table, per-feature reasons, next required evidence, range, source references, candidates/exclusions, provenance and retained evidence/history. Zod validates Rust response shapes and rejects any response claiming an available adapter or Apply permission. Unit fixtures exercise supported, unsupported and mixed sub-capabilities; these synthetic outcomes are never attributed to the physical MG11.

Commands added to the existing Tauri ACL: `get_profile_registry`, `validate_device_profile`, `get_device_compatibility`, `record_device_observation`. Only the last saves bounded user text locally. Registry previews cannot change the active registry or provenance. Schema 3 preserves Phase 2 rows/settings in `%LOCALAPPDATA%\OBTAINER\obtainer.db`.
