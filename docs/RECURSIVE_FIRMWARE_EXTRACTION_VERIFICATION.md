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
and publishes immutable output. These decoder/worker files are unchanged by this
milestone. [Detailed format and sandbox matrix](EXTRACTION_ENGINE_V2.md).

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
Full/native verification results will be appended after execution.
