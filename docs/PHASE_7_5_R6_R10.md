# Phase 7.5 R6-R10 corrections and B1 design handoff

23 October 2026, Africa/Lagos. Branch `phase-7-5-acquisition-engine`. Starting HEAD `b5625783bb0eb2d2b27b3218bd8dd59af10e375e`; its only changes after accepted `09481b9` are 51 simulator screenshots, preserved. R1-R5 are accepted by independent review; Sprint A remains PASS WITH LIMITATIONS. No merge, Phase 8, physical OpenWrt qualification or acquisition claim.

## Findings, changes and reproductions

| Finding | Status | Exact change / evidence | Remaining boundary |
| --- | --- | --- | --- |
| R6 blackhole-peer scheduling | FIXED | `openwrt/transport.rs::tests::blackhole_peer_cancellation_and_timeout_release_child_and_pipes`: server binds and sends its listening address over a channel before the client is spawned; the worker creates its operation control after scheduling. Cancellation waits for actual acceptance, without a five-second accept timeout. Timeout need not reach TCP accept; an owned loopback wake-up releases the listener for cleanup after the operation returns. | Production SSH implementation unchanged. Cancellation has a bounded 60-second guard but is triggered by acceptance, not a sleep. Timeout stays 750 ms. A teardown test is not physical OpenWrt qualification. |
| R7 final-revision repetition | FIXED through final-revision measurement | Committed `scripts/verify-parallel-rust.py` runs the exact full-workspace command twenty times, default parallel. It reads the final HEAD before/after every run, rejects tracked dirt/revision changes, and records command, exit code, counts, elapsed time and log hashes. The final commit/count pairs are in `.tools/phase7-5-r6-r10/final-parallel/results.json` and the delivery response. | This record is generated after the last commit, so it can name the actual tested commit without a self-referential hash. No source/doc edit or extra commit follows the final series. The old 240-test series is historical. |
| R8 stray `<` | FIXED | `acquisition/mod.rs::resource_attributes` advances one byte when `<` is not followed by ASCII letter, `/` or `!`, preserving following real tags. New `acquisition::tests::r8_stray_less_than_keeps_following_resources_candidate_only` covers comparison/number/trailing-less-than/Unicode text and exact equality of genuine capture recognition with and without the prefix. | A real tag can still be spoofed by a page. Confidence remains candidate, not identity or acquisition authority. No approval behavior changes. |
| R9 matrix consistency | FIXED | `docs/HARDWARE_TEST_MATRIX.md`: removed false current statements about reconstructed fixture/not-rerun reader, retained dated evidence and changed `H01?H05` to `H01-H05`. UTF-8 scan of all touched/previously edited evidence documents found no replacement characters or corresponding mojibake. | No new hardware fact inferred from text cleanup. H02/H03 remain independent; H02-H05 have no new observations. |
| R10 evidence authorship | FIXED | Evidence wording edited directly in the diff. Exact owner quotes, date, chat source and web-display-only scope added to the matrix, original report and JSON. Removed ignored `finalize-review.py`, `record-owner-confirmation.py` and the unused read-only `validate-final-evidence.py`; none is invoked for this correction. The new committed runner writes computed fields/logs only. | Owner statements cannot verify internal router storage/firmware bytes. Computed Windows app-data equality is separate. |
| B1 | DESIGN WRITTEN; NOT IMPLEMENTED | `docs/FIRMWARE_ACQUISITION_ENGINE.md` contains proposed trait/proof ownership, evidence rules, vendor allowlists, model-independent selection, approval/errors, B2 classes/migration, B5 HTTP binding and B4 device-JavaScript citations/open gates. | Stop for design review. No B1-B6 code, vendor login or new live path. |

Pre-fix R6 failure is the retained actual `parallel-18.log` from the prior review task: exit 101, 239 passed / 1 failed / 1 ignored, `transport.rs:613` accepted-channel `Timeout`. The test body at the starting HEAD still had that exact five-second race. This correction uses that existing failing test/evidence; it does not claim a new deterministic pre-fix runtime failure or fabricate a fresh red run. The corrected targeted test exited 0. Final default-parallel repetitions qualify the corrected scheduling separately.

R8 was added and run before its implementation: exit 101, zero passed / one failed, `stray text prefix: a < b`, actual candidate count 0 versus expected 1 (`r8-before.log`). After correction, the same command exited 0 (`r8-after.log`). The genuine captured MG11 page's full candidate result remains identical, and existing comment/script-string negatives remain passing. Recognition changes only by R8; approval code is untouched. Therefore the guideline does not require another owner-supervised hardware Yes test, and none was performed.

## Commands actually executed

Logs: `.tools/phase7-5-r6-r10/`. All normal tests use default Rust scheduling, without a serial override. Ignored genuine OpenWrt guest tests are not counted as passing.

