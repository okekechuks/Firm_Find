# S1/S2 follow-up and D1-D6 design revisions

## Post-approval implementation addendum

23 October 2026: the owner accepted the revised B1 design and authorized local B1/B3/B6, the model-name scan and D6a characterization. Implementation and current results are in [PHASE_7_5_B1_IMPLEMENTATION.md](PHASE_7_5_B1_IMPLEMENTATION.md). This addendum supersedes the `confirmation pending / NOT DONE` snapshot below; it does not change the scope of the earlier R6-R10 review.

The five B1 strategies are registered, local-only and return `not_implemented`; B3 registers only bounded `GET /` and `GET /version_num.txt` specs for validation, with no dispatch; B6 adds eleven independently reported compatibility stages. Model-name scanning and user-label invariance tests pass. D6a observed `Repository::open` refusing a schema-6 database after changing journal mode from `delete` to `wal` and changing database bytes. No storage fix or migration was made; that remains a separate review/approval gate.

Latest verification: standard Rust 255 passed/1 ignored; isolated-profile Rust 256 passed/2 ignored; one compile-fail doctest passes in each profile; both Clippy profiles, formatting, 146 frontend tests, lint and production frontend build pass. Native GUI, Windows release builds and hardware qualification were not run. No network traffic or physical-device access occurred. Remaining blocked work: schema-6 migration/artifact classes, authenticated client, transport binding, official-image registry and analysis worker. Phase 8 remains unstarted.

---

23 October 2026, Africa/Lagos. Branch `phase-7-5-acquisition-engine`; starting HEAD `8d6b8d31990de09b81661eabef78d9a5b0e328ce`, clean working tree. R6-R10 accepted; B1 approved with changes. Reviewer independently ran 243 standard / 244 isolated Rust tests, both Clippy profiles, formatting and twenty standard repetitions at that revision. The reviewer's initial loop parser falsely matched `0 failed; 1 ignored`; the reviewer corrected it and reran, reporting all twenty exits 0. That parser error is not a code/test failure. Frontend/native/build/hardware checks were not independently repeated.

## Disposition and files

