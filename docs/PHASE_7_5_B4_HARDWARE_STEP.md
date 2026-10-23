# B4 static menu: H1 correction and supervised hardware evidence

23 October 2026. Branch `phase-7-5-b4-menu-prestep`.
Start `c3486cb0323b25eb9db578ad6e585e748fb402af`.

Scope: accepted B4 pre-step plus H1 only. No authentication, credentials,
schema migration, transport binding, source registry, analysis worker or Phase 8.
No push, tag or merge. Hardware is separate from software verification.

## H1 design and regression evidence

`connection.rs` adds an optional, normally omitted `observedMediaType` failure
field and a bounded sanitizer. The static menu transport in
`connection/windows.rs` still accepts only `application/xml` and `text/xml`,
ignoring parameters and comparing allowed types case-insensitively.
Wrong types retain only a lowercase token of 1–64 ASCII bytes from
`a-z`, `0-9`, `.`, `+`, `-`, `/`; missing, oversized or hostile tokens become
`unavailable`. No arbitrary header text is retained.

`acquisition/static_menu.rs` sanitizes again before adding this field to a
`wrong_content_type` observation. `acquisition/mod.rs::diagnostics` exports
only that code and sanitized type for this failure, never the filename,
raw response, server, endpoint or peer message. `Devices.tsx` renders the
limitation as “Static menu requires XML; observed media type: …”. The strict
TypeScript schema accepts the field only for this failure.

