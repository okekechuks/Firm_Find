# OBTAINER phase status

Updated: **17 October 2026**, Africa/Lagos. Existing [roadmap](IMPLEMENTATION_ROADMAP.md) retained; no planning document was recreated.

**Phase 0: ACCEPTED (PASS). Phase 1: COMPLETE. Phase 2: COMPLETE (PASS). Phase 3: COMPLETE (PASS). Phase 3.1: COMPLETE (PASS). Phase 4: COMPLETE (PASS). Phase 5 readiness: READY; not started.** The [Phase 4 verification](PHASE_4_VERIFICATION.md), [evidence transcript](evidence/phase-4-2026-10-17.txt), [firmware library](FIRMWARE_LIBRARY.md), [job engine](JOB_ENGINE.md), and [vault](FIRMWARE_VAULT.md) document the latest work. Accepted [Phase 3/3.1 verification](PHASE_3_VERIFICATION.md), [device profiles](DEVICE_PROFILES.md), and [capability engine](CAPABILITY_ENGINE.md) remain retained.

## Phase 4 completion evidence

- Additive schema **3 → 4** preserves the original artifact/job tables and adds persistent metadata, import/integrity history and audit relationships. WAL-aware pre-migration backup retained; settings, saved router, profile versions and reported observations compare exactly. Original device/discovery/audit identities remain; computed compatibility cache refreshes normally.
- Real native Open dialog → React confirmation → durable job → 1 MiB streaming copy/SHA-256 → synchronized staging → no-overwrite content-addressed publication → final verification → atomic SQLite metadata completion. No arbitrary path IPC or source mutation. User imports remain passive files with unverified provenance and UNKNOWN format.
- Same/different filenames deduplicate by content; changed bytes remain distinct. Missing/corrupt blobs, Windows junctions/sharing conflicts, SQL failure after publication, orphan reuse, cancellation near commit and recovery are covered in isolated tests. Live tests use generated harmless binaries exclusively.
- One bounded worker, persistent progress, idempotent cancellation and failed/INTERRUPTED recovery work. Actual 256 MiB source copy reaches React progress, cancels without a partial entry, and a separately killed copying job recovers on restart. Known staging is empty after final checks; complete unreferenced blobs are retained conservatively when needed.
- Library search/filter/pagination, details, history, explicit integrity checks, real native manifest Save, persisted Jobs/Activity and actual Dashboard counts work through validated IPC. Development/recovery/production retrieve existing artifacts before reimporting; stale verified labels reset to not_checked.
- **53 frontend tests, 69 Rust tests and 64 final native acceptance checks pass.** Independent TypeScript, lint, formatting, Clippy, frontend production build and optimized Tauri build pass. Native console issue capture is empty; populated development/production screenshots inspected. Earlier failed dialog automation and its correction are recorded in the report.
- All nine destinations, HMR, discovery refresh, original storage/settings and Phase 3.1 native profile regression pass. Connected MG11 keeps all four features UNKNOWN and its software user-reported; no Apply action or router request/configuration change was introduced.
- Final SQLite integrity `ok`, no FK violations: one clearly named generated 1 MiB artifact, 20 import occurrences, 30 integrity records, 34 jobs (30 complete, two deliberate cancellations, two interrupted failures), zero unfinished jobs, 104 retained device/metadata rows. Original preference hash unchanged. No real MiFi firmware was used or overwritten.
- No critical Phase 4 blocker remains. Maximum-size/multi-filesystem/clean-machine qualification remains scoped; no extraction, analysis, execution, flashing or Phase 5 implementation was added.

## Phase 3.1 targeted hardening evidence

