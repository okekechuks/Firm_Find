# Web-management acquisition milestone

29 October 2026. Starting source: `36fc81b36ca036865e5bd06f8baedf4fe648c756`.
Branch: `feature/web-management-recursive-milestone`, continued from the existing
extraction V2 feature worktree. Main and published tags are preserved.

## Source audit before integration

The existing implementation is retained:

Frontend paths below are relative to `apps/desktop/src/`; Rust paths are relative
to `apps/desktop/src-tauri/src/` unless explicitly named otherwise.

| Area | Actual source and boundary |
|---|---|
| Selected-device workflow | `features/Devices.tsx::DeviceDetails` opens the selected device; `components/DeviceFirmwareWorkflow.tsx` calls `get_device_workflow`, then separately approved official acquisition. Cards stay concise. |
| Strategy contracts | `acquisition/strategy_api.rs`, `acquisition/strategies.rs` and its `strategies/` handlers, `acquisition/mod.rs::FirmwareAcquisitionStrategy`: five registered strategies; metadata recognition is not firmware acquisition. |
| Challenge observation | `acquisition/challenge.rs`, `ipc.rs::{approve_auth_challenge,read_auth_challenge}`, `approval.rs::AuthChallengeApproval`: one credential-free closed request with live single-use material endpoint stamp. Physical MG11 success remains pending. |
| HTTP and binding | `connection/windows.rs` uses WinHTTP. `endpoint_binding.rs::Registry::bind` requires fresh discovery for new approvals and preserves only existing live operations. Global route ambiguity blocks; a source-bound authenticated client is absent at the starting commit. |
| Authorization/session ownership | `acquisition/approval.rs`, `adapters/{bootstrap,lifecycle,policy}.rs`, `openwrt/runtime.rs`: scopes, expiry, cancellation and independently qualified identity. Generic HAR routes cannot create those proofs. |
| Immutable Library/provenance | `library/{admission,db,types,official,workflow}.rs`, `storage/schema6.rs`: class-separated occurrences, hash-bound source files, persistent jobs and qualified-source fences. `library/manifest_contract.rs` remains a proposed contract rather than a live producer. No schema change is needed for local evidence. |
| Firmware analysis | `library/analysis.rs` observes signatures including CPIO and BSDIFF; recognition grants no decoder, device support or install permission. |
| Extraction | `library/extraction.rs`, `extraction/layers.rs`, `scripts/{firmware-extract-worker,extraction-engine-v2}.py`: bounded isolated recursive handlers, immutable manifests and explicit unknown ranges. Existing SquashFS implementation remains. |
| Tests and physical evidence | Existing standard/isolated Rust, frontend and native suites are separate from physical qualification. V2 real D-Link/OpenWrt fixtures are firmware files, not connected-device dumps. H02/H03 remain separate revisions. |

No raw HAR, downloaded script, credentials or live session is stored by the new
evidence facility. The normal-data byte baseline contains 12 files / 5,611,701
bytes; the audit reads bytes only and never opens the normal SQLite database.

## Implementation

- `acquisition/web_management/{har,sanitize,static_analysis,store,strategy,ipc}.rs`:
  backend-owned native file selection, bounded memory-only HAR/static evidence,
  conservative request classes and computed web-source requirements. Routes remain
  descriptions rather than executable requests. Configuration export and firmware
  upload never produce firmware acquisition authority.
- `acquisition/{management_auth,management_auth_flow}.rs`: maintained Digest,
  fresh challenge, qop/algorithm/opaque/URI handling and a bounded single-use
  identity-read driver. [Dependencies and limits](WEB_MANAGEMENT_AUTH_BINDING.md).
- `acquisition/{management_transport,management_route_windows,management_binding_ipc}.rs`:
  Windows GUID/LUID/index/source/route checks, source-bound reqwest construction,
  and selected-device route observations without HTTP. Overlap, stale inventory,
  changed device/source and unproven routing fail closed.
- `lib.rs`, `build.rs`, `capabilities/main.json`, generated permissions and
  `library/ipc.rs`: explicit commands/state/ACL. No credential IPC is exposed.
- `components/{WebManagementWorkflow,DeviceFirmwareWorkflow}.tsx`,
  `features/Devices.tsx`, `types/web-management.ts`, `lib/backend.ts`: selected-device
  evidence/source/route views, stale-generation protection, separate blocked
  authentication/acquisition and strict returned DTO validation.
- `library/extraction/structure.rs`, `components/FirmwareStructure.tsx`,
  `types/firmware-structure.ts`: immutable graph/readiness projection described in
  [recursive verification](RECURSIVE_FIRMWARE_EXTRACTION_VERIFICATION.md).
- `scripts/verify-{web-management,firmware-structure}.mjs` and existing native
  drivers: picker/React/IPC, graph, file extraction and restart assertions.
  Synthetic management fixtures are labelled; OEM files stay local.

