# OpenWrt TRX validator source

`otrx.c` is unmodified OpenWrt firmware-utils source at commit `e3853149fb18dfa55f10c10c9aaf99c329747365`:

https://git.openwrt.org/project/firmware-utils/plain/src/otrx.c?id=e3853149fb18dfa55f10c10c9aaf99c329747365

Source SHA-256: `e2ef4e7a08f84543945fec83c43bdebfdb1c252da1c440ebb90454913ec2dc3e`. Upstream license is GPL-2.0-or-later, copyright Rafał Miłecki; SPDX and copyright remain in the source. `COPYING` contains GPL version 2 from https://www.gnu.org/licenses/old-licenses/gpl-2.0.txt . This applies to this vendored component.

Build with `scripts/build-trx-parser.sh` on the qualified Ubuntu environment. GCC `15.2.0-16ubuntu1`, glibc `2.43-2ubuntu2`; executable SHA-256 `66b870899eca6a12079668c23cdd95db069978afc9af5bcbc3681cd9c1e968e5`. Rebuilds on other environments require qualification and a deliberate pin update. The tool is not bundled into Windows installers.

The worker mounts only the correctly hashed non-setuid executable at `/otrx`, read-only. Only literal `otrx check /input.bin` is called inside the isolated namespace. Its create/extract commands are never exposed. Separate bounded header, offset, version and CRC checks narrow support to TRX v1 with a direct SquashFS final partition. Other partitions remain unextracted. This is image-container support and grants no physical-device access or installation authority.
