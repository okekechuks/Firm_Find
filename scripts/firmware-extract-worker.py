"""OBTAINER offline extraction worker. Never executes firmware contents.

Outer supervisor has host access but never parses firmware. Inner parser has
only read-only tools/input and a size-limited private tmpfs. Output is a bounded
manifest followed by ordinary file bytes; no archive links reach Windows.
"""
import argparse
import hashlib
import gzip
import errno
import binascii
import json
import os
from pathlib import Path
import re
import signal
import stat
import struct
import subprocess
import sys
import time
import unicodedata

MAX_INPUT = 512 * 1024**2
MAX_BYTES = 128 * 1024**2
MAX_ENTRIES = 20000
MAX_MANIFEST = 8 * 1024**2
TMPFS_BYTES = 256 * 1024**2
SPACE_RESERVE = 32 * 1024**2
OTRX_BINARY_SHA = "66b870899eca6a12079668c23cdd95db069978afc9af5bcbc3681cd9c1e968e5"
OTRX_SOURCE_SHA = "e2ef4e7a08f84543945fec83c43bdebfdb1c252da1c440ebb90454913ec2dc3e"
PYTHON = "/usr/bin/python3.14"
TOOLS = {
    PYTHON: "fa9796cd3a30878e11a2f40372f773d3fcd913fff35e5bee8dd9a036e22e93ab",
    "/usr/bin/unsquashfs": "003a7e9d70fbdcd05dfe132101c4b7bbb19a217976777f5393573eff21ddc202",
    "/usr/bin/bwrap": "523da3e7399044be5163aee6f57a77a6bef7454376e28f0a0627920bae1b76b6",
}
OPTIONAL_TOOLS = {
    "/usr/bin/zstd": "fe50fd600cad89b775dc5b4b10bc8ff95e2d33ddb4f7632966464b1fbdab8598",
    "/usr/bin/lz4": "4be960d6f6b0d7ef69e01a9e1a056591c17b8687e9851db128018b2ac5f01da0",
    "/usr/sbin/debugfs": "864e1d7b445e7b5bfc831da78330dbcafc590fa82b89ea9de60b7527f989954f",
    "/usr/sbin/e2fsck": "2e51f521c676729920eaba694933d9d4048645f1a5789556fd0027e62d11ecc8",
    "/usr/sbin/fsck.cramfs": "82ccbc99540cc47c751cfb1fe4a0d77ebc97ffbc1324a2b5effb19f84970869e",
    "/usr/lib/x86_64-linux-gnu/libarchive.so.13.8.5": "959dcce452c258ad7016f74b33b6df688cdad8a6caa1a625b1bfbebe85b332b0",
}


class Refusal(Exception):
    pass


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def qualified_binary(path, expected_hash, executable=True):
    try:
        meta=path.lstat()
        if not stat.S_ISREG(meta.st_mode) or meta.st_uid!=0 or meta.st_mode & (stat.S_ISUID | stat.S_ISGID | 0o022) or (executable and not meta.st_mode & 0o111):
            return False
        try:
            if os.getxattr(path,"security.capability"):
                return False
        except OSError as error:
            if error.errno not in (errno.ENODATA,errno.ENOTSUP):raise
        return digest(path)==expected_hash
    except OSError:
        return False


def safe_name(name):
    parts = name.split("/")
    if (not name or len(name.encode("utf-8")) > 240 or len(parts) > 32
            or any(p in ("", ".", "..") or p.endswith((" ", ".")) for p in parts)
            or any(unicodedata.category(c) in ("Cc", "Cf") or c in '\\:*?"<>|' for c in name)
            or any(re.fullmatch(r"(?i)(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\..*)?", p) for p in parts)):
        raise Refusal("unsafe_path")
    return name


def reserve_space(staged_bytes, tree_bytes, entries):
    # Account for simultaneously retained decompression output, data, one page
    # per entry (rounding/inodes/directories), plus bounded listing/scratch slack.
    if min(staged_bytes, tree_bytes, entries) < 0 or staged_bytes + tree_bytes + entries * 4096 + SPACE_RESERVE > TMPFS_BYTES:
        raise Refusal("resource_exhausted")


def resource_error(error):
    return isinstance(error, MemoryError) or isinstance(error, OSError) and error.errno in (errno.ENOSPC, errno.ENOMEM, errno.EDQUOT, errno.EFBIG, errno.EAGAIN)


