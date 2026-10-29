"""Compute signed-index/package/interpreter evidence; never produce a verdict.

Run only on the qualified Ubuntu development host after apt-get update. APT
validates its signed indices. This checks the downloaded archive against the
index, then reads the interpreter member without extracting/executing it.
"""
import argparse
import hashlib
import json
from pathlib import Path
import stat
import subprocess
import tarfile

VERSION = "3.14.4-1ubuntu0.2"
PACKAGE = "python3.14-minimal"


def digest(stream):
    return hashlib.file_digest(stream, "sha256").hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--require-installed", action="store_true")
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    metadata = subprocess.check_output(["/usr/bin/apt-cache", "show", f"{PACKAGE}={VERSION}"], text=True)
    records = [dict(line.split(": ", 1) for line in part.splitlines() if ": " in line and not line.startswith(" ")) for part in metadata.strip().split("\n\n")]
    archive_hashes = {row["SHA256"] for row in records if row.get("Version") == VERSION}
    if len(archive_hashes) != 1:
        raise RuntimeError("Ambiguous or missing authenticated package metadata")
    archive = repo / ".tools/python-security-qualification" / f"{PACKAGE}_{VERSION}_amd64.deb"
    with archive.open("rb") as stream:
        actual_archive = digest(stream)
    if actual_archive not in archive_hashes:
        raise RuntimeError("Downloaded package does not match the APT index")
    process = subprocess.Popen(["/usr/bin/dpkg-deb", "--fsys-tarfile", str(archive)], stdout=subprocess.PIPE)
    binary_hash = None
    try:
        with tarfile.open(fileobj=process.stdout, mode="r|") as members:
            for member in members:
                if member.name.removeprefix("./") != "usr/bin/python3.14":
                    continue
                if binary_hash or not member.isfile() or not 0 < member.size <= 32 * 1024**2 or member.uid != 0 or member.mode & 0o6022:
                    raise RuntimeError("Unexpected interpreter package member")
                with members.extractfile(member) as stream:
                    binary_hash = digest(stream)
        if process.wait(timeout=5) != 0 or binary_hash is None:
            raise RuntimeError("Cannot inspect authenticated interpreter archive")
    finally:
        process.stdout.close()
        if process.poll() is None:
            process.kill()
        process.wait(timeout=5)
    interpreter = Path("/usr/bin/python3.14")
    mode = interpreter.lstat()
    if not stat.S_ISREG(mode.st_mode) or mode.st_uid != 0 or mode.st_mode & 0o6022:
        raise RuntimeError("Installed interpreter ownership/mode is unsafe")
    with interpreter.open("rb") as stream:
        installed_hash = digest(stream)
    packages = subprocess.check_output(["/usr/bin/dpkg-query", "-W", "python3.14-minimal", "python3.14", "libpython3.14-minimal", "libpython3.14-stdlib"], text=True)
    result = {"package": PACKAGE, "version": VERSION, "archiveSha256": actual_archive,
              "archiveBytes": archive.stat().st_size, "archiveInterpreterSha256": binary_hash,
              "installedInterpreterSha256": installed_hash, "interpreter": str(interpreter),
              "installedPackages": dict(line.split("\t", 1) for line in packages.splitlines()),
              "installedMatchesArchive": installed_hash == binary_hash}
    if args.require_installed and (installed_hash != binary_hash or any(version != VERSION for version in result["installedPackages"].values())):
        raise RuntimeError("Installed runtime does not match the authenticated update")
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