Dependencies added/pinned: `digest_auth = 0.3.1`, `zeroize = 1.9.1`,
`reqwest = 0.13.5` (blocking/native TLS, defaults disabled). Cargo.lock and a
computed dependency tree are retained. `diqwest` was rejected because hidden
authentication retries/caching do not fit explicit operation checkpoints.
No SQLite schema, storage location, decoder sandbox or published asset changed.
CI exposed the previously unqualified Python security update. Its signed-package,
executable and sandbox qualification is recorded in
[recursive verification](RECURSIVE_FIRMWARE_EXTRACTION_VERIFICATION.md#maintained-interpreter-qualification);
only the exact interpreter pin and CI diagnostics changed at that boundary.

## Executed software checks

Rust commands used prefix `powershell -NoProfile -ExecutionPolicy Bypass -File`,
`--locked --offline`, stripped dev/test debug information, no incremental build,
two jobs and C: `.tools/management-target` after E: became unavailable. E: was
available again before native builds. Normal data stays on C: throughout.

| Command (Rust prefix above) | Exit/result |
|---|---|
| `scripts/rust.ps1 test --workspace --locked --offline` | 0; 438 passed, two ignored; two doctests passed |
| `scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | 0; 439 passed, three ignored; two doctests passed |
| `scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | 0 |
| `scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0 |
| `scripts/rust.ps1 fmt --all -- --check` | 0 |
| `npm.cmd test` | 0; 196 passed / 25 files |
| `npm.cmd run lint` | 0 |
| `npm.cmd run build` | 0; TypeScript and Vite |
| `npm.cmd run test:catalogue` | 0; pinned signer, manifest and three entries verified |
| `npm.cmd run test:dns-build` | 0; four Windows tests, nine explicit Linux skips |
| `py -B scripts/test-firmware-extraction.py` | 0; 24 tests, six Linux skips |
| `py -B scripts/test-extraction-engine-v2.py` | 0; 34 tests, three Linux skips |

Ignored Rust cases require separately provisioned VM/OS or crash-child fixtures;
they are not claimed executed. The Windows reqwest socket regression used a real
loopback server and observed source `127.0.0.2`, rather than only a mocked route.
Its synthetic route is test-only; production loopback targets remain refused.
This does not qualify a physical adapter. The completed Windows/native/CI checks
are recorded below; physical authentication and source qualification remain open.

## Windows and native verification — 30 October 2026

Runtime source: `6096b644f8466d62697492f897b8230e3fdc457f`. Its
[CI 40940887215](https://github.com/okekechuks/Firm_Find/actions/runs/40940887215)
passed all three jobs: Windows verification/both native builds, DNS helper and
qualified extraction worker. The earlier failed run is retained, not relabelled.
The subsequent native-driver changes add visible-viewport and secret-storage
assertions; they do not alter the compiled application.

| Exact command | Exit and actual result |
|---|---|
| `npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline` | 0; production Windows executable |
| `npm.cmd run build:native-tests` | 0; isolated Windows executable; production hash guard preserved the ordinary executable |
| `npm.cmd run test:web-management-native` | 0; real native file pickers, selected-device React/Rust HAR/static workflow, dangerous GET classification, rejected malformed input, blocked live collection and unqualified route; restart clears memory-only evidence |
| `node scripts/verify-foundation.mjs --manufacturer-package` | 0; genuine local CPIO/RAR import, extraction, graph/UI, independent retained-file hashes and manifest persistence across restart |
| `npm.cmd run test:universal-workflow-native` | 0; synthetic qualified identity, genuine official HTTPS image, associated immutable Library, automatic analysis, actual GUI Cancel/retry, 869 file hashes, graph, text preview, background-event stability and restart/direct Library link |
| `node scripts/verify-foundation.mjs --release-only --phase5 --phase6` | 0; native six-page navigation, SQLite/settings, passive discovery/refresh, protected adapter denial, simulator bootstrap/reads/rollback/expiry/identity-change/Cancel/recovery; no physical simulator dispatch |
| `npm.cmd run test:extraction-review-native` | 0; genuine raw/gzip/TRX extraction, partial UBI inspection, restart, selected-file TAR export, resource exhaustion without publication and Unicode/long-name preflight; inert links/special entries |
| `npm.cmd run lint` | 0 after the final native-driver edits |
| `py -B scripts/audit-data-bytes.py .tools/web-management-evidence/normal-after.json .tools/web-management-evidence/normal-before.json` | 0; all 12 normal files / 5,611,701 bytes identical |

The synthetic HAR deliberately contains a secret sentinel in headers, bodies and
query values. The report/UI exclude it. After clean shutdown, five files under
the isolated app-owned data directory (including SQLite, logs and vault metadata)
were scanned for its UTF-8 and UTF-16 encodings; neither occurred. The deliberately
secret-bearing input fixture is outside app data. The same scan passed after
restart. No firmware job was created by HAR/static import. This is a concrete
sentinel regression, not a proof of universal information-flow noninterference.

Actual native evidence directories under `.tools/foundation/`:

- `2026-10-30T08-15-06.411Z`: HAR/static/error/route/restart/storage checks.
- `2026-10-30T08-15-35.011Z`: genuine CPIO/RAR graphs and restart.
- `2026-10-30T08-17-10.897Z`: selected-device official download/extraction/restart.
- `2026-10-30T08-18-08.493Z`: retained adapter/simulator/GUI regressions.
- `2026-10-30T08-19-40.955Z`: broader extraction/resource/export regressions.

Native console warning/exception lists were empty in each completed run. Real
WebView screenshots of the evidence and structure views were inspected. They
show local evidence and BLOCKED device acquisition; screenshots stay uncommitted.
Windows inventory changed normally between scans, with unique device IDs; the
adapter run observed no connected MG11 and makes no physical MG11 success claim.

Production executable: 19,483,136 bytes, SHA-256
`29763d9d887b6b2b28a012d042d5d1d37f9f770efd40bff968869d3b95d02565`.
Native-test executable: 19,461,120 bytes, SHA-256
`07fab3d12cdbb8ce88024a09a85c2d78a324be3cf1ad582a8365fd0054cfdf73`.
These are local unsigned executables, not a published release.

The real OpenWrt VM/authentication suite requires the separately provisioned
`.tools/phase7-vm/console-identity.log` fixture, which is unavailable here; it is
not claimed executed. The broad development/HMR suite is also not claimed by
these optimized native runs. Current adapter and simulator native regressions
are explicitly identified above rather than substituted for physical OpenWrt.

Main remains `d5d05413d30c01f189968526be2ac80979387bd5`; Beta 1 remains
`7433c96b379dd70d1481f06d645ff7f49570d9fe`; Beta 2 remains
`e4524e80ac38ff0702e0917fc9a8f314d2f1ad82`. No merge, tag move, release,
normal-data migration or storage-location change occurred. The previously used
reviewer scratchpad directory is absent; no new independent acceptance is assumed.

## Initial failures retained

Parser/auth/binding stubs produced 22 behavioral failures before implementation
(79 existing acquisition tests passed). A graph fixture first omitted a required
field; after correction, four graph behaviors failed against the stub and five
passed after implementation. Three missing frontend workflow cases failed first.
Missing source-assessment/route cases subsequently failed before integration;
the implemented acquisition filter then passed 118 tests.

An initial directory-lease test hit a sandbox denial; the guard was retained and
the authorized rerun passed. Two incorrect public RFC vectors were corrected
(password capitalization and SHA-512/256 erratum). An upstream no-qop session
serialization defect was reproduced; that combination is rejected rather than
sending an incomplete Digest response.

Full Clippy initially exited 101 for three `byte_char_slices` findings; corrected
to equivalent byte strings. A format check caught the newly added socket test
before rustfmt. Its first fixture was correctly rejected by production's
loopback-target policy. A test-only binding preserved that rejection; its next
run exposed inherited Windows nonblocking mode in the test server, which was
explicitly reset. The final socket check and both full Rust profiles/Clippy/format
passed. Actual outputs are in `.tools/web-management-evidence/`; evidence contains
text results, not raw captures, firmware, credentials or generated screenshots.

Native graph verification initially clicked before React published the extraction
report. The driver now waits for the actual action; the CPIO/RAR and selected-device
runs passed. A new storage sentinel audit initially tried to read the intentionally
exclusive live `library.lock` and failed with `EBUSY`; moving that audit after clean
shutdown preserved the lock and made both storage passes succeed. Both failed logs
are retained. No application protection was relaxed to make these checks pass.

## Physical and remaining gates

No new MG11 HTTP, challenge, credential, login, authenticated read, flash read,
configuration write or acquisition occurred. No owner hardware claim follows
from software fixtures or loopback. H01-A/H01-B remain separate; H02/H03 remain
revision-separated; H04 packages do not qualify an Airtel hardware revision.

The full connected-router question is currently **NO**. Remaining blockers:

1. No sanitized H01 HAR has been supplied for this milestone; no compiled web
   identity/status route is independently qualified. Imported evidence cannot
   mint that authority.
2. Live resource collection lacks a qualified selected origin and retained loaded
   root/reference proof. Its IPC rejects `IDENTITY_UNQUALIFIED` without I/O.
   Local file analysis is implemented and explicitly labelled.
3. Digest and the source-bound client are tested, but no production authenticated
   connector/native credential flow is enabled. Basic/cookie/form scheme names
   are represented; those session clients remain unsupported.
4. No qualified MG11 installed-firmware export, matching downloadable image or
   documented storage read has produced bytes. No arbitrary probing occurs.
5. Observed FOTA response URL/selectors/signature parsing needs a reviewed family
   response schema; it is not invented from generic HAR bodies. Existing official
   image provenance remains implemented independently.
6. Graphs project completed immutable extraction reports. Quick signature
   analysis alone does not create a complete validated structural graph. UBIFS,
   JFFS2, YAFFS2, PAC and unqualified variants remain unsupported.

Next legitimate step: review a sanitized capture and device-served bundle evidence,
qualify a narrow read-only identity descriptor, then separately authorize a
supervised bound read. Device acquisition/dumping and existing file extraction
remain distinct. Phase 7.5 and the project are incomplete; acceptance is pending.
