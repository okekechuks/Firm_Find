# OBTAINER phase status

Audited: 15 October 2026. Phase numbers refer to the [revised roadmap](IMPLEMENTATION_ROADMAP.md), based on the newly supplied guideline. The earlier phase 3 meant firmware acquisition; revised phase 3 means profiles and capability detection.

**Phase 0: COMPLETE. Phase 1 readiness: READY.** The [corrective technical audit](PHASE_0_AUDIT.md) records current source, environment, commands, native execution, data protection, naming, and migration evidence. The existing `apps/desktop/` structure is retained. This audit implements no Phase 1 or future customization features.

## Baseline evidence

- Initial Git state was clean at `643ce9f2f12db6533f13b33542511b9eec01ef89`, branch `main`. The source relocation already existed in `2c07c61`; no new migration or cleanup occurred.
- Actual versions: Windows build `26200.9457`/25H2, Node `22.17.0`, npm `10.9.2`, Rust/Cargo `1.99.0`, MSVC Build Tools/Windows SDK, WebView2 and matching EdgeDriver. Optional WSL2/Ubuntu is available. Details are in the audit.
- Dependency validation used `npm ci --dry-run --ignore-scripts --offline` and `npm ls`, not a new install. TypeScript, root lint, all 10 frontend tests, production frontend build, Rust format/Clippy, all 11 Rust tests, and fresh native debug and optimized release builds pass.
- Normal-session debug and final release audits pass: React rendering, eight destinations, real `get_snapshot`/`get_discovery`/`refresh_discovery`, native inventory refresh, and existing settings across restart. The Windows token was non-administrator. Screenshots and logs are preserved.
- Restricted execution reproduces WebView2 `0x800700AA` before IPC; normal-session counterchecks succeed. This is not an unresolved host-runtime blocker and did not require a profile reset or owner waiver.
- Existing SQLite integrity is `ok`, schema 1, with no foreign-key violations. Saved preferences remain unchanged. Normal app startup/inventory auditing increased stored events/device rows; no user data migration or destructive cleanup occurred.
- Physical router qualification, native preference overwrites, installer execution/signing, and a remote CI run were not performed. Future-feature placeholders and missing modules are identified individually in the audit.

## Phase 0 acceptance checklist

Each checked item has current evidence in the linked audit. A recorded failed attempt establishes audit evidence, not application health; passing native normal-session counterchecks establish the launch result here.

- [x] Existing repository inspected — audit D.
- [x] Source inventory created — audit D, exact paths and scoped classifications.
- [x] Existing implemented features identified — audit D/G.
- [x] Existing placeholder features identified — audit D/G.
- [x] Windows development environment checked — audit E/F.
- [x] Git status inspected — initial clean `main` worktree, audit A/F.
- [x] User changes protected — no reset, cleanup, source move, profile reset, or data migration; audit B/I/J.
- [x] Existing test commands attempted — 10 frontend and 11 Rust tests pass, audit F.
- [x] Existing build commands attempted — frontend, native debug and optimized release pass, audit F.
- [x] Native Tauri launch attempted — restricted and normal sessions, audit H.
- [x] Desktop launch outcome recorded — normal debug/release pass; restricted failure retained, audit H.
- [x] Frontend-to-Rust IPC verified or blocker documented — real reads and refresh pass, audit F/H; native preference writes intentionally not attempted.
- [x] SQLite and persistent data locations identified — read-only integrity and settings comparisons, audit I.
- [x] Device discovery implementation inspected — Windows APIs, correlation and service files, audit D/G.
- [x] Device discovery test evidence recorded — native capture, fixture tests and runtime aggregate results, audit F/H.
- [x] Migration necessity evaluated — KEEP EXISTING STRUCTURE, audit J.
- [x] Product naming inconsistency evaluated — OBTAINER metadata retained; RFS is the reference name, audit G.
- [x] `docs/PHASE_0_AUDIT.md` completed — sections A–L.
- [x] `docs/PHASE_STATUS.md` updated accurately — current gate and evidence recorded here.
- [x] Phase 1 readiness determined — READY, audit L; no Phase 1 implementation claimed.

## Phase assessment

`Complete` requires the phase's current acceptance gate and evidence. `Partial` means useful existing implementation covers some requirements; it is not completion. `Not started` means no connected implementation for the phase's main workflow. Blockers are recorded with the affected phase.

