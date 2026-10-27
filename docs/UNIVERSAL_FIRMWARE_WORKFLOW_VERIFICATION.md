# Universal device-to-firmware workflow verification

27 October 2026, Africa/Lagos. The integrated software workflow, including durable stage progress and browser retention during background events, passed real Windows/WSL qualification on runtime source `2dd43108feec5f588bbff3d3f5e012d4bb1c038a`. Physical acquisition, rebuilding, Phase 7.5 and the project remain incomplete.

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

Local analysis/extraction remains useful for previously acquired artifacts after device disconnect. Reopening the app reconstructs workflow progress from persisted jobs/occurrences/reports and requires fresh authority for another acquisition. Each operational stage links its current persisted `JobRecord`, reusing pending/running/completed/failed/cancelled states rather than inventing a second progress enum. A failed attempt exposes its error code/message while a previous verified output remains usable. A failing-first projection test initially returned null for running status, then passed after this correction. Failed/cancelled analysis is never automatically retried. Embedded files are not executed; extraction confers no build or installation permission. Original content addresses, no-overwrite export and sandbox/resource safeguards remain intact.

## Test method and actual findings so far

Unit tests use explicitly synthetic identity/evidence. The native-only `--workflow-lab` flag substitutes a labelled synthetic identity in an isolated native profile; production rejects that flag. No physical network transport or simulated successful extraction is provided by this fixture. The native workflow must download the genuine signature-pinned Archer C7 v5 image through production HTTPS, admit it through the actual Library, analyze it, run the real WSL extractor, hash every ordinary extracted file, render its text, and repeat after restart. Synthetic identity success is never physical-device success.