- Schema 1 retains legacy numeric behavior and adds explicit `opaque_exact` literal firmware matching. Unknown/malformed schemes and opaque ranges reject safely. Canonical bundled-profile hashes and stored profile rows remain unchanged; no database migration or registry/UI/discovery rebuild.
- Unique authoritative VERIFIED match now wins over weaker entries regardless of ordering; multiple VERIFIED profiles remain AMBIGUOUS and conflicting verified identity still blocks selection. Weak hints/reports remain visible and cannot grant authority or displace fresh verified evidence.
- **51 Rust tests, 44 frontend tests, 38 native checks pass.** TypeScript, lint, formatting, Clippy, frontend production and final native builds pass. All nine destinations, real React/Zod/Rust opaque-preview IPC, HMR, restart/recovery, settings persistence and empty console issue capture pass; native dev/production screenshots inspected.
- Initial native attempt stopped because MG11 was disconnected. Full suite passed after the owner reconnected USB. Same logical device and observation IDs retained; all four MG11 features stay UNKNOWN, software stays user-reported, adapters/Apply stay unavailable. No router settings changed.
- Read-only backup comparison preserves exact profile-version/observation/settings/saved-router/firmware/job rows and all original device/audit/metadata IDs. Schema remains 3, integrity `ok`, no FK violations, original preference hash unchanged. Original OpenWrt matching remains covered and passing.

## Phase 3 completion evidence

- Strict, versioned JSON profiles, sorted lookup, deterministic matching and source/evidence provenance are connected to real React/Rust IPC. Malformed/future schemas, duplicate keys/IDs, unsafe fields, invalid bounds and unresolved sources fail safely. Local validation remains an unverified preview.
- All four capabilities have independent assessment, readiness, reasons, sub-capabilities, prerequisites and evidence. Ambiguous, conflicting, missing or stale identity cannot select an authoritative profile. All device configuration adapters remain unavailable; no Apply control or hardware writes were added.
- Compatibility is the ninth navigation destination; device details use the same engine. Frontend loading/error/expiry and late-response guards are covered by regression tests.
- **42 frontend tests, 40 Rust tests, 35 native acceptance checks pass.** TypeScript, lint, Rust formatting/Clippy, frontend production build and fresh optimized Tauri build pass. Native development, interrupted-launch recovery, production, actual IPC, HMR and all nine destinations pass; captured console issues are empty. Development and production matrix screenshots were inspected.
- USB `1286:4E31` remains the same MG11 candidate logical ID. `LT800/` and `MF69_AM802L_ZX_DD_N_241106_EU` are saved through the actual React form as user-reported evidence and survive development/production restarts without duplicates. Hardware revision, chipset/modem and four feature capabilities remain unknown; no false retail identity or support claim is made.
- Additive SQLite schema 3 retains settings, saved router, firmware/jobs and all original device/audit/metadata IDs in the same data directory. Integrity is `ok`, foreign-key violations are empty, original preference hash is unchanged. Two profile versions, two reported observations and one evaluation are persisted. A WAL-aware pre-change backup remains locally available.
- Physical feature enforcement, authenticated identity adapters, runtime package checks, router-hosted USSD and firmware work remain later-phase qualification. The curated Archer C7 reference was checked against primary documentation, not physical hardware.

## Phase 2 completion evidence

- Existing Windows discovery/correlation/notifications retained and extended with observed interface paths, manufacturer/compatible IDs, prefixes, DNS, LUIDs and default routes. Two relevant candidates remain distinct from generic/software/Bluetooth records.
- Real manual address save, ownership confirmation, bounded read-only HTTP/HTTPS checks, detailed results, cancellation, retry/remove and historical state are connected through validated React/Rust IPC. Late reads and superseded operations cannot overwrite newer UI/probe results.
- User-reported comFi MG11 at `192.168.100.1`: USB VID `1286`, PID `4E31`, five correlated functions and RNDIS index 49. A fresh HEAD returned HTTP 200 in 84 ms; model/firmware identity remains unverified.
- Direct physical USB trace captured removal at 14:15:14 UTC and reconnection at 14:15:29 UTC, with the same logical ID and five functions restored. The initial missed watch is documented; the later recorded trace is the qualification evidence.
- Final native suite passes 38 checks across development, interrupted-launch recovery and production. A separate physical watch passes four checks. Console warning/error/exception captures are empty; screenshots were inspected.
- TypeScript, lint, formatting, Clippy, **32 frontend tests**, **22 Rust tests**, frontend production build and fresh native release build pass.
- Additive SQLite schema 2 preserves all original settings/inventory/audit IDs and the existing storage directory. Integrity is `ok`; original preference hash is unchanged. One user-supplied MG11 entry remains; fixture entries were removed through the app API.
- No router configuration/firmware writes, vendor authentication, capability profiles or customization features were implemented. Additional phone/VPN/vendor/IPv6 hardware qualification remains scoped, not universal support.

