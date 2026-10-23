# Phase 7.5 Stage 2A Part 2: credential-free challenge software

23 October 2026, Africa/Lagos. Software delivery only; Part 3 hardware remains held for reviewer acceptance and a separate release. No real device request, credential entry, login, status read or logout was performed. No firmware was acquired and extraction remains disabled.

## Scope and numbered delivery

Part 1 is **done**: reviewer decisions Q1–Q11, reviewer-attributed capture verification, Wi-Fi ssid/mode discard rules and conservative Q7 success/shape rules were recorded on the design branch in `ee3e53de90b8d3357f415cac1aa6086d372c5184`. The original task began at `bf41e6c084e762646d5e7a1bd22e5a3134cc4b41`; Part 2 branch `phase-7-5-stage-2a-challenge` starts at `ee3e53d`. The capture verification belongs to the reviewer; Codex did not redownload those files in this task.

| Part 2 item | Status | Implementation and evidence |
| --- | --- | --- |
| 1. Closed spec | Done | `request_registry.rs`: fourth ASR-family entry, Challenge, exact GET `/login.cgi`, no query/body, headers 8192/auth header 2048 bytes, body cap 0, one attempt, five seconds. Login/Logout/StatusRead remain unregistered; query, encoded-path, redirect and deny-word tests retained. |
| 2. Approval | Done | `approval.rs`: `issue_challenge`, `claim_challenge`, non-Clone/non-serializable `AuthChallengeApproval`, exact request scope/binding, 30-second expiry and atomic one-attempt gate. Replay, wrong scope both directions, changed binding, expiry and No denial tested. Native confirmation substitutions are sanitized to one line/80 characters; exact design text/title and IPC registration/ACL checked. Actual native dialog not visually exercised in this delivery. |
| 3. Fresh root | Done | `challenge::root_matches` and `ipc::cached_challenge_report` require a fresh successful earlier same-device/revision/endpoint/interface root and ASR-family candidate. IPC rejects missing/stale root as `no_recognized_family` before dialog/dispatch; collector reports that bounded code if supplied root no longer matches. Synthetic tests assert zero dispatch on rejected preconditions. |
| 4. Dispatch | Done | `connection::challenge_metadata` requires the claimed proof; Windows `probe_challenge` reuses pinned route/subnet/host guards, no proxy/cookie/auth/redirect/retry and existing cancellation/current-lease handling. Header-only return precedes all body-reading APIs. Owned loopback tests cover exact single credential-free GET, large advertised body, cancellation and silent-peer timeout. OS/WinHTTP internal prefetch is not claimed to be zero. |
| 5. Parser | Done | `challenge::parse_headers` accepts only one usable HTTP-401 Digest header with quoted bounded parameters. Borrowed hex nonce is shape-checked and never copied into output. Raw UTF-16/ASCII header buffers erase on drop. Malformed/non-ASCII/folded/escaped/oversized input is conservatively rejected; status/flags and bounded codes only on failures. False flags on rejected input do not prove absence. |
| 6. Result/UI | Done | Additive schema-1 `authChallenge`, endpoint_response/observed/observed and exact unverified label. Sanitized diagnostics, strict Zod and typed IPC, one Management API result line and separate explicit challenge button. No identity/stage/strategy authorization changes; `canObtain=false`. Set-Cookie is a presence flag only; no later authentication is enabled. |
| 7. Regressions | Done | Synthetic fixtures only, explicitly labelled in source. Parser/status/ambiguity/bounds/privacy, approvals, root preconditions, loopback, strict frontend IPC/UI and existing suite regressions. The trace/report/diagnostics/error privacy test checks the synthetic nonce is absent. Compile-fail proof/spec-construction doctests remain. Exact gate results below. |
| 8. Documentation | Done | Acquisition engine, phase tracker and design Q4/gate wording updated. Hardware result remains unknown; neither fixtures nor loopback are described as captures. |

No item is disputed. Part 3 is **not done**, deliberately unreleased. No migration, artifact classes, authenticated client, credentials/Secret use, plain-HTTP binding implementation, official-image registry, analysis worker or Phase 8 work was added.

## Changed files

All application paths below are relative to `apps/desktop/`:

