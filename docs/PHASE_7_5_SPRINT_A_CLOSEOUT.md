# Phase 7.5 Sprint A close-out and Sprint B review gate

Date: 22 October 2026 (Africa/Lagos). Branch: `phase-7-5-acquisition-engine`. Start: `cd0d9af7aa40b93ac364cd2015e50ad8d77fe98c`. Original independent verdict: **PASS WITH LIMITATIONS**. Claude independently ran 221 standard Rust tests, 1 ignored, at the original slice; original frontend/native/hardware results were Codex-reported, not independently re-run. This correction report is also Codex-executed evidence, awaiting re-review. No merge, Phase 8, authenticated vendor adapter or acquisition claim.

## A1-A6 disposition

| Item | Disposition | Changed code and regression evidence | Remaining boundary |
| --- | --- | --- | --- |
| A1 | FIXED, conservative endpoint limitation | `acquisition/approval.rs::{binding,Approvals::issue,Approvals::claim}`, `acquisition/ipc.rs::{approve_firmware_route_probe,discover_firmware_routes}`. Native confirmation replaces caller authorization boolean; random single-use receipt binds device, revision, interface, normalized endpoint and advanced flag. Thirty-second monotonic expiry starts after confirmation. `a1_legacy_boolean_cannot_authorize_network` initially failed; gateway/replay/interface/revision/endpoint/expiry tests now pass. IPC manifest, ACL, frontend contracts and decline recovery updated. | Numeric gateways supported by default. Discovery does not expose DHCP-server metadata; DHCP-only or other private on-link addresses require explicit advanced approval, recorded in report. Tokens permit root and conditional static version only; no session or acquisition grant. |
| A2 | FIXED; fixture provenance limitation | `acquisition/mod.rs::recognize`; `fixtures/asr-root-redacted.html`; acquisition tests. Initial title-only spoof test failed. ASR/Duster XML requires two distinct static references, emits candidate `asr_duster_xml`; E-LINK is brand evidence only. OpenWrt requires title plus uhttpd. ZTE/Huawei/D-Link/VIDA title-only family matching removed. | Reference-only fixture reconstructed from reviewer-supplied genuine-page references, not a full genuine captured page. Candidate classification remains spoofable and grants no identity/API authority. No corrected physical recognition run claimed. |
| A3 | FIXED in software; hardware NOT RE-RUN | `connection{,/windows}.rs::{version_metadata,probe_version,probe_path,request_path}`, `acquisition/mod.rs::{discover_on_networks,version_evidence}`, `Devices.tsx`. Initial static-version test failed against a nonfunctional parser stub. Only recognized ASR family triggers fixed `GET /version_num.txt` under the same approval. Strict bounded parser emits endpoint_response/observed/observed evidence, endpoint/interface bound. Fixed-path credential-free transport, malformed parser, preserved reported/observed versions and never-verified firmware tests pass. | Evidence is in the bounded cached report, not a migration or overwrite of saved identity. New physical GET not executed. Reviewer-reported REL_MRVL_UAPXC_v2.6.0 and owner-reported MF69_AM802L_ZX_DD_N_241106_EU are not merged or inferred to be equivalent. |
| A4 | FIXED | `connection.rs::subnet_conflict`, `connection/windows.rs::probe_path`, `acquisition/ipc.rs` whole-inventory network collection. Initial collision test failed against a nonfunctional helper stub. Selected endpoint checked against all deduplicated active physical interfaces before TCP/HTTP; UI message names conflicting interface. Diagnostics omit custom interface names, with regression. | No route changes or wrong-interface fallback. Future IP_UNICAST_IF client is design-only in FIRMWARE_ACQUISITION_ENGINE.md. |
| A5 | FIXED | CapabilityState adds Observed and NotImplemented; `assess` emits observed device detection and neutral unimplemented downstream capabilities. `types/acquisition.ts` and `Devices.tsx` show five summary stages and collapse the remainder. Initial state regression failed. Backend and five-row/collapsed-list frontend regressions pass. | Unknown identity/source applicability stays unknown. Strategies remain unknown, independent and nonexecutable. No new feature controls enabled. |
| A6 | FIXED | Recognizer enforces the first 16 KiB; uhttpd alone cannot compensate for a title beyond the cap. Initial cap regression failed. React shows "Metadata limited to first 16 KiB." as plain information, with a no-alert regression. | The flag records reaching the collection cap; it is not a claim of full-page parsing. No signals beyond the cap are used. |

