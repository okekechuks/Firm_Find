"""Build one qualified OpenWrt image with upstream DNS defaults, retaining its input.

Windows developer/beta helper. Requires WSL Ubuntu with make, gcc, zstd,
bubblewrap, prlimit and unsquashfs. Never logs into, configures or flashes a router.
The Linux worker runs trusted signature-pinned ImageBuilder tools; image readback
runs separately with no network or host drive access. Generated images are not
official release images and remain unqualified for physical installation.
"""
import argparse
import hashlib
import ipaddress
import json
import os
from pathlib import Path, PureWindowsPath
import shutil
import subprocess
import sys
import uuid

ROOT = Path(__file__).resolve().parent.parent
PROFILE = "tplink_archer-c7-v5"
ARCHIVE = "openwrt-imagebuilder-25.12.6-ath79-generic.Linux-x86_64.tar.zst"
ARCHIVE_HASH = "6f40746139c833336e67b03d5bdb0501d3a80fb789bf7e5b7a9b9984b83a5e23"


def windows_local_path(value):
    path = PureWindowsPath(value)
    if (not path.is_absolute() or len(path.drive) != 2 or not path.drive[0].isalpha()
            or ".." in path.parts or any(":" in part or "\x00" in part for part in path.parts[1:])):
        raise ValueError("Use an absolute local drive path without traversal or alternate streams")
    return path


def local_path(value, must_exist=True):
    # Validate before resolve/open so UNC paths and reparse ancestors cannot
    # cause an implicit remote lookup or redirect these reads to another volume.
    import ctypes
    path = Path(windows_local_path(str(value.absolute())))
    if ctypes.windll.kernel32.GetDriveTypeW(str(path.anchor)) not in (2, 3, 6):
        raise ValueError("Expected an available local drive")
    for ancestor in [*reversed(path.parents), path]:
        try:
            info = ancestor.lstat()
        except FileNotFoundError:
            if must_exist:
                raise
            continue
        if getattr(info, "st_file_attributes", 0) & 0x400 or ancestor.is_symlink():
            raise ValueError("Linked/reparse input, tool or build root refused")
    return path.resolve(strict=must_exist)


def dns_servers(values):
    if not 1 <= len(values) <= 2:
        raise ValueError("Choose one or two upstream DNS addresses")
    result = []
    for value in values:
        if "%" in value or len(value) > 45:
            raise ValueError("Scoped addresses and non-IP input are not supported")
        address = ipaddress.ip_address(value)
        if not address.is_global or address.is_multicast or address.is_loopback or address.is_unspecified:
            raise ValueError("DNS must be a global unicast IP address")
        if str(address) in result:
            raise ValueError("Duplicate DNS address")
        result.append(str(address))
    return result


def defaults_script(servers):
    servers = dns_servers(servers)
    # IP canonicalization excludes shell syntax. No caller-supplied script/path is used.
    lines = ["#!/bin/sh", "# OBTAINER: upstream DNS only; LAN-advertised DNS is unchanged.",
             "uci -q get network.wan >/dev/null || exit 1", "uci set network.wan.peerdns='0' || exit 1",
             "uci -q delete network.wan.dns"]
    lines += [f"uci add_list network.wan.dns='{address}' || exit 1" for address in servers]
    lines += ["uci commit network || exit 1", "exit 0", ""]
    return "\n".join(lines)


def digest(path, maximum):
    if path.is_symlink() or not path.is_file() or path.stat().st_size > maximum:
        raise ValueError("Expected a bounded ordinary file")
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def qualified_source():
    assets = ROOT / "apps/desktop/src-tauri/assets/official"
    subprocess.run([sys.executable, str(ROOT / "scripts/refresh-official-catalogue.py"),
                    "--upstream", str(assets), "--check"], check=True, timeout=30)
    sources = json.loads((assets / "catalogue.json").read_text(encoding="utf-8"))
    matches = [source for source in sources if source["profile"] == PROFILE]
    if len(matches) != 1:
        raise ValueError("No unique qualified build target")
    manifest = (assets / "sha256sums").read_text(encoding="ascii")
    if manifest.splitlines().count(f"{ARCHIVE_HASH} *{ARCHIVE}") != 1:
        raise ValueError("Builder archive is not in the verified release manifest")
    return matches[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--tool-archive", type=Path, required=True)
    parser.add_argument("--dns", nargs="+", required=True)
    parser.add_argument("--work-root", type=Path, required=True)
    args = parser.parse_args()
    if sys.platform != "win32":
        parser.error("Launch this host helper on Windows")
    servers = dns_servers(args.dns)
    source = qualified_source()
    original = local_path(args.input)
    before = digest(original, 128 * 1024 * 1024)
    if before != source["sha256"] or original.stat().st_size != source["sizeBytes"]:
        raise ValueError("Input is not the exact qualified official image; no build started")
    archive = local_path(args.tool_archive)
    if digest(archive, 512 * 1024 * 1024) != ARCHIVE_HASH:
        raise ValueError("ImageBuilder archive failed signed-release hash verification")
    parent = local_path(args.work_root, must_exist=False)
    if shutil.disk_usage(parent.anchor).free < 4 * 1024**3:
        raise ValueError("Build output drive needs at least 4 GiB free")
    parent.mkdir(parents=True, exist_ok=True)
    run_id = uuid.uuid4().hex
    job = parent / run_id
    job.mkdir(exist_ok=False)
    (job / ".obtainer-build.json").write_text(json.dumps({"runId": run_id, "profile": PROFILE}), encoding="utf-8")
    overlay = defaults_script(servers).encode("ascii")
    (job / "99-obtainer-dns").write_bytes(overlay)
    shutil.copyfile(archive, job / ARCHIVE)
    worker = ROOT / "scripts/openwrt-dns-worker.py"
    shutil.copyfile(worker, job / worker.name)
    worker_hash = digest(worker, 1024 * 1024)
    guest_path = subprocess.check_output(["wsl.exe", "-d", "Ubuntu", "--exec", "wslpath", "-a", job.as_posix()], text=True, timeout=15).strip()
    command = ["wsl.exe", "-d", "Ubuntu", "--exec", "python3", guest_path + "/" + worker.name,
               "--job", guest_path, "--run-id", run_id, "--archive-sha256", ARCHIVE_HASH,
               "--overlay-sha256", hashlib.sha256(overlay).hexdigest(), "--worker-sha256", worker_hash]
    with (job / "worker.log").open("wb") as log:
        process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
        try:
            code = process.wait(timeout=540)
            if code:
                raise subprocess.CalledProcessError(code, command)
        except (KeyboardInterrupt, subprocess.TimeoutExpired):
            (job / "cancel.requested").write_text(run_id, encoding="ascii")
            try:
                process.wait(timeout=15)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=10)
            raise
    report = json.loads((job / "build-result.json").read_text(encoding="utf-8"))
    if digest(original, 128 * 1024 * 1024) != before:
        raise ValueError("Original input changed; build result is not admitted")
    output = job / report["outputFilename"]
    if output.parent != job or digest(output, 16 * 1024 * 1024) != report["outputSha256"]:
        raise ValueError("Published build output failed independent verification")
    report.update({"source": source, "originalInput": str(original), "originalSha256Before": before,
                   "originalSha256After": before, "dnsServers": servers, "workerSha256": worker_hash,
                   "generatedImageClass": "custom_build_not_official", "installability": "unknown",
                   "physicalDeviceRequests": 0, "outputPath": str(output)})
    (job / "build-result.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