- `src-tauri/src/acquisition/challenge.rs` (new): observation, bounded parser/sanitizer, fresh-root predicate and collector.
- `src-tauri/src/acquisition/challenge_tests.rs` (new): synthetic parser/privacy/registry/collector regressions.
- `src-tauri/src/acquisition/approval.rs`: scoped approval, one-use dispatch and approval/dialog regressions.
- `src-tauri/src/acquisition/ipc.rs`: separate native approval and read command, fresh-root/binding checks, lease admission before token claim and publication checks.
- `src-tauri/src/acquisition/mod.rs`: additive report field, module exports and diagnostic sanitization.
- `src-tauri/src/acquisition/request_registry.rs`: fourth closed spec and cap accessors; `tests.rs` and `static_menu_tests.rs`: historical three-entry assertions updated to four.
- `src-tauri/src/connection.rs`, `src-tauri/src/connection/windows.rs`: proof-gated dispatch, early header-only branch, erased raw buffers and owned-loopback tests.
- `src-tauri/build.rs`, `src-tauri/src/lib.rs`, `src-tauri/capabilities/main.json`, `src-tauri/permissions/autogenerated/{approve_auth_challenge,read_auth_challenge}.toml`: two explicitly registered/allowed commands.
- `src/types/acquisition.ts`, `src/lib/backend.ts`, `src/features/Devices.tsx`, `src/test/acquisitionBackend.test.ts`, `src/test/Devices.test.tsx`: additive strict contract, explicit action and frontend regressions.
- Repository `docs/FIRMWARE_ACQUISITION_ENGINE.md`, `docs/PHASE_STATUS.md`, `docs/PHASE_7_5_STAGE_2_AUTH_DESIGN.md` and this report: current boundary/results and review handoff.

## Failing tests before implementation

Evidence is local and ignored under `.tools/stage2a-challenge/`. No retained script writes verdicts or owner-confirmation language into documents. Commands below use `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1` abbreviated as `RUST` only within this table.

| Exact command after RUST, or complete npm command | Exit | Initial result/log |
| --- | --- | --- |
| `test --workspace --locked --offline challenge_` | 101 | Placeholder parser/registry: 3 failures, 1 pass; `parser-registry-red.log`. |
| `test --workspace --locked --offline challenge_scope_` | 101 | Missing scoped issue/claim methods, E0599; `scope-red.log`. |
| `test --workspace --locked --offline challenge_loopback_headers_only` | 101 | No parsed challenge from real owned loopback headers; `transport-red.log`. |
| `test --workspace --locked --offline challenge_collector` | 101 | Unwired collector returned NOT_IMPLEMENTED; `collector-red.log`. |
| `test --workspace --locked --offline challenge_diagnostics` | 101 | Missing sanitized diagnostic metadata; `diagnostics-red.log`. |
| `npm.cmd test -- --run src/test/acquisitionBackend.test.ts` | 1 | New strict result contract rejected before implementation; `frontend-red.log`. |
| `npm.cmd test -- --run src/test/Devices.test.tsx` | 1 | Two new explicit-action tests failed before UI implementation; `gui-red.log`. |

The missing methods/placeholder stubs were test-first scaffolding and were replaced, not shipped as functioning behavior. Approval tests never automate acceptance of a real native dialog. Test concurrency remains the default; no `--test-threads=1` workaround.

## Software gate results

Each Rust shell sets `CARGO_INCREMENTAL=0` and `CARGO_BUILD_JOBS=1` (build parallelism only, not test serialization). Full commands and exits are recorded below; logs use the listed basename under `.tools/stage2a-challenge/`.

| Command | Exit/result | Log |
| --- | --- | --- |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | 0; 291 passed, 0 failed, 1 ignored; 2 doctests passed | `standard-final.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | 0; 292 passed, 0 failed, 2 ignored; 2 doctests passed | `isolated-final.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | 0; clean | `clippy-standard.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0; clean | `clippy-isolated.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all -- --check` | 0; clean | `fmt-final.log` |
| `npm.cmd test` | 0; 155 passed across 17 files | `frontend-final.log` |
| `npm.cmd run lint` | 0; no warnings/errors | `lint-final.log` |
| `npm.cmd run build` | 0; TypeScript `tsc -b` and Vite production frontend build passed | `build-final.log` |

Final-commit repeat and normal-data comparison are recorded below after execution. Rust's progress text appears in PowerShell logs as NativeCommandError records because cargo writes progress to stderr; the captured process exit codes above are zero. Git also reports local global-ignore permission and LF/CRLF conversion notices; `git diff --check` passes. These are not runtime GUI findings. This report does not treat a terminal compile as GUI verification. Native launch/visual GUI, optimized Windows release executables, physical MG11 challenge and any authenticated operation were **not run** for this software-only gate.

## Part 3 plan, held

After separate code acceptance/release only: powered USB MiFi, PC Wi-Fi off; quote owner's before web-display statement; existing root dialog Yes; read the actual new native challenge dialog and owner answers Yes; one GET and sanitized metadata only; separate No run proves zero dispatch; owner's after web-display statement; compare normal-data snapshots. Stop on unexpected status, redirect, malformed/multiple headers, cookie presence, timeout, cancellation or subnet conflict. No login/credentials/retry/status/logout. This plan grants no hardware execution authority now.

## Review handoff

`git diff phase-7-5-stage-2a-auth-design..HEAD` is the task-only review diff. Exact final commit, clean-tree and repeat-run evidence are supplied at handoff; no push, tag, merge or history rewrite. Main and the accepted Stage 1 branch remain unchanged.
