"""OBTAINER offline extraction worker. Never executes firmware contents.

Outer supervisor has host access but never parses firmware. Inner parser has
only read-only tools/input and a size-limited private tmpfs. Output is a bounded
manifest followed by ordinary file bytes; no archive links reach Windows.
"""
import argparse
import hashlib
import gzip
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

MAX_INPUT = 512 * 1024**2
MAX_BYTES = 128 * 1024**2
MAX_ENTRIES = 20000
MAX_MANIFEST = 8 * 1024**2
TOOLS = {
    "/usr/bin/unsquashfs": "003a7e9d70fbdcd05dfe132101c4b7bbb19a217976777f5393573eff21ddc202",
    "/usr/bin/bwrap": "8e19e40e7d5f7a7e8b488c7926feb040eab6ed10c58fa360e266d2f70670e92b",
}


class Refusal(Exception):
    pass


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def safe_name(name):
    parts = name.split("/")
    if (not name or len(name.encode("utf-8")) > 240 or len(parts) > 32
            or any(p in ("", ".", "..") or p.endswith((" ", ".")) for p in parts)
            or any(ord(c) < 32 or ord(c) == 127 or c in '\\:*?"<>|' for c in name)
            or any(re.fullmatch(r"(?i)(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\..*)?", p) for p in parts)):
        raise Refusal("unsafe_path")
    return name


def squashfs_region(image):
    """SquashFS v4 little-endian superblock, not a magic-only match."""
    size = image.stat().st_size
    with image.open("rb") as stream:
        scan = stream.read(min(size, 32 * 1024**2))
        candidates = []
        start = 0
        seen = False
        while True:
            offset = scan.find(b"hsqs", start)
            if offset < 0:
                break
            seen = True
            start = offset + 4
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
            if name.casefold() in names:
                raise Refusal("colliding_paths")
            names.add(name.casefold())
            meta = item.stat(follow_symlinks=False)
            entry = {"path": name, "kind": "directory", "size": 0, "sha256": None, "linkTarget": None}
            if stat.S_ISDIR(meta.st_mode):
                pending.append((Path(item.path), name + "/"))
            elif stat.S_ISREG(meta.st_mode):
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


class GzipSquashfsBackend:
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
        return output


