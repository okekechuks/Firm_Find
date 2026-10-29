# Web-management evidence boundary

This note records the local HAR/static analysis workstream. It is not a physical-device acquisition claim.

## Existing integration audited

- `src/acquisition/strategy_api.rs` defines the five stable strategy identifiers and a pure `ProbeContext`, without transport or filesystem access. `strategies.rs` retains the existing closed registry.
- `src/acquisition/approval.rs` binds expiring single-use receipts to the selected endpoint/interface and, for authentication-challenge observation, a mandatory material endpoint stamp. A metadata receipt is not a strategy or authentication receipt.
- `src/acquisition/request_registry.rs` contains the reviewed root, static version, static menu and credential-free challenge descriptions. Imports do not modify that registry.
- `src/acquisition/mod.rs::resource_attributes` scans actual HTML opening tags, excludes comments and raw script/style text, and never executes the page. Static analysis reuses that scanner for resource references.
- `src/library/files.rs::read_file` rejects non-files and reparse points and retains the existing Windows sharing policy. Import uses this reader rather than introducing an unrestricted file-opening API.
- The Library, firmware-vault records, jobs and extraction manifests remain separate from capture evidence. No raw HAR, JavaScript bundle, selected local path or credential is written to those stores.

Paths in this section are relative to `apps/desktop/src-tauri/`.

## Local implementation

`src/acquisition/web_management/` contains the native picker IPC, bounded HAR parser, static lexical analyzer, lossy display normalization, in-memory evidence store and generic web-source assessment.

The two user actions are local file imports. Neither sends network traffic. The native file dialog returns a backend-owned path; IPC never accepts a caller path or script body. HAR input is limited to 8 MiB and 512 entries. A resource input is limited to 1 MiB. Strings, schema names, resource references, merged reports and stored device counts have additional caps.

Only recognized non-secret HTTP metadata survives. Query values, userinfo, raw headers, cookies, challenge material, authorization values, request/response bodies, IMEI/SIM values and initiator URLs are discarded. Schema **names** and route display paths are intentionally lossy: unknown names and path segments are represented by opaque digest labels instead of retaining arbitrary values. MIME types and initiator contexts use fixed vocabulary. The result is a display/evidence description, never a replayable request.

The static analyzer treats HTML/JS/JSON/CSS as UTF-8 data. It scans quoted route strings and actual HTML resource attributes. It never creates a DOM, evaluates scripts, calls extracted code or follows a reference. Dynamic URL construction, escaped JavaScript and unusual minification can be missed; absence of a match is not negative proof.

`har_observed`, `static_candidate` and `both` track only the source of a route description. Cross-reference requires the same normalized HTTP origin and path. `both` does not become `verified`. Every imported endpoint keeps `readOnlyQualified = false`, and `canAcquire` stays false.

A GET classified as reset/reboot/restore/write remains dangerous. Upgrade/upload is not download. Configuration or generic file export is not firmware. The computed `strategy` report is displayed in the selected-device workflow. It exposes observed downloads, unqualified update metadata and missing gates, with no request dispatcher. An observed update check is not labelled an observed FOTA package: no package has been established.

## Live boundary

The production registry currently supplies no independently qualified web-management identity/status route. Imported HAR or user-selected bundle origin cannot supply that qualification. The backend reports live collection/authentication unavailable, and direct live-collection IPC rejects before I/O. A reviewed authenticated implementation must prove current identity, selected-device route binding, fresh scoped approval and a qualified read-only route before resource collection or source probing.

No API enumeration, browser-request replay, firmware upload, reset, reboot, login, password guessing, source acquisition or physical-device extraction is reachable through these evidence IPC functions.

## API wiring

Root manages `WebEvidenceStore` as independent Tauri state and registers:

- `get_web_ui_evidence(device_id) -> WebEvidenceReport`
- `import_web_ui_capture(device_id) -> Option<WebEvidenceReport>`
- `analyze_web_ui_bundle(device_id, origin) -> Option<WebEvidenceReport>`
- `collect_management_ui_resources(device_id) -> IDENTITY_UNQUALIFIED`

`None` means the owner cancelled the local native file dialog. Evidence requires a current connected discovery record before selection and publication. Reopening the selected device returns the sanitized in-memory report. Application restart clears imported evidence rather than accidentally persisting capture secrets. Normal application storage locations and SQLite schemas are unchanged by this module.

## Qualification limits

Synthetic test fixtures are explicitly labelled synthetic. They establish normalization, classification, bounds, cross-reference and denial behavior; they are not owner hardware evidence. No undocumented firmware source, route or physical success is inferred from these tests. Source qualification, actual authenticated reads, model/revision matching, firmware bytes and acquisition hashes require their own evidence.

## Recorded focused verification

On 2026-10-29, the first behavioral run kept the new parser entry points returning `NOT_IMPLEMENTED`: all 18 then-existing evidence tests failed. The complete acquisition filter reported 79 passed and 22 failed (the other four failures belonged to the parallel authentication/binding workstream), exit 101. Actual output is retained locally at `.tools/web-management-evidence/har-static-strategy-red.log`.

After implementation, an initial focused run failed the native file-reader test because the sandbox denied the existing directory lease at `C:\Users\chuka`. The guard was retained. Repeating with the authorized filesystem access and the authentication workstream's vector corrections yielded 113 passed, 0 failed, exit 0, including all 22 then-existing evidence tests. Actual output: `.tools/web-management-evidence/acquisition-green-escalated.log`.

The exact command was:

```powershell
$env:CARGO_TARGET_DIR='C:/Users/chuka/OneDrive/Documents/Firm_Find/.tools/beta2-hardening/.tools/management-target'
$env:CARGO_INCREMENTAL='0'
$env:CARGO_PROFILE_DEV_DEBUG='0'
$env:CARGO_PROFILE_TEST_DEBUG='0'
$env:CARGO_BUILD_JOBS='2'
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --manifest-path apps/desktop/src-tauri/Cargo.toml --lib acquisition:: --locked --offline
```

Later regressions cover HAR's separate `request.queryString` representation (including a reset hidden behind a status URL), computed source-assessment publication and the distinction between update metadata and an observed package. The final full suites include them: 438 standard / 439 isolated Rust tests plus two doctests per profile. Full builds/native results and exact failures are recorded in [milestone verification](WEB_MANAGEMENT_ACQUISITION_VERIFICATION.md). No independent reviewer acceptance or physical acquisition is inferred.
