# Extraction engine V2

28 October 2026. Branch `feature/extraction-engine-v2`, based on feature-line
`eb9f2b5a82d4633b485de1ec55fee20fe86019f7`. Main and published Beta tags are unchanged.
This implementation has not received independent reviewer acceptance.

## Scope and result contract

Only firmware **files already admitted to the Library** enter this pipeline.
No device connection, authentication, credentials, UART/DFU/storage access,
maintenance commands, writes or flashing were added. File decoding contributes
zero to physical-acquisition and device-compatibility metrics. Imported updaters,
scripts and binaries remain data; nothing extracted is executed.

`scripts/extraction-engine-v2.py` contains a closed handler registry. Each handler
provides `detect(View)` (candidate confidence/relative offset), `plan`, `extract`
and `report`. Detection is a hypothesis; validated means syntax/integrity checks,
never authenticated provenance or device support. `View` bounds every source
slice. The worker imports the module only after namespace/resource checks.
Adding a decoder requires fixtures, parser review and, where applicable, an
explicit maintained-tool qualification and hash pin. There is no arbitrary
executable, tool path, command string or plugin supplied by firmware or the GUI.

Every V2 report has source hash, layer hierarchy, byte-coordinate spaces, handler
versions, tool versions/hashes, permissions, ordinary-file sizes/SHA-256, budget
parameters, omitted members and unexplained ranges. Layer coordinates are
relative to their source/member/expanded parent, not interchangeable absolute
flash addresses. `region` remains a compatibility summary; use the layer tree
for coordinates. The legacy `filesystemRoots` field names the virtual browser
root, not proof of filesystem recovery. Implicit parent directories and decoder
prefixes have renderer-default modes; only declared entries carry source modes.
The four completeness values are:

| Verdict | Meaning |
|---|---|
| `complete` | Decoded container/member coverage within the documented reader contract. Not a complete physical flash backup. |
| `partial` | A known layer, member, metadata class, depth or byte range remains omitted, rejected or unknown; reasons are retained. |
| `unsupported` | The recognized variant has no qualified decoder, required tool or safe mapping. |
| `encrypted-or-unknown` | Bytes remain unexplained; entropy alone does not establish encryption. |

`inspection_only` is the job-report status when a partial inspection produces no
ordinary files. Completing an inspection job does not imply successful firmware
extraction. The Library hides export controls when there are no ordinary files.

## Format matrix

“Synthetic” means generated test data, not an OEM image or device evidence.
Genuine packages remain local and uncommitted. Detailed test outcomes and hashes
are recorded in the verification section below.

