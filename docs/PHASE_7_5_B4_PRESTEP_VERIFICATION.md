# Phase 7.5 B4 static-menu pre-step — software verification

23 October 2026, Africa/Lagos. Branch `phase-7-5-b4-menu-prestep` created from local main at `1b7fbce8334203be261d16ddf1c196181284d49c`, initially clean. Reviewer accepted G1/B4 plan amendments and fast-forwarded main; origin/main remains historical until the owner authorizes a push. This delivery implements only the approved software pre-step. No physical MiFi request or hardware confirmation occurred.

## Latest correction and hardware follow-up

The text above records the original `c3486cb` software-only handoff.
The subsequently authorized H1 correction and one supervised MG11 static-menu
run are now documented in [the hardware-step report](PHASE_7_5_B4_HARDWARE_STEP.md):
280 standard /281 isolated Rust tests, 151 frontend tests, both release builds
and the required repeated suites pass. Menu outcome `observed`, filename
`status1`, remains unverified and was not fetched. Separate owner No produced
zero read calls. Owner reports visible settings unchanged; normal-data snapshots
match. No authentication or firmware acquisition is implemented by this step.

## Numbered requirements (original software delivery)

| Item | Status | Implementation and verification |
| --- | --- | --- |
| 1 Approval scope | DONE | `acquisition/approval.rs`: `StaticMenuMetadata { request: StaticMenu }` uses existing device/revision/interface/normalized-endpoint/advanced binding and 30-second expiry. `claim_menu` removes receipt before checking exact scope/binding/expiry and returns separate non-Clone, non-serializable `StaticMenuApproval`. Old metadata and strategy receipts fail menu claims; menu proof fails metadata/strategy checks. Atomic `take_target` at transport entry permits one attempt only, with no reset after failure/cancellation. Tests cover exact request mismatch, replay, both scope directions, changed endpoint/interface/revision, receipt and claimed-proof expiry, No/no receipt, and second dispatch denial. |
| 2 Native confirmation | DONE; visual execution NOT RUN | New `ipc::approve_static_menu` has its own native MessageBox title, Yes/No with No default, and plan text. `menu_confirmation` sanitizes every interpolated field using existing single-line/bidi removal/80-character helper; endpoint is normalized. No frontend Boolean can issue a scope. Cached report and binding are rechecked after dialog. Unit confirmation/caps/No tests and frontend rejection test pass; the actual native popup was not opened. |
| 3 Request spec | DONE | `request_registry.rs` adds exactly `GET /xml/ui_mifi.xml` to `AsrDusterXml`: XML-only, 16 KiB, one attempt, five seconds. Exactly three family specs now exist. Login/logout/XmlAction remain unregistered. Existing method/body/encoding/traversal/query/host/redirect/deny-word validation stays intact. Closed-table fixture expectations updated deliberately. |
| 4 Scoped dispatch | DONE | `ipc::read_static_menu` requires a fresh earlier cached root report for same device/revision/interface/endpoint and recognized family. No family means a bounded limitation and no dispatch. Busy admission precedes receipt claim; existing connection lease/cancellation/current-publication guard reused. `static_menu::collect` validates closed literal spec and typed proof. `connection::menu_metadata` atomically consumes dispatch eligibility before `windows::probe_menu`, reusing route pinning, subnet collision/host/physical-interface guards and credential/cookie-free transport. Root/version transport also checks its own metadata scope/target. Only one static GET; no automatic root refresh, path discovery or follow-up request. |
| 5 Stop rules | DONE | Windows transport uses a total menu deadline of at most five seconds including preflight/remaining approval, zero retries, no proxy/autologon/cookies/redirect following. A payload-free TCP preflight remains part of the existing route path; only one HTTP request is sent. 401 yields `menu_not_readable_without_login`; 403, redirects (including relative), other non-2xx, non-XML type, oversized/truncated data, timeout and cancellation stop. Only application/xml or text/xml (optional parameters) accepted. Content-Length and streamed cap enforced; exact cap requires EOF confirmation. No body from a failure becomes an observation; no retry or credentials. Loopback tests cover these paths with bounded listeners/channels. |
| 6 Parser | DONE | `static_menu.rs::parse_menu` uses pinned, already-cached quick-xml 0.42.0. Rejects DTD/entity declarations before reader and conservatively rejects all entity references. No unescape, expansion or external resolver. UTF-8 only, 16 KiB/depth 64 limits, namespace refusal, ASCII XML-name and strict declaration/envelope checks. Uses documented Tab/Menu shapes, case-exact objdashboard, exactly one match, required attributes and one 1–64 byte ASCII letters/digits/underscore/hyphen token. Invalid/ambiguous input returns no filename; raw response is discarded. Conservative rejections are limitations, not hardware incompatibility. |
| 7 Result/schema/UI/redaction | DONE | Optional schema-1 `staticMenu` report field contains fixed bounded code, nullable filename, `endpoint_response`/`observed`/`observed`, unverified label. It lives only in existing expiring in-memory report cache, outside identity/profile evidence and SQLite; root expiry is not extended. Strict Zod schema enforces provenance, token, code/name consistency; old reports default field to null. Device dialog adds observed line under Management API and a separate Read static menu action/native approval. Tests keep stages/evidence/Obtain unchanged. Diagnostics whitelist excludes filename and menu fields; report/diagnostics and trace-capture tests exclude raw secret/body. No filename feeds another path or request authority. |
| 8 Synthetic tests | DONE | New inline fixtures are explicitly synthetic, both Tab/Menu shapes. Tests include case difference, duplicate matches across shapes, zero/missing/empty/token-invalid/oversized cases, namespace/DTD/entity/malformed/truncated XML, malformed names/declarations, path/query injection, 401/403/redirect/content type/status, cancellation before/during/after request, timeout, stale approval/result, no-family/no traffic, forbidden CGI dispatch, redaction and scope isolation. Owned loopback only; listener bound before request, channel synchronization and bounded accept/read/cleanup; no arbitrary setup sleeps. Existing regressions retained. |
| 9 Docs | DONE | Current architecture, phase tracker and accepted-plan status updated; this report distinguishes software tests from held hardware step. No hardware matrix, captured menu, generated screenshots or owner-confirmation text added. |

