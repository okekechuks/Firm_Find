"""Retain computed test output as normalized UTF-8 text, never invent verdicts.

No raw capture, firmware, screenshot or normal-data snapshot is copied. The
explicit list is deliberate; this is not a recursive evidence-directory upload.
"""
import argparse
import hashlib
import json
import re
from pathlib import Path

BASE = [
    "har-static-strategy-red.log", "integration-red.log", "integration-green.log",
    "structure-rust-red.log", "structure-rust-green.log", "frontend-red.log",
    "route-ui-red.log", "clippy-standard.log", "loopback-source-attempt-1.log",
    "loopback-source-green.log", "rust-standard-final-3.log",
    "rust-isolated-final-3.log", "clippy-standard-final-3.log",
    "clippy-isolated-final-3.log", "format-final-3.log",
    "frontend-full-final.log", "frontend-build.log", "lint-scripts-final.log",
    "extraction-legacy-linux.log", "extraction-v2-linux.log",
]
NATIVE = [
    "production-native-build.log", "isolated-native-build.log",
    "web-management-native.log", "manufacturer-native.log",
    "universal-native.log", "adapters-simulator-native.log",
    "extraction-review-native.log", "lint-native-final.log",
]
SECURITY = [
    "ci-qualified-initial.log", "python-runtime-qualified.log",
    "python-update-pin-red.log", "python-update-legacy-green.log",
    "python-update-v2-green.log", "python-update-dns-green.log",
    "rust-standard-security.log", "rust-isolated-security.log",
    "clippy-standard-security.log", "clippy-isolated-security.log", "format-security.log",
    "manufacturer-native-initial-ui-race.log",
    "web-management-native-initial-lock-audit.log",
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--native", action="store_true")
    parser.add_argument("--security", action="store_true")
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    source = repo / ".tools/web-management-evidence"
    destination = repo / "docs/evidence/web-management-milestone"
    destination.mkdir(parents=True, exist_ok=True)
    records = []
    for name in BASE + (SECURITY if args.security else []) + (NATIVE if args.native else []):
        item = source / name
        if item.is_symlink() or not item.resolve().is_relative_to(source.resolve()):
            raise RuntimeError("Evidence must be an ordinary owned local log")
        raw = item.read_bytes()
        text = raw.decode("utf-16") if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else raw.decode("utf-8-sig")
        # Remove terminal color codes and padding, retaining both original and
        # normalized hashes so this presentation conversion is explicit.
        plain = re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", text)
        normalized = "\n".join(line.rstrip() for line in plain.splitlines()).rstrip()
        retained = (normalized + "\n").encode("utf-8") if normalized else b""
        output = destination / (item.stem + ".txt")
        output.write_bytes(retained)
        records.append({"source": str(item.relative_to(repo)).replace("\\", "/"),
                        "retained": output.name, "sourceSha256": hashlib.sha256(raw).hexdigest(),
                        "retainedSha256": hashlib.sha256(retained).hexdigest(),
                        "retainedBytes": len(retained)})
    (destination / "index.json").write_text(json.dumps({"version": 1, "logs": records}, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"logs": len(records), "bytes": sum(row["retainedBytes"] for row in records)}))


if __name__ == "__main__":
    main()