| Family | Implemented path | Qualification/limits |
|---|---|---|
| SquashFS 4.0 LE | Existing pinned unsquashfs listing, exclusion, shared-space preflight and byte verification | Genuine signed OpenWrt C7 and RT-N16; synthetic kernel/near-limit regressions. Links are excluded before filesystem extraction. Safe relative targets can remain inert metadata; absolute or escaping targets are omitted without exposing the target. Special entries create no nodes. Other endian/versions unqualified. |
| CPIO newc/CRC | Bounded headers, four-byte alignment, trailer, member checksum, portable names and modes; recursive ordinary members | Genuine DWR-930M OTA: 11 files. Synthetic CRC, truncation, traversal, links, privileged modes and root-directory cases. Hardlinks and symlink targets escaping their own archive root are refused; safe relative symlinks remain inert metadata. Special entries create no nodes. Trailer-less archives unqualified. |
| SWUpdate | Closed bounded libconfig subset; records declarations, versions and declared base requirements | Genuine D-Link description and synthetic includes/duplicates. No includes, expression evaluation, signature trust, updater execution or patch application. |
| gzip/xz/LZMA-alone/bzip2 | Streaming standard-library readers; absolute/cumulative and ratio limits; stream-tail ranges | Synthetic recursion/truncation/bomb tests; genuine OpenWrt nested streams. LZMA-alone requires a plausible header; it has no universal carving magic. Concatenated streams are not silently joined. |
| LZ4 frame / Zstandard | Pinned command-line decoders, structured literal arguments, bounded stdout | Synthetic CPIO streams inside production namespace. Legacy LZ4 and unqualified/skippable frame variants are not advertised. |
| UBI v1 | EC/VID CRCs, fixed eraseblock geometry, static data CRCs, unique contiguous volume reconstruction | Synthetic static volume decodes CPIO. Genuine RT-AC68U has incomplete logical mappings: inspection only, **zero files**, no inferred erased bytes or UBIFS dump. No duplicate-LEB arbitration, OOB/ECC correction or wear-level recovery. |
| UBIFS | Signature observation only | No qualified journal/compression/geometry reader; unsupported, not filesystem extraction. |
| ext2/3/4 | Pinned `e2fsck -f -n`, then read-only debugfs inode-number listing/stat/dump | Synthetic filesystems for all three, inside sandbox. No mounts, journal replay or repair. Hardlinks/privileged modes rejected; non-ordinary inodes omitted. Genuine router ext images not tested. |
| CramFS | LE fsid-v2 CRC, pinned read-only fsck, bounded basic/holes block decoder | Synthetic generated filesystem in sandbox. Safe relative links remain inert; unsafe targets are refused relative to this filesystem's root. Big endian, shifted roots, extended block pointers and other variants unsupported. No mounts or fsck extraction mode. |
| JFFS2 | Signature observation only | No qualified node/version/compressor reconstruction. Unsupported; mtd-utils presence is not decoder qualification. |
| YAFFS2 | Unknown/entropy coverage only | Requires independently established NAND page/OOB/ECC geometry and reader. No reliable universal magic or qualified implementation. |
| uImage | Header and data CRC validation, bounded payload, recursive payload and separate trailing image bytes | Synthetic CRC/truncation and genuine C7. Boot addresses ignored; multi-image tables are not reconstructed as trusted partitions. |
| FIT/FDT | Bounded v17 structure and inline image members, SHA-256/CRC32 checks when declared | Synthetic inline FIT/hash-negative tests only. External data references, other versions and signature/boot trust unqualified. Standalone DTB recognition is not a filesystem extraction. |
| Android boot | v0/v3/v4 bounded components; recursively inspect retained component bytes | Synthetic fixtures only. v1/v2 and vendor_boot unqualified; no signature verification or boot execution. |
| Android sparse v1.0 | Bounded RAW/FILL/DONT_CARE/CRC32 chunks and total-size checks | Synthetic chunk/CRC/truncation/budget cases. Don't-care bytes rendered zero are explicitly partial; not claimed as original storage contents. |
| TAR/ZIP | Stream ordinary members to generated flat files; never archive extract APIs | Synthetic path attacks and boundary fixtures. TAR requires a two-block zero terminator; nonzero data after zero padding is an unexplained range. ZIP requires one unambiguous end marker and a bounded comment; bytes after the declared end are unexplained. Ambiguous ZIP end markers, truncated comments and ZIP64 are refused. Sparse, special, linked and privileged entries are rejected; encrypted ZIP members are omitted. |
| RAR5 | Header CRC/length/window/solid preflight; pinned libarchive read API only; bounded member selection | Genuine D-Link archive: 988 ordinary files / 31,744,382 bytes; oversized PAC and other members explicitly omitted. Max 32 MiB/member and 64 MiB selected data. Solid, split/multivolume, unknown-size and unqualified algorithms/windows are unsupported before decoding. No archive_write/extract APIs. |
| Unisoc PAC | Not decoded | Genuine package declares a 34,460,798-byte PAC, above ordinary-member cap. It is omitted rather than expanding the entire declared 318,315,552-byte RAR. No independently qualified PAC parser/layout/checksum contract. No proprietary flashing tool is executed. |
| TRX | Existing qualified OpenWrt otrx `check`, CRC/version/flags/range validation, walk partitions | Genuine RT-N16 and RT-AC68U. Only v1/zero flags; variants unsupported. Other partitions can remain partial. |
| Netgear / TP-Link vendor headers | Generic carving only where a separately validated embedded format exists | Wrapper checksums and models/variants unqualified. Header bytes stay unexplained; no vendor support or complete-container claim. No model-name exceptions. |
| BSDIFF / librsync delta | Identifies delta, records bounded BSDIFF header declarations and SWUpdate requirements | Exact base required. No patch application or fabricated target filesystem. |

Remaining unsupported work is deliberate: UBIFS, JFFS2, YAFFS2, PAC, unqualified
vendor-wrapper checksums/variants, external FIT data, legacy/other boot variants,
encrypted/header-dependent archives and delta reconstruction need separate
fixtures/parser qualification. No format-count claim substitutes for those gates.

## Walking, carving and honesty

