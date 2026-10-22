# OBTAINER phase status

Updated: **21 October 2026**, Africa/Lagos. Existing [roadmap](IMPLEMENTATION_ROADMAP.md) retained; no planning document was recreated.

## Independent review corrections — 21 October

**Software corrections: PASS. Physical OpenWrt qualification: still PENDING.** [Correction report](POST_REVIEW_CORRECTIONS.md) records the eight reported findings and five smaller findings, reproduction failures, design decisions and exact checks. Session retention, unchanged discovery revisions, asynchronous Windows hostname resolution, unavailable relocated-Library startup, effective directory leases, non-destructive backup deduplication, exact hash lookup, readiness deduplication, asynchronous simulator status, idle polling and rejected-refresh cancellation are corrected. The existing Phase 7 remote Ethernet/Wi-Fi selection path was retained and its regression extended; host NIC protection remains enforced.

**Current regression results:** 132 frontend tests, 199 standard Rust tests, 200 native-profile Rust tests, both explicitly run genuine OpenWrt OS tests and 147 native checkpoints pass. TypeScript, lint, formatting, both Clippy profiles, frontend production build and both Windows release builds pass. Actual development/recovery/production windows, GUI Cancel/retry, missing-SIM error followed by DNS preview on the same session, pinned SSH, HMR, backup restore, absent relocated storage and reconnection are verified. [Native evidence](evidence/post-review-native-2026-10-21.json) retains results; [console capture](evidence/post-review-console-2026-10-21.json) is empty. Ordinary database/WAL/settings/saved-device/vault preservation is checked against the pre-correction baseline.

No new phase started. Proprietary MG11 software remains user-reported, all four physical feature assessments remain UNKNOWN, and physical extraction/configuration/flashing/live USSD remain disabled. Earlier phase entries below retain their historical counts.

## Phase 7 — software PASS; hardware PENDING

Existing OpenWrt contract now uses strict pinned SSH, existing key authentication, identity-only bootstrap and separate single-use read approval. Eight fixed UCI/ubus/partition-metadata queries, typed observations, firmware-feasibility reports and a Connected Devices inspection dialog preserve the existing six-page app. Schema 1 is retained; schema 2 permits exact OS-only inspection without inventing manufacturer/revision or granting feature/write support. Expiry, discovery invalidation, all-host-address exclusion, failure cleanup, picker admission/cancellation and stale publication were hardened during review. [Implementation](PHASE_7_IMPLEMENTATION.md) and [verification](PHASE_7_VERIFICATION.md) record the exact scope.

**Final checks:** 130 frontend tests, 192 standard Rust tests, 193 native-profile Rust tests, 2 explicitly executed genuine OpenWrt OS tests and 137 native checkpoints pass. TypeScript, lint, formatting, both Clippy profiles, frontend production build and both optimized Windows executables pass. Actual native development/recovery/release GUI, independent console pin, key dialog, separate approvals, real SSH reads, GUI Cancel/retry, HMR and retained simulator/storage regressions pass; console issues are empty. Full normal DB/WAL/rows/settings/saved-device/vault fingerprints remain identical. Owned native processes and QEMU stopped. [Native evidence](evidence/phase-7-native-2026-10-20.json), [genuine inspection](evidence/phase-7-openwrt-inspection-2026-10-20.json) and screenshots are linked in the report.

**Physical hardware: PENDING.** Genuine target was checksum-verified OpenWrt 25.12.6 x86/64, build r33311-981c978eff, in a restricted QEMU guest. It has no physical radio/modem qualification. Owner has only a regular proprietary MiFi; no request or configuration change was made to it. The exact inspection profile is sealed inside native-test mode. Ordinary production can perform approved pinned identity inspection but has no qualified physical read target yet; unknown targets cannot receive configuration authority. Extraction, rebuilding, installation, recovery, physical writes and live USSD remain unimplemented/disabled. [Research](PHASE_7_OPENWRT_RESEARCH.md), [feasibility](PHASE_7_FIRMWARE_FEASIBILITY.md), [hardware qualification](PHASE_7_HARDWARE_QUALIFICATION.md) and [future firmware pipeline](FIRMWARE_PIPELINE_ROADMAP.md) retain the remaining goals.

**Phase 8 readiness: READY FOR SIMULATOR ONLY.** Phase 8 has not started. The prior phase acceptance entries below retain their original historical counts and readiness statements.

Final export-tooling follow-up: schema generation now handles the refined frontend contract and emits separate schema-1/schema-2 documents. Schema 1 remains byte-identical; version/scope/UNKNOWN assertions and lint pass. No runtime source or executable changed after native qualification.

