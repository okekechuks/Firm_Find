# Stage 4 endpoint consistency binding

## Independent-review correction of `5817cfe` — 26 October 2026

Current disposition: local correction checks PASS; independent acceptance and the physical gate remain pending. This section supersedes the earlier software disposition below. Branch `phase-7-5-stage-4-review-fixes` starts at `5817cfe61e3a5a7740ade968fd63e0e2890b6343`; the original branch and separate artifact/authentication worktrees are preserved. No merge or privileged operation is released. Published-commit CI and repeat-run evidence are supplied with the handoff, not inferred from these local checks.

### R1 — require the material stamp throughout the collector

Confirmed defect: `AuthChallengeApproval::validate` accepted a supplied basic binding with `auth=None`. `challenge::collect` constructed exactly that basic binding, although the claimed proof itself carried a stamp. Validation now requires the supplied stamp to equal the expected stamp and requires both to be live. The collector receives the actual stamped binding from `read_auth_challenge`, checks device/interface/normalized endpoint/advanced-mode selection against the request, and revalidates after dispatch. It does not construct an unstamped replacement. Shared revocation, receipt expiry, claim-once behavior and the transport's single dispatch remain enforced.

Added deterministic synthetic tests for missing, mismatched, expired and revoked stamps; replay; request substitution; no dispatch on missing stamp; and the existing controlled invalidation-during-dispatch regression. Expiry is forced through a test-only deadline setter, without timing sleeps.

### R2 — separate new approval admission from existing operation validation

Confirmed defect: the cached-binding return preceded cancelled/status/timestamp validation. `Registry::bind` now rejects failed/starting/cancelled inventories and absent/future/60-second-old timestamps **before** capture or cached reuse. This is the admission path used before issuing a native approval and before claiming a new request. Rejection does not revoke an already live grant or renew its deadline.

`Registry::validate_existing` cannot mint or renew a binding. It observes successful inventories to revoke material changes, then requires the retained stamp to belong to this registry and remain live. Failed/cancelled refreshes preserve only the original grant's bounded lifetime. `AuthChallengeApproval::validate_publication` uses this path instead of calling `bind` to publish an already claimed operation. It checks the mandatory stamp before and after the registry check. New approvals still need a fresh successful inventory; disconnected/conflicting/materially changed successful inventories revoke existing grants.

Synthetic regressions cover cached admission under every invalid inventory condition, preservation without generation/deadline renewal, fresh/degraded cache reuse, revocation on material change, expiry, foreign registry rejection, and the actual claimed-proof publication path after failed/cancelled scans. No route sockets or physical device requests are used.

### R3 — Windows CI hash safeguard

