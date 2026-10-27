# Firmware filesystem extraction

Current source includes unreleased review corrections: maintained bubblewrap qualification, combined temporary-space accounting, export-name preflight and a narrow TRX v1 adapter. Published Beta 2 binaries retain their original source and runtime pins; they have not been retagged or replaced. See [correction evidence and review status](BETA_2_REVIEW_CORRECTIONS.md).

Library → select firmware → Analyze Firmware → Extract Contents unpacks a supported filesystem into a validated, persistent file index. Browse folders, click a small text file to inspect it, then explicitly select files for TAR export. Selection starts empty. Export requires reviewing the selected files for passwords, keys and personal information.

This unpacks an existing image. It does not dump a connected device, authorize rebuilding, or make an image installable. The separate DNS build helper remains separately qualified. No extracted program or script is executed.

## Supported boundary

- SquashFS 4.0 little-endian, supported decompressor IDs 1–6, with exactly one validated candidate in the first 32 MiB. The candidate offset is discovered, never assumed from a model name.
- One bounded gzip wrapper followed by the same SquashFS backend. This is not an OEM package parser. No automatic recursive extraction.
- TRX v1 with flags zero and a direct SquashFS final partition: bounded header/partition/CRC validation plus the pinned OpenWrt `otrx check` parser inside the namespace. Kernel, loader, padding and other partitions remain unextracted. UBI/UBIFS-containing TRX, v2, unknown flags and gzip-wrapped TRX are rejected. This new adapter is submitted for independent review; it is not present in published Beta 2.
- Maximum input 512 MiB; gzip expansion 128 MiB; ordinary extracted bytes 128 MiB; individual file 32 MiB; 20,000 entries; depth 32; manifest 8 MiB. Unsupported/corrupt/ambiguous structures fail closed.
- Filesystem header size, version, block size/log, inode count and table offsets must fit declared filesystem boundaries. The decoder must also finish successfully. Rust then checks every ordinary file hash, byte count, path, parent directory, duplicate name and complete bundle length.
- Links stay inert manifest records. Special files are excluded before extraction. No link or special file is recreated on Windows or in an export. Permissions, ownership and xattrs are not preserved. Other image regions are listed as unextracted; a filesystem inside a larger image is a partial image extraction.
- FIT, standalone U-Boot components, TAR, UBI/UBIFS, JFFS2, LZMA/XZ/zstd wrappers and proprietary/encrypted containers have no backend. A U-Boot image can contain a supported SquashFS region without granting support for its other regions.

## Qualified worker environment

Windows app requires WSL distribution **Ubuntu**, qualified here on **Ubuntu 26.04 LTS x86-64**, ordinary non-root user, Python 3.14, and these Ubuntu packages:

| Package | Qualified version | Executable SHA-256 |
|---|---|---|
| squashfs-tools | `1:4.7.5-1` | `/usr/bin/unsquashfs`: `003a7e9d70fbdcd05dfe132101c4b7bbb19a217976777f5393573eff21ddc202` |
| bubblewrap | `0.11.1-1ubuntu0.3` | `/usr/bin/bwrap`: `523da3e7399044be5163aee6f57a77a6bef7454376e28f0a0627920bae1b76b6` |

Use the distribution's signed package repositories. The app checks these exact installed binary hashes before launching a parser. It does not install packages, fetch tools, or fall back to direct host extraction. Different/missing tools produce `WORKER_UNAVAILABLE`; additional platforms need independent qualification and an explicit pin update.

Do not downgrade security-patched tools. Ubuntu's signed updated package was downloaded and its archive hash checked against the authenticated package index (`d3a6c1b6b0e0474eaed6dbd055c0d601fd875af153976a0dc4aae75f4ea10868`). Installed binary identity, package verification and namespace/filesystem/cancellation regressions are recorded in the correction report. Executables must additionally be regular, root-owned, executable, non-setuid/non-setgid, not writable by group/others, and have no file capabilities. Unknown hashes fail closed. A future distribution update needs a deliberate qualification/pin update.

```sh
sudo apt-get update
sudo apt-get install python3 squashfs-tools=1:4.7.5-1 bubblewrap=0.11.1-1ubuntu0.3
sha256sum /usr/bin/unsquashfs /usr/bin/bwrap
```

