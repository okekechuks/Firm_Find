# Beta 2 independent-review corrections

27 October 2026. Correction branches: `fix/beta2-review-hardening` (P0 integration) and `fix/beta2-extraction-hardening` (P1–P6 work). No published tag was moved. Phase 7.5 and physical firmware dumping remain incomplete.

## P0 — corrected and integrated

Severity: **P0/high, repository build failure**. Commit `48ca4f0177c2489e87c2d9e852d54aae6204c577` added a literal `+` before the Rust module declarations. The failing-first command, `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all --check`, exited 1 with `expected item, found '+'` at `adapters/mod.rs:1:1`.

Commit `653da84` removes that line only. Merge `d5d05413d30c01f189968526be2ac80979387bd5` preserves main's history and Beta 2's `fb490ec` history. The single README conflict was resolved by retaining current Beta 2 functionality and Beta 1's published fallback link. Main advanced by fast-forward, without reset/rebase/force-push. Local formatting, both Clippy profiles, 366 standard/367 isolated Rust tests and two doctests per profile passed. [Main CI 40524198759](https://github.com/okekechuks/Firm_Find/actions/runs/40524198759) passed both jobs and both Windows builds on that exact merge. Main is source/build healthy; later runtime corrections are reviewed separately.

## P1 — maintained dependency qualification

