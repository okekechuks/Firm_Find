# Phase 4.1 targeted hardening verification

Date: **17 October 2026**, Africa/Lagos. Baseline: accepted Phase 4 at clean commit `af09834`. Existing structure, schema 4, discovery, registry, profiles, library and jobs were retained. No Phase 5 work, router writes, firmware extraction/execution or flashing was added.

**Phase 4.1 verdict: PASS. Phase 5 readiness: READY; not started.** The corrected isolated workflow passes actual Windows development/release verification, not just unit tests. The earlier normal-startup delta below is retained and explicitly excluded from claims of unchanged data.

## Source changes

- `src-tauri/src/native_test.rs`, `native_test_main.rs`, `tauri.native-test.conf.json`, Cargo feature `native-test-profile`: separate test entry point, fixed known-folder root, strict run IDs, marked/locked run directories and guarded cleanup. Ordinary `obtainer.exe` always uses the ordinary entry point and rejects test activation before opening app data.
- `src-tauri/src/lib.rs`, `commands.rs`, `domain.rs`, `storage.rs`: retain normal path/lifecycle; select isolated data only for the dedicated test entry point, configure a separate per-run WebView directory, and expose optional test-context metadata. No migration or persistent schema change.
- `src-tauri/src/library/{types,mod,ipc,db}.rs`: structured export outcome separates filesystem success from activity failure; audit event/link now commit together. Existing manifest JSON, privacy exclusions, synchronization and no-overwrite publication retained.
- `src/types/{contracts,library}.ts`, `lib/backend.ts`, `app/App.tsx`, `features/FirmwareLibrary.tsx`: runtime-validated export result and explicit test banner; saved-with-warning remains a successful save.
- `library/tests.rs`, frontend tests and native-test module tests: seven Rust and three frontend regressions added without removing baseline cases.
- `scripts/{build-native-tests.ps1,native-test-profile.mjs,probe-native-isolation.py,verify-foundation.mjs,verify-firmware.mjs,native-smoke.mjs}`, root `package.json`: controlled build/profile activation, normal-data evidence, generated fixtures, native failure injection, process census and cleanup validation. Legacy smoke also refuses ordinary application binaries.

All paths above are relative to `apps/desktop/` unless prefixed `scripts/`, `docs/` or explicitly root. No existing user data or previous Phase 4 test history was removed.

## Isolation and cleanup strategy

The ordinary application continues to use `C:\Users\chuka\AppData\Local\OBTAINER\obtainer.db` and its existing vault/profile. Test root is derived from Windows `SHGetKnownFolderPath(FOLDERID_LocalAppData)`, not an arbitrary environment path:

```text
C:\Users\chuka\AppData\Local\OBTAINER-NativeTests\<32-lowercase-hex-run-id>\
  data\obtainer.db
  data\{vault,logs,workspaces,reports}\
  webview\
  fixtures\
  exports\
  .obtainer-native-test.json
  .native-owner.lock
```

Only the feature-enabled `obtainer-native-test.exe` can activate this profile, and it requires a validated run ID. Missing IDs, traversal/opaque path input, unknown native-test overrides and general data-directory overrides reject safely. Config/executable identities must match. The normal entry point cannot switch to test mode even when compiled against a feature-enabled library. Normal development command is unchanged.

Cleanup accepts only the current run ID, never a path. It validates marker kind/version/ID/state, canonical containment under the fixed root, exclusive ownership, process census and the complete tree before deletion. It refuses if another native-test executable is alive, a WebView process references the run, ownership inspection fails, staging is nonempty, an unknown root item exists, or any reparse point/link appears. The conservative process rule also serializes cleanup across independent native-test apps. Directory leases protect traversal; the marker becomes `cleaning` before deletion so a partial cleanup cannot be adopted as a fresh run. Uncertain files are retained with a reason.

React and the native window identify **ISOLATED NATIVE TEST** with run ID. Discovery stays real `windows_observed`, explicitly with `simulatedAdapters=false`; it does not establish verified support. Future mock adapters must have explicit separate provenance. No adapter or simulator was implemented.

## Manifest export consistency

Export now returns `{saved, activityRecorded, warning}`:

| Outcome | Result/UI |
| --- | --- |
| User cancels Save | `saved=false`, `activityRecorded=false`, no warning; UI says cancelled. |
| Publication fails or destination already exists | Error, with no overwrite. |
| Publication and audit succeed | `saved=true`, `activityRecorded=true`, no warning. |
| File publishes but audit transaction fails | `saved=true`, `activityRecorded=false`, separate warning explicitly says the manifest is saved. The valid export is retained. |

Filesystem publication remains the save commit point. An audit failure never deletes the saved export or reports that no file exists. Audit text/link inserts use one SQLite transaction; failed logging leaves no partial audit event. Unit failure injection confirms a parsed valid manifest remains, repeated same-name export is rejected without changing bytes, and later logging works again. Frontend regressions cover both warning and cancellation. Native failure injection is required at the completion gate below.

## Normal data baseline

The first read-only probe records schema **4**, integrity `ok`, no foreign-key violations, one artifact, 34 jobs, 20 import occurrences, 30 integrity checks, 74 activity links, 211 audit events, 122 device/metadata rows, one saved router, two profile versions and two reported observations. The final isolated qualification baseline has **213 audit events**; other counts are unchanged. Each isolated suite compares its own complete before/after evidence exactly.

Settings fingerprint: `70ca50b9ab62d27cbc6340ed90d7ebab7596b61ac8875a63e86219af831f4d97`.

Initial complete table-content fingerprint: `48cf60d250be0d3f8839eb98a07129b24b1c7acd6a1d0c5b842ea270b1a57899`. Final isolated baseline **and after**: `5cdc4186397ffbc5dd95e3cee881e2ae674f7fed0b81dc79c005a96d19eb4c68`.

Saved router ID: `b61f3765b1807d02d05f1c6e0236967025f51e671ebc062c70fe62c90dcdb147`. Profile IDs: `observed.comfi-mg11`, `tplink.archer-c7-v5.openwrt-23-05-5`.

Normal main DB SHA-256 throughout: `b3d00cba067dde53dd3a4dc01d0a56e1f325396a1122a25964d5061be518d980`. Initial WAL: `791732d5723caf5e77d7646741e531da4e6a5c079fa44f03b8aa05c05fc15c23`. Final isolated baseline **and after** WAL: `a39098df08f51e94d95eccbf8b0729372cdf883e4e44832f92dc5e58cb6127fa`.

Existing vault contains the retained generated Phase 4 1 MiB blob `c9d16ec151d225feed57262437acf87a52427fcc7bb749c84051a11016e01264`. Nothing removes that blob or its 34-job history. Full isolated-suite comparison includes every table's contents in a collective fingerprint, main DB/WAL bytes, vault path/size/hash inventory, logs/workspaces/reports and integrity/FK checks. No checkpoint, migration or write connection is used by the normal-data probe.

**Earlier normal startup, not an unchanged-data claim:** between the first preflight and the corrected separate-binary build, the normal database records `application_started` (ID 212, `05:02:43.847Z`) and `device_inventory_changed` (ID 213, `05:02:44.322Z`). Its WAL, collective row fingerprint and startup log changed. Removing those two rows only in an in-memory diagnostic does not recreate the initial fingerprint, so existing row content changed too; passive inventory refresh is a likely explanation, not a verified field-by-field delta. There is no initial row snapshot proving that delta, and no unsupported attribution to the user is made. Nothing was deleted or rolled back to conceal this. Schema, main DB bytes, settings, saved-router/profile IDs, artifact/job/history counts and all vault blobs remain equal to the first probe. The final isolated implementation is qualified against the later baseline: the failed hardware/timeout runs, full successful suite, WebDriver smoke and changed-settings round-trip each leave normal data exactly equal to their own before snapshot. This report does **not** claim byte equality across the entire earlier build-debugging interval.

## Executed checks

`CARGO_INCREMENTAL=0` is set for native/Rust compilation. Windows filesystem tests use a normal user session; no administrator access or package installation is required.