| Item | Status | Change / evidence | Left undone |
| --- | --- | --- | --- |
| S1 bounded connected-peer test | DONE | `apps/desktop/src-tauri/src/openwrt/transport.rs::tests::blackhole_peer_cancellation_and_timeout_release_child_and_pipes`: listening and genuine acceptance have 60-second safety limits. Both branches require a live accepted SSH connection; a cleanup wake-up cannot qualify. Timeout is armed for 750 ms only after acceptance; the worker's completion timestamp must reach that deadline. Completion wait is capped at 10 seconds, sink accept/shutdown are bounded, cleanup precedes assertions. | Production SSH adapter and real operation budgets unchanged. These are loopback teardown tests, not physical OpenWrt qualification. |
| S1 deadline regression | DONE | `adapters/lifecycle.rs::OperationControl` has a `#[cfg(test)]` event-armed deadline and helper; real safety deadline remains an upper bound. New `review_deadline_tests::test_deadline_only_expires_after_arming_and_survives_stage_clone` proves an unarmed clock is inactive and arming propagates through a stage clone. Hook/field absent from non-test executables. | No production IPC or native-test-profile runtime can arm this unit-test hook. |
| S2 commit/screenshots | DONE | `.gitignore` excludes new generated `docs/images/phase6-*` and `gui-*.png`. `scripts/verify-foundation.mjs` always configures GUI captures under its ignored run evidence; `verify-gui.mjs` has an ignored fallback and records the actual computed path; `verify-simulator.mjs` writes under the same run's ignored `screenshots/` directory. | Existing 51 tracked screenshots and history preserved. No generated PNG staged/committed. This commit uses an accurate subject; no history rewrite. |
| D1 durable source identity | DONE | `docs/FIRMWARE_ACQUISITION_ENGINE.md`: removed temporary-location claims, retained five verified SHA-256 anchors, added `layout_manager.js` anchor and reproducibility through exact device `/js/...` files plus hash comparison. | No vendor scripts committed or new static fetch performed. |
| D2 complete authentication | DONE, design only | Same document: named challenge, Digest-login, identity-read and logout specs with exact code citations; session-state distinction, hashed query-secret logging, browser displacement and lockout risks. Login/logout are explicit future allowlist entries with their own purpose/approval; Download/Upload denied. Fixed device signing URIs are recorded, not treated as alternate-path guesses. | No auth client, credentials, session requests, retry loop or CGI request of any kind implemented/sent. Future spec bounds/cleanup consent require separate B4 review. |
| D3 primary identity source | DONE, unresolved literal documented | `sysinfo/device_name` and `version_num` are primary. Static code identifies `/xml/ui_mifi.xml`: root line 24 sets `mifi`; `layout_manager.js:121` builds the menu path. Menu `xmlName` mapping is cited at `:55-75,90-91`. Local supplied capture has no menu XML body, so dashboard CGI filename remains unknown. `status1` stays excluded unless the primary source is insufficient and separately reviewed. | No guessed filename/request. Chosen parser must discard IMEI, SIM/subscriber and device-unique fields before any output, with no raw-body storage/logging. |
| D4 HTTP policy | DONE, design only | Per-field `authenticated_management` evidence labelled `transport: plain_http`, session-bound, may support recognition/display/read-only probes. It never alone authorizes writes, flashing, installation or irreversible operations; future stronger checks/fresh same-session identity/USB correlation/owner label/B5 requirements retained. | No new evidence schema, binding implementation or write authority. |
| D5 closed-table tests | DONE as planned tests | Design requires every RequestSpec originate in the closed compiled table; no assembled vendor-JavaScript string can become a request. Full deny list: set, upload, upgrade, reboot, restore, reset, factory, download, delete, format, flash. Table tests include real helper action shapes and encoded/caller/redirect bypasses. | B3 validator/tests are not implemented until confirmation; initial skeleton may register only `/` and `/version_num.txt`. |
| D6 migration gate | DONE; untouched-refusal guarantee DISPUTED by source order | Exact SQL, backup implementation and rollback tests require separate approval. Existing `storage.rs:22-30` rejects version >5 before migrations, but requests WAL at line 26 before reading the version. Existing test at `:521` checks only version 99, not byte/journal equality. Startup call/propagation cited at `lib.rs:124,169`. | Cannot claim refusal without any DB modification. WAL mode may change; a separately reviewed preflight fix and preservation regression are needed before B2. Storage/startup code and schema untouched here. |
| Part 3 B1 local interface | NOT DONE - required confirmation pending | No acquisition code changed. | After confirmation only: five registered local strategies, probes `not_implemented`, nonexecutable plans, no new network scope issued. |
| Part 3 B3 skeleton | NOT DONE - required confirmation pending | No registry/dispatcher code changed. | Closed validator and two existing read specs as data only. |
| Part 3 B6 eleven-stage view | NOT DONE - required confirmation pending | No frontend application components changed. | Missing adapters neutral; no stage grants another stage's authority. |
| Part 3 model-name scan | NOT DONE - required confirmation pending | Planned scan unchanged in design. | Add only after confirmation, starting with a failing test. |

The owner's current guideline requires: "Send the updated design and the S1 test change for a quick review first. After the reviewer replies 'design accepted', implement only these local, non-network items." That is the stop gate for this delivery. B2/B4/B5/B7/B8 remain blocked under their separate reviews; Phase 8 unstarted, no acquisition/artifact claim.

## S1 verification details

The former cancellation branch's unbounded `recv()` is removed. Sink readiness/acceptance and worker completion have independent safety bounds. The test sink uses nonblocking accept with channel-driven shutdown; a failed SSH startup or failed cleanup wake-up cannot leave it in an unbounded accept. It holds its accepted stream until the observed child/pipe operation returns and main releases it.

The unit-test-only `OnceLock<Instant>` is unarmed during client process startup. After a genuine SSH accept is reported, the parent either cancels or arms the deadline at now +750 ms. A cleanup connection made after the operation finished is marked ineligible, and both branches assert live establishment. The timeout branch also compares the worker's return timestamp against the armed deadline, so a setup safety timeout cannot pass as a connected blackhole timeout. Existing `run` cleanup must kill/wait/drain before return, while the sink still owns its silent socket.

