# OBTAINER phase status

Audited: 15 October 2026. Phase numbers refer to the [revised roadmap](IMPLEMENTATION_ROADMAP.md), based on the newly supplied guideline. The earlier phase 3 meant firmware acquisition; revised phase 3 means profiles and capability detection.

Phase 0 is complete. The existing app source is under `apps/desktop/`; root npm commands, the Cargo workspace, and the shared npm lockfile were updated. A fresh offline npm install, root frontend checks, Rust checks/tests, and the relocated Windows debug build passed. Existing fixtures, screenshots, project notes, tools, and application data were retained.

## Baseline evidence

- Code inspected: React navigation/screens/contracts, Rust discovery/application/domain/storage modules, SQLite migration, Tauri configuration/permissions, scripts, and Windows CI definition.
- Fresh tool-version checks: Node `v22.17.0`, npm `10.9.2`, Cargo `1.99.0`.
- Current verification: offline `npm ci` (scripts disabled) passed with 297 packages and no reported vulnerabilities; root lint, 10 frontend tests, and production build passed; Cargo format, Clippy, and all 11 Rust tests passed; Tauri Windows debug build passed after the clean install.
- Native UI smoke after migration still fails before IPC: WebView2 reports HRESULT `0x800700AA` (“The requested resource is in use”), and the native WebDriver session closes. The app panic confirms the same WebView2 startup error recorded before migration; tracked as a Phase 1 blocker.
- This turn confirmed the local Rust toolchain and MSVC/Windows SDK by building the app. WebView2 is present but fails to create a new view. WSL2 and compatible router/modem availability remain unverified.
- Source is now under `apps/desktop/`; the root keeps the shared lockfile, Cargo workspace, scripts, docs, and tools.
- Planning validation passed: phases 0–19 each appear once in the roadmap and tracker; all 20 local documentation links resolve; the archived guideline's SHA-256 matches the supplied file; documentation `git diff --check` is clean.

## Phase assessment

`Partial` means useful existing implementation covers some requirements; it is not completion. `Not started` means no connected implementation for the phase's main workflow. Blockers are recorded alongside the phase they affect rather than hiding progress elsewhere.

| Phase | Status | Reusable work | Remaining acceptance work |
| --- | --- | --- | --- |
| 0: Audit and structure | Complete | Audit and baseline; app relocated, root npm workspace/Cargo membership/wrappers updated; clean install and frontend, Rust, and Windows build checks passed. | None. Native WebView2 launch failure is isolated and tracked under Phase 1. |
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

1. Start phase 1 by resolving or containing the WebView2 startup failure and verifying a native launch.
2. Close phase 1 launch verification and phase 2 discovery gaps.
3. Implement phase 3 profiles and capability decisions before device-management writes.
4. Build phases 4–6 into a durable library, adapter layer, and real simulator.
5. Implement phase 8 simulator DNS transactions; phase 7 may proceed with suitable hardware and must pass before real DNS changes in phase 9.

The first implementation checkpoint is a migrated, runnable desktop application with honest device/capability reporting. The first customization checkpoint is a complete simulator DNS workflow, including failure and rollback, not four disconnected forms.
