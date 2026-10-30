# Recursive firmware structure milestone

29 October 2026. Starting source `36fc81b36ca036865e5bd06f8baedf4fe648c756`;
branch `feature/web-management-recursive-milestone`. This continues the existing
feature implementation; it does not replace main, published Beta binaries or
immutable Library manifests. Independent acceptance is pending.

## Implementation retained and extended

`scripts/extraction-engine-v2.py` already supplies closed `Handler` implementations
with detect/plan/extract/report, bounded views, all-offset carving, recursive
member/stream/filesystem walking, entropy samples and explicit unknown coverage.
`scripts/firmware-extract-worker.py` supplies the no-network qualified Linux
namespace and tool verification; `library/extraction.rs` independently validates
and publishes immutable output. Decoder implementations and isolation budgets
are retained; the interpreter security pin was separately updated and qualified
as documented below. [Detailed format/sandbox matrix](EXTRACTION_ENGINE_V2.md).

New `library/extraction/structure.rs::derive` projects those immutable reports into
typed regions. `LibraryService::firmware_structure` and `get_firmware_structure`
expose it to the lazy **Artifact Structure** view inside Library/device extraction.
It keeps parent/children, input-view hash, relative bounds, handler/version,
validation states, unknown intervals, entropy observations, package version
declarations and tool provenance. The SHA-256 of an unknown subrange or legacy
subset is null when no such hash was collected; no hash is invented.

Bounds describe the recorded parser input view, which may include unrecognized
trailing bytes. They do not assert an exact physical flash partition. Absolute
source positions exist only through contiguous source-coordinate ancestry.
Expanded streams and extracted members establish a new coordinate space; neither
they nor their descendants receive fabricated physical offsets. Unknown ranges
remain explicit children of their correct coordinate-space parent.

Filesystem candidates all receive an explicitly unverified rootfs heuristic.
Boot containers receive only a kernel-candidate heuristic. The first SquashFS is
never selected as authoritative rootfs. SWUpdate versions remain unsigned package
declarations. EXTRACTED is independent of MODIFIABLE, REPACKABLE,
SIGNATURE_COMPATIBLE and INSTALLABLE; the latter four stay unknown/nonexecutable.
No packer, signature bypass, installation or physical storage reader is added.

## Parser/tool evaluation