def run_tool(command, output=None, listing=False):
    import resource
    def listing_limit():
        resource.setrlimit(resource.RLIMIT_FSIZE, (MAX_MANIFEST,) * 2)
    result = subprocess.run(command, stdout=output if output is not None else subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL, timeout=100,
                            preexec_fn=listing_limit if listing else None, check=False)
    if result.returncode:
        free = os.statvfs("/work")
        if result.returncode in (-signal.SIGXFSZ, -signal.SIGXCPU, -signal.SIGKILL) or free.f_bavail * free.f_frsize < SPACE_RESERVE // 2:
            raise Refusal("resource_exhausted")
        raise Refusal("extraction_failed")


def squashfs_region(image, start_offset=0, end_offset=None):
    """SquashFS v4 little-endian superblock, not a magic-only match."""
    size = image.stat().st_size if end_offset is None else end_offset
    with image.open("rb") as stream:
        stream.seek(start_offset)
        scan = stream.read(min(size - start_offset, 32 * 1024**2))
        candidates = []
        start = 0
        seen = False
        while True:
            relative = scan.find(b"hsqs", start)
            if relative < 0:
                break
            seen = True
            start = relative + 4
            offset = start_offset + relative
            stream.seek(offset)
            raw = stream.read(96)
            if len(raw) != 96:
                continue
            values = struct.unpack("<5I6H8Q", raw)
            _, inodes, _, block, fragments, compression, log, _, ids, major, minor, root, used, idtable, xattr, inode, directory, fragment, lookup = values
            if (major != 4 or minor != 0 or not 0 < inodes <= MAX_ENTRIES
                    or block not in [1 << n for n in range(12, 21)] or block != 1 << log
                    or fragments > inodes or ids == 0 or not 96 < used <= size - offset
                    or not 96 <= inode <= directory < used or not 96 <= idtable < used
                    or root >> 16 >= used
                    or any(v != 0xffffffffffffffff and not 96 <= v < used for v in (xattr, fragment, lookup))):
                continue
            if compression not in (1, 2, 3, 4, 5, 6):
                raise Refusal("unsupported_compression")
            candidates.append({"format": "squashfs", "offset": offset, "length": used, "compression": compression})
        if len(candidates) != 1:
            raise Refusal("corrupt_image" if seen else "unsupported_format")
        return candidates[0]


def collect_tree(root):
    entries = []
    total = 0
    names = set()
    pending = [(root, "")]
    while pending:
        parent, prefix = pending.pop()
        for item in sorted(os.scandir(parent), key=lambda item: item.name):
            name = safe_name(prefix + item.name)
            folded = unicodedata.normalize("NFC", name).casefold()
            if folded in names:
                raise Refusal("colliding_paths")
            names.add(folded)
            meta = item.stat(follow_symlinks=False)
            entry = {"path": name, "kind": "directory", "size": 0, "sha256": None, "linkTarget": None, "mode": stat.S_IMODE(meta.st_mode)}
            if stat.S_ISDIR(meta.st_mode):
                pending.append((Path(item.path), name + "/"))
            elif stat.S_ISREG(meta.st_mode):
                if meta.st_mode & (stat.S_ISUID | stat.S_ISGID):
                    raise Refusal("unsafe_mode")
                total += meta.st_size
                if meta.st_size > 32 * 1024**2 or total > MAX_BYTES:
                    raise Refusal("output_limit")
                entry.update(kind="file", size=meta.st_size, sha256=digest(Path(item.path)))
            elif stat.S_ISLNK(meta.st_mode):
                target = os.readlink(item.path)
                # Links are inert metadata only; even in-root links are not opened.
                if len(target) > 1024 or any(ord(c) < 32 for c in target):
                    raise Refusal("unsafe_link")
                entry.update(kind="symlink_metadata", linkTarget=target)
            else:
                raise Refusal("unsafe_special_file")
            entries.append(entry)
            if len(entries) > MAX_ENTRIES:
                raise Refusal("entry_limit")
    if not any(e["kind"] == "file" for e in entries):
        raise Refusal("empty_output")
    return sorted(entries, key=lambda e: e["path"]), total


class SquashfsBackend:
    name = "raw"

    @staticmethod
    def accepts(prefix):
        return not prefix.startswith(b"\x1f\x8b")

    @staticmethod
    def prepare(image):
        return image

    @staticmethod
    def inspect(image):
        return squashfs_region(image), None


