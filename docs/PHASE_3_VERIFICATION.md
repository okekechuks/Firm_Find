# Phase 3 verification — device profiles and capability decisions

Verified **16 October 2026**, Africa/Lagos. **Phase 3: COMPLETE / PASS. Phase 4: READY, not started.** Scope follows the latest supplied Phase 3 prompt. Existing plans and accepted Phase 0–2 evidence were retained.

## Implementation and requirement classifications

| Requirement | Classification | Actual evidence |
| --- | --- | --- |
| Retain functional baseline, structure and changes | VERIFIED | Started from clean `d82bc7b`; retained `apps/desktop`, discovery/connection services, storage directory, product identity, locks and existing commands. No cleanup/reset/restructure. |
| Structured profiles and separate schema/content versions | VERIFIED | `device-profiles/*.json`, `schema-v1.json`; Rust `profiles::validate`, strict serde models; TypeScript `types/profiles.ts` with Zod response validation. Schema 1, profile versions 1.0.0. |
| Untrusted input bounds and safe rejection | VERIFIED | 64 KiB/depth/string/collection limits, duplicate-key visitor, enum/unknown-field checks, calendar/source/range/sub-capability validation; negative unit tests and real native invalid JSON/future schema/script-field rejection. |
| Registry, source provenance and version lookup | VERIFIED | `Registry::load`, `bundled`, `by_id`; two valid entries, deterministic ID order/digest, duplicate-ID quarantine; imported/experimental/observation records cannot become authoritative. Local validator previews only. |
| Actual Phase 2 evidence and deterministic matching | VERIFIED | `profiles::gather_evidence`, `evaluate`, `match_profile` in `src-tauri/src/profiles/engine.rs`; live MG11 USB/physical/network/user evidence through `get_device_compatibility`. Exact revision/firmware gates, exclusion reasons, ambiguity and contradictions covered by fixtures. |
| Evidence preservation and freshness | VERIFIED | Typed source/method/time/reliability/verification/applicability/association/conflicts; five-minute TTL, future-time/disconnect/foreign-subject checks, current-discovery publish guard, frontend generation/expiry protection. |
| Four independent decisions and readiness | VERIFIED | Assessment separate from prerequisites/adapter availability. Supported/unsupported/unknown and mixed sub-capabilities are exercised in synthetic tests. All live decisions prohibit Apply; missing adapters do not imply unsupported hardware. |
| DNS distinctions | VERIFIED | Separate upstream, LAN IPv4, LAN IPv6, forwarding and persistence decisions. Exact reference-profile DNS documentation is gated by identity and authentication/package/manual prerequisites; no configuration calls. |
| Wi-Fi distinctions | VERIFIED | AP, managed BSS, client limit/range, persistence, reload and management-risk records. Unknown MG11 range; no invented client count or enforcement. |
| APN distinctions | VERIFIED | Modem/interface, current APN read, manual/automatic/operator/reconnect states remain separate; no modem commands or APN changes. |
| USSD distinctions | VERIFIED | Modem identity/interface, one-shot/interactive/response/cancel and router service/web UI remain separate; no AT execution, service installation or requests. |
| Native UI and IPC integration | VERIFIED | `features/Compatibility.tsx`, connected-device details and ninth navigation destination; real Windows native dev/recovery/production UI, matrix, registry validation and observation form tested. Development/production screenshots inspected. |
| Persistence and decision changes | VERIFIED | Additive `migrations/003_profiles.sql`; `Repository::install_registry`, `identity_observations`, `save_compatibility`, `prior_compatibility`; version/hash archives, one prior report, change reasons and no cached authority. Unit migration/restart test and real dev/production saved-text checks. |
| Regression tests/builds/lifecycle | VERIFIED | 42 frontend tests, 40 Rust tests, 35 native checks, typecheck, lint, formatting, Clippy, frontend and optimized native builds pass. Native HMR, recovery and console checks pass. |
| Data and read-only boundaries | VERIFIED | Read-only pre/post probes and WAL-aware baseline backup comparison retain original rows/IDs/settings. No device configuration, firmware modification, flashing, authentication adapter or new router request during Phase 3 native verification. |
| Real MG11 feature support or exact hardware identity | BLOCKED — qualification only | Available evidence establishes a Windows candidate, not independent manufacturer/model/revision/modem/firmware identity. User text cannot resolve this. All four assessments stay UNKNOWN. This is expected fail-closed Phase 3 behavior, not a baseline implementation blocker. |
| Older profile-schema migration | NOT APPLICABLE | No older profile format existed. Unsupported future schemas reject safely; schema-2 SQLite data migrates additively to 3. |

