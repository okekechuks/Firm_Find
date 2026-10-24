# Stage 4 endpoint consistency binding

24 October 2026, Africa/Lagos. Software verification in progress; this is not hardware qualification or Phase 7 completion.

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
| `py scripts/verify-parallel-rust.py --runs 20 --output .tools/stage4-binding/final-20` | Pending final hardened commit; initial `179fc44` passed 20 runs with 305 tests and 2 doctests each (`code-20/results.json`) |

Normal-data preservation uses a filesystem byte-hash audit, not a SQLite connection or a running normal application. Before: `.tools/stage4-binding/normal-before-files.json`, 12 files. No normal SQLite database is opened by the audit; no normal vault file is a test fixture. After the software gates, all 12 paths, sizes and SHA-256 values match (`normal-after-gates-files.json`): zero added, removed or changed files. No SQLite connection was made. Final post-repeat comparison is still required.

Not run: native Windows release rebuilds, native GUI launch, physical discovery qualification, supervised challenge/No run or MiFi verification. Owner explicitly deferred hardware until they confirm presence with MG11 connected and PC Wi-Fi off. Prior replies are not reused as new presence. No hardware result or post-run owner confirmation is invented. This branch adds no network request; software suites exercise existing loopback transport only.

Review: `git diff def018836166151c0b781add711dcaa5d50efbdb..phase-7-5-stage-4-binding`. End commit and clean-tree status will be supplied with final computed repeat-run evidence. Stage 3 design additions are next; authenticated client, real firmware/publisher work, real sandbox and Phase 8 remain blocked.
