"""Linux side of the single-target DNS build. Called only by build-openwrt-dns.py.

ImageBuilder runs in a namespace with only its workspace writable. The input
firmware is never mounted or executed. Output readback runs offline with one
read-only image bind; no recursive extraction or installation is performed.
"""
import argparse
import fcntl
import hashlib
import ipaddress
import json
import os
from pathlib import Path
import re
import resource
import selectors
import shutil
import signal
import subprocess
import tarfile
import tempfile
import time
from urllib.parse import urlsplit

PROFILE = "tplink_archer-c7-v5"
ARCHIVE = "openwrt-imagebuilder-25.12.6-ath79-generic.Linux-x86_64.tar.zst"
ARCHIVE_HASH = "6f40746139c833336e67b03d5bdb0501d3a80fb789bf7e5b7a9b9984b83a5e23"
HOST_LINKS = {
    "file": "/usr/bin/file", "egrep": "/bin/egrep", "g++": "/builder/openwrt-25_12_ath79_generic/ccache_cxx.sh",
    "perl": "/usr/bin/perl", "git": "/usr/bin/git", "getopt": "/usr/bin/getopt",
    "gcc": "/builder/openwrt-25_12_ath79_generic/ccache_cc.sh", "awk": "/usr/bin/gawk", "bash": "/bin/bash",
    "gzip": "/bin/gzip", "wget": "/usr/bin/wget", "grep": "/bin/grep",
    "python": "/opt/venv/bin/python3.9", "python3": "/opt/venv/bin/python3.9",
    "xxd": "/builder/shared-workdir/build/scripts/xxdi.pl", "unzip": "/usr/bin/unzip",
    "which": "/usr/bin/which", "rsync": "/usr/bin/rsync", "ldconfig": "/builder/shared-workdir/build/scripts/noop.sh", "diff": "/usr/bin/diff",
}


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def bound_compressor(text):
    invocation = "-comp $(SQUASHFSCOMP) $(SQUASHFSOPT)\n"
    if text.count(invocation) != 1:
        raise ValueError("Unexpected pinned ImageBuilder compression invocation")
    return text.replace(invocation, invocation[:-1] + " -processors 1 -mem 256M\n")


def validate_overlay(raw):
    text = raw.decode("ascii")
    lines = text.splitlines()
    first = ["#!/bin/sh", "# OBTAINER: upstream DNS only; LAN-advertised DNS is unchanged.",
             "uci -q get network.wan >/dev/null || exit 1", "uci set network.wan.peerdns='0' || exit 1",
             "uci -q delete network.wan.dns"]
    if not 8 <= len(lines) <= 9 or lines[:5] != first or lines[-2:] != ["uci commit network || exit 1", "exit 0"]:
        raise ValueError("Only the fixed upstream-DNS customization is supported")
    addresses = []
    for line in lines[5:-2]:
        match = re.fullmatch(r"uci add_list network\.wan\.dns='([^']+)' \|\| exit 1", line)
        if not match or "%" in match[1]:
            raise ValueError("Unexpected customization script")
        address = ipaddress.ip_address(match[1])
        if not address.is_global or address.is_multicast or str(address) != match[1] or str(address) in addresses:
            raise ValueError("DNS must be canonical unique global-unicast IP addresses")
        addresses.append(str(address))
    if not 1 <= len(addresses) <= 2 or not text.endswith("\n"):
        raise ValueError("Invalid DNS count or script framing")
    return addresses


def sandbox(work=None, image=None, network=False):
    # APK must preserve root ownership inside its image tree. Map only this
    # unprivileged host user to namespace uid 0; no host root/capability is used.
    # All capabilities are dropped before running the builder.
    command = ["unshare", "--user", "--map-root-user", "--"] if work is not None else []
    command += ["bwrap", "--unshare-all", "--die-with-parent", "--new-session", "--cap-drop", "ALL"]
    if network:
        command += ["--share-net"]  # Trusted package builder only; readback never shares network.
    for location in ["/usr", "/bin", "/sbin", "/lib", "/lib64"]:
        if Path(location).exists():
            command += ["--ro-bind", location, location]
    command += ["--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp"]
    if work is not None:
        command += ["--bind", str(work), "/work"]
        for location in ["/etc/resolv.conf", "/etc/ssl/certs", "/etc/ssl/openssl.cnf", "/etc/alternatives", "/etc/nsswitch.conf", "/etc/passwd", "/etc/group"]:
            if Path(location).exists():
                command += ["--ro-bind", location, location]
        command += ["--dir", str(Path.home())]
        command += ["--setenv", "OPENSSL_CONF", "/etc/ssl/openssl.cnf"]
    if image is not None:
        command += ["--ro-bind", str(image), "/input.bin"]
    return command