| Command | Exit/result |
| --- | --- |
| `npm.cmd test` | 0; **56 tests / 8 files**, prior 53 retained. |
| `node node_modules/typescript/bin/tsc --project apps/desktop/tsconfig.json --noEmit` | 0. |
| `npm.cmd run lint` | 0; zero warnings. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all` | 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all --check` | 0, including final check. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0 final; initial collapsible-if warning corrected. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | 0; **76 tests**, prior 69 retained. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | 0; **76 tests**, including final process/cleanup guards. |
| `npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline` | 0; normal frontend production/optimized Tauri binary, latest normal compile 50.21 s. |
| `npm.cmd run build` | 0; TypeScript project build and Vite production assets, also run by the native build helper. |
| `npm.cmd run build:native-tests` | 0 final; dedicated optimized test binary compiled in 1m21s; ordinary executable hash unchanged. |
| `npm.cmd run test:foundation -- --phase3 --phase4 --phase4-1` | 0 final; **83 acceptance checks** across real development/recovery/release. |
| `npm.cmd run test:foundation -- --settings-roundtrip` | 0; **17 checks**, changed release setting → development → restored release setting. |
| `npm.cmd run test:native -- target/release/obtainer-native-test.exe --audit` | 0; retained WebDriver workflow, six logged PASS checkpoints, including two native launches. |
| `git diff --check` | 0; Git reports existing LF/CRLF conversion notices, no whitespace errors. |

Initial test-build experiments exposed Tauri CLI package-binary build/rename behavior. Setting `mainBinaryName` renamed the normal executable; removing that option alone still changed the normal target, which the hash guard rejected. The normal target was rebuilt, and the final helper uses Cargo's explicit extra-binary selection with Tauri test config and `tauri/custom-protocol` to embed production assets. It leaves the normal executable byte-identical. These were build-output failures, not application-data changes. A first native preflight stopped because the normal target had been renamed; before/after normal-data evidence remained equal and no app window was launched. Raw failed evidence is retained at `.tools/foundation/2026-10-17T04-55-45.592Z/`.

## Native completion gate

**VERIFIED:** full suite evidence `.tools/foundation/2026-10-17T05-15-40.890Z/`, run ID `590b48b960a7bfe686b62af03177b96d`. All data was beneath `C:\Users\chuka\AppData\Local\OBTAINER-NativeTests\590b48b960a7bfe686b62af03177b96d\`; final guarded cleanup exited 0 and removed this run only. Before/after normal-data JSON is identical. Console issues are `[]`. Development library and optimized-release library/compatibility screenshots were visually inspected: actual rendered React, isolated notice, real generated artifacts and UNKNOWN capability matrix were visible.

Actual development command (three launches):

```powershell
npm.cmd run tauri -- dev --config src-tauri/tauri.native-test.conf.json --features native-test-profile -- --bin obtainer-native-test --locked --offline
```

Native development URLs are `http://127.0.0.1:1420/`; optimized release is `http://tauri.localhost/` from embedded assets. Development wrappers exit **0, 1, 0** respectively: the middle launch is intentionally killed during an observed copying job, not a startup failure. Release `.\target\release\obtainer-native-test.exe` exits **0**. Real native windows, React navigation, validated Rust/SQLite IPC, same-directory restart persistence, loading/error recovery, unchanged app/Vite PIDs during HMR and exact temporary-source restoration pass. No owned app, WebView, Vite or listening port remains after shutdown.