| Exact command | Exit / actual result |
| --- | --- |
| `git status --short`; `git rev-parse HEAD`; `git log -4 --oneline`; `git diff --stat 09481b9..HEAD` | 0; clean baseline b562578, intervening screenshots preserved |
| `py scripts/probe-native-isolation.py .tools/phase7-5-r6-r10/normal-before.json` | 0; read-only baseline |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --locked --offline r8_stray_less_than_keeps_following_resources_candidate_only` | Before 101; after 0, one test; genuine-page equality included |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --locked --offline blackhole_peer_cancellation_and_timeout_release_child_and_pipes` | 0; one test, both cancellation and timeout branches |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all` | 0; targeted source formatting |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | 0; 243 passed / 1 ignored plus successful binary/doc targets |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --features native-test-profile --locked --offline` | 0; 244 passed / 2 ignored |
| `npm.cmd test` | 0; 145 tests / 17 files |
| `npm.cmd run lint` | 0 |
| `npm.cmd run build` | 0; TypeScript and Vite production build |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all -- --check` | 0 |
| `py scripts/probe-native-isolation.py .tools/phase7-5-r6-r10/normal-after.json .tools/phase7-5-r6-r10/normal-before.json` | 0; database/WAL/rows/settings/saved-device/vault/logs snapshot unchanged; integrity ok, no FK violations |
| `py scripts/verify-parallel-rust.py --runs 20 --output .tools/phase7-5-r6-r10/final-parallel` | Final-commit command; individual exits, commit hashes and parsed counts recorded in `final-parallel/results.json`. Delivery states the measured outcome. Every invocation is the full workspace command above. |
| `git diff --check`; `git diff --cached --check` | 0 at review staging |

No new native GUI or Windows release build was required by this cleanup's exit gate or run here. Prior native/build/hardware results remain explicitly historical Codex-reported results, not fresh checks. No router request or credential entry was made. Device JavaScript was only searched/read locally; several speculative file paths did not exist and were not used as citations. A shell regex quoting error was corrected during local search; it produced no device request or source modification.

## Evidence authorship and file accounting

The two retired wording-generation scripts in the prior task modified exactly these four tracked files:

- `docs/PHASE_7_5_REVIEW_FIXES.md`
- `docs/PHASE_STATUS.md`
- `docs/HARDWARE_TEST_MATRIX.md`
- `docs/evidence/phase-7-5-review-2026-10-23.json`

They are not preserved or silently moved into a committed generator. This correction directly edits the report/matrix/evidence/status; it does not run a document-writing script. The retired validation script read/asserted evidence and did not modify tracked documents.

The committed `scripts/verify-parallel-rust.py` writes only `.tools/phase7-5-r6-r10/final-parallel/run-01.log` through `run-20.log` and `results.json`. It never writes a review verdict, owner statement or document. Raw logs are direct command output; JSON counts/hashes/revisions/exits/times are computed. Existing committed `scripts/probe-native-isolation.py` wrote `normal-before.json` and `normal-after.json` under the task directory; shell redirection captured its output in corresponding `.log` files. Rust/Cargo/npm generated their usual build/test outputs and the command logs named above; none authored the evidence documents.

Owner words on 23 October 2026, Africa/Lagos, from this Codex conversation: "Unchanged; Wi-Fi off; USB ready" after Wi-Fi-on; "Settings and firmware unchanged" after initial USB; "Settings and firmware unchanged" after final USB and earlier No. These confirmations cover only the router's settings/firmware web display, not its internal storage. No new owner acknowledgement was solicited or invented for R6-R10.

## B1 review boundary and delivery

B1 is a proposal only. The cited device code supplies literal GET call sites, but does not yet establish a safe, sufficient exact-identity response; Digest URI mismatch and missing DHCP/source-binding evidence remain explicit blockers for future authenticated implementation. No endpoint is borrowed from another vendor. Configuration backups can never become firmware by relabelling or SHA deduplication. No model-name runtime branch, new adapter, strategy executor, schema change or physical write was added.

Changed implementation/test files: `src/acquisition/mod.rs`, `src/acquisition/tests.rs`, `src/openwrt/transport.rs` under `apps/desktop/src-tauri/`; one committed measurement script under `scripts/`. Documentation changes: this report, phase tracker, matrix, prior correction report/evidence and B1 design document. All other working modules/data/history preserved.

Review handoff: `git diff 09481b9..HEAD`. Start is b562578; the final end commit and final-revision repetition result are supplied in delivery and the computed record, avoiding a self-reference. Sprint A remains PASS WITH LIMITATIONS; extraction/acquisition remains unqualified. B1 awaits reviewer approval; B1-B6 implementation and Phase 8 remain unstarted.