Genuine source: [OpenWrt 25.12.6 Archer C7 v5 image](https://downloads.openwrt.org/releases/25.12.6/targets/ath79/generic/openwrt-25.12.6-ath79-generic-tplink_archer-c7-v5-squashfs-sysupgrade.bin), 7,405,862 bytes, SHA-256 `0ec9643bb9f43d65c689761d916a9e0c13bad5968656d74e516df010deb0be9d`. Its compiled catalogue/signature evidence is reused, without changing acceptance thresholds. Actual native manifests and their hashes are recorded below.

Initial implementation checks found compile errors in the new fence variable/fixture initialization, one redundant struct update (Clippy), a React cleanup-ref warning and a test query matching repeated explanatory text. These were corrected; none is reported as a successful initial run. Six focused Rust regressions passed, then 378 isolated-profile Rust tests and two doctests passed. A later unchanged-scan binding regression increases final counts; the final-source results below supersede these intermediate counts. Frontend: 173 tests passed after correcting the query. Native builds and actual workflow/restart qualification subsequently passed as recorded below.

The prior hardening CI Windows job passed but Ubuntu 22.04's old bubblewrap lacked `--disable-userns`; the root-mount integration test failed before any extraction. CI now adds an explicit privileged **disposable Ubuntu 26.04 container** running the entire extraction suite as an ordinary user. The legacy runner explicitly reports its unsupported primitive; production sandbox controls/pins are unchanged. This does not substitute for the actual Windows/WSL run.

## Commands and final evidence

Logs are retained under the development worktree's `.tools/` directory. The executed qualification below records exits, counts, hashes, CI and native screenshot inspection. Command entry points:

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

Before and after integration, the byte-only normal-data audit matched the P0 baseline: 12 files, 5,611,701 bytes, including all existing settings/database/vault/log files. It does not open SQLite. All tests use temporary or isolated storage; no normal application launch, data migration, physical request or write was performed.

## Executed qualification and limitations

Runtime source `7ebd028dca89760b2846206a998ac0b0ae46983d` has fully green [Windows/Linux CI 40578734639](https://github.com/okekechuks/Firm_Find/actions/runs/40578734639), including both Windows executables and the production-EXE hash safeguard. Stage projection source `b5430ad97acf09dddc10315fec8265a1985047f4` has its own [CI 40582757385](https://github.com/okekechuks/Firm_Find/actions/runs/40582757385); all three jobs passed, including both Windows builds. Main stays `d5d0541`; no merge or release tag is included.

| Exact command | Exit / actual result |
|---|---|
| `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all --check` | 0, clean |
| `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | 0; final projection source: 379 passed, 2 ignored, 2 doctests passed |
| `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | 0; 380 passed, 3 ignored, 2 doctests passed |
| `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | 0 |
| `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | 0 |
| `npm.cmd run lint` | 0, zero warnings |
| `npm.cmd test` | 0; final runtime: 176 tests, 22 files |
| `npm.cmd run build` | 0; actual script executes `tsc -b` and Vite production build |
| `npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline` | 0 on final projection source |
| `npm.cmd run build:native-tests` | 0 on final projection source; production executable remained identical |
| `wsl.exe -d Ubuntu --cd 'C:\Users\chuka\OneDrive\Documents\Firm_Find\.tools\beta2-hardening' --exec python3 -B scripts/test-firmware-extraction.py` | 0; all 23 tests, including actual kernel namespace and near-limit tmpfs checks |
| `py -B scripts/test-firmware-extraction.py` | 0; 17 applicable tests pass, six Linux-only skips |
| `npm.cmd run test:catalogue` | 0; three exact entries and pinned signature/manifests verified |
| `npm.cmd run test:universal-workflow-native` | 0 on final projection source; both first launch and restart confirm all three persisted stage jobs completed |
| `npm.cmd run test:extraction-review-native` | First exit 1 at owned Save-dialog automation; corrected driver rerun exit 0 |
| `py scripts/audit-data-bytes.py .tools/workflow-normal-after.json .tools/workflow-normal-before.json` | 0; complete normal file set, sizes and hashes identical |

The ignored entries are explicit subprocess entry points driven by parent tests and the separately operated OpenWrt VM interoperability test. The latter was not rerun as part of this task. No physical-device identity success is inferred from an ignored test, fixture or VM.

The native workflow run at `.tools/foundation/2026-10-27T20-47-02.351Z` used genuine public HTTPS download and actual SQLite/Library admission, automatic analysis, native GUI Cancel, retry, WSL extraction, per-file independent hashes, rendered text and restart. It retained 869 ordinary files, 128 directories and 13,411,262 bytes. Manifest SHA-256: `ba0ba4fef87c87f46fb2897007c7dbddba4d784eed4f0af6a14281a48297d057`; bundle SHA-256: `3e4c62186d7eed588d42edf1c98faf8e0e5c749c8cd436e20a535dd2385aae02`. Source hash stayed the pinned `0ec9643b…be9d`. Actual tree/text screenshots were opened and inspected after scrolling the dialog to the content. Merely starting a terminal was not accepted as GUI proof. Console issue list was empty; owned processes stopped and test cleanup guards passed. The final projection-source run below retains its own manifest rather than reusing this timestamped metadata.

The broader native run at `.tools/foundation/2026-10-27T20-44-03.953Z` also verified six real navigation pages, Windows discovery and three refreshes (58 unique host records, not 58 verified routers), real settings IPC, restart, selected-file TAR bytes, raw/gzip file equality, genuine RT-N16 TRX extraction, genuine UBI-containing TRX rejection, synthetic space exhaustion and individual USTAR-name rejection before Save. The preceding failed run is retained at `2026-10-27T20-39-58.580Z`. DirectUI's owned action was not completed by the driver's parent IDOK message; `drive-file-dialog.ps1` now uses UI Automation filename value/readback and the actual owned Open/Save button, with bounded closure acknowledgement. Destination, ownership and overwrite guards remain unchanged. This was a test-driver correction, not permission to alter app or router safeguards.

| Required scenario | Evidence / limit |
|---|---|
| Qualified official route | Real bytes, admission, analysis, extraction, browsing and restart through the device view; identity explicitly synthetic |
| Unknown proprietary MiFi | Rust/UI fixtures keep reported/observed evidence separate, return no source and no enabled acquisition; no packet or fake artifact |
| Different hardware revisions | v5/v2/unregistered revision tests reject cross-revision inheritance; H02/H03 physical qualification remains separate |
| Unsupported genuine format | Signed real RT-AC68U TRX admitted as an import, nested UBI rejected; source and previous successful extraction retained |
| Interruption / corruption | GUI extraction cancellation and retry; unit publication-fence disconnect/interface/expiry tests; retained `sprint_interrupted_download_and_analysis_fail_without_retry_or_admission`, `corrupt_and_missing_blobs_are_detected_and_never_overwritten`, `restart_recovers_partial_and_published_jobs_without_false_success`, cancellation-at-publication and job-transition regressions |
| Resource exhaustion | Real Linux near-limit tmpfs and native `EXTRACTION_RESOURCES`; ordinary Windows drive exhaustion was not forced; `stream_cancels_between_chunks_and_maps_disk_full` checks error mapping |
| Authorization negatives | Denied approval creates no job/artifact; changed binding/source, expiry, protected host, ambiguous identity, missing revision and label-only hints refuse authority |

The current device workflow requires trusted identity bound to an active interface and literal gateway endpoint. Existing generic Windows network adapters and saved labels cannot provide that identity. Physical firmware readers, an exact proprietary identity producer, hostname-based workflow binding and a qualified target rebuild executor are not supplied by this integration. Those are explicit remaining dependencies, not hidden successes.

## End-to-end product state

### Final delivery qualification

Runtime **`2dd43108feec5f588bbff3d3f5e012d4bb1c038a`** and [CI 40586111702](https://github.com/okekechuks/Firm_Find/actions/runs/40586111702) passed all three jobs, including both Windows builds, 379 standard / 380 isolated Rust tests, two doctests per profile, 176 frontend tests, lint/typecheck/formatting/Clippy and the modern Linux worker suite. Earlier runtime/counts below are historical intermediate qualification, superseded for delivery by this paragraph.

`npm.cmd run test:universal-workflow-native` exited 0 on the final runtime with strengthened browser-retention and viewport assertions. Final evidence directory: `.tools/foundation/2026-10-27T21-35-04.467Z`. A real integrity job triggered a Library event on first launch and again after restart; React consumed the event, finished its backend reload and kept the same selected extracted-file preview. Viewport metadata records connected content at y=350.75–518.75 within the 840-pixel window, and final screenshots were opened and visually inspected. Acquisition/analysis/extraction jobs remained completed; GUI cancellation/retry, original and per-file hashes, direct selected Library navigation, no console errors, process shutdown and safe cleanup all passed.

Final original: 7,405,862 bytes, SHA-256 `0ec9643bb9f43d65c689761d916a9e0c13bad5968656d74e516df010deb0be9d`. Final manifest: `e8dbd608856970c33fe341605e0c00724103fe2bfeb00ead73057024b8405064`. Bundle: `3e4c62186d7eed588d42edf1c98faf8e0e5c749c8cd436e20a535dd2385aae02`; 869 files, 128 directories, 13,411,262 ordinary bytes. Identity is explicitly synthetic; `physicalDeviceSuccess=false`.

Final local production EXE SHA-256: `da01d520fc39f9955509580b03fedabe68e76257c8c0f23a4bd85e0f49e60398`; isolated EXE: `ed73438b2bacb1ed12a037dabbdcfe7b3196efa28bf7af7aa2028d366d153f3f`. Both optimized builds exited 0, including the production-EXE preservation check. Normal data remains byte-identical across all runs: 12 files, 5,611,701 bytes. No new physical router request, credentials, firmware dump, device write, reset, install or flash occurred. The current branch is not merged and no release is tagged.

**Software integration verdict: VERIFIED WITH PHYSICAL/BUILD LIMITATIONS. Full Phase 7.5/project verdict: INCOMPLETE.** The missing physical acquisition and qualified rebuild/install/recovery paths remain concrete product work; successful software-image processing does not close them.

Final runtime `b5430ad97acf09dddc10315fec8265a1985047f4` passed `npm.cmd run test:universal-workflow-native` with exit 0 at `.tools/foundation/2026-10-27T21-09-44.271Z`. Acquisition, analysis and extraction stage jobs each remained `completed` on reopening. Actual files and preview were rendered; the same original hash and all 869 ordinary file hashes matched. Manifest SHA-256: `94c4086e6ebc3441fe5596e214d7ee48fd2685b0db4152c8e739e7f7f6ce3c4f`. Bundle SHA-256: `3e4c62186d7eed588d42edf1c98faf8e0e5c749c8cd436e20a535dd2385aae02`. Native console issues: zero. Safe owned cleanup and normal-data byte preservation passed. This run uses synthetic identity and genuine image processing; `physicalDeviceSuccess=false` remains explicit in evidence.

Local optimized production EXE SHA-256: `58a30a3dfc22f30faec7f21b0d6b8e07ac3162eb178b716609b40f750bd403d2`; dedicated isolated EXE: `14a60bce3b0879a056363fd8ee7cb214a85380187735decba200359c64e4c2d0`. Both build commands exited 0. The native-test build's production hash safeguard passed. These are local artifacts, not newly published release assets.

Final [CI 40582757385](https://github.com/okekechuks/Firm_Find/actions/runs/40582757385) passed Windows verification, legacy Linux helpers and all 23 tests in the modern extraction qualification container. Documentation close-out commits after the runtime SHA do not change executable behavior. No generated screenshots or firmware binaries are committed.

| Stage | Actual state |
|---|---|
| Physical detection | Existing Windows inventory/refresh works; router identity remains separate |
| Physical identity | H01 partial observations; no full trusted fleet identity |
| Firmware acquisition | Genuine qualified official image route works through workflow; installed-device dump unavailable |
| Immutable Library | Associated official occurrence, exact original hash and restart retention verified |
| Analysis / extraction | Real qualified image analysis, supported filesystem extraction, browsing/preview/export verified |
| Customization / rebuilding | Backend readiness/reasons visible; no integrated target-qualified executor enabled |
| Validation / installation / recovery | Original/extracted integrity verified; rebuilt image validity and install/recovery feasibility remain unqualified |

OBTAINER now connects the existing working image tools in one executable device workflow. It still cannot accept an arbitrary connected proprietary router, dump its installed firmware and rebuild an installable replacement.

## Physical gates and next task

H01 remains an observed USB/RNDIS candidate with partial separately approved management observations. Exact manufacturer/model/revision, installed firmware relationship, challenge qualification and legitimate firmware-byte source remain unverified. H02 and H03 are separate unqualified revisions; H04/H05 have no physical qualification. No new hardware result is claimed. [Per-device matrix](HARDWARE_TEST_MATRIX.md).

Highest-impact next task: qualify an exact identity reader and legitimate firmware source for the first physical target, beginning with the separately owner-supervised credential-free MG11 challenge gate. Credentials/login and firmware reads require their separate review/approval; no guessed CGI, arbitrary AT/debug command, privileged flash read, reset, write or flashing is enabled. A missing supported source remains a device-specific blocker rather than universal incompatibility.

## Exact integration file inventory

### Visual audit follow-up

The `b5430ad` run passed its data/transition assertions, but the later screenshot showed the preview could be reset by background Library refresh. This was a real frontend state-retention defect, not a decoder failure. A failing-first frontend test reproduced removal of the existing view during an unresolved background request. `2dd43108feec5f588bbff3d3f5e012d4bb1c038a` keeps the current browser mounted while reloading, scopes consent to the complete binding/source key, clears consent for material changes/errors and after acquisition starts, and exposes reload state accessibly. The full frontend suite then passed 176 tests with zero lint warnings. Rust is unchanged from the verified stage-projection source.

The strengthened native harness now creates an actual integrity-verification job, waits for React to consume its Library event and finish reloading, then asserts that the selected extracted-file preview remains present. It performs this on both first launch and restart. Its final outcome and hashes are recorded in the delivery close-out below; earlier screenshots alone do not prove this regression fixed. Capture now also waits for two rendered frames and verifies the preview rectangle is on-screen; computed viewport JSON accompanies the images.

Paths relative to repository root; extraction dependencies are separately inventoried in the Beta 2 correction report. `28ef64d` corrects the owned-dialog test helper. `b5430ad` adds stage-level durable progress; later evidence-only commits do not alter application behavior.

```text
.github/workflows/windows.yml
apps/desktop/src-tauri/build.rs
apps/desktop/src-tauri/capabilities/main.json
apps/desktop/src-tauri/permissions/autogenerated/get_device_workflow.toml
apps/desktop/src-tauri/permissions/autogenerated/start_device_firmware_acquisition.toml
apps/desktop/src-tauri/src/acquisition/mod.rs
apps/desktop/src-tauri/src/discovery/service.rs
apps/desktop/src-tauri/src/lib.rs
apps/desktop/src-tauri/src/library/admission.rs
apps/desktop/src-tauri/src/library/analysis.rs
apps/desktop/src-tauri/src/library/db.rs
apps/desktop/src-tauri/src/library/mod.rs
apps/desktop/src-tauri/src/library/workflow.rs
apps/desktop/src-tauri/src/library/workflow/lab.rs
apps/desktop/src-tauri/src/library/workflow/tests.rs
apps/desktop/src/app/App.tsx
apps/desktop/src/app/navigation.ts
apps/desktop/src/components/DeviceFirmwareWorkflow.tsx
apps/desktop/src/features/Devices.tsx
apps/desktop/src/features/FirmwareLibrary.tsx
apps/desktop/src/lib/backend.ts
apps/desktop/src/styles.css
apps/desktop/src/test/App.test.tsx
apps/desktop/src/test/DeviceFirmwareWorkflow.test.tsx
apps/desktop/src/test/Devices.test.tsx
apps/desktop/src/test/workflow-fixture.ts
apps/desktop/src/test/workflowContracts.test.ts
apps/desktop/src/types/contracts.ts
apps/desktop/src/types/workflow.ts
docs/HARDWARE_TEST_MATRIX.md
docs/PHASE_STATUS.md
docs/UNIVERSAL_FIRMWARE_WORKFLOW_VERIFICATION.md
package.json
scripts/drive-file-dialog.ps1
scripts/verify-foundation.mjs
scripts/verify-universal-workflow.mjs
```
