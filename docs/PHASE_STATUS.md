# OBTAINER phase status

Audited: 15 October 2026. Phase numbers refer to the [revised roadmap](IMPLEMENTATION_ROADMAP.md), based on the newly supplied guideline. The earlier phase 3 meant firmware acquisition; revised phase 3 means profiles and capability detection.

This turn produced planning documents only. Application sources, package layout, device operations, and stored user data were not changed. Existing uncommitted work was retained. No root `AGENTS.md` or prior master-plan file was found during the repository audit.

## Baseline evidence

- Code inspected: React navigation/screens/contracts, Rust discovery/application/domain/storage modules, SQLite migration, Tauri configuration/permissions, scripts, and Windows CI definition.
- Fresh tool-version checks: Node `v22.17.0`, npm `10.9.2`, Cargo `1.99.0`.
- Previous verification record: frontend lint/build and 10 tests; Rust format, Clippy, workspace check and 11 tests; native Windows debug build passed. These commands were not rerun for this documentation-only planning turn.
- Latest native UI verification remains blocked: WebView2 creation returned `0x800700AA` before the smoke test reached IPC. Previous foundation startup evidence predates the latest discovery work.
- Prior records confirm MSVC/Windows SDK, local Rust tools, WebView2, and development installers. Their full installation/runtime state was not revalidated during this turn. WSL2 and compatible router/modem availability remain unverified.
- Source layout remains root `src/` and `src-tauri/`. The `apps/desktop/` tree is planned, not migrated.
- Planning validation passed: phases 0–19 each appear once in the roadmap and tracker; all 21 local documentation links resolve; the archived guideline's SHA-256 matches the supplied file; documentation `git diff --check` is clean.

## Phase assessment

`Partial` means useful existing implementation covers some requirements; it is not completion. `Not started` means no connected implementation for the phase's main workflow. Blockers are recorded alongside the phase they affect rather than hiding progress elsewhere.

| Phase | Status | Reusable work | Remaining acceptance work |
| --- | --- | --- | --- |
| 0: Audit and structure | Partial | Source/config/document audit, tool versions, target layout, roadmap and tracker. | Preserve baseline, perform structure migration, repair paths/CI, revalidate environment and commands. |
| 1: Desktop foundation | Partial | Tauri/React navigation, themes, real IPC, SQLite, typed boundaries; historical launch/installers. | Revalidate launch after migration and latest changes; close current WebView2 runtime blocker and error-handling gaps. |
| 2: Discovery | Partial | Native USB/serial/network inventory, correlation, notifications, refresh, evidence, UI, fixtures. | Manual management-address entry, richer candidate identity/diagnostics, native UI and physical hot-plug evidence. |
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

1. Finish phase 0 migration as a mechanical change with preserved root commands and application data.
2. Close phase 1 launch verification and phase 2 discovery gaps.
3. Implement phase 3 profiles and capability decisions before device-management writes.
4. Build phases 4–6 into a durable library, adapter layer, and real simulator.
5. Implement phase 8 simulator DNS transactions; phase 7 may proceed with suitable hardware and must pass before real DNS changes in phase 9.

The first implementation checkpoint is a migrated, runnable desktop application with honest device/capability reporting. The first customization checkpoint is a complete simulator DNS workflow, including failure and rollback, not four disconnected forms.