Severity: **high maintenance/security boundary**. The old exact pin rejected patched bubblewrap and tied extraction availability to an outdated binary. Ubuntu [USN-8288-1](https://ubuntu.com/security/notices/USN-8288-1) documents the setuid sandbox-setup issue. The installed old binary was not setuid; that does not justify requiring an old package.

`apt-get update` completed using authenticated Ubuntu repository metadata. Only bubblewrap was upgraded to `0.11.1-1ubuntu0.3`; no downgrade or broad system upgrade was performed. Downloaded package SHA-256 matches the signed index: `d3a6c1b6b0e0474eaed6dbd055c0d601fd875af153976a0dc4aae75f4ea10868`. Installed `/usr/bin/bwrap` SHA-256 is `523da3e7399044be5163aee6f57a77a6bef7454376e28f0a0627920bae1b76b6`, mode 0755, root owned; `dpkg --verify bubblewrap` returned no discrepancies. Package identity and dependency output is local in `.tools/bubblewrap-identity.log` and `.tools/bubblewrap-upgrade.log`; a first stat-inspection command had a quoting error and was rerun directly.

The worker retains exact hash checks and now refuses privileged, writable or linked tool files and file capabilities. No unisolated fallback exists. Persisted job failures identify missing/incompatible WSL tools and give the qualified package versions. New frontend regressions cover the actionable error state. Squashfs-tools 4.7.5 stays qualified; optional TRX parser qualification is separate below. Namespace, near-limit filesystem, cancellation/lost-owner/timeout and genuine-image checks subsequently passed as recorded in the close-out below; no native result is inferred from unit tests.

## P2 — combined resource budget

Severity: **medium, extraction failure/availability and isolation accounting**. Separate 128 MiB expansion and 128 MiB tree caps could fill a 256 MiB tmpfs without space for allocation rounding, directories or decoder scratch. A failing-first regression demonstrated the missing combined budget. The corrected preflight reserves 32 MiB plus one 4 KiB page per entry and checks current tmpfs availability. Listing output is separately capped; unneeded decoder logs no longer consume tmpfs. ENOSPC/ENOMEM/EDQUOT/EFBIG/process-cap and resource-signal failures have resource-specific classification rather than corrupt-firmware classification.

A namespace test also showed the root mount lacked a read-only flag. It is now explicitly remounted read-only, verified by the inner worker and a kernel integration test; `/work` stays writable. Near-limit actual Linux tests allow 120 MiB staged expansion plus a 96 MiB ordinary tree and reject 120 MiB expansion plus a 128 MiB tree, with zero output bundle and unchanged source. Native no-publication/preservation checks subsequently passed; see the executed close-out below.

## P3 — export preflight and filenames

Severity: **medium export failure; low filename ambiguity/spoofing**. USTAR eligibility was checked only after Save opened, and only the last separator was considered. Failing-first Rust tests showed a valid earlier split rejected and Unicode names accepted without a portable encoding policy.

One shared validator now runs before Save, during selection resolution and while writing headers. It reports the individual ineligible filename and supports every eligible prefix/name split. Export uses a deliberate ASCII USTAR policy; NFC/NFD filenames stay preserved but are not exported under silently changed names. The worker refuses normalized-equivalent collisions and bidi/format controls. The UI marks each unexportable file and disables its checkbox; explicit selection and the sensitive-information review checkbox remain required for normal UI export. Tests retain inert links/special files, destination non-overwrite, internal/unsafe-path refusal and source/Library preservation. Native preflight checks are pending final execution.

## P4 — immutable Beta 2 evidence

Severity: **low audit reproducibility**. The release body pointed to a moving feature branch. Both documentation URLs now point to permanent commit `fb490ec50d81f29404730159a35d8cd95a8b0912`, which contains the actual final qualification report. [Immutable report](https://github.com/okekechuks/Firm_Find/blob/fb490ec50d81f29404730159a35d8cd95a8b0912/docs/FIRMWARE_EXTRACTION_VERIFICATION.md) retains source `e4524e8`, CI 40475441212, source/manifest hashes, exact results and limitations. Before/after release snapshots confirm all five asset IDs, filenames, byte counts and SHA-256 digests are identical. Beta 1 stays at `7433c96b379dd70d1481f06d645ff7f49570d9fe`; Beta 2 stays at `e4524e80ac38ff0702e0917fc9a8f314d2f1ad82`. No binary or checksum manifest was replaced.

## P5 — narrow TRX adapter, submitted for independent review

This is new format coverage, not a corrected claim that all TRX contents are unpacked. The bounded TRX v1 route validates magic, declared length, flags/version, ordered partition offsets and CRC; then invokes unmodified maintained OpenWrt `otrx check` inside the namespace. Only a direct SquashFS final partition is extracted. Source commit `e3853149fb18dfa55f10c10c9aaf99c329747365`, source SHA `e2ef4e7a08f84543945fec83c43bdebfdb1c252da1c440ebb90454913ec2dc3e`, executable SHA `66b870899eca6a12079668c23cdd95db069978afc9af5bcbc3681cd9c1e968e5`; source/license/build recipe are committed. Rust independently checks the structured container evidence and filesystem bounds. Other TRX partitions, v2, unknown flags, UBI/UBIFS and gzip-wrapped TRX stay unextracted. No format observation authorizes a device operation.

Both genuine OpenWrt fixtures were verified against their PGP-signed release checksum manifests using the already pinned release primary/signing fingerprints. `scripts/verify-trx-fixtures.py` records computed receipt values only.

| Genuine fixture | SHA-256 | Actual controlled-worker result |
|---|---|---|
| OpenWrt 25.12.6 RT-N16, 7,213,056 bytes | `0f29bd64f7a8f931cf1121dd54b514f1ecb58486ffa1713a282c97928c703d5d` | 851 files, 124 directories, 12,421,418 bytes; SquashFS offset 2,634,752, length 4,288,508; source unchanged |
| OpenWrt 25.12.6 RT-AC68U, 7,733,248 bytes | `1827a286bc03df7a83ccd93e84cb1375cc1e10c1857ded3eb9c0ad413f761d36` | `unsupported_nested_format`, zero output; source unchanged; nested UBI is not falsely called extracted |

These standalone results precede the final root-mount hardening and do not substitute for final native qualification. Public fixture URLs and exact signed-manifest hashes are retained in `.tools/trx/verified-fixtures.json`. Test images are not added to the app's official-source catalogue or represented as a connected-device dump. The new adapter is untagged source for independent review; no external acceptance is claimed.

## P6 — physical acquisition remains gated

The owner was asked for current MG11 availability, displayed firmware and visible-settings state. No new availability/owner-baseline confirmation has arrived. The supervised challenge run has therefore not been started. The existing helper was connected to a dedicated `--mg11-challenge` native harness path and corrected to open Connected Devices before selecting its card. Hardware requests still require the owner's actual native approvals; no automatic Yes, credentials, login, raw nonce retention, retries, configuration writes or flashing.

MG11 exact installed identity is not independently qualified. Previous user-reported `MF69_AM802L_ZX_DD_N_241106_EU` and observed web/static-family signals remain at their original evidence strengths. [Movingcomm's MG11 page](https://www.movingcomm.com/products/mg11-4g-wifi-6-mifi.html) offers specifications, not a demonstrated exact firmware image or installed-firmware export interface. No reviewed MG11 storage-read method has produced bytes.

Next-device investigation uses the official [OpenWrt NOR backup guidance](https://openwrt.org/docs/guide-user/installation/generic.backup) and [flash-memory/NAND guidance](https://openwrt.org/docs/techref/flash): a separately owner-authorized, identified OpenWrt router with known storage geometry is a candidate for future read qualification. NOR and NAND require different treatment; configuration backup is not firmware. No second physical device has been identified or tested, and no privileged partition command is implemented or executed here.

## Final verification checkpoint (historical)

The earlier checkpoint above is superseded by the executed evidence below. P0 and P1–P5 software/native checks passed; the new TRX route still requires independent source acceptance. P6 is BLOCKED on owner supervision and a qualified firmware-byte source. Neither physical dumping nor Phase 7.5 completion is claimed.

## Executed close-out — 27 October 2026

Core correction `f7f8952` and CI environment corrections `a3a6f92` / `a6898c4d5439557f82e5bb8f5830ee741b2f654b` have fully green [CI 40577625304](https://github.com/okekechuks/Firm_Find/actions/runs/40577625304): 371 standard / 372 isolated Rust tests, two doctests per profile, both Clippy profiles, formatting, 169 frontend tests, lint, TypeScript, installers and the dedicated native executable/hash safeguard. `3d67914` adds the locally verified owned-dialog driver correction; it changes no production Rust/worker code. No branch is merged over main by this close-out.

Initial CI 40574187583 failed because Ubuntu 22.04's old bubblewrap lacked `--disable-userns`. A new modern-tool container on `ubuntu-latest` then failed loopback namespace setup with `RTM_NEWADDR: Operation not permitted`. The same full modern-tool container suite passed on the established Ubuntu 22.04 namespace-capable host. Ubuntu documents [unprivileged-user-namespace restrictions](https://documentation.ubuntu.com/security/security-features/privilege-restriction/apparmor/); the exact host LSM audit cause was not captured. No production sandbox flag or hash safeguard was removed, and no host-wide policy switch was changed. The legacy runner explicitly skips its unsupported primitive; the qualified container executes all 23 extraction tests.

Actual Windows/WSL native regression ran against integrated runtime `7ebd028dca89760b2846206a998ac0b0ae46983d`, which retains the correction's worker/extractor unchanged. It has green [CI 40578734639](https://github.com/okekechuks/Firm_Find/actions/runs/40578734639). Evidence: `.tools/foundation/2026-10-27T20-44-03.953Z`. `npm.cmd run test:extraction-review-native` exited 0 after the owned DirectUI Save driver was corrected to use filename ValuePattern/readback and the actual owned button. The preceding failure at `2026-10-27T20-39-58.580Z` is retained, not relabelled as success. Production no-overwrite/path guards are unchanged.

The native suite verified real window/React/SQLite IPC, six navigation pages, three Windows refreshes (58 unique host records, not verified routers), real settings read/write and restart; genuine source acquisition/hash/provenance; actual GUI Cancel and retry; raw and gzip extraction; browsed text; independently checked every ordinary file; selected-file TAR export; maintained-tool sandbox; genuine TRX; and negative resource/filename/unsupported-format cases. Console issue lists were empty. All operations used isolated data. No MG11 network request, credential, login or physical write occurred.

| Artifact / native result | Actual SHA-256 / result |
|---|---|
| Genuine Archer C7 v5 original | `0ec9643bb9f43d65c689761d916a9e0c13bad5968656d74e516df010deb0be9d`; 7,405,862 bytes unchanged |
| Raw manifest / bundle | `35fe86bfbd3604bd7d63ebeaf746eff3525dcc2781fb350d2a6ad21e51151c1c` / `3e4c62186d7eed588d42edf1c98faf8e0e5c749c8cd436e20a535dd2385aae02`; 869 files, 128 directories, 13,411,262 bytes |
| Local gzip wrapper / manifest | `7b9a3f5e89352ac509b0b0ef459965d5b809ac5d83db3441bba6c057e5c1bf25` / `a232c5170bbaa57b3259bbee105e57a20558e00d1ad5f00e74753f224d019011`; every file matches raw route |
| Genuine RT-N16 TRX manifest / bundle | `a73659c31c24caaa7e14ba44d742bf39e57b140adf1a0e659213b4db390a6361` / `37aae93bc989129b5e6b84192476d6453f28f55bb8e71dbaf9aaebf0b00a98c6`; 851 files, 124 directories, 12,421,418 bytes |
| Genuine RT-AC68U nested UBI | `UNSUPPORTED_NESTED_FORMAT`; no successful report; earlier extraction and source retained |
| Synthetic staged-expansion/tree exhaustion | `EXTRACTION_RESOURCES`; no successful report; source and prior Library output retained |
| Synthetic Unicode/long-name fixture | Per-file `UNEXPORTABLE_FILENAME` before Save; links/specials inert; browse output retained |

`wsl.exe -d Ubuntu --cd 'C:\Users\chuka\OneDrive\Documents\Firm_Find\.tools\beta2-hardening' --exec python3 -B scripts/test-firmware-extraction.py` exited 0: 23/23. `py -B scripts/test-firmware-extraction.py` exited 0: 17 applicable tests plus six explicit Linux skips. `npm.cmd run test:catalogue` exited 0. Existing Linux DNS helper regressions also passed 13/13 after the maintained package update; no new DNS image build or device configuration was performed. Commands and additional integrated-runtime results are in [the workflow report](UNIVERSAL_FIRMWARE_WORKFLOW_VERIFICATION.md).

The original P0 baseline and final byte audit remain identical: 12 normal files / 5,611,701 bytes, including SQLite/settings/vault/logs. The audit never opens normal SQLite. Successful test runs clean only owned profiles; failed-run evidence is retained. Beta 1/Beta 2 tag SHAs and all five published Beta 2 asset IDs/digests remain unchanged. The permanent release report link still resolves at `fb490ec50d81f29404730159a35d8cd95a8b0912`.

Remaining limits: narrow format coverage; other TRX regions/UBI/UBIFS remain unextracted; exact qualified tool environment required; new TRX source awaiting independent acceptance; Windows physical disk exhaustion was not forced; P6's physical challenge and firmware-byte source remain pending. Working image extraction is not physical device firmware dumping.