| Initial failing command | Exit | Evidence |
| --- | --- | --- |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline menu_` | 101 | 12 pass, two fail: new `menu_wrong_media_type_limitation_exports_only_sanitized_type` and extended `menu_loopback_is_one_xml_get_and_enforces_stop_rules`; missing media-type field. `.tools/b4-h1/h1-red.log` |
| `npm.cmd test -- --run src/test/acquisitionBackend.test.ts` | 1 | New schema test rejects intended limitation; five existing tests pass. `.tools/b4-h1/frontend-red.log` |

After correction, targeted commands return 0 (14 Rust tests and six frontend
tests). Loopback cases cover XML with charset, `text/plain`, `text/html`, hostile
and oversized tokens, and missing Content-Type. Collection tests cover missing
metadata and diagnostics excluding response text.

Initial lint failed with `no-useless-escape` for an escaped trailing hyphen in
the new schema regex (`.tools/b4-h1/lint-precommit.log`); removing the unnecessary
escape corrected it. A pre-hardware source inspection found Tauri's `invoke`
property is immutable. The harness was corrected before use to call real IPC
directly, retaining neither approval tokens nor raw HTTP bodies. This was a
source-inspection correction, not an observed hardware failure.

## Software exit gate — done

H1 code commit `f30dae7e20319ea1c24ceda47007b9bec70a4e7e`; final harness/code
commit `429cde56f07aebe26db3389d2c605febd591c82c`. Full checks below ran with
that final committed code and a clean tracked tree. Rust used
`CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=1`; test scheduling remained default
parallel, without `--test-threads=1`.

| Exact command | Exit | Actual result / evidence under `.tools/b4-h1/` |
| --- | --- | --- |
| `py scripts/verify-parallel-rust.py --runs 20 --output .tools/b4-h1/final-parallel` | 0 | Twenty consecutive `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` runs on `429cde5`; each 280 passed, 0 failed, 1 ignored; two doctests passed. `final-parallel/results.json`; log hashes, commit pairs, clean-tree fields and three menu regression markers independently checked across all logs. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | 0 | 281 passed, 0 failed, 2 ignored; two doctests passed. `final-isolated.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | 0 | `final-clippy.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0 | `final-clippy-isolated.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all -- --check` | 0 | `final-format.log` |
| `npm.cmd test` | 0 | 151 tests / 17 files. `final-frontend.log` |
| `npm.cmd run lint` | 0 | No lint warnings/errors. `final-lint.log` |
| `npm.cmd run build` | 0 | `tsc -b` and Vite production build. `final-frontend-build.log` |
| `npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline` | 0 | Normal Windows release; `release-build.log` |
| `npm.cmd run build:native-tests` | 0 | Dedicated isolated Windows release; normal EXE hash preserved. `isolated-release-build.log` |
| `py scripts/probe-native-isolation.py .tools/b4-h1/normal-after-software.json .tools/b4-h1/normal-before.json` | 0 | Identical database/WAL hashes, all rows, settings, saved-device/profile IDs, vault files and other captured data after software checks. |

The ordinary release SHA-256 after the isolated build is
`216094F2E0F7D0D757B933822B38ACA0FDA2ACCA802BFE022EF68683E849D1C6`.
These are Codex-run checks; no independent reviewer rerun is claimed.

## Supervised harness

`scripts/verify-static-menu.mjs` is selected by
`node scripts/verify-foundation.mjs --release-only --device-gui --acquisition-mg11-menu`.
It uses an owned isolated profile, real React popup navigation and Rust IPC. It records
computed invocation outcomes and reads native dialog text through
`scripts/read-native-dialog.ps1`. It never answers a native dialog.

An existing root/version approval precedes the separate menu approval. The only
new authorized device request is one `GET /xml/ui_mifi.xml`. A successful
read outcome is followed immediately by a separate menu approval for the owner
to decline. The harness checks that decline produces no second
`read_static_menu` call, receipt or result. If the positive run hits a stop
condition, it stops device requests without another fetch or workaround; a
separate No dialog remains a request-free rejection check. Native invocation
counts plus proof/loopback tests support rejection evidence; no packet capture
is claimed. Generated captures remain ignored; no screenshots are committed.

## Hardware record — done, observation only

Before statement, 23 October 2026: owner replied “yes confirmed” to the bundled
USB/power/Wi-Fi-off and visible-settings question. An exact firmware version was
then requested and supplied before hardware execution. Software exit gates
were completed before the native launch.

The owner's subsequent before-display words, 23 October 2026, are quoted
verbatim below. Combined with the earlier “yes confirmed” response to the
bundled preconditions/settings question, these are user-reported display
observations, not independently verified firmware evidence:

> Software Upgrade
> Software Information
> Current Software Version:
> MF69_AM802L_ZX_DD_N_241106_EU

Windows adapter/route observation in `.tools/b4-h1/network-before.json` shows
Wi-Fi disconnected and the USB RNDIS interface up. This local inspection sent
no device request.

Initial native-launch authorization was rejected by automatic approval review
before execution: it interpreted the root/version requests as outside the one
static-menu GET limit. Reading guideline lines 28 and 32 established explicit
authorization for the preliminary root/version check. The same command was
then allowed with that evidence; no workaround or request occurred during the
rejected launch. Live native execution uses
`node scripts/verify-foundation.mjs --release-only --device-gui --acquisition-mg11-menu`.
Evidence: `.tools/foundation/2026-10-23T13-00-44.637Z/`; driver log
`.tools/b4-h1/native-driver.log`. Command exit 0; application exit 0.

The actual native sequence, 23 October 2026:

| Step | Status | Actual evidence |
| --- | --- | --- |
| Owner preconditions and before web display | done | Quotes above; local Windows observation shows Wi-Fi disconnected and USB RNDIS up. |
| Root/version check, separate owner Yes | done | Root HTTP 200, `identity_unknown`, `asr_duster_xml` candidate. `/version_num.txt` observed `REL_MRVL_UAPXC_v2.6.0`, unverified; distinct from owner's display value. Root metadata was capped at 16 KiB (`metadataTruncated=true`); this is the existing root collector's cap, not a truncated menu result. |
| Static-menu check, own native Yes | done | Exactly one `read_static_menu` invocation and its single-use transport proof; outcome `observed`, filename `status1`, source `endpoint_response`, reliability/verification `observed`. No firmware acquired. Menu parser/transport returned no truncation, size or media-type failure; exact menu body length/type were not recorded. |
| Separate native No | done | Own second menu confirmation; `CANCELLED`, `readCalls=0`, no receipt/result from a read and no second menu fetch. Native approval code returns before any transport on No; proof/No tests and bounded loopback transport tests passed. This is application-call/control-flow evidence, not a packet capture of all Windows traffic. |
| Owner after web display | done | Exact dated quotes below; version matches before, visible settings reported unchanged. Scope is web display only. |
| Normal application data | done | Native before/after comparison and independent `.tools/b4-h1/normal-after-hardware.json` comparison against original baseline both exit 0. |

The menu confirmation text was read from the actual Windows dialog (not merely
copied from source). Title: `OBTAINER - approve static menu check`. The same text
appeared for positive and negative runs:

```text
Approve one read-only static menu check?

Device: USB Composite Device
Interface: {7F7C5ABA-3722-40E6-A506-E36DF47013EF}
Endpoint: http://192.168.100.1/
Request: GET /xml/ui_mifi.xml (maximum 16 KiB, one attempt)

