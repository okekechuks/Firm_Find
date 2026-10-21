# Isolated Windows native testing

## Phase 6 simulator

`npm.cmd run dev:simulator` creates a new marked native-test run and launches the dedicated development executable with `--simulator`. It exposes the laboratory inside Connected Devices; the user must then select Start simulation, approve the virtual endpoint handshake, authenticate with the fictional `lab-only` password and authorize feature reads separately. Stop simulation exits the runtime; closing the app releases it. Development runs retain their isolated profile for inspection. Neither an absent real router nor a frontend simulation flag enables this mode.

After building both release executables below, `npm.cmd run test:simulator` runs the retained complete Phase 4/4.1/5 GUI suite plus Phase 6 in development, interrupted-launch recovery and optimized isolated release. Only these explicitly flagged child launches enable simulation. Existing `test:gui -- --phase5` remains a non-simulator qualification path. The normal executable rejects `--simulator` before opening normal data; the normal capability list does not include simulation commands.

Phase 6 screenshots are actual native WebView captures under `docs/images/phase6-<run-id>/`, with simulation labels visible. Per-run protocol/console/process/data evidence remains under `.tools/foundation/`. The physical discovery registry still reports `windows_observed` and `simulatedAdapters:false`; `simulationAvailable:true` and the separate simulation IPC describe the isolated laboratory. Virtual identities/configuration/snapshots/history stay in memory and are not written to discovery, profile, firmware, backup or production-log tables. App restart starts a fresh virtual fixture. See [Phase 6 verification](PHASE_6_VERIFICATION.md) for actual results and [architecture](SIMULATOR_ARCHITECTURE.md) for boundaries.

Phase 5 adapter qualification adds `npm.cmd run test:gui -- --phase5` after the ordinary and dedicated releases are built. This runs the existing full GUI/Phase 4/4.1 suite plus passive adapter registry/match queries, malformed/direct-IPC rejection, host-NIC protection, the actual saved-endpoint form/status flow and four-feature UNKNOWN checks. It performs no router management request or physical configuration change. The small mock connector exists only in Rust unit tests (`cfg(test)`), never in either native executable.

Phase 5 native captures use a new per-run evidence folder under `.tools/foundation/`; historical GUI screenshots remain preserved. The retained GUI verifier now selects the real **Check compatibility** / **More info** buttons and uses the existing Appearance heading for its temporary HMR edit after the owner's earlier filler removal. No removed product copy is restored. Final Phase 5 execution evidence is in [the verification report](PHASE_5_VERIFICATION.md).

Normal `obtainer.exe` and `npm run tauri -- dev -- --locked --offline` continue using `%LOCALAPPDATA%\OBTAINER\` and the existing production WebView profile. Native automation uses **a different executable**, `obtainer-native-test.exe`, compiled only when Cargo feature `native-test-profile` is explicitly enabled. The test configuration has its own application identifier. No production data path override exists.

Build from the repository root, then run in a normal interactive Windows user session:

```powershell
$env:CARGO_INCREMENTAL='0'
npm.cmd run tauri -- build --no-bundle --ci -- --locked --offline
npm.cmd run build:native-tests
npm.cmd run test:foundation -- --phase3 --phase4 --phase4-1
```

The build helper runs the frontend production script, then Cargo release compilation with the Tauri test configuration, `native-test-profile,tauri/custom-protocol` features and the explicit `obtainer-native-test` target. The custom-protocol feature embeds production assets; development still uses Vite. It checks that the ordinary release executable's SHA-256 remains unchanged. Do not set Tauri `mainBinaryName` to select a Cargo target: that option renames the package executable. The helper avoids the Tauri CLI's package-binary build/rename behavior when compiling this additional target. The dedicated entry point is selected with `--bin`, and test mode cannot be reached through the ordinary entry point even if a feature-enabled library is compiled.

The harness creates a random 32-character lowercase hexadecimal run ID and supplies only `OBTAINER_NATIVE_TEST_RUN_ID` to its child processes. The test executable requires this ID and refuses missing/malformed IDs, arbitrary path arguments, and `OBTAINER_DATA_DIR`, `OBTAINER_TEST_DATA_DIR` or unknown `OBTAINER_NATIVE_TEST_*` overrides. The ordinary executable rejects native-test activation before opening its database. Root discovery uses the current user's Windows LocalAppData known folder, rather than an environment-supplied data directory.

Each run is confined to:

```text
%LOCALAPPDATA%\OBTAINER-NativeTests\<run-id>\
  .obtainer-native-test.json   # application/run/state marker
  .native-owner.lock          # exclusive process ownership
  data\obtainer.db
  data\logs\
  data\vault\
  data\workspaces\
  data\reports\
  webview\                    # separate WebView2 user-data directory
  fixtures\                   # generated harmless binaries
  exports\                    # test manifests
