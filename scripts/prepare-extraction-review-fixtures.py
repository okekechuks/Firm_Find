"""Create explicitly synthetic native regression inputs in owned .tools storage."""
import gzip
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT=Path(__file__).resolve().parent.parent


def main():
    destination=ROOT/".tools/review-fixtures"
    destination.mkdir(parents=True,exist_ok=False)
    with tempfile.TemporaryDirectory(prefix="obtainer-review-fixtures-") as directory:
        base=Path(directory)
        large=base/"large"
        large.mkdir()
        for i in range(4):
            with (large/str(i)).open("wb") as file:file.truncate(32*1024**2)
        image=base/"large.sqfs"
        subprocess.run(["mksquashfs",str(large),str(image),"-noappend","-processors","1","-quiet"],check=True)
        with image.open("r+b") as file:file.truncate(120*1024**2)
        with image.open("rb") as source,gzip.open(destination/"synthetic-resource-limit.gz","wb") as output:
            shutil.copyfileobj(source,output,65536)
        names=base/"names"
        names.mkdir()
        for name in ["a"*101,"café","resume\u0301","safe.txt"]:
            (names/name).write_text("Synthetic export fixture\n",encoding="utf-8")
        (names/"inert-link").symlink_to("/etc/shadow")
        os.mkfifo(names/"inert-pipe")
        subprocess.run(["mksquashfs",str(names),str(destination/"synthetic-export-names.sqfs"),"-noappend","-processors","1","-quiet"],check=True)
    records=[{"filename":file.name,"bytes":file.stat().st_size,"sha256":hashlib.sha256(file.read_bytes()).hexdigest(),"synthetic":True} for file in sorted(destination.iterdir()) if file.is_file()]
    (destination/"computed-fixtures.json").write_text(json.dumps(records,indent=2)+"\n",encoding="utf-8")
    print(json.dumps(records,indent=2))


if __name__=="__main__":main()