**Phase 6 post-review correction: PASS. Phase 7: READY, not started.** Both busy-flag defects were reproduced with failing tests and corrected with scoped operation ownership. Final results: **165 Rust tests per profile, 113 frontend tests, 130 native checkpoints**, both optimized Windows executable builds and formatting/Clippy/TypeScript/lint pass. Actual GUI Cancel followed by reconnect/read passes in development, recovery and production. Normal database/WAL/settings/saved-device/vault fingerprints remain identical. [Current correction report](PHASE_6_VERIFICATION.md) records all fixes, initial failures, commands and [evidence](evidence/phase-6-post-review-2026-10-20.txt).

**Phase 0: ACCEPTED (PASS). Phase 1: COMPLETE. Phase 2: COMPLETE (PASS). Phase 3: COMPLETE (PASS). Phase 3.1: COMPLETE (PASS). Phase 4: COMPLETE (PASS). Phase 4.1: COMPLETE (PASS). GUI desktop foundation: COMPLETE (PASS). Phase 5: COMPLETE (PASS). Phase 6: COMPLETE (PASS). Phase 7 readiness: READY.** Latest results: [Phase 6 verification](PHASE_6_VERIFICATION.md), [simulator implementation](PHASE_6_IMPLEMENTATION.md), [architecture](SIMULATOR_ARCHITECTURE.md), [Phase 6 evidence](evidence/phase-6-2026-10-20.txt) and [isolated native testing](NATIVE_TESTING.md). Accepted [Phase 5 verification](PHASE_5_VERIFICATION.md), [adapter implementation](PHASE_5_IMPLEMENTATION.md), [GUI redesign verification](GUI_REDESIGN_VERIFICATION.md), [Phase 4.1 verification](PHASE_4_1_VERIFICATION.md), [Phase 4 verification](PHASE_4_VERIFICATION.md) and [Phase 3/3.1 verification](PHASE_3_VERIFICATION.md) remain retained.

## Phase 6 router and modem simulator

**COMPLETE (PASS).** Explicit isolated development/release activation exposes one fictional stateful router inside Connected Devices with a persistent Simulation Mode label. The six destinations, physical device grid/details, Windows discovery, profiles, library/jobs and settings remain intact. Phase 5 cold-start authentication, endpoint/identity validation, separate read authorization, read contracts and resource cleanup are reused. Backend-private provenance scopes virtual mutations; physical writes, extraction, flashing and live USSD stay disabled.

DNS read/edit/validation/preview/snapshot/apply/readback/independent verification/restore passes through actual React/Rust controls, including mismatch and failed rollback/reset-required results. Two-radio Wi-Fi limits, synthetic APN/modem/operator/SIM states and interactive scripted USSD are state-backed. Deterministic fault scenarios cover stale authority, endpoint/identity conflict, bounded timeout/cancellation, service/resolver/modem failures, disconnection and recovery. Live identity changes revoke existing grants/cached results; cellular restart recovers only after scheduled virtual time.

Post-review hardening gives requests, scenario selection and time controls the same exclusive RAII guard. Cancel/reset/stop drain requests within a bound; rejected controls never release another owner. Scenario revisions reject queued stale work, exact virtual-time expiry revokes reads, Wrong Identity renders actual configuration, and Wi-Fi/APN/scripted-USSD mutations require their own read grants. Fourteen Rust regressions and two frontend regressions extend the accepted suite.

Final qualification: **113 frontend tests, 165 Rust tests in each profile, 130 native checkpoints**, TypeScript, lint, formatting, both Clippy profiles and both optimized Windows builds pass. Native development/recovery/release windows, React/IPC, six-page navigation, themes/help/HMR, real GUI Cancel/reconnect/read, retained generated-binary storage regressions and process cleanup pass; console issues are empty. **48 final actual simulator screenshots** are linked in the report. Full normal database/WAL/rows/settings/IDs/vault/other-files fingerprints equal the correction baseline; no storage migration or physical router request occurred. MG11 was absent and its saved user-reported firmware/UNKNOWN feature status remains unchanged. Virtual state is ephemeral and modeled; durable configuration transactions and physical OpenWrt qualification remain later work. **Phase 7: READY, not started.**

## Phase 5 adapter framework

**COMPLETE (PASS).** [Implementation map and security model](PHASE_5_IMPLEMENTATION.md) document reused modules, typed contracts, validated registry, evidence/ranking, scoped authorization, authentication, deadlines/cancellation and resource cleanup. [Verification report](PHASE_5_VERIFICATION.md) records exact commands, failures/corrections and acceptance. Current results after bootstrap correction: **99 frontend tests, 130 Rust tests in each profile, 103 native checkpoints**, TypeScript, lint, formatting, both Clippy profiles and ordinary/dedicated optimized Windows builds pass. Real development/recovery/isolated-release GUI/IPC, saved-endpoint form, HMR, retained library/backups/settings workflows and zero console issues pass. Full normal DB/WAL/rows/settings/vault fingerprints remain identical; successful marked test run cleaned after all owned processes stopped. Host PCIe Wi-Fi/Ethernet interfaces remain protected. MG11 was absent; saved software remains user-reported and all four unknown-capability regressions pass. No physical adapter, router write, extraction or Phase 6 simulator was enabled. Phase 6 is READY when requested.

