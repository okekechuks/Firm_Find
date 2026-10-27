#!/bin/sh
# Build the reviewed, unmodified OpenWrt parser source; no firmware is involved.
set -eu
task_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
task_source="$task_root/workers/extraction/vendor/otrx.c"
task_output=${1:-/opt/obtainer-tools/otrx}
printf '%s  %s\n' e2ef4e7a08f84543945fec83c43bdebfdb1c252da1c440ebb90454913ec2dc3e "$task_source" | sha256sum -c -
export SOURCE_DATE_EPOCH=0
mkdir -p -- "$(dirname -- "$task_output")"
gcc -O2 -D_FORTIFY_SOURCE=3 -fstack-protector-strong -Wformat -Werror=format-security \
    -fPIE -pie -Wl,-z,relro,-z,now -o "$task_output" "$task_source"
chmod 0755 -- "$task_output"
sha256sum "$task_output"
