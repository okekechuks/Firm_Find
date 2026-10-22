# Phase 7.5 verification — Sprint A

Latest: [Sprint A close-out corrections and re-review handoff](PHASE_7_5_SPRINT_A_CLOSEOUT.md). Results below are the original slice and USB-only follow-up; they do not physically qualify the corrected recognition/version reader.

22 October 2026, Africa/Lagos. **Phase 7.5 IN PROGRESS. Sprint A read-only discovery software checks PASS; independent acceptance PENDING. H01 selected-USB root HTTP response VERIFIED after owner disconnected competing Wi-Fi; exact identity and acquisition UNKNOWN.** No Phase 8 work, executable acquisition route, firmware artifact, isolated analysis success or physical write is claimed.

## Baseline and reproduction

Starting branch `main`, HEAD `369e74173c570009b47bfd4957a4852ed0856678`; no commits since the previously inspected revision. Dedicated branch `phase-7-5-acquisition-engine` created. Three pre-existing documentation modifications were preserved. Normal data captured read-only in `.tools/phase7-5/normal-before.json` using `scripts/probe-native-isolation.py`; schema 5, integrity `ok`, no foreign-key violations.

Source reproduction: `Devices.tsx:DeviceDetails.verify()` called `getDeviceCompatibility()` / `getDeviceAdapterMatches()`, checked freshness then discarded outcomes. `device-status.ts:extractionEligibility()` always denied acquisition; UI always displayed Cannot locate firmware. No acquisition-discovery operation was invoked. This proves missing engine behavior, not a confirmed MG11 hardware limit.

## Implemented and verified source boundaries

`acquisition::discover()` resolves current Windows/evidence state, validates selected local endpoint/interface and optionally collects one bounded root response. `recognize()` emits allowlisted candidate family signals only. `identity()` separates trusted, observed and reported identity, rejects stale/conflicting/substituted evidence and never promotes USB IDs or titles. `assess()` enumerates five strategy candidates with independent missing requirements and 16 separate capabilities. No candidate is executable.

`acquisition::ipc` re-resolves current inventory, rejects changed/disconnected results and publishes only through `ProbeLease::if_current()`, serialized with cancellation/supersession. Cache is bounded and expires; it is neither authority for acquisition nor persistent firmware data. Export uses backend-redacted metadata and existing guarded Library file-save infrastructure, with exclusive publication and no overwrite. No database schema/storage relocation.

Windows root GET shares validated local routing, pinning/TLS, no proxy/auth/cookies/redirects, 16 KiB headers, 16 KiB body, bounded deadlines and callback-owned read buffer cleanup. Device cards remain amber when routes are unknown. Host NICs are labelled transport to a remote router rather than verified router models. Obtain Firmware remains disabled; browser contracts reject fabricated executable acquisition.

## Commands and results

Commands run from repository root. Raw outputs remain in `.tools/phase7-5/`.

| Command | Exit/result |
| --- | --- |
| `git status --short`; `git branch --show-current`; `git rev-parse HEAD`; `git log --oneline 369e74173c570009b47bfd4957a4852ed0856678..HEAD` | 0; baseline above |
| `git switch -c phase-7-5-acquisition-engine` | 0 |
| `py scripts/probe-native-isolation.py .tools/phase7-5/normal-before.json` | 0; read-only baseline |
| `npm.cmd test` baseline | 0; 136 tests |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --locked --offline` baseline | Sandbox: 101, 32 directory-lease access failures; unrestricted: 0, 207 pass / 1 ignored |
| `npm.cmd test -- --run src/test/Devices.test.tsx` against old UI expectations after behavior change | 1; 3 old-label/old-IPC assertions failed; updated to new route contract and added regressions |
| `npm.cmd run lint` | 0 |
| `npm.cmd test` final frontend | 0; 142 tests / 17 files |
| `npm.cmd run build` | 0; TypeScript and Vite production output |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all -- --check` | 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | Initial 101, collapsible-if warning; corrected and final hardened runs 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0; final hardened run 0 |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --locked --offline` final | 0; final quiet run 221 pass / 1 ignored; timing failure recorded below |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --lib --features native-test-profile --locked --offline` final | 0; final hardened run 222 pass / 2 ignored |
| `npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline` | 0; rebuilt after ACL and global-conflict hardening; `target/release/obtainer.exe` |
| `npm.cmd run build:native-tests` | 0; `target/release/obtainer-native-test.exe`; script verified ordinary executable not overwritten |
| `node scripts/verify-foundation.mjs --device-gui --phase4 --phase4-1 --phase6 --acquisition-mg11` | Initial attempts 1: obsolete Library selector, then missing new Tauri ACL; retained evidence |
| `node scripts/verify-foundation.mjs --device-gui --phase4 --phase4-1 --phase6` | 0; 106 native checkpoints, no console issues; before final identity-filter hardening |
| `node scripts/verify-foundation.mjs --release-only --device-gui --acquisition-mg11` first run | 0 software checks; physical USB HTTP BLOCKED by selected-interface route guard, not physical identification PASS |
| `node scripts/verify-foundation.mjs --release-only --device-gui --acquisition-mg11` USB-only repeat | 0; 21 checkpoints, zero console issues; actual owner-approved root GET returned HTTP 200, `identity_unknown`, candidate `e_link_oem`; all acquisition routes nonexecutable |
| `node scripts/verify-foundation.mjs --gui-preview --device-gui` | 0; final hardened development window, route IPC, HMR, error/loading recovery, clean shutdown and preserved data |
| `node scripts/verify-foundation.mjs --release-only --device-gui` | 0; final hardened release window, route IPC, diagnostics save/redaction/no-overwrite, clean shutdown and preserved data |