[Binwalk v3](https://github.com/ReFirmLabs/binwalk/blob/master/README.md) supports
Rust integration and entropy-based firmware inspection. This milestone retains
the repository's already bounded equivalent and closed qualified handler registry
instead of adding an unqualified automatic-extractor dispatch chain.

[ubi_reader output code](https://github.com/onekey-sec/ubi_reader/blob/main/ubireader/ubifs/output.py)
can create links, special nodes and restored metadata. Integrating it requires a
separately qualified read-only, flat-output adapter, exact pins, budget enforcement
and complete genuine fixtures; merely invoking its extraction CLI would not
satisfy OBTAINER's output contract. UBIFS remains unsupported. The genuine UBI
fixture lacks a complete safe logical-block mapping; no blocks are inferred.

[U-Boot's FIT documentation](https://docs.u-boot.org/en/latest/usage/fit/index.html)
describes image packaging independently of filesystem or boot authorization.
The existing bounded inline FIT/uImage parsers remain the selected adapters here.
No `dumpimage` executable is added or claimed qualified. TRX retains the pinned
OpenWrt firmware-utils parser and its existing reviewed checksum/range contract.

## Actual format boundary

| Structure | Actual available qualification |
|---|---|
| SquashFS; supported gzip-wrapped paths | Genuine OpenWrt C7 and RT-N16 files, existing malicious/resource/cancel/restart regressions. |
| CPIO newc/CRC and SWUpdate declarations | Genuine DWR-930M SEA delta: 11 retained members. Exact base remains required; no reconstructed installed firmware. |
| Selected standard compression, TAR/ZIP, inline FIT/uImage, ext2/3/4, CramFS, Android containers | Existing bounded synthetic fixtures; no new blanket OEM/device qualification. |
| RAR5 | Genuine D-Link package, bounded 988 retained members; oversized PAC/other members omitted explicitly. |
| UBI | Synthetic static-volume reconstruction; genuine incomplete mapping stays partial inspection. |
| UBIFS, JFFS2, YAFFS2, PAC and unqualified vendor variants | Unsupported. Signatures or wrappers never imply successful extraction. |

Real package paths remain local/uncommitted. Their source hashes and limitations
are retained in [V2 evidence](EXTRACTION_ENGINE_V2.md); this milestone reruns the
actual native file workflows and records new graph evidence separately. Public
packages are not independently matched to H04's exact physical revision.

## Verification

The graph projection began with a failing stub: four behavioral failures and one
negative passing test, after correcting an incomplete synthetic fixture. Five
backend graph tests then passed. Three missing-UI tests failed before the lazy
tree/readiness component was added. The final milestone report records complete
suite results and actual native results; no physical acquisition follows from
these file tests.

Windows worker checks: 24 legacy tests (six Linux-only skips), 34 V2 tests (three
Linux-only skips), exit 0. WSL Ubuntu ran all 24 and all 34 with no skips, exit 0.
Full/native verification results are recorded below.

## Real-file and native results — 30 October 2026

Runtime `6096b644f8466d62697492f897b8230e3fdc457f` has
[green CI](https://github.com/okekechuks/Firm_Find/actions/runs/40940887215),
438 standard / 439 isolated Rust tests plus two doctests per profile, 196 frontend
tests, clean formatting/both Clippy profiles/lint/TypeScript and both Windows
builds. Ignored tests and exact commands are listed in
[web-management verification](WEB_MANAGEMENT_ACQUISITION_VERIFICATION.md).

All five local genuine fixtures were rerun using
`wsl.exe -d Ubuntu --cd <worktree> --exec python3 -B scripts/qualify-extraction-worker.py --input <fixture> --evidence <owned-output>`
on the patched qualified interpreter. Each exited 0, independently checked all
retained file hashes and confirmed the input hash unchanged. These are real
firmware **files**, not firmware obtained from a connected device.

| Fixture | Source SHA-256 | Actual result |
|---|---|---|
| DWR-930M SEA OTA, CPIO CRC despite `.zip` suffix | `a592a30abac46942dc8c8b9fdc4254882f4be1b3fa1261e140d6ca5bddfa5743` | 11 files / 10,733,303 bytes; partial, exact delta base required |
| DWR-930M ME RAR5 | `4996408e418225b41478bd79ff2e98f3889911b4b75de9ca4abdcd61d81ddcd9` | 988 files / 31,744,382 bytes; oversized PAC/AXF/MAP members omitted, not decoded |
| OpenWrt 25.12.6 Archer C7 v5 | `0ec9643bb9f43d65c689761d916a9e0c13bad5968656d74e516df010deb0be9d` | 869 files / 13,411,262 bytes; qualified SquashFS path retained |
| OpenWrt RT-N16 TRX | `0f29bd64f7a8f931cf1121dd54b514f1ecb58486ffa1713a282c97928c703d5d` | 1,158 ordinary files / 12,489,630 bytes including nested layers; direct SquashFS subset remains 851 |
| OpenWrt RT-AC68U TRX/UBI | `1827a286bc03df7a83ccd93e84cb1375cc1e10c1857ded3eb9c0ad413f761d36` | zero files; incomplete logical-block mapping, partial inspection only |

The actual native CPIO/RAR suite exited 0, then reopened both immutable reports
and graphs. The selected-device software workflow exited 0 with a real official
C7 download, native GUI Cancel/retry, extraction/browsing, every ordinary file hash
and restart. Source, bundle, graph and manifest comparisons survived reopening.
No extracted program was executed. Rebuild/install remain unknown/nonexecutable.

`npm.cmd run test:extraction-review-native` also exited 0 on the same Windows
executables and patched WSL tools. It verified genuine raw/C7 and locally wrapped
gzip equivalence, native Cancel/retry and selected-file TAR export, the genuine
RT-N16 TRX, zero-file partial UBI inspection, synthetic resource exhaustion with
no publication/prior results retained, and long/Unicode export rejection before
the Save dialog. Links and special entries remained inert. Its native evidence
directory is `.tools/foundation/2026-10-30T08-19-40.955Z`.

| Native immutable report | Manifest SHA-256 | Structural graph SHA-256 before and after restart |
|---|---|---|
| CPIO CRC | `72f2ff185cbae00e6bbdac23c7583a467229ef9fac38902854b5e708a1fc9fcf` | `f96b121743327e49177d5da3f4e553506d2bc10c225f9fb5aa701819d8f27f52` |
| RAR5 | `4ff9797bd585f03f1a9094e059340f9ba1c27ae14eef6abd625ea704666f0bc1` | `cd186cbc3180d45c310cf102ce14701b60033a8103803e8faa89ccbfc07f501c` |
| C7 selected-device workflow | `4fab7ba05c69ef857c12cf7d40fbbdba4b770cddbffc541cea39eb9890f51f0e` | `1fe90479c2bffacd1e9367d144e05cfa8254a4d615246332aafda0dbd492ec82` |

Graph hashes cover the retained pretty JSON plus terminal newline; manifest
hashes cover the actual immutable Library bytes. They are separate from the
standalone worker's pretty-report hash. C7 bundle SHA-256 is
`fff0593b182e14bbb6a2279d298b1b789c7a54b6b0dadfc3a696477134bebbee`;
CPIO bundle is `e664c44d3531dcc3b4c7374eee4e87e5635347dd86a5fe01da30245ad77c8f2f`;
RAR bundle is `7f60d54dfa2da100ce0a12dec8c038f3a48834536df2ea8d5364d00318fac661`.

Additional native immutable manifests: RT-N16 TRX
`d5f81dcdecf64d1440539bfc0d63d72668b3574f36b304239f06a0fabc4f7a4b`;
partial UBI inspection
`43378ecb9e55f40381733171b6224646ef60525f5f287df47603ea7cde79123f`;
owner-created gzip wrapper
`7e8d39e8316a2430c15507c9ee24e6be938ee6bbe45d39e6c38b900a8615b912`.
The wrapper is a local test transformation, not a separately acquired OEM image.

Initial native graph failure was a test-driver publication race. Waiting for the
actual React action fixed the driver without changing the application. Final
screenshots were viewed and show relative coordinates, explicit unknown/delta
regions and separate readiness. No source offsets are invented for expanded
streams or extracted members. Normal data remains 12 byte-identical files.

Text red/green/build/native logs are retained under
`docs/evidence/web-management-milestone/`, with computed original and normalized
hashes in `index.json`. Raw firmware, captures, screenshots and normal-data byte
maps remain local/uncommitted. Independent acceptance remains pending.

## Maintained interpreter qualification

Initial [CI 40937767308](https://github.com/okekechuks/Firm_Find/actions/runs/40937767308)
correctly failed closed: clean Ubuntu installed Python security package
`3.14.4-1ubuntu0.2`, while the prior local V2 environment/pin used `3.14.4-1`.
[Ubuntu USN-8744-1](https://ubuntu.com/security/notices/USN-8744-1) identifies the
patched package. No downgrade or direct-host extraction fallback was introduced.

APT refreshed its signed indices. Downloaded `python3.14-minimal` archive:
2,579,044 bytes, SHA-256
`0a7ea8a52834f9b3de0f11758924f80f7a6ebcbb4b02a55ba4f457503569f55f`.
`scripts/qualify-python-runtime.py` verifies that checksum against APT metadata,
reads the interpreter member without extracting/executing it and compares it
with the installed root-owned executable. Both now match
`52e0a13e60a981d8c4b6478be2ba5176f69da07948a056bf49cf6f077e30cb41`.
The requested runtime package update also upgraded its already-installed matching
development/venv components; no blanket system upgrade was performed.

Before changing pins, local Linux regressions reproduced the same two
`worker_unavailable` failures. After matching worker and independent Rust pins,
all 24 legacy, 34 V2 and 13 DNS tests passed with no skips on the patched runtime.
Namespace, read-only root, cancellation/reaping, limits, paths and real synthetic
filesystem/compression checks remain enabled. CI also prints installed tool
versions/hashes and still rejects skipped qualified tests. New jobs refuse the
old interpreter; existing immutable reports are retained without rewriting.

Toolchain changes can change provenance-bound manifest hashes. Determinism is
checked within the same input/qualified toolchain; earlier manifests remain
historical evidence rather than being silently replaced.