```

Development and optimized release use the same run directory across restart/recovery, but different runs have different IDs. A visible **ISOLATED NATIVE TEST** window title and React notice show the run ID. Snapshot IPC identifies the profile and explicitly says discovery is `windows_observed`, with no simulated adapters. Real Windows observations still do not establish verified identity or feature support. Future mock adapters must introduce explicit mock provenance rather than reuse physical-device evidence; no adapters are implemented by this hardening.

All native runners require isolated profiles, including the legacy WebDriver smoke runner. `--attach-dev` is rejected because an arbitrary existing process cannot be adopted safely. To run the WebDriver audit after building both releases:

```powershell
npm.cmd run test:native -- target/release/obtainer-native-test.exe --audit
```

It still needs the existing Tauri/Edge drivers described in the development guide. The normal application binary is no longer an accepted automated-test target. Screenshots/logs go into a unique evidence directory, not an existing product screenshot.

## Evidence and failure injection

The foundation runner captures read-only normal-data evidence before any native launch and compares it again afterward: schema, complete table contents fingerprint, settings, counts, saved-router/profile IDs, main DB/WAL hashes, vault filenames/sizes/hashes, integrity and foreign-key checks. Close ordinary OBTAINER sessions before testing; another process legitimately writing normal data invalidates this equality check. Probes do not migrate, checkpoint, repair or reset the normal database.

`--phase3` additionally requires the owner's connected MG11 USB candidate. Its reported identifier/version are saved only in the isolated database. `--phase4` drives real native Open/Save dialogs, React confirmation, jobs, integrity, deduplication, cancellation and forced import interruption. `--phase4-1` adds actual queued identical 256 MiB imports, isolated blob corruption/deletion, injected SQLite commit/activity failures, and graceful shutdown during an active integrity check. Only generated bytes are selected. Deterministic near-commit cancellation and additional boundary cases run against isolated Rust temporary vaults.

The harness temporarily instruments existing frontend code to verify errors/loading/HMR, then restores the exact source it read. Do not edit those files or run frontend builds/tests concurrently with native verification. It enables loopback CDP only for its own WebView2 processes and closes owned app/Vite/debug sessions afterward. Evidence is retained under `.tools/foundation/<UTC timestamp>/`, including failures and normal-data comparison. A failed run's test data is retained for diagnosis.

## Controlled cleanup

Only the dedicated test executable implements `--cleanup-native-test-run`. No directory path is accepted. It derives the root from the known folder and validated current run ID, checks canonical parent containment, matching marker/version/kind/ID/state, exclusive ownership, a Windows process check, and an entire no-reparse preflight. Cleanup conservatively refuses while any other native-test executable is alive, or while a WebView2 process references this run; failed process inspection also retains files. Active ownership, unknown root entries, nonempty staging, links, access errors or uncertain state cause refusal with a reason and retained files. Cleanup cannot target the normal app directory or a different run through a path argument.

After validation, the marker becomes `cleaning`, preventing another startup from adopting a partially deleted run. Directory leases protect traversal; deletion does not follow links. Failed partial cleanup retains its marker/context where possible and may be retried for the same run after resolving the reported condition. Unknown staging is never silently deleted. The native acceptance suite checks active ownership, mismatched markers, unknown staging, a junction pointing outside the permitted root, and successful cleanup after shutdown. The outside sentinel must survive.

Successful foundation verification removes only its own isolated run. Evidence stays in the workspace. Historical Phase 4 test artifacts/jobs already present in normal OBTAINER data are **not removed**. Manual deletion of those records requires separate owner approval.

Implementation references: `src-tauri/src/native_test.rs`, `native_test_main.rs`, `lib.rs`, and `scripts/native-test-profile.mjs`. Windows root resolution follows [SHGetKnownFolderPath](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shgetknownfolderpath); WebView separation uses Tauri's [window dataDirectory](https://v2.tauri.app/reference/config/#windowconfig). Final observed results belong in [Phase 4.1 verification](PHASE_4_1_VERIFICATION.md).

## GUI acceptance

After the normal and dedicated native releases are built, run `npm.cmd run test:gui`. This executes the retained Phase 4/4.1 generated-file regressions plus six-page GUI acceptance. It drives real React/Rust IPC, native file/folder dialogs, help keyboard behavior, an owned window resize, active-job navigation/cancellation, backups/restores, removal confirmations, themes and persistent settings. A reviewed copy plan is executed only inside the marked run, then the optimized app restarts to verify the new Library location and preserved metadata/backup path. It captures real screenshots under `docs/images/gui-*.png`.

No extraction or build adapter is simulated. The extraction screenshot explicitly shows the unavailable state. A connected MG11 is optional for GUI acceptance; add the separate `--phase3` hardware regression only when the device is available. Normal-data probes, marker/path/ownership checks, failure retention and guarded cleanup remain mandatory.

`npm.cmd run test:foundation -- --gui-preview --gui --phase4` runs a development-only diagnostic with generated files. HMR verification reloads the restored backend module graph after fault injection, then proves a component edit and restoration without another page reload or changed app/Vite PIDs. Do not run builds/tests or edit watched frontend/Rust files while native verification is active.

## Genuine OpenWrt OS qualification

Phase 7 adds `npm.cmd run test:openwrt`, retaining the GUI, adapter, simulator and generated-file regressions. It requires the existing QEMU/Ubuntu WSL lab described in [Phase 7 verification](PHASE_7_VERIFICATION.md), both Windows executables already built, and the checksum-verified image under ignored `.tools/phase7-vm/`. This is a genuine OpenWrt OS test, not physical hardware qualification.

Keep the VM launcher running in an owned foreground session:

```powershell
wsl -d Ubuntu -- python3 -u /mnt/c/Users/chuka/OneDrive/Documents/Firm_Find/scripts/openwrt-test-vm.py serve
```

The script starts a fresh disposable overlay, or resumes the retained immutable base after verifying its checksum. It refuses a duplicate VM. Generated credentials apply only to that guest. The fixed lab endpoint is loopback `127.0.0.1:22270`; restricted QEMU networking cannot contact physical routers. Independent trust comes from the guest serial-console fingerprint in `console-identity.log`, not the network observation.

The dedicated native executable alone accepts `--openwrt-lab`, with a valid isolated run. Normal production rejects that flag before opening data. The exact VM OS profile stays in memory and does not become physical support in SQLite. The harness drives the native key picker, separate authentication/read approvals, real SSH UCI/ubus inspection, firmware feasibility and actual GUI Cancel/retry. No caller-selected host, private-key path or command can be supplied to simulation IPC.

After all checks, stop only the owned guest:

```powershell
wsl -d Ubuntu -- python3 /mnt/c/Users/chuka/OneDrive/Documents/Firm_Find/scripts/openwrt-test-vm.py stop
```

Shutdown checks the recorded process command line, exact owned disk and serial socket. VM images, failed logs and verification evidence are retained. No normal application data is cleaned.