| Required failure scenario | Classification and actual evidence |
| --- | --- |
| Normal import | VERIFIED: native Open → React confirmation → SQLite job → original source and SHA-256 vault byte comparison, in dev/recovery/release. |
| Duplicate import | VERIFIED: native filename alias imports retain one blob and distinct unverified occurrences. |
| Concurrent identical imports | VERIFIED: two native selections submitted together, pending/running overlap, both complete; one 256 MiB blob, two occurrences, exactly one duplicate. Durations 23.10s and 38.48s, excluding second job's queue time. |
| Cancel while copying | VERIFIED: observed actual nonzero 256 MiB progress, React Cancel, terminal cancelled/idempotent retry, no incomplete artifact. |
| Cancel near commit | VERIFIED in deterministic Rust worker-barrier tests `jobs_enforce_transitions_and_cannot_commit_after_cancellation` and `cancellation_at_publication_does_not_poison_an_existing_valid_artifact`; no timing claim from a native UI click. |
| Forced termination during import | VERIFIED: job `89ec866bdf2f6a0c80975bcb152677dd` observed copying before killing owned launch. |
| Restart recovery | VERIFIED: same job fails `INTERRUPTED`, known staging removed, no false success, saved artifacts/history remain. |
| Integrity verification | VERIFIED: native streamed verification/history and real React status across launches. |
| Corrupt blob | VERIFIED: only isolated generated blob changed; real IPC reports `HASH_MISMATCH` and matching artifact state. |
| Missing blob | VERIFIED: only isolated generated blob deleted; real IPC reports `FILE_MISSING`; source reimport restores missing bytes. |
| Database failure after publication | VERIFIED: isolated SQLite trigger fails occurrence commit; failed `DATABASE_UNAVAILABLE` job, complete orphan retained without catalog entry; retry verifies/reuses it. |
| Shutdown during verification | VERIFIED: job `0fca7941b559d67ccc965bffcc587c73` observed actively verifying before native window closure; restart retains cancelled state and artifact is not stuck checking. Rust barrier test also passes. |
| Repeated startup | VERIFIED: production reopens the same interrupted terminal job/finished timestamp without rerunning it; dev/release use the same marked database. |
| Clean shutdown | VERIFIED: process/port census after every launch and guarded cleanup exit 0. |
| Manifest audit failure | VERIFIED: actual Save publishes valid private schema-1 JSON, injected activity-link failure rolls back audit and React confirms saved-with-warning. Unit test additionally proves same-name retry cannot overwrite. |
| Cleanup boundaries | VERIFIED: active owner, wrong marker/run ID, unknown staging and real junction refuse deletion; unrelated sentinel survives. Final valid cleanup succeeds. |
| MG11 regression | VERIFIED: powered-on USB `1286:4E31`, real discovery/refresh, candidate identity, four UNKNOWN features and no Apply. `LT800/` and `MF69_AM802L_ZX_DD_N_241106_EU` remain `user_reported` across dev/recovery/release. No router request or configuration change is introduced. |

Settings round-trip evidence: `.tools/foundation/2026-10-17T05-18-32.204Z/`, run ID `15f1fbb565ede4c5e12e18689cfbf69b`, 17 checks and cleanup 0. Smoke evidence: `.tools/native-audit/2026-10-17T05-18-07.242Z/`, run ID `eb60244f6391d162599d070dfc44be92`, WebDriver relaunch and cleanup 0. All normal-data comparisons pass. Ordinary release SHA-256 remains `C70F4D1B41162B4E6CC8FEBFE47F85F5AD1A5FF49910F0ED7063B25756C1436A` after both compilation targets and final checks.

Earlier failures are retained, not replaced by the passing run:

- `.tools/foundation/2026-10-17T05-10-09.419Z/`: exit 1 because the MG11 was off. Real development GUI/IPC/navigation passed; physical qualification did not. Run `ae4064e06b86172a22b75b6c657e88de` retained. Owner subsequently powered it on. RNDIS/composite interfaces are OK; Windows reports diagnostic/AT driver errors, which this phase does not repair or reinterpret as device support.
- `.tools/foundation/2026-10-17T05-11-44.292Z/`: exit 1 because the second queued 256 MiB import exceeded the harness's 60s deadline. Persisted evidence showed first job completed, second progressing through `checking_vault`, not deadlocked. Large concurrent-case deadline is now bounded at 180s including queue time. Run `85395ec4092ca7848674c7a5634f90df` retained; no failed-run cleanup attempted.
- Initial Clippy and build-selection errors were corrected before final qualification. A diagnostic attempt to compare all current rows to the first probe after excluding only the two added audit rows exits 1; existing row content changed too, and that diagnostic is not presented as preservation proof. Early read-only diagnostic mistakes (`created_at` is absent from `jobs`, and shell quoting in a one-line Python query) also exited 1 and were corrected without writes.

Portable commands/checkpoints and preservation evidence: [Phase 4.1 transcript](evidence/phase-4-1-2026-10-17.txt). No critical startup/runtime or isolation blocker remains. Failed test directories intentionally remain for diagnosis. Qualification is limited to this Windows host/filesystem and generated 1 MiB/256 MiB sources; maximum-size, additional filesystems and clean-machine qualification are not claimed.

## Scope

All imported sources are generated harmless bytes. No real MiFi firmware is used. Final qualification does not drive the ordinary production GUI against everyday data. The ordinary release is compiled and activation refusal is tested, while the real shared Tauri development/optimized-release application flows run under the dedicated isolated entry point. This distinction and the earlier normal-startup delta are explicit in native evidence.

See [isolated native testing](NATIVE_TESTING.md) for exact activation/build/cleanup mechanics. Phase 5 is ready and remains unstarted.