## Accepted Phase 1 evidence (historical)

- Existing Tauri development command, initial React/Rust connection, reload and all eight destinations pass in the native Windows app. Production uses the fresh optimized executable.
- Fixed failed/stale workspace state, initial-inventory metadata refresh, unsaved-theme loss, confirmed-save reporting, subscription/read error handling, initial discovery retry, and view error containment. Existing Rust/discovery/storage modules retained.
- Final checks pass: TypeScript, lint, 24 frontend tests, Rust formatting/Clippy, 11 Rust tests, frontend production build and native Tauri build.
- Native lifecycle acceptance passes: HMR with unchanged app/Vite identities, three discovery refreshes per session with unique IDs, clean shutdown, reopen, controlled interruption before native rendering, and recovery. Final console warning/error/exception capture is empty.
- Changed appearance saved in production is read in development from the same database; development restores the original and production confirms it. Final SQLite integrity is `ok`, schema 1, preference hash unchanged from the pre-test baseline.
- Final known counts: 48 Windows inventory records, zero firmware artifacts/jobs. These are not verified routers; feature support remains UNKNOWN and flashing UNSUPPORTED.
- Repository/data paths, lockfiles, schema, identity, original commands and accepted plans were preserved. Detailed failures, counterchecks, regression coverage and remaining hardware/release limits are in the report.

## Accepted Phase 0 evidence (historical)

These records describe the previously accepted Phase 0 run, before the Phase 1 changes above.

- Initial checkout was clean on `main` at `a8e51a92ad48bc40d2acae8b19a742aa6fb31bfe`. `apps/desktop/` already existed in committed history; **KEEP EXISTING STRUCTURE**.
- Node `22.17.0`, npm `10.9.2`, Rust/Cargo `1.99.0`, installed MSVC target, VS C++ tools, Windows SDK and WebView2 were checked locally. No package/prerequisite install was required.
- Typecheck, root lint, all 10 frontend tests, production frontend build, Rust formatting/Clippy, all 11 Rust tests and a fresh optimized Tauri release build pass.
- Final normal-session native audit exits 0: native React rendering, all eight destinations, actual `get_snapshot`/`get_discovery`/`refresh_discovery`, device refresh and unchanged settings across restart. Actual inventory: 48 Windows records, `ready`, watcher registered, no warnings; no verified-router count is claimed.
- Restricted automation failed before verified render. First normal run passed rendering/navigation but asserted while discovery was `starting`. Only the native runner's bounded readiness wait was corrected; subsequent lint/native audit pass. Both failed attempts remain recorded.
- SQLite integrity remains `ok`, schema 1, no foreign-key violations; theme `system` and preference fingerprint unchanged. Normal startup/inventory updates changed audit rows 21 → 26 and device cache 54 → 48.
- Application code, storage path, schema, locks, product identity, roadmap and existing evidence were preserved. New changes are the two reports, separate evidence artifacts and the native-runner wait. No restructuring, destructive cleanup, data migration or Phase 1 work occurred.

## Phase 0 requirement classifications

VERIFIED requires executed evidence or direct source inspection appropriate to that requirement. FAILED attempts are retained in the audit; BLOCKED would mean missing required evidence. A future feature is not made working by passing this baseline.