class GzipSquashfsBackend(SquashfsBackend):
    """One bounded gzip layer followed by the same validated SquashFS parser."""
    name = "gzip"

    @staticmethod
    def accepts(prefix):
        return prefix.startswith(b"\x1f\x8b")

    @staticmethod
    def prepare(image):
        output = Path("/work/expanded.bin")
        total = 0
        with gzip.open(image, "rb") as source, output.open("xb") as destination:
            while chunk := source.read(65536):
                total += len(chunk)
                if total > MAX_BYTES:
                    raise Refusal("output_limit")
                destination.write(chunk)
        with output.open("rb") as source:
            if source.read(4) == b"HDR0":
                raise Refusal("unsupported_nested_format")
        return output


def trx_header(image):
    size=image.stat().st_size
    with image.open("rb") as source:
        raw=source.read(28)
        if len(raw)!=28 or size > MAX_INPUT:
            raise Refusal("corrupt_image")
        magic,length,expected_crc,flags_version,*offsets=struct.unpack("<7I",raw)
        if magic!=0x30524448 or not 28 < length <= size:
            raise Refusal("corrupt_image")
        if flags_version != 0x10000:
            raise Refusal("unsupported_container_version")
        parts=[offset for offset in offsets if offset]
        if not parts or offsets != parts + [0]*(3-len(parts)) or parts != sorted(set(parts)) or any(not 28 <= offset < length for offset in parts):
            raise Refusal("corrupt_image")
        source.seek(12)
        remaining=length-12
        crc=0
        while remaining:
            chunk=source.read(min(remaining,65536))
            if not chunk: raise Refusal("corrupt_image")
            crc=binascii.crc32(chunk,crc)
            remaining-=len(chunk)
        if crc ^ 0xffffffff != expected_crc:
            raise Refusal("corrupt_image")
    return {"format":"trx","version":1,"flags":0,"length":length,"partitionOffsets":parts,
            "crc32":f"{expected_crc:08x}","parser":"OpenWrt otrx",
            "sourceCommit":"e3853149fb18dfa55f10c10c9aaf99c329747365",
            "sourceSha256":OTRX_SOURCE_SHA,"parserSha256":OTRX_BINARY_SHA}


class TrxBackend(SquashfsBackend):
    name="trx"

    @staticmethod
    def accepts(prefix):
        return prefix.startswith(b"HDR0")

    @staticmethod
    def inspect(image):
        info=trx_header(image)
        if not Path("/otrx").is_file():
            raise Refusal("trx_worker_unavailable")
        # Literal read-only subcommand; create/extract and caller arguments are unreachable.
        run_tool(["/otrx","check",str(image)])
        root=info["partitionOffsets"][-1]
        with image.open("rb") as source:
            source.seek(root)
            if source.read(4)!=b"hsqs":
                raise Refusal("unsupported_nested_format")
        region=squashfs_region(image,root,info["length"])
        if region["offset"]!=root:
            raise Refusal("corrupt_image")
        return region,info


# Explicit plugin registry. Recognition does not advertise unsupported formats.
BACKENDS = (TrxBackend, GzipSquashfsBackend, SquashfsBackend)