## Changed source inventory

Paths below under `apps/desktop/` unless otherwise indicated:

- Rust: `src-tauri/src/acquisition/{approval.rs,ipc.rs,mod.rs,request_registry.rs,static_menu.rs,static_menu_tests.rs,tests.rs}`; `src-tauri/src/connection.rs`, `connection/windows.rs`; `src-tauri/src/lib.rs`.
- Tauri command registration/permissions: `src-tauri/build.rs`, `capabilities/main.json`, generated source permissions `permissions/autogenerated/{approve_static_menu.toml,read_static_menu.toml}`.
- Dependency: `src-tauri/Cargo.toml`, root `Cargo.lock`. Adds direct exact quick-xml 0.42.0 already present transitively; no package version changed or downloaded.
- Frontend: `src/types/acquisition.ts`, `src/lib/backend.ts`, `src/features/Devices.tsx`; regression tests `src/test/{Devices.test.tsx,acquisitionBackend.test.ts}`.
- Docs: this report, `FIRMWARE_ACQUISITION_ENGINE.md`, `PHASE_STATUS.md`, `PHASE_7_5_B4_PRESTEP_PLAN.md`.

## Failing-first and correction record

All logs under `.tools/b4-menu/`. Test/compile commands use repository `scripts/rust.ps1`, with CARGO_INCREMENTAL=0 and CARGO_BUILD_JOBS=1 for compiler resource limits; test scheduling stays default-parallel. PowerShell's NativeCommandError formatting of compiler stderr does not determine exit status.

- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --locked --offline menu_`: exit 101 before implementation, unresolved parser/request variant (`red.log`), then missing new scope issuance/claim methods (`approval-red.log`).
- `npm.cmd test -- src/test/acquisitionBackend.test.ts`: exit 1, one new schema test failed /three passed before result contract existed (`frontend-red.log`).
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 check --workspace --offline`: exit 101 while registering the direct cached XML dependency; four string/byte API mismatches corrected after reading local quick-xml source (`check.log`). Lock changed only direct dependency list.
- Same Rust `menu_` filter: exit 101, eight passed /two failed, showing malformed XML names accepted and oversized header masking 401 (`hardening-red.log`). Fixed name validation and successful-status-only body checks; follow-up targeted run passed.
- Rust filter `menu_malformed_xml_names_are_not_observations`: exit 101, one failed (`xml-red.log`). Extended negative envelopes required strict declaration placement/attributes and rejecting literal `<` in attribute values and forbidden text terminators; fixed before final suites.
- Rust filter `menu_approval_replay`: exit 101, missing take_target guard (`one-dispatch-red.log`); added atomic one-attempt enforcement, verified second internal use cannot dispatch.
- Standard Clippy initially exited 101 for unnecessary get(...).is_some(); changed to contains_key, then both profiles pass. No lint suppression or serialized-test workaround.
- First final isolated-profile attempt exited 101, 279 passed /one failed /two ignored: accepted Windows loopback socket inherited listener nonblocking mode, causing a WouldBlock peer read and unreachable client result. Fixed test harness only: accepted stream explicitly blocking, three-second read bound and complete header reads capped at 4096 bytes. Listener remains nonblocking with bounded accept for cleanup; no setup sleep or serialized tests.