Additional corrections during verification: one token fixture used a public documentation-only address and was correctly rejected by endpoint policy; changed fixture to a private gateway. Clippy found two needless borrows in new tests; corrected. Collision diagnostics briefly included UI message text containing the other interface name; final export explicitly projects safe probe fields and a new regression excludes custom names. No unrelated simulator or adapter implementation changed.

## Tests and commands

Commands run from repository root. Logs: `.tools/phase7-5-closeout/`. All commands below actually ran; GUI/preservation evidence is appended after completion.

| Command | Exit and result |
| --- | --- |
| `py scripts/probe-native-isolation.py .tools/phase7-5-closeout/normal-before.json` | 0, read-only normal-data baseline |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --locked --offline acquisition::tests::a -- --nocapture` | 101: all six initial A1-A6 tests failed; no implementation success claimed |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --locked --offline acquisition::` | Initial 101: 17 passed, token fixture private-address error; fixture corrected |
| `npm.cmd test` | Final 0: 145 tests, 17 files |
| `npm.cmd run lint` | 0 |
| `npm.cmd run build` | 0: TypeScript and frontend production build |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all -- --check` | 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | Initial 101: two new test-only needless borrows; final 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --locked --offline` | First 228 passed / 1 ignored before final regressions. Subsequent parallel run: 101, 229 passed / 1 failed / 1 ignored; retained simulator failure described below. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --locked --offline -- --test-threads=1` | Final 0: 231 passed / 1 ignored |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --features native-test-profile --locked --offline` | Final 0: 232 passed / 2 ignored |

The parallel standard run failed retained `simulator::runtime::tests::ussd_scripts_followup_busy_cancellation_and_timeout_use_virtual_time_only` at simulator/tests.rs:590: `Unauthorized`, "No unique exact adapter match exists." This occurred with release compilation active. Cause is not conclusively established; serial full-suite reruns pass. No simulator production/test changes made and the initial failure is retained. Ignored tests still require an explicitly started genuine OpenWrt guest; they were not run or counted as passes. Previous accepted physical OpenWrt qualification remains pending.

## Sprint B and hardware disposition

| Item | Status | Files/tests/commands for this item | Left undone |
| --- | --- | --- | --- |
| B1 strategy trait | NOT STARTED - re-review gate | None | Static catalog unchanged; trait/plan framework pending |
| B2 artifact classes | NOT STARTED - re-review gate | None | Existing database/categories/Library preserved; no migration |
| B3 vendor allowlist registry | NOT STARTED - re-review gate | None | Only Sprint A's fixed root/static-version paths; no general vendor registry |
| B4 authenticated ASR identity | NOT STARTED - re-review gate | None | No credentials, digest client or authenticated requests; GET-only file inventory and owner supervision pending |
| B5 plain-HTTP binding | NOT STARTED - re-review gate | None | No MAC identity claims or new session authorization |
| B6 eleven-stage UI | NOT STARTED - re-review gate | None | Only five-row A5 summary and existing collapsed capabilities |
| B7 official-image registry | NOT STARTED - re-review gate | None | No publisher URL, downloads, staging or source qualification |
| B8 analysis worker design/stub | NOT STARTED - re-review gate | None | Existing design retained; no worker, tool execution or signature/extraction claim |

No new physical router requests were made during close-out. H01's earlier USB HTTP 200 evidence remains historical; corrected family/version behavior has software-fixture evidence only. HARDWARE_TEST_MATRIX.md records the reviewer-reported static file separately. H02 and H03 remain independent pending hardware rows; H04/H05 pending. New label photos with identifiers hidden, exact revisions and owner-executed stage evidence have not been supplied. No firmware artifact, hash or provenance exists from acquisition. Normal data protection is rechecked below. No firmware writes, extraction, flashing, reset, live USSD or authenticated vendor operation.

## Review handoff