**20 October post-review correction: PASS.** The original established gate demonstrably blocked cold-start authentication. A separate explicit, single-use, endpoint/interface/pin/evidence-bound approval now permits only a bounded authentication/identity handshake. Endpoint attestation and a unique verified authoritative identity are required before separate established read approval. All 29 established adapter tests are retained, with 15 additional cold-start regressions passing without preseeded authenticated evidence. Native verifier reload and folder-picker races were corrected without changing product GUI/IPC/storage behavior; diagnostic failures remain documented. Phase 7 must supply the reviewed approval flow and physical connector; neither was implemented here.

## GUI redesign verification

**COMPLETE (PASS) for the implemented GUI foundation.** [GUI report](GUI_REDESIGN_VERIFICATION.md) records the source inventory, per-phase implementation, actual failures/corrections and screenshots. This GUI work does not start Phase 5 or qualify a device adapter.

- Existing structure, Windows discovery/profile matching, real IPC, SQLite and bounded firmware worker retained. Six blue destinations replace the former primary navigation: Overview, Connected Devices, Library, Backup Manager, Logs and Settings. Overview has four actual counts and active jobs; history stays in Logs.
- Contextual keyboard-accessible help, dark/light/system appearance, shared progress/cancellation and notifications, conservative device selection, firmware details/type tags and confirmed reversible removal are integrated with actual backend records. Unsupported extraction, four device customization categories and Rebuild remain disabled with reasons.
- Actual independently hashed backups and `_DEFAULT` restoration use generated binaries in tests. Native directory selection, reviewed copy/verification and restart activation preserve metadata, previous Library bytes and the separate backup location. Additive schema 5 is qualified on isolated/nonempty regression databases; the owner's normal schema-4 database remains unopened by the redesigned app.
- **84 frontend tests, 85 Rust tests (both default and native-test feature), 91 full native checkpoints and retained WebDriver audit pass.** TypeScript, lint, Rust formatting/Clippy and ordinary/dedicated optimized Windows builds pass. Final centered-dialog geometry, native storage-copy restart and actual integrity verification from relocated storage pass. Console issues are empty; all owned processes/listeners close and successful run cleanup exits 0.
- Complete normal DB/WAL, settings, all table rows, saved router/profile IDs, vault and other data fingerprints match the GUI task's original read-only baseline. No cleanup/reset, router configuration or firmware write. MG11 is not connected today; no new physical qualification is claimed. Historical device-phase results below remain unchanged.

## Phase 4.1 completion evidence

- Separate feature-enabled native test executable, strict run IDs, fixed Windows known-folder root, matching ownership markers/locks, per-run database/vault/logs/workspaces/reports and WebView data. Normal executable cannot activate this mode or accept a data-path override. Existing schema 4, normal location and functional modules remain.
- Actual native dev/recovery/optimized-release GUI, React, nine destinations, real IPC, dialogs, HMR and process cleanup pass. **56 frontend tests, 76 Rust tests, 83 main native checks, 17 changed-settings round-trip checks** and the retained WebDriver smoke pass. TypeScript, lint, formatting, Clippy, frontend/native builds and final whitespace checks exit 0. Ordinary release binary remains byte-identical after the separate test build.
- Generated binaries exercise concurrent deduplication, live cancellation, forced import termination/recovery, corruption/missing bytes, SQL failure after publication, shutdown during verification and repeated restart. Near-commit cancellation is verified with deterministic Rust barriers. No real MiFi firmware is selected.
- Manifest publication now returns saved state separately from activity recording. Injected audit transaction failure preserves valid exports and shows saved-with-warning through actual React/native Save. No overwrite, manifest-schema or privacy weakening.
- Cleanup refuses active owners/processes, mismatched run markers, unknown staging and junctions; unrelated sentinel survives. Successful suites remove only their own marked runs. Failed runs are retained. Existing normal firmware/test history is not cleaned.
- Final isolated suites leave complete normal database/WAL, settings, router/profile IDs, vault and other data-directory fingerprints identical to their own before snapshots; integrity is `ok`, FK violations empty. **Earlier build-debugging interval is separately disclosed:** a normal startup added audit IDs 212/213 and changed the WAL/collective row fingerprint before final qualification. No rollback or deletion concealed that delta. Settings, vault, schema, saved IDs and artifact/job counts remain equal to the first probe. See report for both baselines and limits.
- Powered-on MG11 discovery passes in dev/recovery/release; all four features stay UNKNOWN, owner text stays user-reported and no Apply is offered. Diagnostic/AT Windows driver errors remain observed; RNDIS/composite are available. No router configuration change or Phase 5 adapter work.