# Explicit plugin registry. Recognition does not advertise unsupported formats.
BACKENDS = (GzipSquashfsBackend, SquashfsBackend)


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
    if not caps or int(caps[1], 16) or Path("/mnt/c").exists() or Path("/home").exists():
        raise Refusal("worker_unavailable")
    if any(name != "lo" for _, name in __import__("socket").if_nameindex()):
        raise Refusal("worker_unavailable")
    with image.open("rb") as source:
        prefix = source.read(4)
    backend = next(plugin for plugin in BACKENDS if plugin.accepts(prefix))
    filesystem_image = backend.prepare(image)
    region = squashfs_region(filesystem_image)
    root = Path("/work/tree")
    # List inside the same sandbox. Reject ambiguous text listing rather than
    # guessing names; links and special files are excluded before extraction.
    with Path("/work/listing").open("wb") as listing:
        subprocess.run(["/usr/bin/unsquashfs", "-lln", "-max-depth", "33", "-o", str(region["offset"]),
                        str(filesystem_image)], stdout=listing, stderr=subprocess.DEVNULL, check=True, timeout=20)
    with Path("/work/listing").open("rb") as listing:
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
    Path("/work/exclude").write_text("".join(e["path"] + "\n" for e in excluded))
    # Dropped capabilities prohibit device creation. Strict errors prohibit
    # publishing any partial tree after a special-file/permission failure.
    command = ["/usr/bin/unsquashfs", "-processors", "1", "-data-queue", "16", "-frag-queue", "16",
               "-strict-errors", "-no-xattrs", "-no-progress", "-max-depth", "33", "-d", str(root),
               "-no-wildcards", "-exclude-file", "/work/exclude", "-o", str(region["offset"]), str(filesystem_image)]
    with Path("/work/tool.log").open("wb") as log:
        subprocess.run(command, stdout=log, stderr=log, check=True, timeout=100)
    entries, total = collect_tree(root)
    if {e["path"] for e in entries} != {name for name, kind in observed.items() if kind in ("-", "d")}:
        raise Refusal("incomplete_tree")
    entries = sorted(entries + excluded, key=lambda e: e["path"])
    if digest(image) != expected:
        raise Refusal("source_changed")
    report = {"version": 1, "inputSha256": expected, "inputBytes": size, "region": region,
              "container": backend.name, "expandedBytes": filesystem_image.stat().st_size,
              "unextractedRegions": ([{"offset": 0, "length": region["offset"]}] if region["offset"] else []) + ([{"offset": region["offset"] + region["length"], "length": filesystem_image.stat().st_size - region["offset"] - region["length"]}] if region["offset"] + region["length"] < filesystem_image.stat().st_size else []),
              "extractor": "unsquashfs 4.7.5", "toolSha256": TOOLS["/usr/bin/unsquashfs"],
              "entries": entries, "totalBytes": total, "fileCount": sum(e["kind"] == "file" for e in entries),
              "directoryCount": sum(e["kind"] == "directory" for e in entries), "filesystemRoots": ["/"], "workerExit": 0,
              "status": "partially_extracted" if region["offset"] or region["length"] != filesystem_image.stat().st_size or excluded else "extracted_verified",
              "limitations": ["Only one SquashFS filesystem is unpacked; other image regions remain unextracted.",
                              "Links retained as metadata only; xattrs and ownership not restored.",
                              "No nested extraction, executable launch, hardware compatibility or rebuild authorization."],
              "isolation": {"network": False, "hostDrives": False, "capabilities": "0000000000000000",
                            "tmpfsBytes": 268435456, "maxBytes": MAX_BYTES, "maxEntries": MAX_ENTRIES, "maxDepth": 32}}
    raw = json.dumps(report, ensure_ascii=True, separators=(",", ":")).encode("utf-8")
    if len(raw) > MAX_MANIFEST:
        raise Refusal("manifest_limit")
    output = sys.stdout.buffer
    output.write(b"OBTX1\n" + struct.pack("<I", len(raw)) + raw)
    for entry in entries:
        if entry["kind"] == "file":
            with (root / entry["path"]).open("rb") as file:
                while chunk := file.read(65536):
                    output.write(chunk)
    output.flush()


def sandbox(script, image):
    command = ["/usr/bin/bwrap", "--unshare-all", "--unshare-user", "--die-with-parent", "--new-session", "--cap-drop", "ALL",
               "--disable-userns", "--clearenv", "--setenv", "PATH", "/usr/bin:/bin", "--setenv", "LC_ALL", "C"]
    for path in ("/usr", "/bin", "/lib", "/lib64"):
        if Path(path).exists():
            command += ["--ro-bind", path, path]
    command += ["--proc", "/proc", "--dev", "/dev", "--size", "268435456", "--tmpfs", "/work",
                "--ro-bind", str(script), "/worker.py", "--ro-bind", str(image), "/input.bin",
                "--chdir", "/work", "/usr/bin/python3", "-I", "-B", "/worker.py", "--inner"]
    return command


def supervise(job, expected, size):
    if os.getuid() == 0:
        raise Refusal("worker_unavailable")
    for name, expected_hash in TOOLS.items():
        if not Path(name).is_file() or digest(Path(name)) != expected_hash:
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
                raise Refusal(message if message in ("worker_unavailable", "corrupt_image", "unsupported_format", "unsupported_compression", "output_limit", "entry_limit", "unsafe_path", "unsafe_link", "source_changed") else "extraction_failed")
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
        except (Refusal, OSError, subprocess.SubprocessError) as error:
            code = str(error) if isinstance(error, Refusal) else "worker_unavailable"
            (args.job / "error.txt").write_text(code, encoding="ascii")
            raise SystemExit(1) from None


if __name__ == "__main__":
    try:
        main()
    except (Refusal, OSError, ValueError, subprocess.SubprocessError) as error:
        print(str(error) if isinstance(error, Refusal) else "extraction_failed", file=sys.stderr)
        raise SystemExit(1) from None