The walker descends containers, streams, filesystems and recognized ordinary
archive members. Bounds are shared: depth 6 (a depth-limit result can occupy level
7), 256 layers, 20,000 output entries, 32 MiB ordinary files, 128 MiB retained
ordinary data, 128 MiB cumulative decoded stream expansion, ratio 1024
plus 1 MiB slack. Nested output is under logical `.contents` directories; names
are never silently normalized or renamed.

Carving scans every input byte for registered magic, including adjusted ext and
USTAR header offsets. At most 4,096 candidates are retained and the cap is
reported. It validates candidates before admitting their output; corrupt
speculative parses roll back. It does not guess OOB geometry or discover every
signature-less format. The entropy map samples 64 KiB in each 1 MiB source window
(at most 512 windows); it is explicitly sampled, not a full entropy proof.

Malformed/unsafe top-level input fails without publication. A rejected nested
ordinary archive rolls back all child output, retains only its already-safe outer
file, adds a rejected-layer range/reason and makes the parent partial. No path,
node or executable permission is installed by this process. Resource, cancellation
and global-budget failures remain fatal to the job and publish no new result.
Existing successful results remain intact. Missing required base tools fail
closed; missing optional tools produce unsupported layers with exact reasons.

Link containment is evaluated against each archive or filesystem's own logical
root, including nested layers. CPIO and CramFS reject a layer with an escaping
target; SquashFS excludes links before decoding and records unsafe targets only
as omitted entries, without exposing their target. These links are never created,
followed or exported. TAR and ZIP refuse linked entries. Separately, archive
termination does not explain appended data: the TAR and ZIP readers retain the
unexplained suffix and classify that layer as partial. Ambiguous ZIP end records
and unsupported ZIP64 boundaries are refused rather than guessed.

## Sandbox and immutable publication

Existing isolation is retained: unshared user/PID/network/IPC/UTS namespaces,
disabled nested user namespaces, zero capabilities, read-only root/tools/input,
no Windows drives/home/network, private 256 MiB tmpfs. Host-root ownership,
non-writable/nonprivileged modes, no capabilities and exact hashes are checked
**before UID remapping**. Only that computed tool map enters the cleared
environment. Inner code rechecks hashes/modes/versions; it does not weaken the
ownership check to accept unmapped UID 65534.

Retained stages, output data, 4 KiB entry overhead and 32 MiB decoder reserve
share the existing space budget. CPIO/tar/zip/RAR never create host paths from
member names. ext commands reference numeric inodes and generated destinations;
fsck is read-only. Only generated private workspace paths are rolled back.
Supervisor cancellation, heartbeat, CPU/RAM/process/descriptor limits, deadlines
and kill/reap behavior remain. There is no unsandboxed fallback.

Rust embeds both worker modules, independently validates V2 topology, byte ranges,
provenance, declared bounds, statuses, modes and every ordinary file hash/count,
then atomically publishes immutable metadata/bundle references. V1 immutable
reports remain readable with their original strict path. No database migration or
normal data-directory change occurs. Source firmware is reverified before commit.

V2 uses canonical JSON, deterministic ordering and `createdAt: 0` in the content
manifest; actual collection time stays in the SQLite occurrence/job. Identical
inputs on the same qualified toolchain produce the same bundle/content manifest
hash. Cross-toolchain reproducibility is not claimed. Export still requires
explicit ordinary-file selection and sensitive-information review, rejects unsafe
or unrepresentable USTAR names/overwrite, and writes mode 0600 without links.

## Sources and maintained tools