| Requirement | Classification | Evidence |
|---|---|---|
| Inspect current repository | VERIFIED | Audit D/F; clean initial Git state. |
| Inventory frontend and Rust | VERIFIED | Audit D, exact paths/functions. |
| Identify commands and frontend IPC | VERIFIED | Audit D/H; four commands, real native reads/refresh. |
| Inspect Windows discovery | VERIFIED | Audit D; actual APIs and Windows capture test. |
| Identify database/persistent storage | VERIFIED | Audit D/I; source and actual directories. |
| Identify existing firmware functionality | VERIFIED | Audit D/G; placeholders/reserved storage explicitly identified. |
| Identify tests/mocks/missing modules | VERIFIED | Audit D; fixture evidence separated from native evidence. |
| Inspect Git/protect user work | VERIFIED | Audit C/I; no reset/cleanup/source or data move. |
| Verify Node/npm/dependencies | VERIFIED | Audit E/F; actual versions/workspace listing. |
| Verify Rust/Cargo/target | VERIFIED | Audit E/F; local MSVC toolchain. |
| Verify MSVC/SDK/WebView2 | VERIFIED | Audit E; files, complete installation, build/render outcomes. |
| Frontend typecheck/lint/tests/build | VERIFIED | Audit F; all exit 0, 10 tests. |
| Rust checks/tests/compilation | VERIFIED | Audit F; all exit 0, 11 tests. |
| Tauri release build | VERIFIED | Audit F; fresh optimized executable and SHA-256. |
| Native executable/window/React | VERIFIED | Audit H; final exit 0, screenshot visually inspected. |
| Native navigation | VERIFIED | Audit H; eight destinations. |
| Actual IPC/discovery invocation | VERIFIED | Audit H; reads and React Refresh → Rust revision increase. |
| Restart settings/data integrity | VERIFIED | Audit I; native comparison and read-only SQLite before/after. |
| Evaluate migration/naming | VERIFIED | Audit G/J; keep existing structure and OBTAINER identity. |
| Update evidence and determine readiness | VERIFIED | Audit L and this tracker; PASS / READY. |
| Source/data migration | NOT APPLICABLE | No move necessary. |
| Firmware operations/hardware qualification/installers/remote CI/Phase 1 dev launch | NOT APPLICABLE | Outside this baseline; not represented as verified functionality. |

No unresolved Phase 0 requirement is BLOCKED or FAILED. Raw failed attempts are recorded separately; the final requirement outcome follows the successful countercheck, not the existence of these documents.

## Phase assessment

`Complete` requires the phase's current acceptance gate and evidence. `Partial` means useful existing implementation covers some requirements; it is not completion. `Not started` means no connected implementation for the phase's main workflow. Blockers are recorded with the affected phase.

| Phase | Status | Reusable work | Remaining acceptance work |
| --- | --- | --- | --- |
| 0: Repository/environment audit | Complete (PASS) | Current inventory/environment/commands/data/migration evidence; checks and fresh release build pass; native render/navigation/IPC/discovery/restart verified on 16 October. | None at the Phase 0 gate; scoped unverified work is recorded in the audit. |
| 1: Desktop foundation | Complete | Native development/production render, navigation, IPC, cross-mode UI settings, HMR, lifecycle recovery, clean console, 24 frontend/11 Rust tests; report linked above. | No critical foundation blocker; future feature navigation evolves in its own phases. |
| 2: Discovery | Complete (PASS) | Native topology/evidence, correlation, classification, saved manual targets, bounded checks, cancellation, restart persistence; real MG11 HTTP and physical USB reconnect; 32 frontend/22 Rust tests and native suites pass. | No critical blocker. Broader phone/VPN/vendor/IPv6 hardware qualification is explicitly unverified. |
| 3: Profiles/capabilities | Complete (PASS), including 3.1 | Versioned registry, numeric/opaque firmware matching, trusted-profile precedence, reasoned independent feature assessments, native IPC/UI and preserved reported evidence. | No critical Phase 3/3.1 blocker; authenticated identity and physical feature qualification remain later work. |
| 4: Storage/jobs/library | Complete (PASS) | Schema 4, safe native import, streamed SHA-256 vault, deduplication, integrity/history, bounded persistent jobs, cancellation/recovery, manifests, backend library queries and real UI. 53 frontend/69 Rust/64 native checks pass. | No critical blocker. Broad filesystem/maximum-size/installer qualification remains unverified; see Phase 4 report. |
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

1. Phase 4 is complete; Phase 5 typed read-only adapter contracts and registry are ready when requested.
2. Keep unknown support until exact device/version/profile evidence supports a decision. Current MG11 model is user-supplied.
3. Continue broader hardware qualification as suitable devices become available.
4. Reuse the completed durable library and jobs for Phase 5 adapters, then Phase 6's local simulator.
5. Implement simulator transactions before authorized hardware configuration changes in later phases.

This tracker records completed Phase 4 work and retains historical phase evidence. Phase 5 and device customization have not begun.