Initial repeated Rust run hung in existing `adapters::tests::discovery_changes_during_connection_prevent_late_session_publication`: its 15 ms operation could expire before reaching an unconditional barrier. Verified owned test process was stopped; log retained as `tests-standard-final.log`. Test-only fixture now signals connector admission over a bounded channel, creates its deadline in the worker and retains actual discovery invalidation assertions. Corrected standard/native reruns pass. No production session behavior was changed for this harness correction.

First native run failed an obsolete Library selector searching card text for a SHA-256 removed by an earlier owner-requested GUI change. Harness now selects the visible original filename, retaining independent backend hash/byte validation. Second native run exposed a new integration defect: acquisition handlers were registered in Rust but missing from the Tauri manifest/capability allowlist, so native invocation was denied. Added only three narrow command permissions, a manifest/ACL regression, and made the native check require a route report instead of accepting an error panel. Both failed profiles/logs are retained as evidence; normal-data comparisons passed after both failures. Corrected native regression suite passed all 106 checkpoints. Final identity-filter hardening is followed by focused development/release checks; the full retained native suite was executed before that last filter change.

New regressions cover MG11 unknown-route behavior; USB/title signals never granting authority; stale/reported/conflicting evidence; distinct hardware revisions; endpoint substitution; protected/public/missing targets; cancellation and successor ownership; raw diagnostic redaction; credential-free bounded root GET, 401 and redirect handling; stale UI replies; explicit endpoint approval; disabled Obtain; fabricated backend enablement; browser-preview rejection.

## Remaining gates

Independent Claude review of the architecture and implementation is pending. 7.5A/B/C remain partial identification/recognition/strategy infrastructure; documented authenticated vendor APIs and real source qualification remain necessary. Sprints B/C and 7.5D/E/F remain pending. No actual firmware bytes acquired; firmware hashes/provenance are not applicable. Five-device status is tracked in [hardware matrix](HARDWARE_TEST_MATRIX.md). No generic OEM/chipset inference, guessed credentials, physical storage read, configuration write, flash, reset or live USSD.

Review handoff: inspect the committed diff from starting HEAD, especially `acquisition/{mod,ipc,tests}.rs`, `connection{,/windows}.rs`, report export, acquisition schemas and `Devices.tsx`. Check route/identity authority, cancellation ownership, asynchronous buffer lifetime, bounds/redaction and GUI claims. Return findings through the owner; no external reviewer message has been sent automatically. Do not accept Sprint A or enable acquisition/analysis until independent review is addressed.

## Additional source hardening and actual hardware boundary

The endpoint-selection conflict regression initially failed (`verified` instead of `conflicting`). Identity assessment now detects contradictions across all fresh authoritative evidence for the same device before selecting endpoint-bound fields; endpoint substitution cannot hide those contradictions. Final unit suites pass with the new regression.

One new authenticated-evidence fixture initially attempted to construct manufacturer evidence through the user-report API, which correctly rejected it. The fixture was corrected to model a trusted adapter observation; production user-report restrictions remain intact. One retained simulator expiry test (`post_review_virtual_expiry_boundary_revokes_reads_without_status_poll`) flapped during parallel build load: it advances to one millisecond before expiry and expects a read before real time crosses that margin. Native-profile suite passed; the exact standard suite rerun with compilation idle passed 221 tests. Simulator production code was not modified. This existing timing sensitivity remains a reviewer note.

The genuine H01 USB candidate was observed in the native application: USB 1286/4E31, Remote NDIS network transport, local management candidate 192.168.100.1. Root-check approval, backend discovery, independent unknown routes and redacted diagnostics were exercised. The transport returned `no_route` with no HTTP status: Windows preferred Wi-Fi over the selected USB interface. Read-only `route.exe print -4` and `ipconfig.exe /all` confirmed both Wi-Fi and USB use 192.168.100.0/24, with Wi-Fi's lower metric. The route guard returned before TCP/HTTP dispatch, so this run does not prove a physical HTTP response or E-LINK recognition. Historical E-LINK HTTP 200 observation remains historical evidence only.