No new failing assertion was reported during these corrections: the prior review's concrete S1 defects were confirmed by reading the accepted test; the corrected test and the new deterministic clock regression passed. Initial targeted run predates the final nonblocking cleanup/timestamp strengthening; the final full suites and repetitions include it. No serial scheduling or longer sleep substitutes for acceptance synchronization.

## Commands and actual results

Command logs and computed snapshots are under `.tools/phase7-5-design-review/`. NativeCommandError formatting of successful compiler stderr is not the exit status.

| Exact command | Exit / result |
| --- | --- |
| `git status --short`; `git log -3 --oneline` | 0; clean 8d6b8d3 baseline; previous history retained |
| `py scripts/probe-native-isolation.py .tools/phase7-5-design-review/normal-before.json` | 0; read-only normal-data baseline |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --locked --offline blackhole_peer_cancellation_and_timeout_release_child_and_pipes` | 0; initial corrected targeted test, both branches; `s1-targeted.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all` | 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | 0; final 244 passed / 1 ignored, binary/doc targets successful; `standard-final.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --features native-test-profile --locked --offline` | 0; 245 passed / 2 ignored; `isolated.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | 0; production and unit-test targets |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all -- --check` | 0 |
| `npm.cmd test` | 0; 145 tests / 17 files |
| `npm.cmd run lint` | 0, including changed native verification scripts |
| `npm.cmd run build` | 0; TypeScript and Vite production frontend |
| `node --check scripts/verify-foundation.mjs`; `node --check scripts/verify-gui.mjs`; `node --check scripts/verify-simulator.mjs` | 0 each; capture-path syntax checks |
| `git check-ignore --no-index docs/images/phase6-example/generated.png docs/images/gui-production-example.png .tools/foundation/example/gui/generated.png` | 0; all three prospective generated paths ignored; no files created |
| `py scripts/probe-native-isolation.py .tools/phase7-5-design-review/normal-after.json .tools/phase7-5-design-review/normal-before.json` | 0; full normal DB/WAL/rows/settings/saved-device/vault/log snapshot unchanged; integrity ok, no FK errors |
| `py scripts/verify-parallel-rust.py --runs 20 --output .tools/phase7-5-design-review/final-parallel` | Final-commit repetition; computed commit/exit/count/log-hash record in `final-parallel/results.json`, measured outcome and final hash in delivery. Full workspace command above, default parallel. |
| `git diff --check`; `git diff --cached --check` | 0 before commit |

Repetition verification checks exactly: twenty run records; each `exitCode == 0`; commitBefore/After equal final HEAD; no tracked changes before/after; first result is 244 passed / 0 failed / 1 ignored; every recorded log SHA-256 matches its bytes; the connected blackhole and new armed-deadline tests each have an ` ... ok` line in every log. This uses structured counts and exit codes, not a grep that matches the words "0 failed" as a failure. Final hash is computed after commit, not guessed into this document. The committed repeat runner writes logs/computed JSON only; no script writes review decisions or owner confirmations.

Not run: native GUI launch/screenshot capture, either Windows release build, genuine OpenWrt guest tests, physical hardware tests, authentication, menu fetch or any live CGI request. S2 path behavior is source/syntax/ignore-rule checked, not claimed visually verified. Unit transport tests use owned loopback peers only. No network traffic was sent to the MiFi or Internet; no credentials requested. No new owner confirmation or hardware-matrix observation was invented.

## Review handoff

Changed files: `.gitignore`; `apps/desktop/src-tauri/src/adapters/lifecycle.rs`; `apps/desktop/src-tauri/src/openwrt/transport.rs`; `scripts/verify-foundation.mjs`, `verify-gui.mjs`, `verify-simulator.mjs`; `docs/FIRMWARE_ACQUISITION_ENGINE.md`; this report; `docs/PHASE_STATUS.md`. No generated screenshots, vendor scripts, normal database or new acquisition implementation is committed.

Review `git diff 8d6b8d3..HEAD`, particularly S1 bounded cleanup/acceptance/deadline proof, D2 named session operations, D3 unknown filename and D6 WAL-order caveat. Confirm S1/S2 and revised design before Part 3. Final branch/start/end and exact repetition revision are supplied in delivery; no merge or history rewrite.
