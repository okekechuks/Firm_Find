"""Controlled Linux worker qualification. Records computed evidence only.

No firmware is executed; all parsing still occurs inside the production sandbox.
Evidence must stay in this repository's disposable .tools directory.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import struct
import tempfile
import threading

ROOT=Path(__file__).resolve().parent.parent
SPEC=importlib.util.spec_from_file_location("worker",ROOT/"scripts/firmware-extract-worker.py")
worker=importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(worker)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input",type=Path,required=True)
    parser.add_argument("--evidence",type=Path,required=True)
    args=parser.parse_args()
    evidence=args.evidence.resolve()
    if not evidence.is_relative_to((ROOT/".tools").resolve()):
        raise ValueError("Evidence must be in this worktree's .tools directory")
    evidence.mkdir(parents=True,exist_ok=False)
    original=worker.digest(args.input)
    record={"inputSha256":original,"inputBytes":args.input.stat().st_size}
    stop=threading.Event()
    with tempfile.TemporaryDirectory(prefix="obtainer-qualified-worker-") as directory:
        job=Path(directory)
        shutil.copyfile(args.input,job/"input.bin")
        def heartbeat():
            while not stop.is_set():
                (job/"heartbeat").write_bytes(b"alive")
                stop.wait(.1)
        thread=threading.Thread(target=heartbeat)
        thread.start()
        try:
            worker.supervise(job,original,record["inputBytes"])
            with (job/"output.bin").open("rb") as file:
                header=file.read(10)
                if header[:6]!=b"OBTX1\n":raise ValueError("Missing output framing")
                report=json.loads(file.read(struct.unpack("<I",header[6:])[0]))
                files=0
                for entry in report["entries"]:
                    if entry["kind"]!="file":continue
                    content=file.read(entry["size"])
                    if len(content)!=entry["size"] or hashlib.sha256(content).hexdigest()!=entry["sha256"]:
                        raise ValueError("Extracted bytes disagree with manifest")
                    files+=1
                if file.read(1):raise ValueError("Trailing output")
            shutil.copyfile(job/"output.bin",evidence/"contents.bundle")
            raw=json.dumps(report,sort_keys=True,indent=2).encode()
            (evidence/"worker-report.json").write_bytes(raw)
            record.update({"reportSha256":hashlib.sha256(raw).hexdigest(),"bundleSha256":worker.digest(job/"output.bin"),"fileCount":files,"directoryCount":report["directoryCount"],"totalBytes":report["totalBytes"],"container":report["container"],"region":report["region"]})
        except worker.Refusal as error:
            record["error"]=str(error)
            record["outputBytes"]=(job/"output.bin").stat().st_size if (job/"output.bin").exists() else 0
        finally:
            stop.set()
            thread.join(timeout=5)
    record["inputUnchanged"]=worker.digest(args.input)==original
    (evidence/"computed-evidence.json").write_text(json.dumps(record,indent=2)+"\n",encoding="utf-8")
    print(json.dumps(record,indent=2))
    return 1 if "error" in record else 0


if __name__=="__main__":
    raise SystemExit(main())
