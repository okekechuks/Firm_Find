# Universal device-to-firmware workflow verification

27 October 2026, Africa/Lagos. Implementation and verification are in progress; this document does not declare physical acquisition, rebuilding, Phase 7.5, or the project complete.

## Repository and dependencies

Starting main: `d5d05413d30c01f189968526be2ac80979387bd5`, with green [main CI](https://github.com/okekechuks/Firm_Find/actions/runs/40524198759). Development branch: `feature/universal-firmware-workflow`. Main, historical branches and published tags/assets are preserved. The branch includes the separately retained extraction corrections `f7f8952` and their CI environment correction `a3a6f92`; these are dependencies, not an assertion of independent acceptance. No release tag is created.

## Audit and integration

| Existing implementation | Finding and resulting integration |
|---|---|
| `discovery/service.rs::DiscoveryService`, Windows discovery and `profiles/engine.rs::gather_evidence/evaluate` | Real host inventory and evidence exist. Inventory never establishes exact router identity. Reused without replacing Windows discovery. |
| `acquisition/mod.rs::identity`, `AcquisitionStrategyRegistry` | Five pure strategy contracts existed, with non-executable probes. Device UI ended with `canObtain=false`. New workflow evaluates the official source against fresh, unique trusted manufacturer/model/revision evidence; four other strategies remain explicitly non-executable. Existing probe approvals and wire reports are unchanged. |
| `library/official.rs::catalogue/qualify/download`, `LibraryService::acquire_official` | Real HTTPS acquisition, signed-catalogue pins, hash/length checks and transactional immutable admission existed. New `acquire_for_device/queue_official` supplies device association and a live publication fence. Standalone Library downloads remain supported and do not establish hardware identity. |
| `library/analysis.rs`, `library/extraction.rs` | Existing jobs, worker sandbox, integrity checks, file index, preview/export and durable manifests reused directly. Successful device acquisition is analyzed automatically in the workflow; extraction requires explicit user action and never requires re-import. Gzip receives an explicit metadata classification. |
| `CustomizationPanel.tsx`, separate DNS build helper | Existing image-customization controls have no integrated qualified build executor. Twelve backend readiness entries distinguish live router/client DNS, APN, Wi-Fi limits, USSD/modem parameters and firmware overlays/packages/layout inputs. Customization, rebuild, install and recovery execution stay disabled. Connecting the separate helper requires target-specific build input/layout/recovery qualification. |
| `Devices.tsx`, `FirmwareLibrary.tsx` | More Info exposes the connected workflow, source approval, persistent jobs, cancellation, associated artifacts, analysis and real extraction browser. `#firmware/<sha256>` navigates directly to the retained image. Existing cards/navigation and native device-request approvals are retained. |

Paths in this table are under `apps/desktop/src-tauri/src/` for Rust and `apps/desktop/src/` for frontend. New integration resides in `library/workflow.rs`, `library/workflow/{tests,lab}.rs`, `components/DeviceFirmwareWorkflow.tsx`, `types/workflow.ts`, `lib/backend.ts`, Tauri command/ACL registration and `scripts/verify-universal-workflow.mjs`.

## Authority and persistence decisions

The backend-owned record is a projection of current inventory/evidence plus existing durable SQLite jobs and official occurrences. No schema migration or storage-location change. Latest 100 device/associated-artifact jobs are returned; older records remain retained. Ordinary scan timestamps and user labels do not revoke unchanged authority or extend trusted evidence lifetime. A new approval requires current noncancelled inventory, connected eligible hardware, a unique bound active interface/endpoint and exact catalogue target. Protected host adapters, ambiguous identities, missing revisions, expired approvals and substituted sources are refused.

The download approval binds device ID, inventory revision, trusted-evidence digest, expiry and the compiled exact source ID. The source-specific operation is public-image acquisition only. The binding receipt persists in the existing job metadata; it is historical evidence, never reusable authority. Validation runs before network I/O and again under repository/inventory publication locks. Interrupted or changed-device downloads cannot commit successful Library admission. Repeated pending/running requests reuse the associated job. Successful bytes retain `official_image_download` / `official_vendor_download` provenance and the source catalogue; they never become `physical_flash_dump`, a configuration backup or installed-device firmware.

Local analysis/extraction remains useful for previously acquired artifacts after device disconnect. Reopening the app reconstructs workflow progress from persisted jobs/occurrences/reports and requires fresh authority for another acquisition. Failed/cancelled analysis is never automatically retried. Embedded files are not executed; extraction confers no build or installation permission. Original content addresses, no-overwrite export and sandbox/resource safeguards remain intact.

## Test method and actual findings so far

Unit tests use explicitly synthetic identity/evidence. The native-only `--workflow-lab` flag substitutes a labelled synthetic identity in an isolated native profile; production rejects that flag. No physical network transport or simulated successful extraction is provided by this fixture. The native workflow must download the genuine signature-pinned Archer C7 v5 image through production HTTPS, admit it through the actual Library, analyze it, run the real WSL extractor, hash every ordinary extracted file, render its text, and repeat after restart. Synthetic identity success is never physical-device success.

Genuine source: [OpenWrt 25.12.6 Archer C7 v5 image](https://downloads.openwrt.org/releases/25.12.6/targets/ath79/generic/openwrt-25.12.6-ath79-generic-tplink_archer-c7-v5-squashfs-sysupgrade.bin), 7,405,862 bytes, SHA-256 `0ec9643bb9f43d65c689761d916a9e0c13bad5968656d74e516df010deb0be9d`. Its compiled catalogue/signature evidence is reused, without changing acceptance thresholds. Current native manifest hashes will be recorded only after execution.

Initial implementation checks found compile errors in the new fence variable/fixture initialization, one redundant struct update (Clippy), a React cleanup-ref warning and a test query matching repeated explanatory text. These were corrected; none is reported as a successful initial run. Six focused Rust regressions passed, then 378 isolated-profile Rust tests and two doctests passed. A later unchanged-scan binding regression increases final counts; final-source checks are pending. Frontend: 173 tests passed after correcting the query. Native builds and actual workflow/restart run are pending.

The prior hardening CI Windows job passed but Ubuntu 22.04's old bubblewrap lacked `--disable-userns`; the root-mount integration test failed before any extraction. CI now adds an explicit privileged **disposable Ubuntu 26.04 container** running the entire extraction suite as an ordinary user. The legacy runner explicitly reports its unsupported primitive; production sandbox controls/pins are unchanged. This does not substitute for the actual Windows/WSL run.

## Commands and final evidence

Logs are retained under the development worktree's `.tools/` directory. Exact final exits, counts, source/manifest/bundle hashes, CI URL and native screenshot inspection will be appended after execution:

- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all --check`
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings`
- Same Clippy command with `--features native-test-profile` before `--locked`.
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline`
- Same test command with `--features native-test-profile`.
- `npm.cmd run lint`, `npm.cmd test`, `npm.cmd run build`.
- `npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline`.
- `npm.cmd run build:native-tests` (retains production executable hash safeguard).
- `npm.cmd run test:universal-workflow-native`.
- `npm.cmd run test:extraction-review-native` (retained extraction review regressions).
- `py scripts/audit-data-bytes.py .tools/workflow-normal-after.json .tools/workflow-normal-before.json`.

Before integration, the byte-only normal-data audit matched the P0 baseline: 12 files, 5,611,701 bytes. It does not open SQLite. All tests use temporary or isolated storage; no normal application launch, data migration, physical request or write is authorized by this task.

## Physical gates and next task

H01 remains an observed USB/RNDIS candidate with partial separately approved management observations. Exact manufacturer/model/revision, installed firmware relationship, challenge qualification and legitimate firmware-byte source remain unverified. H02 and H03 are separate unqualified revisions; H04/H05 have no physical qualification. No new hardware result is claimed. [Per-device matrix](HARDWARE_TEST_MATRIX.md).

Highest-impact next task: qualify an exact identity reader and legitimate firmware source for the first physical target, beginning with the separately owner-supervised credential-free MG11 challenge gate. Credentials/login and firmware reads require their separate review/approval; no guessed CGI, arbitrary AT/debug command, privileged flash read, reset, write or flashing is enabled. A missing supported source remains a device-specific blocker rather than universal incompatibility.