This reads the dashboard menu mapping only. No login, settings change, firmware transfer or other request is authorized. Plain HTTP is not cryptographically verified. Proceed only if you own or administer this device.
```

Dialog observations (UTC): root `13:00:53.1924648Z`, first menu
`13:04:37.9459488Z`, second menu `13:04:59.2938850Z`.
The owner replied **“done”** to the explicitly sequenced Yes / Yes / No
instructions. Actual native results establish successful approvals for the first
two operations and cancellation before a read for the third. The harness never
answered any of those dialogs.

After-display words, 23 October 2026, quoted verbatim:

> Software Upgrade
> Software Information
> Current Software Version:
> MF69_AM802L_ZX_DD_N_241106_EU
> Current Software Date:
> Nov 7 2024

The owner then explicitly stated:

> Visible settings unchanged

No request to `status1`, login, logout, CGI, JavaScript or any further device
path was sent. The permitted preliminary root/version reads are separate from
the one new static-menu GET. No retry, credentials, firmware transfer, writes,
reboot, restore, reset or guessed authentication occurred. `canObtain` remains
false. `status1` is a candidate filename only; it grants no request authority
and does not resolve the design's identity/data-minimization questions.

Native baseline checks also verified an actual window handle, React and SQLite
IPC, six navigation destinations, three discovery refreshes without duplicate
IDs, no captured console warnings/exceptions, preserved isolated settings, clean
shutdown and guarded cleanup. The `production-devices.png` capture was visually
inspected. Hardware read results were collected through real native IPC and
dialogs; no claim is made that the React menu button rendered this physical
result, since the harness invoked the read directly. GUI button/response behavior
remains covered by the frontend tests.

Computed native evidence is `b4-menu-observations.json`, `results.json`,
`production-summary.json`, `console-issues.json`, `normal-data-before.json` and
`normal-data-after.json` under the native evidence directory above. Captures
remain ignored and are not committed.

Owner statements cover only the router's web display, not internal storage.
Normal application baseline `.tools/b4-h1/normal-before.json`: schema 5,
integrity `ok`, no foreign-key errors. First snapshot attempt exited 1 because
the evidence directory did not exist; creating it and rerunning returned 0.
No normal data was changed.

## Changed files, scope and review handoff

| Directory relative to repository root | Files in that directory | Purpose |
| --- | --- | --- |
| `apps/desktop/src-tauri/src` | `connection.rs`, `connection/windows.rs` | Bounded wrong-content-type metadata and synthetic WinHTTP tests; XML allowlist unchanged. |
| `apps/desktop/src-tauri/src/acquisition` | `static_menu.rs`, `static_menu_tests.rs`, `mod.rs` | Observed limitation, repeated sanitization, diagnostics whitelist and regression coverage. |
| `apps/desktop/src` | `types/acquisition.ts`, `features/Devices.tsx`, `test/acquisitionBackend.test.ts` | Strict additive wire field, displayed limitation, schema regression. |
| `scripts` | `verify-static-menu.mjs`, `read-native-dialog.ps1`, `verify-device-gui.mjs` | Explicit supervised harness flag and read-only native dialog observer; no automated Yes. |
| `docs` | `PHASE_7_5_B4_HARDWARE_STEP.md`, `PHASE_7_5_B4_PRESTEP_VERIFICATION.md`, `PHASE_STATUS.md` | Actual checks, limitations, owner quotes and current status. |

Not done, deliberately blocked: B2/schema 6, authenticated B4 client including
all login/CGI requests, B5 binding, B7 registry, B8 worker, Phase 8 and physical
firmware acquisition. No production development-mode relaunch was repeated in
this correction; the required supervised run used the rebuilt isolated release.
No independent reviewer test execution is claimed. No disputed H1 item remains.

Verdict: H1 and the authorized static-menu observation/decline checks are done,
with the evidence limitations above. This is not firmware acquisition or verified
device identity. Further requests need their own approved guideline.

Branch `phase-7-5-b4-menu-prestep`; task start `c3486cb0323b25eb9db578ad6e585e748fb402af`;
branch base/main remains `1b7fbce8334203be261d16ddf1c196181284d49c`;
origin/main remains `369e74173c570009b47bfd4957a4852ed0856678`.
Ending documentation-commit SHA and clean-tree result are supplied in delivery.
The exact final HEAD and additional computed repeat results are recorded at
`.tools/b4-h1/handoff-parallel/results.json`. Review command: `git diff main..HEAD`;
its saved UTF-8 diff is `.tools/b4-h1/review.diff`. No push, tag, merge or history
rewrite was performed.

## Interpretation, without further action

- `observed`: candidate filename, observed and unverified; no authority for
  another request. Authenticated work needs its own approved plan.
- `no_unique_dashboard_entry`: readable menu did not select exactly one entry.
  Stop for reviewer/owner inspection. Do not store body; owner may separately
  share a redacted browser capture.
- `menu_not_readable_without_login`: protected menu; record limitation;
  authentication remains blocked.
- `wrong_content_type`: record sanitized type; any narrow allowance requires
  a separate reviewer decision.
- Other failures, cancellation, decline, timeout, redirects, truncation and
  subnet conflicts: record and stop, without retry.