The following completion evidence is historical and retains each phase's original counts.

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
| GUI redesign: desktop foundation | Complete (PASS) | Six blue sections, shared real jobs, backup/restore, storage-copy restart, persisted preferences, 84 frontend/85 Rust/91 native checks and screenshots. Additive schema 5 qualified in isolation. | Device acquisition/rebuild unavailable; no fresh MG11 qualification. No critical tested GUI blocker. |
| 5: Adapters | Complete (PASS) | Typed optional interfaces, validated deterministic registry, existing evidence/profile integration, separate bounded cold-start approval and established read policy, in-memory secrets, trusted simulated handshake, cancellation/deadlines/cleanup, strict IPC and bounded GUI status. 99 frontend/130 Rust per profile/103 native checkpoints pass. | No critical Phase 5 blocker. Physical transports remain Phase 7; full simulator Phase 6. |
| 6: Simulator | Complete (PASS), including post-review corrections | Explicit isolated stateful router/modem, exclusive operation ownership, cancellation/reset/expiry recovery, Phase 5 feature authorization, DNS/Wi-Fi/APN/USSD and deterministic faults; 113 frontend/165 Rust per profile/130 native checks and 48 final screenshots. | No critical Phase 6 correction blocker. State is virtual and ephemeral; no physical qualification. |
| 7: OpenWrt read-only | Not started (READY) | Simulator-qualified cold authentication, trust/identity/read gates and feature contracts. | Verified interface/version research, physical trusted connector, reads/probes, and selected hardware evidence. |
| 8: Backup/transactions | Not started | Simulator-only configuration snapshots/previews/apply/verify/restore and deterministic failure/recovery tests. | Durable journal, production backups, crash recovery, physical authorization and transaction engine. |
| 9: DNS MVP | Not started | Complete simulator DNS demonstration with separate upstream/LAN sources and actual state validation/readback. | Durable production transaction integration and authorized hardware validation. |
| 10: Wi-Fi limits | Not started | Two virtual radios/BSS records, client counts/ranges/limits and interruption/recovery. | Production radio/SSID UI, verified driver mapping/enforcement and hardware restoration tests. |
| 11: APN | Not started | Synthetic automatic/manual APN, IP family/authentication, modem/SIM/operator faults and cellular recovery. | Actual operator evidence, adapter/credentials/reconnection and hardware qualification. |
| 12: USSD backend | Not started | Scripted virtual session state machine, follow-up/busy/rate limits/expiry/cancellation; no carrier transport. | Physical modem adapter, trusted authorization and supported-hardware qualification. |
| 13: Router-local USSD | Not started | No router-side packages. | Authenticated router-hosted service/page, security tests, compatible hardware, operation with desktop closed. |
| 14: Studio integration | Not started | Existing shell and planned-screen placeholders. | Connected four-feature studio, shared transaction UX and end-to-end verification. |
| 15: OpenWrt builds | Not started | Builder placeholder and worker notes. | Worker, supported targets, packages/overlays, provenance/logs/manifests and validated artifacts. |
| 16: OEM feasibility | Not started | Analysis placeholder. | Isolated inspection, evidenced vendor routes and conservative feasibility reports. |
| 17: Hardware qualification | Not started | Windows host inventory test; no feature-qualified router profile. | Exact hardware/firmware/modem matrix, feature-by-feature persistence/interruption/recovery evidence. |
| 18: Hardening | Partial | CSP/ACL, typed validation, parameterized SQL, limited audit data, baseline risk tests. | Threat review and regression/fuzz/corruption/credentials/router-API/accessibility/resource qualification. |
| 19: Release | Partial | Historical unsigned NSIS/MSI creation and icons/metadata. | Final-feature installers, clean-machine install, upgrade/uninstall/data-retention, signing and release qualification. |

## Next execution order

1. Phase 6 is verified. Phase 7 is READY for separately requested physical OpenWrt read-only work; do not start physical writes.
2. Keep unknown support until exact device/version/profile evidence supports a decision. Current MG11 model is user-supplied.
3. Continue broader hardware qualification as suitable devices become available.
4. Reuse completed library/jobs, adapter framework and simulator fault/state model for later durable transaction qualification.
5. Complete durable transaction and authorized hardware validation before production configuration changes.

This tracker records completed Phases 0–6, targeted hardening and GUI foundation work while retaining historical evidence. Physical OpenWrt communication and device customization have not begun.
