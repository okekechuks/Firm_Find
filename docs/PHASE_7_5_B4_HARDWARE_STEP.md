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
metadata and diagnostics excluding response text. Full final-commit checks
and repeated suites are pending at this commit.

## Supervised harness

`scripts/verify-static-menu.mjs` is selected by
`node scripts/verify-foundation.mjs --release-only --device-gui --acquisition-mg11-menu`.
It uses an owned isolated profile, real React clicks and Rust IPC. It records
computed invocation outcomes and reads native dialog text through
`scripts/read-native-dialog.ps1`. It never answers a native dialog.

An existing root/version approval precedes the separate menu approval. The only
new authorized device request is one `GET /xml/ui_mifi.xml`. A successful
observation is followed immediately by a separate menu approval for the owner
to decline. The harness checks that decline produces no second
`read_static_menu` call, receipt or result. If the positive run hits a stop
condition, it stops without another fetch or workaround. Native invocation
counts plus proof/loopback tests support rejection evidence; no packet capture
is claimed. Generated captures remain ignored; no screenshots are committed.

## Hardware record — pending

Before statement, 23 October 2026: owner replied “yes confirmed” to the bundled
USB/power/Wi-Fi-off and visible-settings question. This is not an exact firmware
version report. An explicit version and before-display statement were requested;
hardware execution is pending that answer and final software gates.

Owner statements cover only the router's web display, not internal storage.
Normal application baseline `.tools/b4-h1/normal-before.json`: schema 5,
integrity `ok`, no foreign-key errors. First snapshot attempt exited 1 because
the evidence directory did not exist; creating it and rerunning returned 0.
No normal data was changed.

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