## Verified native results

Command `npm.cmd run test:foundation -- --phase3` exited **0**, with **35 PASS records**. Evidence directory:

`.tools/foundation/2026-10-16T15-21-59.441Z/`

Development (`http://127.0.0.1:1420/`) and production (`http://tauri.localhost/`) were verified through actual Windows WebView2. Each session had exactly one OBTAINER process with a nonzero native window handle; development had one Vite process, production none. React connected to SQLite through Rust, all nine destinations worked, and three refreshes per session returned unique device IDs and no discovery warnings. Actual inventory count was 79 Windows records, not 79 verified routers.

The suite checked registry IPC, valid-unverified preview, invalid JSON/future-schema/script rejection with unchanged registry digest, the real four-feature support table, reported-text save/re-evaluation, and the same engine in connected-device details. It verified normal reopen, deliberately interrupted owned launch before rendering, recovery, hot reload with unchanged app/Vite identities, injected read-error retry and loading behavior. Temporary source edits were restored. All sessions closed; no app/Vite/debugging listener remained. `console-issues.json` contains `[]`.

Screenshots `dev-reopen-compatibility.png` and `production-compatibility.png` were opened and visually inspected: the native React interface shows the candidate match, no authoritative profile, four UNKNOWN rows, manual verification / NOT IMPLEMENTED readiness, and expandable evidence/prerequisites. Terminal output alone was not used as rendering proof.

MG11 candidate retains logical ID `8198d94a197a40c7d6aa47967a7a713fd468e6be5bae947e15d3c013ab77086f`, USB `1286:4E31`. Owner-supplied text was entered through the React form:

| Field | Value | Result |
| --- | --- | --- |
| Unclear reported identifier | `LT800/` | `user_supplied`, `user_reported`, `hint`; never converted into a hardware revision or model alias. |
| Reported software version | `MF69_AM802L_ZX_DD_N_241106_EU` | Preserved verbatim as opaque, unverified firmware-version evidence; never parsed into a verified numeric release. |

Both records retained the same IDs across dev, recovery and production; exactly two observations remain. Overall match is CANDIDATE; DNS/Wi-Fi user limit/APN/USSD are UNKNOWN, numeric client range is null, adapter availability and Apply permission are false. No unauthenticated HTTP response was promoted into identity or configuration API evidence.

## Commands and results

`CARGO_INCREMENTAL=0` was set for Cargo/native builds, and `DEBUG_PRINT_LIMIT=1200` for frontend tests. Existing scripts and cached dependencies were used; no dependencies or Windows prerequisites were installed.