def inner(expected, size):
    import resource
    image = Path("/input.bin")
    if image.stat().st_size != size or size > MAX_INPUT or digest(image) != expected:
        raise Refusal("source_changed")
    resource.setrlimit(resource.RLIMIT_AS, (768 * 1024**2,) * 2)
    resource.setrlimit(resource.RLIMIT_CPU, (90,) * 2)
    resource.setrlimit(resource.RLIMIT_FSIZE, (MAX_BYTES + MAX_MANIFEST + 65536,) * 2)
    resource.setrlimit(resource.RLIMIT_NOFILE, (64,) * 2)
    resource.setrlimit(resource.RLIMIT_NPROC, (64,) * 2)
    status = Path("/proc/self/status").read_text()
    caps = re.search(r"^CapEff:\s+([0-9a-f]+)$", status, re.MULTILINE)
    if not caps or int(caps[1], 16) or Path("/mnt/c").exists() or Path("/home").exists() or not os.statvfs("/").f_flag & os.ST_RDONLY:
        raise Refusal("worker_unavailable")
    if any(name != "lo" for _, name in __import__("socket").if_nameindex()):
        raise Refusal("worker_unavailable")
    # The format module is trusted application code, mounted read-only. Import
    # occurs after namespace/resource checks; the outer supervisor never parses.
    import importlib.util
    spec = importlib.util.spec_from_file_location("obtainer_layers", "/engine.py")
    engine = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(engine)
    report, files = engine.extract(sys.modules[__name__], image, Path("/work/layers"))
    if digest(image) != expected:
        raise Refusal("source_changed")
    raw = json.dumps(report, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode("utf-8")
    if len(raw) > MAX_MANIFEST:
        raise Refusal("manifest_limit")
    output = sys.stdout.buffer
    output.write(b"OBTX1\n" + struct.pack("<I", len(raw)) + raw)
    for entry in report["entries"]:
        if entry["kind"] == "file":
            with files[entry["path"]].open("rb") as source:
                while chunk := source.read(65536):
                    output.write(chunk)
    output.flush()


def extract_squashfs(filesystem_image, region, root, workspace, retained_bytes=0, existing_entries=0):
    """Existing qualified decoder, with explicit shared-workspace accounting."""
    workspace.mkdir(parents=True, exist_ok=True)
    # List inside the same sandbox. Reject ambiguous text listing rather than
    # guessing names; links and special files are excluded before extraction.
    with (workspace / "listing").open("wb") as listing:
        run_tool(["/usr/bin/unsquashfs", "-lln", "-max-depth", "33", "-o", str(region["offset"]),
                  str(filesystem_image)], output=listing, listing=True)
    with (workspace / "listing").open("rb") as listing:
        raw_listing = listing.read(MAX_MANIFEST + 1)
    if len(raw_listing) > MAX_MANIFEST:
        raise Refusal("entry_limit")
    excluded = []
    observed = {}
    listed_bytes = 0
    for line in raw_listing.decode("utf-8", "strict").splitlines():
        match = re.fullmatch(r"([dlbcps-])[rwxstST-]{9}\s+\d+/\d+\s+([\d, ]+)\s+\d{4}-\d{2}-\d{2} \d{2}:\d{2} squashfs-root(?:/(.*))?", line)
        if not match:
            raise Refusal("unsafe_listing")
        if match[3] is None:
            continue
        name = match[3]
        target = None
        if match[1] == "l":
            if name.count(" -> ") != 1:
                raise Refusal("unsafe_listing")
            name, target = name.split(" -> ")
            if len(target) > 1024 or any(ord(c) < 32 for c in target):
                raise Refusal("unsafe_link")
        safe_name(name)
        if name in observed or len(observed) >= MAX_ENTRIES:
            raise Refusal("entry_limit")
        observed[name] = match[1]
        if match[1] == "-":
            listed_bytes += int(match[2])
            if int(match[2]) > 32 * 1024**2 or listed_bytes > MAX_BYTES:
                raise Refusal("output_limit")
        elif match[1] != "d":
            excluded.append({"path": name, "kind": "symlink_metadata" if target is not None else "special_omitted",
                             "size": 0, "sha256": None, "linkTarget": target})
    (workspace / "exclude").write_text("".join(e["path"] + "\n" for e in excluded))
    reserve_space(retained_bytes, listed_bytes, len(observed) + existing_entries)
    remaining = os.statvfs("/work")
    if listed_bytes + len(observed) * 4096 + SPACE_RESERVE > remaining.f_bavail * remaining.f_frsize:
        raise Refusal("resource_exhausted")
    # Dropped capabilities prohibit device creation. Strict errors prohibit
    # publishing any partial tree after a special-file/permission failure.
    command = ["/usr/bin/unsquashfs", "-processors", "1", "-data-queue", "16", "-frag-queue", "16",
               "-strict-errors", "-no-xattrs", "-no-progress", "-max-depth", "33", "-d", str(root),
               "-no-wildcards", "-exclude-file", str(workspace / "exclude"), "-o", str(region["offset"]), str(filesystem_image)]
    run_tool(command)
    entries, total = collect_tree(root)
    if {e["path"] for e in entries} != {name for name, kind in observed.items() if kind in ("-", "d")}:
        raise Refusal("incomplete_tree")
    entries = sorted(entries + excluded, key=lambda e: e["path"])
    return entries, total


def sandbox(script, image):
    # Host ownership must be checked before user-namespace UID remapping. The
    # inner process receives only this computed allowlist, rechecks bytes/mode,
    # and verifies version. No caller/firmware-supplied tool path is accepted.
    qualified = {name: value for name, value in {**TOOLS, **OPTIONAL_TOOLS}.items()
                 if qualified_binary(Path(name), value, executable=".so." not in name)}
    command = ["/usr/bin/bwrap", "--unshare-all", "--unshare-user", "--die-with-parent", "--new-session", "--cap-drop", "ALL",
               "--disable-userns", "--clearenv", "--setenv", "PATH", "/usr/bin:/bin", "--setenv", "LC_ALL", "C"]
    for path in ("/usr", "/bin", "/lib", "/lib64"):
        if Path(path).exists():
            command += ["--ro-bind", path, path]
    optional=Path("/opt/obtainer-tools/otrx")
    if qualified_binary(optional,OTRX_BINARY_SHA):
        command += ["--ro-bind",str(optional),"/otrx"]
        qualified["/otrx"] = OTRX_BINARY_SHA
    command += ["--setenv", "OBTAINER_QUALIFIED_TOOLS", json.dumps(qualified, sort_keys=True)]
    command += ["--proc", "/proc", "--dev", "/dev", "--size", "268435456", "--tmpfs", "/work",
                "--ro-bind", str(script), "/worker.py", "--ro-bind", str(image), "/input.bin",
                "--ro-bind", str(script.with_name("extraction-engine-v2.py")), "/engine.py",
                "--remount-ro", "/", "--chdir", "/work", PYTHON, "-I", "-B", "/worker.py", "--inner"]
    return command


def supervise(job, expected, size):
    if os.getuid() == 0:
        raise Refusal("worker_unavailable")
    for name, expected_hash in TOOLS.items():
        if not qualified_binary(Path(name),expected_hash):
            raise Refusal("worker_unavailable")
    command = sandbox(Path(__file__).resolve(), job / "input.bin") + ["--sha256", expected, "--size", str(size)]
    with (job / "output.bin").open("xb") as output:
        child = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.PIPE, start_new_session=True)
        deadline = time.monotonic() + 150
        try:
            while child.poll() is None:
                if (job / "cancel").exists():
                    raise Refusal("cancelled")
                if time.time() - (job / "heartbeat").stat().st_mtime > 10:
                    raise Refusal("owner_disconnected")
                if time.monotonic() >= deadline:
                    raise Refusal("timeout")
                if os.fstat(output.fileno()).st_size > MAX_BYTES + MAX_MANIFEST + 10:
                    raise Refusal("output_limit")
                time.sleep(0.05)
            if child.returncode != 0:
                message = child.stderr.read(128).decode("ascii", "replace").strip()
                raise Refusal(message if message in ("worker_unavailable", "corrupt_image", "unsupported_format", "unsupported_compression", "output_limit", "entry_limit", "unsafe_path", "unsafe_link", "source_changed", "resource_exhausted", "trx_worker_unavailable", "unsupported_nested_format", "unsupported_container_version", "unsafe_mode", "unsafe_special_file", "corrupt_description", "description_limit", "manifest_limit", "decompression_bomb", "colliding_paths", "ambiguous_image") else "extraction_failed")
        finally:
            if child.poll() is None:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait(timeout=5)
            child.stderr.close()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--inner", action="store_true")
    parser.add_argument("--job", type=Path)
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--size", required=True, type=int)
    args = parser.parse_args()
    if not re.fullmatch("[0-9a-f]{64}", args.sha256) or not 0 < args.size <= MAX_INPUT:
        raise Refusal("invalid_input")
    if args.inner:
        inner(args.sha256, args.size)
    else:
        try:
            supervise(args.job.resolve(strict=True), args.sha256, args.size)
        except (Refusal, OSError, MemoryError, subprocess.SubprocessError) as error:
            code = str(error) if isinstance(error, Refusal) else "resource_exhausted" if resource_error(error) else "worker_unavailable"
            (args.job / "error.txt").write_text(code, encoding="ascii")
            raise SystemExit(1) from None


if __name__ == "__main__":
    try:
        main()
    except (Refusal, OSError, ValueError, MemoryError, subprocess.SubprocessError) as error:
        print(str(error) if isinstance(error, Refusal) else "resource_exhausted" if resource_error(error) else "extraction_failed", file=sys.stderr)
        raise SystemExit(1) from None
