"""Record repeated repository Rust commands, their revision and computed results.

Only logs and computed JSON are written, under a new directory in .tools/.
This program never edits documentation or supplies review/owner verdicts.
"""

import argparse
import hashlib
import json
import re
import subprocess
import time
from pathlib import Path


def git(root, *args):
    return subprocess.check_output(["git", *args], cwd=root, text=True).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", type=int, default=20)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.runs < 1:
        parser.error("--runs must be positive")
    root = Path(__file__).resolve().parent.parent
    output = (root / args.output).resolve()
    if not output.is_relative_to(root / ".tools"):
        parser.error("--output must be a new directory inside the repository .tools/")
    initial = git(root, "rev-parse", "HEAD")
    if git(root, "status", "--porcelain", "--untracked-files=no"):
        parser.error("tracked working tree must match HEAD")
    output.mkdir(parents=True, exist_ok=False)
    command = [
        "powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
        "scripts/rust.ps1", "test", "--workspace", "--locked", "--offline",
    ]
    summary = {
        "commit": initial,
        "branch": git(root, "branch", "--show-current"),
        "command": command,
        "requestedRuns": args.runs,
        "runs": [],
    }
    result_path = output / "results.json"
    for number in range(1, args.runs + 1):
        before = git(root, "rev-parse", "HEAD")
        dirty_before = bool(git(root, "status", "--porcelain", "--untracked-files=no"))
        if before != initial or dirty_before:
            return 2
        log = output / f"run-{number:02}.log"
        started = time.monotonic()
        with log.open("wb") as stream:
            completed = subprocess.run(command, cwd=root, stdout=stream, stderr=subprocess.STDOUT)
        raw = log.read_bytes()
        text = raw.decode("utf-8", errors="replace")
        counts = [
            {key: int(value) for key, value in zip(
                ["passed", "failed", "ignored", "measured", "filteredOut"], match
            )}
            for match in re.findall(
                r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out",
                text,
            )
        ]
        after = git(root, "rev-parse", "HEAD")
        dirty_after = bool(git(root, "status", "--porcelain", "--untracked-files=no"))
        summary["runs"].append({
            "number": number,
            "commitBefore": before,
            "commitAfter": after,
            "trackedChangesBefore": dirty_before,
            "trackedChangesAfter": dirty_after,
            "exitCode": completed.returncode,
            "elapsedSeconds": round(time.monotonic() - started, 3),
            "testResults": counts,
            "log": str(log.relative_to(root)),
            "logSha256": hashlib.sha256(raw).hexdigest(),
        })
        result_path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(summary["runs"][-1]), flush=True)
        if completed.returncode or after != initial or dirty_after or not counts:
            return completed.returncode or 2
    print(str(result_path.relative_to(root)), flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