| Exact command | Exit | Result |
| --- | --- | --- |
| `node node_modules/typescript/bin/tsc --project apps/desktop/tsconfig.json --noEmit` | 0 | TypeScript passes. Final Tauri build also runs `tsc -b`. |
| `npm.cmd run lint` | 1, then 0 | Initially two `react-hooks/exhaustive-deps` cleanup-ref warnings. Fixed with stable invalidation callbacks; final zero warnings. |
| `$env:DEBUG_PRINT_LIMIT='1200'; npm.cmd test` | 0 | 42 tests in 7 files; includes new matrix, expiry, stale-response, reported-text and transport-contract cases. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 check --workspace --locked --offline` | 0 | Initial Rust profile/IPC implementation compiled; final Clippy/tests/build validate subsequent edits. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all` | 0 | Formatted changes. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all --check` | 0 | Formatting passes. |
| `$env:CARGO_INCREMENTAL='0'; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | 0 | All targets, zero warnings. |
| `$env:CARGO_INCREMENTAL='0'; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | 0 | Final 40 Rust tests, zero failures/ignored; includes 17 new profile cases and schema-2 persistence integration coverage. |
| `node scripts/generate-profile-schema.mjs` | 1, then 0 | Initial generator assumed unavailable Zod 4 API; corrected for locked Zod 3 primitives without dependency upgrades. Final schema generated successfully. |
| `npm.cmd run build` | 0 | TypeScript + optimized Vite frontend. Final native build rebuilds final assets (1,605 modules). |
| `$env:CARGO_INCREMENTAL='0'; npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline` | 0 | Fresh optimized `target/release/obtainer.exe`; final Rust release compilation 1m26s. Installer bundling/signing is outside this check. |
| `npm.cmd run test:foundation -- --phase3` | 0 | 35 actual native acceptance checks. |
| `npm.cmd run tauri -- dev -- --locked --offline` | 0 / 1 / 0 | Native runner's normal development and recovery sessions exit 0; middle test-owned launch is deliberately terminated and records exit 1, followed by clean successful recovery. |
| `.\target\release\obtainer.exe` | 0 | Production native React/IPC/matrix/settings/discovery/restart checks pass. |
| `py .tools/phase-0-audit/probe-data.py .tools/phase-3/data-before.json` | 0 | Read-only baseline, schema 2, integrity ok. |
| `py .tools/phase-0-audit/probe-data.py .tools/phase-3/data-after.json` | 0 | Read-only final probe, schema 3, integrity ok, no FK violations. |
| `py .tools/phase-3/compare-data.py` | 0 | Original settings/manual routers/firmware/jobs unchanged; original device/audit/discovery metadata IDs retained. |

The [retained evidence transcript](evidence/phase-3-2026-10-16.txt) summarizes actual native records, data comparisons and artifact paths. Local raw results, launch logs, console captures, profile reports and screenshots remain available under the evidence directory above. `git diff --check` found no whitespace errors; Git's existing global-ignore access and LF/CRLF notices are host/repository notices, not application runtime errors.

## Data preservation

The repository was inspected before edits. `%LOCALAPPDATA%\OBTAINER\obtainer.db` remains the database; `%LOCALAPPDATA%\com.obtainer.desktop\EBWebView\` remains the existing browser profile. Neither was moved or reset. `.tools/phase-3/data-before-backup.db` is a SQLite backup taken from a read-only source connection, including committed WAL contents.

Baseline → final: 79 devices, 79 discovery metadata, 1 saved router, 1 settings row, zero firmware artifacts/jobs remain. Normal startup/settings audit events grow 83 → 92. New tables hold 2 profile versions, 2 reported observations and 1 current evaluation with prior history. SQLite integrity remains `ok`; foreign-key violations remain empty. Settings fingerprint remains `70ca50b9ab62d27cbc6340ed90d7ebab7596b61ac8875a63e86219af831f4d97`. Existing settings and saved-router rows compare exactly equal to the backup. Vault/workspace/report files remain empty and unchanged.

## Limits and next action

No critical Phase 3 startup/runtime blocker remains. Exact supported/unsupported fixture outcomes demonstrate matching logic, not physical feature qualification. The MG11's independent identity, hardware revision, chipset/modem, management API availability, runtime packages, Wi-Fi enforcement and router-hosted USSD remain unverified. Authenticated read-only adapters and physical feature qualification belong to later phases. The Archer C7 profile is documentation-based and was not tested on physical Archer hardware. Local profile validation is intentionally a preview, with no activation, remote updates or signing system.

The smallest next development step is the roadmap's **Phase 4 persistence/jobs/firmware-library work**, reusing schema 3 and the existing structure. Phase 4 has not begun.