Upstream tools are not bundled into the Windows installer. Squashfs-tools is GPL-2.0-or-later; bubblewrap is LGPL-2.0-or-later. Source and documentation: [Squashfs-tools 4.7.5](https://github.com/plougher/squashfs-tools/tree/master/Documentation/4.7.5), [bubblewrap](https://github.com/containers/bubblewrap). Ubuntu package provenance is independent of the source firmware's OpenWrt release signature.

Qualified shared-library packages, recorded with `dpkg-query -W` and `ldd` on the trusted installed tools: `libc6 2.43-2ubuntu2`, `liblzma5 5.8.3-1`, `libzstd1 1.5.7+dfsg-3`, `liblz4-1 1.10.0-8`, `liblzo2-2 2.10-3build2`, `zlib1g 1:1.3.dfsg+really1.3.1-1ubuntu3`, `libxxhash0 0.8.3-2build1`, `libselinux1 3.9-4build1`, `libcap2 1:2.75-10ubuntu2`, `libpcre2-8-0 10.46-1build1`. The two executable hashes are enforced; these distribution library versions are qualification evidence, not a claim of a fully reproducible container. Package copyright files independently confirmed the two tool licenses.

## Isolation and supervision

Rust holds the original vault read lease, verifies hash/size, makes a scoped working copy, and launches the embedded Python supervisor through structured WSL arguments. The supervisor verifies tools; only the inner namespace process parses firmware.

Bubblewrap creates separate user/PID/network/IPC/UTS namespaces, disables further user namespaces, drops all capabilities, clears the environment, exposes only read-only system tools, read-only input and worker code, and mounts a single 256 MiB writable tmpfs. No home directory or Windows drives enter this namespace. The inner worker checks effective capabilities, absence of host paths and absence of network interfaces other than loopback.

Resource limits: 768 MiB address space, 90 CPU seconds per process, 64 processes, 64 descriptors, bounded file size, one decompressor processor, 16 MiB data/fragment queues. Decoder deadline 100 seconds; supervisor deadline 150 seconds; host deadline 165 seconds. The Linux supervisor checks cancellation and a host heartbeat. On cancellation, timeout, lost heartbeat or startup failure, its finally block kills/reaps its own process group; the PID namespace and tmpfs disappear. Rust also owns a scoped child guard. No arbitrary PID from firmware or output is killed.

The namespace root is explicitly remounted read-only; `/work` remains the bounded writable file store. A combined preflight accounts for retained gzip expansion, ordinary tree bytes, 4 KiB per entry for allocation rounding/directory overhead, and 32 MiB reserved for listing/decoder overhead. Both the 256 MiB budget and actual remaining tmpfs space must fit before extraction. Listing output is capped at 8 MiB; unnecessary decoder logs are discarded. Resource exhaustion has its own error code and cannot publish success. Near-limit tests exercise 120 MiB staged expansion plus 96 MiB tree success, and reject 120 MiB expansion plus 128 MiB tree before any output bundle.

The decoder first lists entries inside the namespace. An ambiguous listing, unsafe name, oversized declared output or excess entry count is refused. It excludes links/special entries and extracts ordinary files with strict errors. The output tree is walked without following links, hashed, then serialized as `OBTX1` (bounded JSON plus ordinary bytes). Rust never recursively unpacks an untrusted archive onto Windows.

## Storage, restart and export

No schema migration. Schema 6 `artifact_blobs` stores the immutable output bundle and JSON manifest. Only the JSON manifest gets a `metadata_report` occurrence, linked to the parent firmware hash and extraction job. The binary bundle has no firmware occurrence or firmware admission. Its hash is referenced by the manifest. Neither output enters firmware cards or becomes an official image.

Publication verifies both content addresses before a single SQLite transaction publishes the relationship and completes the job. Failed/interrupted jobs have no successful extraction record. Unrecognized leftovers are retained for review, never recursively deleted. Repeated pending/running requests for the same source reuse the existing job. Intentional completed reruns retain distinct audit occurrences. Library relocation copies derived blobs and manifests too.

Browse/preview reopens the retained manifest and validates the bundle's content address; text preview additionally checks the selected file hash, UTF-8, size and control characters. React renders text as text, never HTML. Export writes only explicitly selected ordinary files to a new TAR, with relative validated names and mode 0600; it never follows links or overwrites a destination. Original bundle and manifest stay retained.

Export eligibility is checked before opening Save and again before writing. Portable USTAR names use ASCII, a name up to 100 bytes and optional prefix up to 155 bytes; the validator searches every valid separator rather than assuming the final separator works. Unsupported Unicode/overlong filenames remain preserved in the browse index with an individual export explanation and disabled selection. Names are never silently normalized or renamed. Normalization-equivalent collisions and bidi/format controls are refused by the worker; escaping links and special files remain metadata only. Existing immutable reports gain derived eligibility on read without rewriting their stored bytes.

## Optional TRX validator

The new route requires the separately qualified OpenWrt parser at `/opt/obtainer-tools/otrx`. Its source commit, source hash, GPL license and reproducible build recipe are in [the vendored parser notice](../workers/extraction/vendor/README.md). On the qualified Ubuntu/GCC environment:

```sh
sudo sh scripts/build-trx-parser.sh /opt/obtainer-tools/otrx
```

Expected executable SHA-256: `66b870899eca6a12079668c23cdd95db069978afc9af5bcbc3681cd9c1e968e5`. Only the correctly qualified executable is mounted read-only at `/otrx`. The only reachable invocation is literal `otrx check` on the read-only input; create/extract subcommands and caller-supplied arguments are not exposed. Missing/incompatible optional tools disable this route, while basic SquashFS remains available. TRX CRC is integrity metadata, not proof of authenticity, device compatibility or permission to install.

## Physical acquisition status

MG11 user-reported software remains `MF69_AM802L_ZX_DD_N_241106_EU`; this extraction feature adds no authenticated identity evidence. `acquisition/strategies.rs::AcquisitionStrategyRegistry::can_obtain()` remains false for physical acquisition. The documented-export and storage-read strategies have no qualified MG11 implementation. No legitimate MG11 firmware-export endpoint or raw partition-read interface has been demonstrated; physical authentication-challenge qualification remains pending. No credentials, login, privileged read, configuration write, reset or flashing occurs in this workstream. Official OpenWrt bytes and their extracted files are not MG11 firmware.

Public-source check, 27 October 2026: Movingcomm's [MG11 product page](https://www.movingcomm.com/products/mg11-4g-wifi-6-mifi.html) lists ASR1803 and offers a product-specification PDF in Data Download. That page provides no firmware image or documented installed-firmware export procedure. Its marketing specifications do not verify the owner's particular unit or revision. Searches for the exact user-reported software identifier did not establish an official image. This is a bounded research result, not proof that no vendor-supported route exists. The next physical prerequisite remains reviewed identity/read-interface evidence or an exact vendor-supplied image; no login or undocumented request was attempted.
