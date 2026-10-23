# Phase 7.5 Stage 2A Part 2: credential-free challenge software

23 October 2026, Africa/Lagos. Software delivery only; Part 3 hardware remains held for reviewer acceptance and a separate release. No real device request, credential entry, login, status read or logout was performed. No firmware was acquired and extraction remains disabled.

## C1 follow-up, 24 October 2026

The reviewer accepted `ee3e53d..ca231dd` conditional on C1 and separately released Part 3 after C1 is committed and its software gate passes. The historical held statements below describe the initial delivery. The hardware run is now authorized conditionally, **not yet executed**. Login, StatusRead, Logout, credentials/Secret use and the other implementation gates remain blocked.

C1 accepts unquoted bounded token values for algorithm/stale/charset/userhash, while realm/nonce/qop/opaque/domain still require quotes. Existing algorithm output cap stays 16; the other released token forms are at most 32 bytes. Opaque/domain remain transient borrowed values and are discarded. Fixed Rust enums and strict Zod expose header count (0/1/2/3+), first scheme category, known parameter names/quoting in header order, count of unknown names and a fixed rejection reason. Shape facts cover bounded syntactically reachable parameters only; malformed tails/oversized headers are not searched heuristically. Unknown names and all rejected parameter values are excluded. Diagnostics revalidate output; UI adds a shape line beside the challenge result. No protocol stage/authority changes.

Changed for C1: `src-tauri/src/acquisition/challenge.rs`, `challenge_tests.rs`, `src-tauri/src/connection.rs`, `src-tauri/src/connection/windows.rs`, `src/types/acquisition.ts`, `src/features/Devices.tsx`, `src/test/acquisitionBackend.test.ts`, `src/test/Devices.test.tsx` (application paths relative to apps/desktop), plus `scripts/verify-auth-challenge.mjs` and its selection in `scripts/verify-device-gui.mjs`, this report and the engine/design/status documents. The committed harness records computed observations/dialog text only, never owner-confirmation language or generated verdicts; it never answers Yes and stops the positive run on any released stop condition. Separate No requires zero read IPC invocations; no packet-capture claim.

Initial C1 failures: `RUST test --workspace --locked --offline c1_synthetic` exited 101, two failing synthetic tests (`c1-red.log`); `npm.cmd test -- --run src/test/acquisitionBackend.test.ts` exited 1, one failed/eight passed (`c1-frontend-red.log`); `npm.cmd test -- --run src/test/Devices.test.tsx` exited 1 for missing shape UI (`c1-gui-red.log`). Here RUST is the full scripts/rust.ps1 invocation in the command table below. Tests assert exact closed shape output for quoted/unquoted algorithm, stale/charset/userhash, opaque/domain, two qop tokens, case variants, Basic/missing/multiple headers, missing/bad nonce, duplicates and oversize. Privacy regression now checks nonce/opaque/domain sentinels in report, diagnostics, captured trace and errors.

Latest C1 checks: full standard command below exits 0, 293 passed/0 failed/1 ignored plus two doctests (`c1-standard-final.log`); isolated command exits 0, 294 passed/0 failed/2 ignored plus two doctests (`c1-isolated.log`). `npm.cmd test` exits 0, 156 tests/17 files (`c1-frontend.log`); `npm.cmd run lint` and `npm.cmd run build` exit 0 (`c1-lint.log`, `c1-build.log`). Rust fmt command exits 0 (`c1-fmt.log`), as does `node --check scripts/verify-auth-challenge.mjs`. Remaining final evidence is appended after execution.

Both exact Clippy commands in the table below also exit 0 after C1, no warnings (`c1-clippy-standard.log`, `c1-clippy-isolated.log`). Normal read-only comparison after C1 software exits 0 (`normal-after-c1-software.json`, `c1-data-software.log`), unchanged from the original snapshot. Release builds, final C1 repeats and hardware evidence remain pending at this code commit.

### Completed C1 gate and incomplete supervised attempt

C1 implementation commit: `a76c98a3ae6e7129680eabd8d5301add20310183`. `py scripts/verify-parallel-rust.py --runs 20 --output .tools/stage2a-challenge/c1-parallel` exits 0. All twenty runs use the default parallel scheduler and the same clean tracked HEAD: 293 passed/0 failed/1 ignored plus two doctests. Every log SHA-256, commit-before/after, dirty flag and C1/privacy regression marker was checked against `c1-parallel/results.json`. These are C1 results, separate from the earlier ca231dd runs.

`npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline` exits 0 (`c1-production-native.log`), optimized Windows production build. `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-native-tests.ps1` exits 0 (`c1-isolated-native.log`), dedicated isolated executable; script confirms production executable unchanged by the test build. Binary hashes/lengths are computed in `c1-binaries.json`: production SHA-256 `0278DDB4776A03458A4C24C0C219A7B0C32F74FAE264D6D549883C94F4780C65`, isolated `E539EDB214999815906AF9E12BA0FCE7F159CBB9AC7BAB622AFB863DC553F091`.

Owner's before statement, **24 October 2026, Africa/Lagos**, quoted exactly from the reply (web display only; not independently verified firmware evidence):

> Software Upgrade
> Software Information
> Current Software Version:
> MF69_AM802L_ZX_DD_N_241106_EU

The earlier reply `done` was to the powered/USB/Wi-Fi preparation question. No explicit before-test settings-condition statement was supplied; do not expand the version quote into one.

Native command: `node scripts/verify-foundation.mjs --release-only --device-gui --acquisition-mg11-challenge`, exit **1**, `hardware-driver.log`. Evidence directory `.tools/foundation/2026-10-23T23-27-41.980Z/` is UTC-named; the run occurred on 24 October in Lagos. Actual native window/nonzero HWND, rendered React DOM, all six navigation destinations, real SQLite IPC, three discovery refreshes/unique IDs and unknown feature support passed. The real root native dialog was read using the existing helper at 23:27:50 UTC. Its exact observed body (newline formatting preserved here) was:

```text
Approve one bounded, read-only check?

Device: USB Composite Device
Interface: {7F7C5ABA-3722-40E6-A506-E36DF47013EF}
Endpoint: http://192.168.100.1/
Advanced endpoint: false

GET / and, only for a recognized ASR family, GET /version_num.txt. No login or device changes.
```

Root approval did not complete within the bounded 300-second wait. The harness stopped with `Native approval/read did not finish`, shut down its owned application and retained its marked test profile for inspection. `challenge-observations.json` has root/challenge/decline null and only the root-dialog observation; no challenge IPC/read stage was reached. This is **not** a router transport timeout, a declined approval, a successful native challenge dialog or evidence about the device's challenge format. No challenge response/status/realm/shape or No-test result exists. No actual challenge request is claimed; no packet capture was made. Q4/Q5 hardware answers remain unknown, and no retry/fetch/workaround was dispatched. An extra read-only CDP state inspection attempted after shutdown exited 1 with `ECONNREFUSED 127.0.0.1:9223`; it recovered no state and sent no router request.

The native runner's normal-before/after JSON snapshots compare equal, including all rows, settings, DB/WAL hashes, saved devices and vault/log files. A separate `py scripts/probe-native-isolation.py .tools/stage2a-challenge/normal-after-c1-native.json .tools/stage2a-challenge/normal-before.json` exits 0, also unchanged. No owner after-test web statement has been collected for this incomplete attempt; no router-settings preservation result is invented. Generated screenshots stay local and uncommitted. Development/HMR, adapter/simulator native suites and physical OpenWrt were not rerun at this targeted gate; retained unit/isolated regressions passed.

**C1 software verdict: PASS. Supervised challenge: PENDING (root approval wait expired). Authenticated code remains BLOCKED.** Phase 7/7.5 is not complete.

### Parallel design handoff

The separately released documentation track is complete on `phase-7-5-design-track`, in the local `.tools/design-track` worktree, based on accepted `ee3e53de90b8d3357f415cac1aa6086d372c5184`. Four commits, each changing its one design and status entry: Stage 3 `82dc4ab`, Stage 4 `a2651cf`, Stage 5 `0005e38`, Stage 6 `5f2678ace8e783ec90464cd85d8fe6b7a3d9861c`. Only five documentation files differ from the accepted base; worktree is clean. Documentation section/fence/source/citation checks and `git diff --check` exit 0; no SQL/tool/code execution or owner-data test was performed for those documents. Open questions require reviewer answers; every implementation remains blocked. Inspect with `git diff phase-7-5-stage-2a-auth-design..phase-7-5-design-track`; no merge was performed.

Initial code `ca231dd631391a30f21ff1fc48f0fb3b6279e35b` completed `py scripts/verify-parallel-rust.py --runs 20 --output .tools/stage2a-challenge/code-parallel`, exit 0: every run 291/0/1 plus two doctests, unchanged HEAD and clean tracked tree. Metadata/log hashes are in `code-parallel/results.json`. C1 changes require their own new final-commit repeat; these earlier runs are not claimed for C1. `py scripts/probe-native-isolation.py .tools/stage2a-challenge/normal-after-software.json .tools/stage2a-challenge/normal-before.json` exited 0 with identical normal data before C1; it will be compared again after later checks.

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