Actual failed run: [40100911818](https://github.com/okekechuks/Firm_Find/actions/runs/40100911818), job `120892683482`, commit `5817cfe`. Tests, Clippy and the production Tauri installer build passed there; `build:native-tests` exited 1 at `scripts/build-native-tests.ps1:6`: `Get-FileHash` was not recognized in the Windows PowerShell subprocess launched by npm. The outer Actions shell was PowerShell 7 on Windows Server 2025. The log establishes cmdlet unavailability; no unverified module-path diagnosis is claimed.

`production-executable-guard.ps1` hashes through .NET SHA-256 with disposed streams/providers. `build-native-tests.ps1` retains the before-build digest, compares the production executable after the isolated build, and rejects changed or missing production bytes. No safeguard is removed. `test-production-executable-guard.ps1` deliberately makes `Get-FileHash` unusable and checks a known SHA-256 vector, unchanged bytes, mutation and deletion using one disposable file. Windows CI runs that regression in the same Windows PowerShell used by the native-test build.

### R4 — physical evidence and unchanged boundaries

**Physical Stage 4 gate: PENDING.** The previous supervised MG11 attempt stopped waiting for root approval; it did not reach a successful authentication-challenge read. No challenge status, cookie observation, after-test owner statement or successful physical binding qualification is inferred. This correction performs no MG11 request and collects no new owner confirmation. Credentials, login, authenticated identity reads, firmware extraction and configuration writes remain unauthorized; `canObtain` stays false. No merge or downstream authenticated operation may proceed before independent acceptance of this correction.

### Failing-first evidence

Local logs are under `.tools/stage4-review-fixes/.tools/review-evidence/` relative to the original repository root; they contain computed command output, not generated verdicts or owner wording.

- Before the Rust fixes, `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline review_r -- --nocapture` exited 101: `review_r1_missing_stamp_rejected` failed and R2 reported cached admissions for `failed`, `cancelled`, `missing_time`, `future_time` and `stale_time`. The mismatched/expired/revoked cases already passed. That broad filter also selected an existing library test which failed on sandbox-denied ancestor handles (OS 5); it is distinguished from the two reproduced defects. See `rust-red.log`.
- The guard regression first exited 1 because the new guard helper did not exist (`guard-red.log`); this is a test-first missing API, not a claimed reproduction of the CI environment. The actual cmdlet failure is recorded by the Actions run above. After implementation, the same regression exits 0 with `Get-FileHash` unusable (`guard-green.log`).

### Verification results

All local commands below exited 0. Rust commands use the existing repository wrapper, with the shared local `CARGO_HOME`, `RUSTUP_HOME` and `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=1`, and at least 15 GiB free before each build. The standard test count is superseded/pinned by the post-commit repeat evidence because the foreign-registry assertion was strengthened afterward; isolated tests and both Clippy checks include that assertion. No production code changed after these checks. Rust tests ran outside the sandbox because existing directory-lease regressions require ancestor handles denied inside it. Test storage is disposable; HTTP peers are owned loopback fixtures.

| Exact command | Actual local outcome | Log |
| --- | --- | --- |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | 314 passed, 1 ignored; 2 doctests passed | `rust-standard-final.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | 315 passed, 2 ignored; 2 doctests passed | `rust-isolated-final.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | clean | `clippy-standard.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | clean | `clippy-isolated.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all -- --check` | clean | `fmt.log` |
| `npm.cmd test` | 158 passed, 17 files | `frontend.log` |
| `npm.cmd run lint` | clean | `lint.log` |
| `npm.cmd run build` | TypeScript and Vite production build passed | `frontend-build.log` |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-production-executable-guard.ps1` | known vector, unchanged/mutated/deleted fixtures checked with cmdlet unusable | `guard-green.log` |

Post-commit commands: `python scripts/verify-parallel-rust.py --runs 20 --output .tools/review-evidence/final-20` records each standard-suite run with its exact HEAD, exit code, test counts and log SHA-256. The published branch's [Windows verification check](https://github.com/okekechuks/Firm_Find/actions?query=branch%3Aphase-7-5-stage-4-review-fixes) runs both test/Clippy profiles, frontend checks, the guard regression, `npm run tauri build` (installers) and `npm run build:native-tests` (the dedicated isolated binary plus production-byte comparison). Computed `ci-final.json` under the same local evidence directory and the handoff identify the actual run URL, SHA and conclusion. Neither optimized Windows build nor GUI was rerun locally for this correction; cloud builds do not prove a rendered GUI or physical qualification.

Preservation: the already committed `scripts/audit-data-bytes.py` on preserved authentication worktree `3280294` was invoked there as `python scripts/audit-data-bytes.py .tools/stage4-review/before.json` and `python scripts/audit-data-bytes.py .tools/stage4-review/after-local.json .tools/stage4-review/before.json`, both exit 0. These ordinary byte audits make two independent passes, refuse reparse points and never import/connect to SQLite. **All 12 normal application files, 5,611,701 bytes, match by path, size and SHA-256**, including persisted database/settings/saved-device records and managed data. This proves byte preservation, not a new logical database audit. No normal GUI launch, storage relocation or schema change occurred. Final post-verification comparison is retained as `after-final.json` beside these snapshots.

### Changed files

- `apps/desktop/src-tauri/src/acquisition/approval.rs`: mandatory stamp, selection check, publication validation and regressions.
- `apps/desktop/src-tauri/src/acquisition/challenge.rs`: stamped collector input and pre/post validation.
- `apps/desktop/src-tauri/src/acquisition/challenge_tests.rs`: pass the stamped test context.
- `apps/desktop/src-tauri/src/acquisition/ipc.rs`: carry the actual binding through worker/publication.
- `apps/desktop/src-tauri/src/endpoint_binding.rs`: fresh admission, existing-operation validation and test-only deadline control.
- `apps/desktop/src-tauri/src/endpoint_binding_tests.rs`: invalid cached admission and existing-operation regressions.
- `scripts/build-native-tests.ps1`: retain production-executable safeguard without `Get-FileHash`.
- `scripts/production-executable-guard.ps1`: SHA-256 and unchanged-executable checks.
- `scripts/test-production-executable-guard.ps1`: isolated hash/mutation/deletion regression.
- `.github/workflows/windows.yml`: run the guard regression in Windows PowerShell.
- `docs/PHASE_7_5_STAGE_4_BINDING_VERIFICATION.md` and `docs/PHASE_STATUS.md`: current correction evidence and gate state.

## Original Stage 4 delivery (historical)

24 October 2026, Africa/Lagos. **DONE: Stage 4 software exit gate PASS**, awaiting reviewer acceptance. Hardware qualification is NOT DONE (owner hold). Nothing is disputed. Phase 7 is not complete.

Branch: `phase-7-5-stage-4-binding`. Start: `def018836166151c0b781add711dcaa5d50efbdb`, the accepted challenge software tip. No merge, push, tag, history rewrite, dependency addition or schema change. Accepted design decisions were recorded separately on the design branch at `387983b397d748b35b7abc6646a14bfd1fad418d` and copied into the Stage 4 design here. Other stages are not included in this branch.

## Implementation and boundaries

`apps/desktop/src-tauri/src/endpoint_binding.rs` implements `Selection`, `capture`, canonical `Facts`, `Stamp`, `BoundEndpoint` and `Registry`. It binds the discovery device key, normalized 16-byte GUID, current-boot LUID, normalized numeric endpoint, actual route-selected local source/prefix/scope, selected next hop, sorted default gateways/routes and observed container/interface association references. Optional DHCP has an explicit unknown discriminator. Labels, DNS and global discovery revision are excluded. This is consistency evidence, **not cryptographically verified** identity.

Canonical fields use a versioned domain, fixed order, big-endian integers, length-prefixed bytes, explicit optional discriminators and sorted/deduplicated sets. An in-memory random process epoch and monotonic generation prevent reuse across registry restart or invalidation. SHA-256 is not an authentication mechanism. No stamp implements serialization; no binding is written to SQLite, settings, reports or diagnostics.

`Registry::bind` checks fresh inventory before creating a binding. Its 60-second maximum age starts at the inventory observation, bounded by wall and monotonic clocks; recapture never renews an existing binding. `Registry::observe` skips failed/cancelled refreshes and invalidates material changes on successful snapshots. Expiry revocation is irreversible even after a wall-clock rollback. Entry drop, removal, conflict, cancel/reset, expiry, changed DHCP (including known/unknown), source, prefix, route, interface/LUID or association revokes shared proof liveness. A subsequent explicit approval can create a fresh generation. Unknown DHCP permits this read-only path.

`connection::binding_route` / `connection/windows.rs::binding_route` query the local `GetBestRoute2` table and selected source without DNS resolution, socket creation or requests. Source must belong to the selected adapter and have one known prefix; route LUID/index must agree. IPv6 link-local scope must agree. Existing same-subnet collision rejection and transport remain unchanged. This does not implement or qualify an interface-bound socket.

`discovery/dhcp.rs::server` copies only bounded native fields from the owned `GetAdaptersAddresses` buffer: returned record length must include `Dhcpv4Server`, the DHCP-enabled flag must be present, and a bounded IPv4 `SOCKET_ADDRESS` must contain a usable address. Disabled/missing/truncated/outside-buffer/invalid data returns unknown. No DNS/gateway fallback or DHCP request exists. `NetworkAdapter.dhcp_server` defaults to `None` for old fixtures. Native discovery calls this parser; Windows-driver qualification on the MiFi is pending.

`acquisition/approval.rs` adds a stamp only to `AuthChallenge`; metadata, static-menu and strategy scopes retain their behavior and existing tests. Challenge approval rechecks before/after its native dialog, before dispatch and before publication. Receipt lifetime remains 30 seconds, network budget remains five seconds and is additionally capped by binding lifetime. A cached conflicting/ambiguous acquisition identity cannot qualify a root observation. Discovery and cancellation revoke liveness. The collector rejects stale root revisions; publication compares current selected facts rather than treating an unrelated global revision change as endpoint identity change.

Legacy root-cache admission and frontend result invalidation remain conservatively revision-based. They can require a fresh root check after unrelated inventory changes; no claim of a persistent authenticated session is made. Login, StatusRead, Logout, credentials and physical firmware acquisition remain blocked. No `canObtain` or verified-stage authority is granted by a binding.

`Devices.tsx`, `ManualRouters.tsx` and the frontend contract display separate DHCP observations/unknown and the exact plain-HTTP qualifier. Existing JSX structure, effects, accessibility and request guards were reviewed using the React review skill. No new effect, subscription, request or dependency was added.

## Tests and initial failures

All new route/inventory fixtures are synthetic. Tests use fake route providers and no host-route mutations, owner database, owner firmware or physical device. Existing HTTP transport regressions use owned loopback peers.

- `discovery-red.log`: missing DHCP contract assertion failed, exit 101. `api-red.log`: test-first binding API did not compile (missing fixture/registry; the newly added field also required the native initializer), exit 101.
- `contracts-red.log`: DHCP parser stub returned unknown for the valid native fixture; fixed-vector placeholder was not yet populated. Five binding tests passed and those two checks failed, exit 101. The placeholder failure is scaffolding, not a product defect.
- `frontend-red.log` and `manual-red.log`: absent DHCP row / absent plain-HTTP qualifier, each exit 1.
- `scope-red.log`: IPv6 scope mismatch was accepted, exit 101. Fixed selected-scope and route-family validation.
- `identity-red.log`: conflicting acquisition identity qualified a root, exit 101. Fixed challenge root predicate.
- `clock-red.log`: an expired binding became usable after a synthetic wall-clock rollback, exit 101. Fixed irreversible liveness revocation and kept the freshness test chronological. Expanded the vector test to reorder multiple addresses/prefixes, adapters, routes and USB interfaces, choose a source that is not the first address, and revoke a changed USB parent.
- Two isolated-profile rebuild attempts failed with OS error 112 (disk full), exit 101 (`rust-isolated-hardened.log`, `rust-isolated-recovered.log`). After resolved-path/reparse checks, reclaimed 4,103,186,776 bytes from 9 generated library/symbol files and 20 older Tauri archives under `target/debug`; no source, executable or application data was removed. `cache-prune.json` records archive names/sizes. The recovered run passes.
- Initial sandboxed whole-suite run: 270 passed, 32 failed, 1 ignored, exit 101. Existing folder-protection tests could not open `C:\Users\chuka` ancestor handles (`Access is denied`, OS 5). No storage code was weakened; the same suite outside the sandbox passed 302 tests, and a later run passed 304 before the final conflict regression was added.

New regressions cover every bound material field, stable labels/revisions, no freshness extension, failed/cancelled scans, DHCP transitions, replug, restart, cancellation, expiry, collision, unknown/invalid source, protected classes, old schema defaults, IPv6 scope/family, receipt cross-scope/replay and a controlled challenge invalidated during dispatch. No stale success survives; a new approved controlled request succeeds afterward. Existing adapter, simulator, vault, database and challenge regressions remain in the whole suite.

`scripts/verify-binding-vectors.py` independently computes three synthetic vectors using Python `struct`, `ipaddress` and `hashlib`; it writes no verdict, owner confirmation or documentation. SHA-256 values asserted by Rust:

| Vector | Bytes | SHA-256 |
| --- | --- | --- |
| IPv4, unknown DHCP | 170 | `95db4defe08ddb98ca3ddeb5b2a0595bf659e3a61cfbd824ebb7ffc0f86815a9` |
| IPv4, known DHCP | 183 | `68ceebd77b14779d93c969fe898d56c6be149bfeb7688d42fe0a3c8bfec32619` |
| IPv6, scope 7 | 190 | `0957db8ce132de35f22e5ed5dcbcb459b5d688a71759c35d24393445d4087e35` |

## Exit evidence

Command logs live in ignored `.tools/stage4-binding/`. Rust commands use `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=1`; libtest parallelism stays at its default. No serial workaround.

| Exact command | Exit / result |
| --- | --- |
| `npm.cmd test -- --run src/test/Devices.test.tsx src/test/ManualRouters.test.tsx` | 0; 27 passed |
| `npm.cmd test` | 0; 158 passed, 17 files |
| `npm.cmd run lint` | 0; clean |
| `npm.cmd run build` | 0; TypeScript and frontend production build |
| `py scripts/verify-binding-vectors.py` | 0; computed values above |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | 0; 306 passed, 1 ignored, plus 2 doctests (`rust-standard-hardened.log`) |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | 0; 307 passed, 2 ignored, plus 2 doctests (`rust-isolated-disk-recovered.log`) |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | 0; clean, final source (`clippy-standard-hardened.log`) |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0; clean (`clippy-isolated-hardened.log`) |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all -- --check` | 0; clean (`fmt-hardened.log`) |
| `py scripts/verify-parallel-rust.py --runs 20 --output .tools/stage4-binding/code-20` | 0; 20/20 on `179fc44b67c0888c39b98edf8ddd3d1ceea9c91e`, 305 tests + 2 doctests each; superseded by clock correction |
| `py scripts/verify-parallel-rust.py --runs 20 --output .tools/stage4-binding/hardened-20` | 0; 20/20 on final code `3d2db2def0cf91f31061d33eff7165d635376611`, 306 tests + 2 doctests each; clean tracked tree throughout |

Normal-data preservation uses a filesystem byte-hash audit, not a SQLite connection or a running normal application. Before: `.tools/stage4-binding/normal-before-files.json`, 12 files. The audit reads ordinary file bytes only; no normal database is opened through SQLite and no normal vault file is a test fixture. After all gates and the hardened twenty runs, all 12 paths, sizes and SHA-256 values match (`normal-after-hardened20-files.json`): zero added, removed or changed files, including DB/WAL/SHM and vault. Log SHA-256 values and counts in both repeat-run JSON files were independently recomputed and checked. No SQLite connection was made.

Not run: native Windows release rebuilds, native GUI launch, physical discovery qualification, supervised challenge/No run or MiFi verification. Owner explicitly deferred hardware until they confirm presence with MG11 connected and PC Wi-Fi off. Prior replies are not reused as new presence. No hardware result or post-run owner confirmation is invented. This branch adds no network request; software suites exercise existing loopback transport only.

The documentation-only delivery commit is subjected to the same final audit: `py scripts/verify-parallel-rust.py --runs 20 --output .tools/stage4-binding/final-20`. Its computed JSON pins the delivery end commit; delivery requires twenty exit-0 entries, unchanged HEAD and a clean tracked tree throughout. The final filesystem audit is `normal-after-final20-files.json`. These are computed records, not generated verdicts or owner statements.

## Changed-file inventory and handoff

The 24 changed paths are:

```text
apps/desktop/src-tauri/src/acquisition/approval.rs
apps/desktop/src-tauri/src/acquisition/challenge.rs
apps/desktop/src-tauri/src/acquisition/challenge_tests.rs
apps/desktop/src-tauri/src/acquisition/ipc.rs
apps/desktop/src-tauri/src/acquisition/mod.rs
apps/desktop/src-tauri/src/connection.rs
apps/desktop/src-tauri/src/connection/windows.rs
apps/desktop/src-tauri/src/discovery/dhcp.rs
apps/desktop/src-tauri/src/discovery/mod.rs
apps/desktop/src-tauri/src/discovery/service.rs
apps/desktop/src-tauri/src/discovery/windows.rs
apps/desktop/src-tauri/src/endpoint_binding.rs
apps/desktop/src-tauri/src/endpoint_binding_tests.rs
apps/desktop/src-tauri/src/lib.rs
apps/desktop/src/features/Devices.tsx
apps/desktop/src/features/ManualRouters.tsx
apps/desktop/src/test/Devices.test.tsx
apps/desktop/src/test/ManualRouters.test.tsx
apps/desktop/src/types/contracts.ts
docs/FIRMWARE_ACQUISITION_ENGINE.md
docs/PHASE_7_5_STAGE_4_BINDING_DESIGN.md
docs/PHASE_7_5_STAGE_4_BINDING_VERIFICATION.md
docs/PHASE_STATUS.md
scripts/verify-binding-vectors.py
```

Review: `git diff def018836166151c0b781add711dcaa5d50efbdb..phase-7-5-stage-4-binding`. This branch remains unmerged; final end SHA and clean status are independently available from `git rev-parse phase-7-5-stage-4-binding`, `git status --short` and the delivery run JSON. No generated screenshots were committed. Next released work is Stage 3 design additions and isolated implementation; authenticated client, real firmware/publisher work, real sandbox and Phase 8 remain blocked.
