# Firmware filesystem extraction

Library → select firmware → Analyze Firmware → Extract Contents unpacks a supported filesystem into a validated, persistent file index. Browse folders, click a small text file to inspect it, then explicitly select files for TAR export. Selection starts empty. Export requires reviewing the selected files for passwords, keys and personal information.

This unpacks an existing image. It does not dump a connected device, authorize rebuilding, or make an image installable. The separate DNS build helper remains separately qualified. No extracted program or script is executed.

## Supported boundary

- SquashFS 4.0 little-endian, supported decompressor IDs 1–6, with exactly one validated candidate in the first 32 MiB. The candidate offset is discovered, never assumed from a model name.
- One bounded gzip wrapper followed by the same SquashFS backend. This is not an OEM package parser. No automatic recursive extraction.
- Maximum input 512 MiB; gzip expansion 128 MiB; ordinary extracted bytes 128 MiB; individual file 32 MiB; 20,000 entries; depth 32; manifest 8 MiB. Unsupported/corrupt/ambiguous structures fail closed.
- Filesystem header size, version, block size/log, inode count and table offsets must fit declared filesystem boundaries. The decoder must also finish successfully. Rust then checks every ordinary file hash, byte count, path, parent directory, duplicate name and complete bundle length.
- Links stay inert manifest records. Special files are excluded before extraction. No link or special file is recreated on Windows or in an export. Permissions, ownership and xattrs are not preserved. Other image regions are listed as unextracted; a filesystem inside a larger image is a partial image extraction.
- FIT, TRX, standalone U-Boot components, TAR, UBI/UBIFS, JFFS2, LZMA/XZ/zstd wrappers and proprietary/encrypted containers have no backend. A U-Boot image can contain a supported SquashFS region without granting support for its other regions.

## Qualified worker environment

Windows app requires WSL distribution **Ubuntu**, qualified here on **Ubuntu 26.04 LTS x86-64**, ordinary non-root user, Python 3.14, and these Ubuntu packages:

| Package | Qualified version | Executable SHA-256 |
|---|---|---|
| squashfs-tools | `1:4.7.5-1` | `/usr/bin/unsquashfs`: `003a7e9d70fbdcd05dfe132101c4b7bbb19a217976777f5393573eff21ddc202` |
| bubblewrap | `0.11.1-1` | `/usr/bin/bwrap`: `8e19e40e7d5f7a7e8b488c7926feb040eab6ed10c58fa360e266d2f70670e92b` |

Use the distribution's signed package repositories. The app checks these exact installed binary hashes before launching a parser. It does not install packages, fetch tools, or fall back to direct host extraction. Different/missing tools produce `WORKER_UNAVAILABLE`; additional platforms need independent qualification and an explicit pin update.

```sh
sudo apt-get update
sudo apt-get install python3 squashfs-tools=1:4.7.5-1 bubblewrap=0.11.1-1
sha256sum /usr/bin/unsquashfs /usr/bin/bwrap
```

Upstream tools are not bundled into the Windows installer. Squashfs-tools is GPL-2.0-or-later; bubblewrap is LGPL-2.0-or-later. Source and documentation: [Squashfs-tools 4.7.5](https://github.com/plougher/squashfs-tools/tree/master/Documentation/4.7.5), [bubblewrap](https://github.com/containers/bubblewrap). Ubuntu package provenance is independent of the source firmware's OpenWrt release signature.

## Isolation and supervision

Rust holds the original vault read lease, verifies hash/size, makes a scoped working copy, and launches the embedded Python supervisor through structured WSL arguments. The supervisor verifies tools; only the inner namespace process parses firmware.

Bubblewrap creates separate user/PID/network/IPC/UTS namespaces, disables further user namespaces, drops all capabilities, clears the environment, exposes only read-only system tools, read-only input and worker code, and mounts a single 256 MiB writable tmpfs. No home directory or Windows drives enter this namespace. The inner worker checks effective capabilities, absence of host paths and absence of network interfaces other than loopback.

Resource limits: 768 MiB address space, 90 CPU seconds per process, 64 processes, 64 descriptors, bounded file size, one decompressor processor, 16 MiB data/fragment queues. Decoder deadline 100 seconds; supervisor deadline 150 seconds; host deadline 165 seconds. The Linux supervisor checks cancellation and a host heartbeat. On cancellation, timeout, lost heartbeat or startup failure, its finally block kills/reaps its own process group; the PID namespace and tmpfs disappear. Rust also owns a scoped child guard. No arbitrary PID from firmware or output is killed.

The decoder first lists entries inside the namespace. An ambiguous listing, unsafe name, oversized declared output or excess entry count is refused. It excludes links/special entries and extracts ordinary files with strict errors. The output tree is walked without following links, hashed, then serialized as `OBTX1` (bounded JSON plus ordinary bytes). Rust never recursively unpacks an untrusted archive onto Windows.

## Storage, restart and export

No schema migration. Schema 6 `artifact_blobs` stores the immutable output bundle and JSON manifest. Only the JSON manifest gets a `metadata_report` occurrence, linked to the parent firmware hash and extraction job. The binary bundle has no firmware occurrence or firmware admission. Its hash is referenced by the manifest. Neither output enters firmware cards or becomes an official image.

Publication verifies both content addresses before a single SQLite transaction publishes the relationship and completes the job. Failed/interrupted jobs have no successful extraction record. Unrecognized leftovers are retained for review, never recursively deleted. Repeated pending/running requests for the same source reuse the existing job. Intentional completed reruns retain distinct audit occurrences. Library relocation copies derived blobs and manifests too.

Browse/preview reopens the retained manifest and validates the bundle's content address; text preview additionally checks the selected file hash, UTF-8, size and control characters. React renders text as text, never HTML. Export writes only explicitly selected ordinary files to a new TAR, with relative validated names and mode 0600; it never follows links or overwrites a destination. Original bundle and manifest stay retained.

## Physical acquisition status

MG11 user-reported software remains `MF69_AM802L_ZX_DD_N_241106_EU`; this extraction feature adds no authenticated identity evidence. `acquisition/strategies.rs::AcquisitionStrategyRegistry::can_obtain()` remains false for physical acquisition. The documented-export and storage-read strategies have no qualified MG11 implementation. No legitimate MG11 firmware-export endpoint or raw partition-read interface has been demonstrated; physical authentication-challenge qualification remains pending. No credentials, login, privileged read, configuration write, reset or flashing occurs in this workstream. Official OpenWrt bytes and their extracted files are not MG11 firmware.