On `phase-7-5-acquisition-engine`, review `git diff cd0d9af7aa40b93ac364cd2015e50ad8d77fe98c..HEAD`, especially native confirmation/receipt binding, request scope, bounded static version transport, observed-evidence authority, whole-inventory collision handling and redaction. Re-run the two complete Rust profiles and frontend tests above. Check native GUI evidence separately from fixtures and independent reviewer results. Return A1-A6 acceptance/findings to the owner before any B1-B8 implementation. Do not merge to main. End commit is reported in the delivery response to avoid a self-referential commit hash.


## Final native/build and data evidence

| Command | Exit/result |
| --- | --- |
| `npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline` | 0; final ordinary Windows executable rebuilt after diagnostics privacy change |
| `npm.cmd run build:native-tests` | 0; dedicated optimized native-test executable; normal executable preserved |
| `node scripts/verify-foundation.mjs --release-only --device-gui` | Initial 1: UI Automation driver could not locate the native approval window. Win32 owned-window lookup and IDNO-only click corrected test driver. Final 0: 20 checkpoints, zero console issues. |
| `node scripts/verify-foundation.mjs --gui-preview --device-gui` | 0: 23 checkpoints, zero console issues, actual development React/SQLite IPC, six destinations, HMR, loading/error recovery and clean shutdown |
| `py scripts/probe-native-isolation.py .tools/phase7-5-closeout/normal-final.json .tools/phase7-5-closeout/normal-before.json` | 0; complete final snapshot byte-identical to baseline |

Native release test found the actual owned native confirmation and clicked No, then confirmed React showed "Endpoint check not approved.", had no report and allowed another check. This test never clicks Yes or sends a router request. Its final screenshot was visually inspected at `.tools/foundation/2026-10-22T22-49-25.282Z/production-route-approval-rejected.png`. Actual exported diagnostics from the final isolated binary omit raw probe messages. Failed native profile/evidence remain retained; successful owned profiles were safely cleaned after their processes stopped. No normal SQLite/WAL rows, settings, firmware vault, saved identities or other captured files changed. No storage move, migration, history reset or merge.

[Redacted close-out evidence](evidence/phase-7-5-closeout-2026-10-22.json) retains initial six failures, final test totals, native results and equal baseline/final snapshot digests. Full retained simulator/firmware native suite and genuine OpenWrt guest tests were not re-run; retained simulator/adapter/storage unit regressions are included in both full Rust suites. Physical A2/A3 HTTP/version collection and owner-supervised authenticated reads are not claimed verified. No new artifact exists, so acquisition hash/provenance is not applicable.

**Close-out verdict:** A1-A6 software corrections verified with stated fixture/DHCP/hardware limitations; pending independent re-review. Sprint B NOT STARTED; Phase 8 NOT STARTED. Smallest next step: reviewer inspects this committed correction diff and returns acceptance or findings before B1-B8 work.

## Exact files changed

- `apps/desktop/src-tauri/build.rs`
- `apps/desktop/src-tauri/capabilities/main.json`
- `apps/desktop/src-tauri/fixtures/asr-root-redacted.html`
- `apps/desktop/src-tauri/permissions/autogenerated/approve_firmware_route_probe.toml`
- `apps/desktop/src-tauri/src/acquisition/approval.rs`
- `apps/desktop/src-tauri/src/acquisition/ipc.rs`
- `apps/desktop/src-tauri/src/acquisition/mod.rs`
- `apps/desktop/src-tauri/src/acquisition/tests.rs`
- `apps/desktop/src-tauri/src/connection.rs`
- `apps/desktop/src-tauri/src/connection/windows.rs`
- `apps/desktop/src-tauri/src/lib.rs`
- `apps/desktop/src/features/Devices.tsx`
- `apps/desktop/src/lib/backend.ts`
- `apps/desktop/src/test/Devices.test.tsx`
- `apps/desktop/src/test/acquisitionBackend.test.ts`
- `apps/desktop/src/types/acquisition.ts`
- `docs/DEVICE_VENDOR_DISCOVERY.md`
- `docs/FIRMWARE_ACQUISITION_ENGINE.md`
- `docs/HARDWARE_TEST_MATRIX.md`
- `docs/PHASE_7_5_SPRINT_A_CLOSEOUT.md`
- `docs/PHASE_7_5_VERIFICATION.md`
- `docs/PHASE_STATUS.md`
- `docs/evidence/phase-7-5-closeout-2026-10-22.json`
- `scripts/reject-route-approval.ps1`
- `scripts/verify-device-gui.mjs`
