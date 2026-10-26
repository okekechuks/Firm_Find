"""Ordinary file-byte audit only: never import or connect to SQLite.

Records computed hashes and lengths, including DB/WAL/SHM, settings, saved-device
records, logs and vault bytes. Does not infer logical integrity from these hashes.
No traversal through reparse points. No data contents or owner assertions exported.
"""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path

def reject_reparse(path):
    meta=path.lstat()
    if path.is_symlink() or getattr(meta,"st_file_attributes",0)&0x400:
        raise RuntimeError("Reparse point refused: "+str(path))
    return meta

def snapshot(root):
    reject_reparse(root)
    result={}
    pending=[root]
    while pending:
        parent=pending.pop()
        for item in sorted(parent.iterdir()):
            meta=reject_reparse(item)
            if item.is_dir():
                pending.append(item)
            elif item.is_file():
                with item.open("rb") as stream:
                    digest=hashlib.file_digest(stream,"sha256").hexdigest()
                after=item.stat()
                if (meta.st_size,meta.st_mtime_ns,meta.st_ino)!=(after.st_size,after.st_mtime_ns,after.st_ino):
                    raise RuntimeError("File changed while hashing: "+str(item))
                result[str(item.relative_to(root))]={"bytes":meta.st_size,"sha256":digest}
            else:
                raise RuntimeError("Unexpected data entry: "+str(item))
    return {"root":str(root),"files":dict(sorted(result.items()))}

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output",type=Path)
    parser.add_argument("before",nargs="?",type=Path)
    args=parser.parse_args()
    repo=Path(__file__).resolve().parent.parent
    output=args.output.resolve()
    if not output.is_relative_to(repo/".tools"):
        parser.error("Audit output must stay under repository .tools")
    buf=ctypes.create_unicode_buffer(32768)
    if ctypes.windll.shell32.SHGetFolderPathW(None,28,None,0,buf)!=0:
        raise RuntimeError("Windows LocalAppData unavailable")
    report=snapshot(Path(buf.value)/"OBTAINER")
    # A second independent byte pass detects an active writer during this audit.
    if report!=snapshot(Path(report["root"])):
        raise RuntimeError("Data changed during byte audit")
    output.write_text(json.dumps(report,indent=2)+"\n",encoding="utf-8")
    if args.before and report!=json.loads(args.before.read_text(encoding="utf-8")):
        raise RuntimeError("Data differs from before snapshot")
    print(json.dumps({"output":str(output.relative_to(repo)),"fileCount":len(report["files"]),"totalBytes":sum(v["bytes"] for v in report["files"].values())}))
if __name__=="__main__":
    main()