| Phase | Status | Reusable work | Remaining acceptance work |
| --- | --- | --- | --- |
| 0: Repository/environment audit | Complete | Exact inventory/environment/commands/data/naming/migration report; checks and fresh debug/release builds pass; native normal-user launch, IPC and restart persistence verified. | None at the Phase 0 gate. Scoped unverified work remains recorded in the audit. |
| 1: Desktop foundation | Partial | Existing shell, themes, typed IPC and SQLite; current bundled native navigation, reads/refresh and existing-settings restart verification. | Verify development-mode launch; review and close demonstrated loading/error/reconnect gaps; evolve navigation as real features arrive. |
| 2: Discovery | Partial | Native USB/serial/network inventory, correlation, notifications, refresh, evidence, UI and fixtures; real native read/refresh verified in normal-session audits. | Manual management-address entry, richer candidate identity/diagnostics, and physical hot-plug/reconnect evidence. |
| 3: Profiles/capabilities | Partial | Four unknown feature fields and primitive evidence/confidence display. | Versioned profiles, exact matching, seven new reasoned statuses, evaluator, constraints, conflict tests, support UI. |
| 4: Storage/jobs/library | Partial | SQLite migration, settings, hashed device IDs, aggregate audit, reserved jobs/artifact tables. | Real jobs/history, controlled file import, immutable vault, hashing/dedup/integrity, manifests, library UI and restart/cancellation tests. |
| 5: Adapters | Not started | Discovery provider is reusable infrastructure, not a router adapter framework. | Contracts, registry, authentication, typed errors, protected credentials, mock adapter and contract tests. |
| 6: Simulator | Not started | Discovery fixtures exist; no interactive router/modem simulator. | Stateful local simulator, four feature reads, authentication, fault injection, desktop connection. |
| 7: OpenWrt read-only | Not started | No authenticated router management sessions. | Verified interface/version research, trusted endpoint, reads/probes, and selected hardware evidence. |
| 8: Backup/transactions | Not started | No configuration write APIs. | Snapshots, previews, approvals, durable execution, verification, reconnect, rollback/recovery and failure tests. |
| 9: DNS MVP | Not started | Feature name/status only. | Complete simulator DNS transaction; separate upstream/LAN semantics; authorized hardware validation. |
| 10: Wi-Fi limits | Not started | Feature name/status only. | Radio/SSID/range UI, verified driver mapping/enforcement, interruption and restoration tests. |
| 11: APN | Not started | Feature name/status only. | Operator evidence/source, auto/manual UI, adapter, credentials, reconnection, SIM/error cases and recovery. |
| 12: USSD backend | Not started | Feature name/status only. | Session state machine, simulated modem, supported adapter, authorization, validation/timeouts/cancellation. |
| 13: Router-local USSD | Not started | No router-side packages. | Authenticated router-hosted service/page, security tests, compatible hardware, operation with desktop closed. |
| 14: Studio integration | Not started | Existing shell and planned-screen placeholders. | Connected four-feature studio, shared transaction UX and end-to-end verification. |
| 15: OpenWrt builds | Not started | Builder placeholder and worker notes. | Worker, supported targets, packages/overlays, provenance/logs/manifests and validated artifacts. |
| 16: OEM feasibility | Not started | Analysis placeholder. | Isolated inspection, evidenced vendor routes and conservative feasibility reports. |
| 17: Hardware qualification | Not started | Windows host inventory test; no feature-qualified router profile. | Exact hardware/firmware/modem matrix, feature-by-feature persistence/interruption/recovery evidence. |
| 18: Hardening | Partial | CSP/ACL, typed validation, parameterized SQL, limited audit data, baseline risk tests. | Threat review and regression/fuzz/corruption/credentials/router-API/accessibility/resource qualification. |
| 19: Release | Partial | Historical unsigned NSIS/MSI creation and icons/metadata. | Final-feature installers, clean-machine install, upgrade/uninstall/data-retention, signing and release qualification. |

## Next execution order

1. Begin Phase 1 with development-mode launch verification and a bounded shell loading/error/reconnect review.
2. Close phase 1 launch verification and phase 2 discovery gaps.
3. Implement phase 3 profiles and capability decisions before device-management writes.
4. Build phases 4–6 into a durable library, adapter layer, and real simulator.
5. Implement phase 8 simulator DNS transactions; phase 7 may proceed with suitable hardware and must pass before real DNS changes in phase 9.

The first implementation checkpoint is a migrated, runnable desktop application with honest device/capability reporting. The first customization checkpoint is a complete simulator DNS workflow, including failure and rollback, not four disconnected forms.