Format references: [Linux newc/CRC](https://www.kernel.org/doc/html/latest/driver-api/early-userspace/buffer-format.html),
[libconfig grammar](https://hyperrealm.github.io/libconfig/libconfig_manual.html),
[Linux UBI media structures](https://github.com/torvalds/linux/blob/master/drivers/mtd/ubi/ubi-media.h),
[CramFS structures](https://github.com/torvalds/linux/blob/master/include/uapi/linux/cramfs_fs.h),
[U-Boot image header](https://github.com/u-boot/u-boot/blob/master/include/image.h),
[FDT layout](https://github.com/dgibson/dtc/blob/main/libfdt/fdt.h),
[AOSP sparse format](https://github.com/aosp-mirror/platform_system_core/blob/main/libsparse/sparse_format.h),
[AOSP boot format](https://android.googlesource.com/platform/system/tools/mkbootimg/+/refs/heads/main/include/bootimg/bootimg.h),
[RAR5 specification](https://www.rarlab.com/technote.htm),
[libarchive read API](https://github.com/libarchive/libarchive/blob/master/libarchive/archive_read.3),
[e2fsprogs](https://git.kernel.org/pub/scm/fs/ext2/e2fsprogs.git/).
These references describe formats/APIs; the support matrix records what this code actually qualifies.

Tool identities, licenses, installation changes and computed integrity evidence:
[V2 toolchain](../workers/extraction/TOOLCHAIN_V2.md). Existing otrx source/license
remain unchanged in [vendor notice](../workers/extraction/vendor/README.md).

## Verification

### Tests and review

On 28 October 2026, against the V2 branch based on `eb9f2b5a82d4633b485de1ec55fee20fe86019f7`:

| Command | Result |
| --- | --- |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 fmt --all --check` | Exit 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --locked --offline` | Exit 0; 390 passed, 2 ignored, and 2 doctests passed. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 test --workspace --features native-test-profile --locked --offline` | Exit 0; 391 passed, 3 ignored, and 2 doctests passed. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --locked --offline -- -D warnings` | Exit 0. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/rust.ps1 clippy --workspace --all-targets --features native-test-profile --locked --offline -- -D warnings` | Exit 0. |
| `npm run lint`, `npm test`, `npm run build` | Each exit 0; 178 frontend tests passed; build includes TypeScript checking. |
| `wsl -d Ubuntu --cd /mnt/c/Users/chuka/OneDrive/Documents/Firm_Find/.tools/beta2-hardening --exec python3 -B scripts/test-firmware-extraction.py` | Exit 0; 24 tests, no skips. |
| `wsl -d Ubuntu --cd /mnt/c/Users/chuka/OneDrive/Documents/Firm_Find/.tools/beta2-hardening --exec python3 -B scripts/test-extraction-engine-v2.py` | Exit 0; 34 tests, no skips. |
| `py -B scripts/test-firmware-extraction.py`; `py -B scripts/test-extraction-engine-v2.py` | Exit 0; 24 and 34 tests, respectively; Windows skips 6 and 3 Linux-only checks. |

The ignored Rust tests require separately provisioned OpenWrt/QEMU environments,
or are crash fixtures driven by their parent test. The qualified CI job now fails
if the Linux suites skip checks because a pinned tool changed. It does not silently
run outside the namespace.

Red/green evidence is retained in [the committed test logs](evidence/extraction-v2/).
Among the defects reproduced first were unbounded archive trailers falsely marked
complete, an interpreter selected independently of its pinned hash, nested link
targets escaping their archive root, unsafe SquashFS link targets, and expansion
budget refunds after rejected candidates. Corrected tests pass. Earlier tests also
exposed stream padding, false magic, and nested unsafe-archive rollback issues;
those cases retain explicit partial or rejected results. Full command output stays
in the worktree's disposable `.tools/extraction-v2-final-*` logs.

### Real-file sandbox qualification

`wsl -d Ubuntu --cd /mnt/c/Users/chuka/OneDrive/Documents/Firm_Find/.tools/beta2-hardening --exec python3 -B scripts/qualify-extraction-worker.py --input <local-file> --evidence <local-evidence-directory>`
ran once per listed file and twice for the CPIO package; all six invocations exited
0. Each rehashed the source afterward, which remained unchanged. Worker report
hashes below refer to the formatted worker index, not Rust's published Library
manifest; bundle hashes include the worker output framing and contents.

| Input | Source SHA-256 | Result | Bundle SHA-256 |
| --- | --- | --- | --- |
| D-Link SEA OTA CPIO | `a592a30abac46942dc8c8b9fdc4254882f4be1b3fa1261e140d6ca5bddfa5743` | 11 files; partial: delta requires exact base. Repeat index `66b2c2871fe7f79bfb143622e7c90848b9468b744d38d18eab673815328cbef7` and bundle hashes match. | `8e3b77e34c0863f08e3e546efe86f02171844a9a158b9439c09235d224d85b3c` |
| D-Link RAR5 | `4996408e418225b41478bd79ff2e98f3889911b4b75de9ca4abdcd61d81ddcd9` | 988 files; oversized PAC omitted. | `bbf0a7c683735a94e8e2a452ae49af8cf0645971c16ef6c8158cb801bf696d30` |
| OpenWrt Archer C7 | `0ec9643bb9f43d65c689761d916a9e0c13bad5968656d74e516df010deb0be9d` | 869 ordinary files. | `559d94d579f289c0700184bd4b5241451e0b196c4771ad8997fb3663a4b4769d` |
| OpenWrt RT-N16 TRX | `0f29bd64f7a8f931cf1121dd54b514f1ecb58486ffa1713a282c97928c703d5d` | 1,158 ordinary files including recognized nested contents. | `372064b6873d16263cbbde292f6f37529f59216691c432cc52b8fcb5fae6a737` |
| OpenWrt RT-AC68U TRX/UBI | `1827a286bc03df7a83ccd93e84cb1375cc1e10c1857ded3eb9c0ad413f761d36` | Partial inspection; incomplete UBI mapping, zero filesystem files. | `13d6836e578577aeb6b2ccb2d7128cdb813ab3c65191d6bdb4d3084341f7c57a` |

These are existing firmware files, not physical acquisition. They are local and
not committed.

### Native Windows verification

- `npm run tauri -- build --no-bundle --ci -- --locked --offline` and `npm run build:native-tests`: each exit 0.
- `npm run test:extraction-review-native`: initial exit 1 because the synthetic
  test expected the previously exposed unsafe link. It was corrected to require
  `special_omitted` and a null target. Rerun exit 0: actual GUI Cancel/retry,
  869-file C7 extraction, text browsing, selected-file export and restart;
  genuine RT-N16 extraction; UBI inspection with zero files; resource exhaustion
  with no publication; and unsafe-name/link preflight passed. The rendered C7
  partial verdict, layer tree, and unexplained-region controls were visually checked.
- `node scripts/verify-foundation.mjs --manufacturer-package`: initial exit 1
  because the driver searched for a button where the UI uses a disclosure
  summary. After correcting the driver, exit 0: CPIO and RAR import, extraction,
  omissions, hashes and restart passed. The rendered RAR partial verdict and
  omitted PAC were visually checked. Normal application bytes were unchanged by
  both isolated suites. Initial failures are retained in the evidence directory.
- `npm run test:universal-workflow-native`: exit 0 on the final build. A synthetic
  selected target and a genuine official image passed download, immutable import,
  GUI cancel/retry, extraction, browsing and restart. It reports
  `physicalDeviceSuccess: false`.
- `npm run test:openwrt`: three attempts exited 1. The first timed out in CDP; the
  second failed on the removed card-level “Check compatibility” selector. The
  passive adapter test was updated for the current device dialog without making
  device requests. The third reached adapter and simulator checks, then stopped
  because `.tools/phase7-vm/console-identity.log` is missing. The genuine OpenWrt
  VM portion is unverified. Historical saved-connections GUI controls are no
  longer mounted and are explicitly not tested.
- `npm run test:simulator`: exit 1 after simulator checks passed because the
  development hot-reload step did not update the view. This worktree is nested
  under `.tools`, while Vite ignores `**/.tools/**`; this is a plausible cause,
  not a confirmed diagnosis. `node scripts/verify-foundation.mjs --release-only
  --phase5 --phase6` exited 0 and independently exercised the final release's
  adapter and simulator checks, including GUI cancellation and recovery.
- `py -B scripts/audit-data-bytes.py .tools/extraction-v2-normal-after-all.json .tools/extraction-v2-normal-before.json`:
  exit 0; 12 normal-data files, 5,611,701 bytes, byte-identical to the pre-task
  snapshot. No normal SQLite connection was made for this audit.

Published Library manifest SHA-256 values verified through native IPC and vault
bytes: C7 `67d8d8f2272ca80dc93aa6d4ad51b3b992349d893a89f9c37ef89677688ada1e`,
RT-N16 `82d413bca3d0f0df316481e6ef41d3185f8561ea5210c4af565bb6ecd198e67c`,
CPIO `96aaff4af492354479132201d85886efc3b85e0691c62dd246306e3d41c68e83`,
and RAR `ef661bae1cc6612700e333c526c98b47422603e354e9ee6b94c4a675ad66261c`.
These differ from the standalone worker index hashes above by design. Native
evidence and screenshots remain under the worktree's `.tools/foundation/`
directories; screenshots are not committed.

Local success does not imply independent acceptance, Phase 7.5 completion, or
completion of the project.
