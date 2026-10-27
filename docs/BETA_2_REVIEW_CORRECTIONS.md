# Beta 2 independent-review corrections

27 October 2026. Correction branches: `fix/beta2-review-hardening` (P0 integration) and `fix/beta2-extraction-hardening` (P1–P6 work). No published tag was moved. Phase 7.5 and physical firmware dumping remain incomplete.

## P0 — corrected and integrated

Severity: **P0/high, repository build failure**. Commit `48ca4f0177c2489e87c2d9e852d54aae6204c577` added a literal `+` before the Rust module declarations. The failing-first command, `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all --check`, exited 1 with `expected item, found '+'` at `adapters/mod.rs:1:1`.

Commit `653da84` removes that line only. Merge `d5d05413d30c01f189968526be2ac80979387bd5` preserves main's history and Beta 2's `fb490ec` history. The single README conflict was resolved by retaining current Beta 2 functionality and Beta 1's published fallback link. Main advanced by fast-forward, without reset/rebase/force-push. Local formatting, both Clippy profiles, 366 standard/367 isolated Rust tests and two doctests per profile passed. [Main CI 40524198759](https://github.com/okekechuks/Firm_Find/actions/runs/40524198759) passed both jobs and both Windows builds on that exact merge. Main is source/build healthy; later runtime corrections are reviewed separately.

## P1 — maintained dependency qualification

Severity: **high maintenance/security boundary**. The old exact pin rejected patched bubblewrap and tied extraction availability to an outdated binary. Ubuntu [USN-8288-1](https://ubuntu.com/security/notices/USN-8288-1) documents the setuid sandbox-setup issue. The installed old binary was not setuid; that does not justify requiring an old package.

`apt-get update` completed using authenticated Ubuntu repository metadata. Only bubblewrap was upgraded to `0.11.1-1ubuntu0.3`; no downgrade or broad system upgrade was performed. Downloaded package SHA-256 matches the signed index: `d3a6c1b6b0e0474eaed6dbd055c0d601fd875af153976a0dc4aae75f4ea10868`. Installed `/usr/bin/bwrap` SHA-256 is `523da3e7399044be5163aee6f57a77a6bef7454376e28f0a0627920bae1b76b6`, mode 0755, root owned; `dpkg --verify bubblewrap` returned no discrepancies. Package identity and dependency output is local in `.tools/bubblewrap-identity.log` and `.tools/bubblewrap-upgrade.log`; a first stat-inspection command had a quoting error and was rerun directly.

The worker retains exact hash checks and now refuses privileged, writable or linked tool files and file capabilities. No unisolated fallback exists. Persisted job failures identify missing/incompatible WSL tools and give the qualified package versions. New frontend regressions cover the actionable error state. Squashfs-tools 4.7.5 stays qualified; optional TRX parser qualification is separate below. Namespace, near-limit filesystem, cancellation/lost-owner/timeout and genuine-image checks are being completed on final source; no final native result is inferred from unit tests.

## P2 — combined resource budget

Severity: **medium, extraction failure/availability and isolation accounting**. Separate 128 MiB expansion and 128 MiB tree caps could fill a 256 MiB tmpfs without space for allocation rounding, directories or decoder scratch. A failing-first regression demonstrated the missing combined budget. The corrected preflight reserves 32 MiB plus one 4 KiB page per entry and checks current tmpfs availability. Listing output is separately capped; unneeded decoder logs no longer consume tmpfs. ENOSPC/ENOMEM/EDQUOT/EFBIG/process-cap and resource-signal failures have resource-specific classification rather than corrupt-firmware classification.

A namespace test also showed the root mount lacked a read-only flag. It is now explicitly remounted read-only, verified by the inner worker and a kernel integration test; `/work` stays writable. Near-limit actual Linux tests allow 120 MiB staged expansion plus a 96 MiB ordinary tree and reject 120 MiB expansion plus a 128 MiB tree, with zero output bundle and unchanged source. Native no-publication/preservation checks are pending final execution.

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

## Final verification in progress

The focused before/after test logs and final native/CI hashes will be recorded after execution. At this checkpoint, P0 main CI is green; 169 frontend tests and 23 Linux worker tests pass. Six Linux-only tests are explicitly skipped on Windows. Full Rust profiles, Clippy, both Windows builds, native restart/export/resource/TRX checks and final byte-preservation audit are pending final-source completion. The normal-data baseline is `.tools/beta2-review/.tools/normal-before.json`: 12 files, 5,611,701 bytes. Normal SQLite is never opened by the audit.