Other regression additions extend positive and negative coverage; they are not claimed to have all failed independently before implementation. No physical test was substituted with a synthetic result.

## Executed exit checks

Rust commands below use `$env:CARGO_INCREMENTAL='0'; $env:CARGO_BUILD_JOBS='1'`. Tests need normal Windows filesystem permissions for existing path-lease checks; databases/settings are isolated temporary files.

| Exact command | Exit / result |
| --- | --- |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | 0; final standard suite 279 passed /0 failed /1 ignored, two doctests pass |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | 0; final isolated suite 280 passed /0 failed /2 ignored, two doctests pass (`isolated-verified.log`) |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0 |
| `npm.cmd test` | 0; 150 passed /17 files |
| `npm.cmd run lint` | 0 |
| `npm.cmd run build` | 0; TypeScript project validation and Vite production build |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all` | 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all -- --check` | 0 |
| `py scripts/probe-native-isolation.py .tools/b4-menu/normal-after.json .tools/b4-menu/normal-before.json` | 0; full normal-data snapshot identical |

`py scripts/verify-parallel-rust.py --runs 20 --output .tools/b4-menu/final-parallel` exited 0: twenty consecutive default-parallel full-workspace runs on final implementation commit `2cf2e182a1e3f8ed7f4af6b1790269d38a7b24d0`, each 279 passed /0 failed /1 ignored plus two doctests passed. Every recorded before/after revision matches, tracked tree clean throughout. All twenty log hashes match bytes; eight key approval/parser/stop-rule/redaction/loopback/CGI-refusal markers have `... ok` in every log. Computed evidence: `.tools/b4-menu/final-parallel/results.json`. Script writes only computed records/logs; prose edited directly. Subsequent commit records documentation only; application and dependency code remain identical to tested revision.

Preservation baseline `py scripts/probe-native-isolation.py .tools/b4-menu/normal-before.json` exited 0. Only explicit read-only preservation probes access owner's normal DB; all tests use temporary databases. Final `py scripts/probe-native-isolation.py .tools/b4-menu/normal-final.json .tools/b4-menu/normal-before.json` exited 0 after all twenty runs: identical DB/WAL bytes/hashes, all rows/settings, saved devices/profiles, vault and logs. Schema remains 5; integrity ok, foreign-key check empty.

Not run: actual native popup/GUI rendering, Windows release builds, physical MiFi static menu read, physical OpenWrt, firmware acquisition on H01–H05. Native popup is source-compiled and its approval logic/frontend contract tested; no visual or hardware result is claimed. Part 2 is held until reviewer accepts Part 1 and sends its separate hardware guideline. No credentials, login/challenge/logout/CGI, B2 migration/artifact classes, B5 binding, B7 registry, B8 worker or Phase 8. No firmware write, flashing, reboot/reset, live USSD, push, merge or tag.

## Review handoff

Software exit gate PASS; Part 1 re-review pending, Part 2 hardware HELD. Branch `phase-7-5-b4-menu-prestep` starts at `1b7fbce8334203be261d16ddf1c196181284d49c`; final tested implementation is `2cf2e182a1e3f8ed7f4af6b1790269d38a7b24d0`. Evidence-only end commit supplied in delivery; repository committed and clean at handoff. Local main remains `1b7fbce8334203be261d16ddf1c196181284d49c`, origin/main `369e74173c570009b47bfd4957a4852ed0856678`; no push, merge or tag. This branch alone contains B4 changes. Review `git diff main..HEAD`. No acquisition claim until a real artifact has hash/provenance; static filename is not firmware and grants no follow-up request.