def run_logged(command, log, budget, cancel=None):
    with log.open("xb") as output:
        process = subprocess.Popen(command, stdout=output, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL, start_new_session=True)
        deadline = time.monotonic() + budget
        try:
            while process.poll() is None:
                if cancel is not None and cancel.exists():
                    raise InterruptedError("Owner cancelled this build")
                if time.monotonic() >= deadline or log.stat().st_size > 16 * 1024**2:
                    raise TimeoutError("Build exceeded time or log limit")
                time.sleep(0.1)
            if process.returncode != 0:
                raise RuntimeError(f"Build command exited {process.returncode}; inspect retained log")
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=10)


def extract_tools(archive, destination, cancel=None):
    total = 0
    count = 0
    decoder = subprocess.Popen(["zstd", "--decompress", "--stdout", str(archive)], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    try:
        with tarfile.open(fileobj=decoder.stdout, mode="r|") as stream:
            for member in stream:
                if cancel is not None and cancel.exists():
                    raise InterruptedError("Owner cancelled tool preparation")
                count += 1
                total += member.size
                if count > 50000 or total > 2 * 1024**3 or member.size > 512 * 1024**2:
                    raise ValueError("Verified tool archive exceeded extraction limits")
                if not (member.isfile() or member.isdir() or member.issym() or member.islnk()):
                    raise ValueError("Special archive entry refused")
                if member.issym() and member.linkname.startswith("/"):
                    # These exact signed-SDK entries refer to its publisher's build
                    # machine. Omit them; ordinary make prerequisite checks resolve
                    # this machine's tools. Never create absolute archive links.
                    prefix = "openwrt-imagebuilder-25.12.6-ath79-generic.Linux-x86_64/staging_dir/host/bin/"
                    name = member.name.removeprefix(prefix)
                    if not member.name.startswith(prefix) or HOST_LINKS.get(name) != member.linkname:
                        raise ValueError("Unapproved absolute tool link")
                    continue
                stream.extract(member, path=destination, filter="data")
        if decoder.wait(timeout=10) != 0:
            raise ValueError("Tool archive decompression failed")
    finally:
        if decoder.poll() is None:
            decoder.kill()
            decoder.wait(timeout=10)
        decoder.stdout.close()
    return count, total


def readback(image, offset):
    command = sandbox(image=image) + ["prlimit", "--as=536870912", "--cpu=10", "--fsize=1048576", "--nofile=64", "--",
                "unsquashfs", "-processors", "1", "-data-queue", "16", "-frag-queue", "16",
                "-o", str(offset), "-cat", "/input.bin", "etc/uci-defaults/99-obtainer-dns"]
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, stdin=subprocess.DEVNULL, start_new_session=True)
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    selector.register(process.stderr, selectors.EVENT_READ)
    output = bytearray()
    diagnostic = bytearray()
    deadline = time.monotonic() + 20
    try:
        while selector.get_map():
            if time.monotonic() >= deadline:
                raise TimeoutError("Sandboxed single-file readback timed out")
            for key, _ in selector.select(timeout=0.1):
                raw = os.read(key.fd, 4096)
                if not raw:
                    selector.unregister(key.fileobj)
                    continue
                target = output if key.fileobj is process.stdout else diagnostic
                target.extend(raw)
                if len(target) > 8192:
                    raise ValueError("Sandboxed readback exceeded output cap")
        if process.wait(timeout=2) != 0:
            return None
        return bytes(output)
    finally:
        selector.close()
        if process.poll() is None:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait(timeout=10)
        process.stdout.close()
        process.stderr.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["job", "run-id", "archive-sha256", "overlay-sha256", "worker-sha256"]:
        parser.add_argument("--" + name, required=True)
    args = parser.parse_args()
    if not re.fullmatch("[0-9a-f]{32}", args.run_id) or args.archive_sha256 != ARCHIVE_HASH:
        raise ValueError("Invalid worker scope")
    job = Path(args.job).resolve(strict=True)
    marker = json.loads((job / ".obtainer-build.json").read_text())
    if job.name != args.run_id or marker != {"runId": args.run_id, "profile": PROFILE}:
        raise ValueError("Build ownership mismatch")
    if digest(Path(__file__)) != args.worker_sha256 or digest(job / ARCHIVE) != ARCHIVE_HASH:
        raise ValueError("Worker/tool copy changed")
    raw = (job / "99-obtainer-dns").read_bytes()
    if len(raw) > 4096 or hashlib.sha256(raw).hexdigest() != args.overlay_sha256:
        raise ValueError("Customization input changed")
    servers = validate_overlay(raw)
    if os.getuid() == 0:
        raise ValueError("Run ImageBuilder as a non-root user")
    for tool in ["make", "gcc", "zstd", "bwrap", "unshare", "prlimit", "unsquashfs"]:
        if not shutil.which(tool):
            raise ValueError(f"Required Linux tool missing: {tool}")
    resource.setrlimit(resource.RLIMIT_AS, (3 * 1024**3, 3 * 1024**3))
    resource.setrlimit(resource.RLIMIT_FSIZE, (2 * 1024**3, 2 * 1024**3))
    # A scoped flock prevents simultaneous expensive helpers; no PID from an input file is killed.
    lock_path = Path("/tmp/obtainer-dns-builder.lock")
    descriptor = os.open(lock_path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        work = Path(tempfile.mkdtemp(prefix=f"obtainer-dns-{args.run_id}-"))
        (work / ".owner").write_text(args.run_id)
        (job / "linux-workspace.txt").write_text(str(work))
        shutil.copyfile(job / ARCHIVE, work / ARCHIVE)
        if digest(work / ARCHIVE) != ARCHIVE_HASH:
            raise ValueError("Tool source changed during copy")
        tool_root = work / "tool"
        tool_root.mkdir()
        cancel = job / "cancel.requested"
        if cancel.exists():
            raise InterruptedError("Build cancelled before preparation")
        files, unpacked = extract_tools(work / ARCHIVE, tool_root, cancel)
        top = list(tool_root.iterdir())
        if len(top) != 1 or not top[0].is_dir() or not top[0].name.startswith("openwrt-imagebuilder-25.12.6-ath79-generic"):
            raise ValueError("Unexpected verified ImageBuilder layout")
        (top[0] / "tmp").mkdir(exist_ok=True)
        if "CONFIG_SIGNATURE_CHECK=y" not in (top[0] / ".config").read_text().splitlines():
            raise ValueError("Package signature verification must remain enabled")
        image_make = top[0] / "include/image.mk"
        original_make_hash = digest(image_make)
        image_make.write_text(bound_compressor(image_make.read_text()))
        patched_make_hash = digest(image_make)
        repositories = (top[0] / "repositories").read_text().splitlines()
        for reference in repositories:
            url = urlsplit(reference)
            if url.scheme != "https" or url.hostname != "downloads.openwrt.org" or url.port is not None or url.username or url.password or url.query or url.fragment or not url.path.startswith("/releases/25.12.6/"):
                raise ValueError("Unapproved build package repository")
        overlay = work / "files/etc/uci-defaults/99-obtainer-dns"
        overlay.parent.mkdir(parents=True)
        overlay.write_bytes(raw)
        overlay.chmod(0o755)
        output = work / "output"
        output.mkdir()
        command = sandbox(work=work, network=True) + ["--chdir", "/work/tool/" + top[0].name,
            "prlimit", "--as=3221225472", "--cpu=300", "--nproc=64", "--fsize=2147483648", "--",
            "make", "-j1", "image", f"PROFILE={PROFILE}", "FILES=/work/files", "BIN_DIR=/work/output", "EXTRA_IMAGE_NAME=obtainer-dns"]
        run_logged(command, job / "imagebuilder.log", 420, cancel)
        candidates = [path for path in output.rglob("*sysupgrade.bin") if path.is_file() and not path.is_symlink()]
        if len(candidates) != 1:
            raise ValueError("Build did not produce one exact sysupgrade image")
        image = candidates[0]
        if PROFILE not in image.name or "25.12.6" not in image.name or image.stat().st_size > 15728640:
            raise ValueError("Output target, release or hardware image-size limit mismatch")
        data = image.read_bytes()
        offsets = [index for index in range(len(data) - 3) if data[index:index + 4] == b"hsqs"]
        if not 1 <= len(offsets) <= 8:
            raise ValueError("No bounded unique SquashFS readback candidates")
        verified = [offset for offset in offsets if readback(image, offset) == raw]
        if len(verified) != 1:
            raise ValueError("Offline sandbox could not verify exact DNS defaults in built image")
        if cancel.exists():
            raise InterruptedError("Build cancelled before output publication")
        destination = job / image.name
        with destination.open("xb") as target, image.open("rb") as source:
            shutil.copyfileobj(source, target)
            target.flush()
            os.fsync(target.fileno())
        result = {"profile": PROFILE, "target": "ath79/generic", "version": "25.12.6",
                  "toolArchiveSha256": ARCHIVE_HASH, "toolFiles": files, "unpackedToolBytes": unpacked,
                  "builderResourcePatch": {"file": "include/image.mk", "beforeSha256": original_make_hash,
                                           "afterSha256": patched_make_hash, "compressionProcessors": 1,
                                           "compressionMemoryMiB": 256},
                  "outputFilename": image.name, "outputBytes": len(data), "outputSha256": digest(destination),
                  "dnsServers": servers, "dnsDefaultsSha256": args.overlay_sha256,
                  "packageRepositories": repositories, "packageSignatureCheck": True,
                  "verifiedSquashfsOffset": verified[0], "readbackNetwork": "separate_namespace",
                  "readbackHostDrivesMounted": False, "imageContentsExecuted": False}
        (job / "build-result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result))
        # Keep a failed workspace. Successful temporary tools may be removed only in this owned /tmp tree.
        if work.parent == Path("/tmp") and (work / ".owner").read_text() == args.run_id and shutil.rmtree.avoids_symlink_attacks:
            shutil.rmtree(work)


if __name__ == "__main__":
    main()