The owner then confirmed Wi-Fi disconnected and USB ready. The same native release command was repeated without code changes or route-guard bypass. All 21 checkpoints passed; console warning/exception count was zero. In the actual React popup, the owner-approved USB root GET returned HTTP 200 with `identity_unknown` and candidate management family `e_link_oem`. Metadata was truncated at the existing bounded response limit (`metadataTruncated: true`); no raw page, credentials or firmware were retained. The screenshot `production-h01-acquisition.png` in `.tools/foundation/2026-10-22T22-19-23.217Z/` was visually inspected: USB 1286/4E31, Ethernet 2, candidate family, unknown exact model/revision/firmware and no qualified acquisition method rendered. This verifies selected-USB HTTP reachability and candidate recognition, not an authenticated vendor API or exact device identity.

No Windows routing/interface configuration was changed automatically. The owner was told Wi-Fi can be reconnected after the check. Exact MG11 identity, acquisition/export/storage access and all four hardware feature capabilities remain UNKNOWN. Every route remains `executable: false`, and `canObtain` remains false. No firmware bytes acquired, so firmware SHA-256/provenance are not applicable. Test Library imports use generated 1 MiB/256 MiB harmless bytes, never the only copy of real firmware.

Ignored tests require an explicitly started genuine OpenWrt QEMU guest (`genuine_openwrt_windows_ssh_interoperability`, plus isolated-profile `genuine_openwrt_full_bootstrap_approval_and_inspection`). They were not silently counted as passes or restarted for this stock-device slice. Previously accepted Phase 7 OpenWrt software evidence remains intact; physical OpenWrt qualification remains pending.

## Final evidence and review gate

Frontend: 142 tests pass. Final standard Rust: 221 pass / 1 ignored. Final isolated-profile Rust: 222 pass / 2 ignored. Formatting, lint, both Clippy profiles, frontend build and both optimized Windows executables pass. Retained native run: 106 checkpoints; final hardened development/release results are in [native evidence](evidence/phase-7-5-native-2026-10-22.json). Console issue counts are zero. Successful native profiles cleaned safely; failed-run profiles/logs remain for review. Generated screenshots remain in task-owned local evidence, preserving accepted images.

[Preservation evidence](evidence/phase-7-5-preservation-2026-10-22.json) proves normal SQLite/WAL bytes, all rows, settings, saved-device/profile rows, vault blobs and captured logs/workspaces/reports equal baseline. Read-only preferences confirm Library override is null: vault remains `%LOCALAPPDATA%/OBTAINER/vault`, database/settings remain `%LOCALAPPDATA%/OBTAINER/obtainer.db`. No relocation or migration. Final command `py scripts/probe-native-isolation.py .tools/phase7-5/normal-after.json .tools/phase7-5/normal-before.json` exited 0.

[Latest H01 redacted evidence](evidence/phase-7-5-h01-redacted-2026-10-22.json) records actual USB-only HTTP 200 / `identity_unknown` / candidate `e_link_oem`. [Prior blocked evidence](evidence/phase-7-5-h01-routing-blocked-2026-10-22.json) retains `no_route` with absent HTTP status from the competing-interface run. No verified vendor/model/firmware identity or authenticated API success claimed. No real firmware acquired; firmware hash/provenance not applicable. No configuration, flash, reset, privileged storage access or live USSD dispatched.

USB-only follow-up commands: `node scripts/verify-foundation.mjs --release-only --device-gui --acquisition-mg11` exited 0 (21 checkpoints); `py scripts/probe-native-isolation.py .tools/phase7-5/normal-after-usb.json .tools/phase7-5/normal-before.json` exited 0 (full normal data still equal baseline). Native command log is `.tools/phase7-5/native-usb-only.log`; raw isolated evidence is `.tools/foundation/2026-10-22T22-19-23.217Z/`. Owned processes stopped and successful isolated profile cleaned. No runtime code changed, so previous unit/build results above apply to the same executable; these checks were not needlessly repeated.

**Verdict:** Sprint A read-only discovery slice software-verified; H01 USB root response and candidate recognition verified, exact identity/acquisition unknown; independent review pending. Phase 7.5 IN PROGRESS: 7.5A/B/C partial infrastructure; 7.5D/E/F pending. Sprints B/C require source/worker qualification and review. Phase 8 UNSTARTED. Smallest implementation gate: Claude reviews committed slice before acceptance or executable acquisition/analysis. A documented identity/export protocol or publisher-qualified source is still required for MG11 acquisition.
