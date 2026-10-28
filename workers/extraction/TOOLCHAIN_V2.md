# Qualified V2 extraction toolchain

Local qualification: 28 October 2026, WSL `Ubuntu`, Ubuntu 26.04 x86-64,
ordinary user. Tools are distribution-installed, not shipped in the Windows
installer. Unknown hashes fail closed; do not downgrade patched tools to satisfy
a pin. A newer build requires independent runtime qualification and an explicit
pin update. No extraction outside the namespace is used as a fallback.

| Executable/library | Runtime / installed package | SHA-256 |
|---|---|---|
| `/usr/bin/python3.14` | Python **3.14.4**, `python3.14 3.14.4-1`; `python3` metapackage is separately `3.14.3-0ubuntu2` | `fa9796cd3a30878e11a2f40372f773d3fcd913fff35e5bee8dd9a036e22e93ab` |
| `/usr/bin/bwrap` | `0.11.1-1ubuntu0.3` | `523da3e7399044be5163aee6f57a77a6bef7454376e28f0a0627920bae1b76b6` |
| `/usr/bin/unsquashfs` | `1:4.7.5-1` | `003a7e9d70fbdcd05dfe132101c4b7bbb19a217976777f5393573eff21ddc202` |
| `/usr/bin/zstd` | `1.5.7+dfsg-3` | `fe50fd600cad89b775dc5b4b10bc8ff95e2d33ddb4f7632966464b1fbdab8598` |
| `/usr/bin/lz4` | `1.10.0-8` | `4be960d6f6b0d7ef69e01a9e1a056591c17b8687e9851db128018b2ac5f01da0` |
| `/usr/sbin/debugfs` | `e2fsprogs 1.47.2-3ubuntu4` | `864e1d7b445e7b5bfc831da78330dbcafc590fa82b89ea9de60b7527f989954f` |
| `/usr/sbin/e2fsck` | `e2fsprogs 1.47.2-3ubuntu4` | `2e51f521c676729920eaba694933d9d4048645f1a5789556fd0027e62d11ecc8` |
| `/usr/sbin/fsck.cramfs` | `util-linux-extra 2.41.3-3ubuntu2.2` | `82ccbc99540cc47c751cfb1fe4a0d77ebc97ffbc1324a2b5effb19f84970869e` |
| `/usr/lib/x86_64-linux-gnu/libarchive.so.13.8.5` | `libarchive13t64 3.8.5-1ubuntu2.3` | `959dcce452c258ad7016f74b33b6df688cdad8a6caa1a625b1bfbebe85b332b0` |
| `/opt/obtainer-tools/otrx` (mounted `/otrx`) | Existing qualified OpenWrt source/build | `66b870899eca6a12079668c23cdd95db069978afc9af5bcbc3681cd9c1e968e5` |

The supervisor verifies host-root ownership, regular-file identity, safe mode,
no file capabilities and hashes before namespace UID mapping. The inner module
checks that computed allowlist, rehashes the file and checks its version before
use. libarchive is bound to the exact versioned shared object and checked with
`archive_version_string`; no arbitrary library search or extraction API is used.
Python is pinned too. Standard-library gzip/bzip2/lzma readers run in that pinned
interpreter and the same read-only distribution runtime. Shared dependencies are
distribution-maintained; this is not a reproducible container or a claim that
every transitive library is individually hash-pinned.

The extension installed signed-repository packages with:

```sh
apt-get update
apt-get install --no-install-recommends -y libarchive-tools lz4 mtd-utils util-linux-extra
dpkg -V python3.14 libpython3.14-stdlib bubblewrap squashfs-tools libarchive13t64 e2fsprogs util-linux-extra lz4 zstd
```

The install brought required util-linux dependencies to `2.41.3-3ubuntu2.2`.
`dpkg -V` exited 0 with no differences. Existing bubblewrap/unsquashfs pins were
preserved. mtd-utils/ubinize was considered for synthetic fixtures; its presence
does **not** enable device access, JFFS2 or UBIFS extraction. No mtd/flash utility
is reachable through the worker registry. Fixture generators operate on synthetic
files only; production never invokes mke2fs/mkfs or creates a mount.

Licenses/source: [Python PSF](https://docs.python.org/3/license.html),
[bubblewrap LGPL](https://github.com/containers/bubblewrap),
[Squashfs-tools GPL](https://github.com/plougher/squashfs-tools),
[zstd dual BSD/GPL](https://github.com/facebook/zstd),
[LZ4](https://github.com/lz4/lz4),
[e2fsprogs GPL](https://git.kernel.org/pub/scm/fs/ext2/e2fsprogs.git/),
[util-linux](https://github.com/util-linux/util-linux),
[libarchive BSD](https://github.com/libarchive/libarchive).
No upstream source was copied into the V2 module; format readers are application
code. Existing vendored otrx and its GPL notice remain unchanged in
[vendor/README.md](vendor/README.md).
