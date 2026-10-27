"""Verify already downloaded public TRX qualification images; never contacts a router.

Produces computed hashes/signature identities only. Source filenames and exact
expected hashes are test fixtures, not hardware support or acquisition rules.
"""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
FIXTURES = ROOT / ".tools/trx"
PRIMARY = "8A8BC12F46B836C0F9CDB36F1D53D1877742E911"
SIGNER = "92C561DE55AE6552F3C736B82B0151090606D1D9"
SOURCES = [
    ("sha256sums-bcm47xx", "sha256sums-bcm47xx.asc", "bcm47xx/mips74k", "openwrt-25.12.6-bcm47xx-mips74k-asus_rt-n16-squashfs.trx", "0f29bd64f7a8f931cf1121dd54b514f1ecb58486ffa1713a282c97928c703d5d"),
    ("sha256sums", "sha256sums.asc", "bcm53xx/generic", "openwrt-25.12.6-bcm53xx-generic-asus_rt-ac68u-squashfs.trx", "1827a286bc03df7a83ccd93e84cb1375cc1e10c1857ded3eb9c0ad413f761d36"),
]


def main():
    gpg = shutil.which("gpg") or "C:/Program Files/Git/usr/bin/gpg.exe"
    def tool_path(path):
        text=Path(path).as_posix()
        return "/"+text[0].lower()+text[2:] if "/git/" in gpg.replace("\\","/").lower() and Path(path).drive else text
    rows=[]
    with tempfile.TemporaryDirectory(prefix="obtainer-trx-signatures-") as home:
        command=[gpg,"--batch","--no-options","--homedir",tool_path(home)]
        subprocess.run(command+["--import",tool_path(ROOT/"apps/desktop/src-tauri/assets/official/openwrt-release-key.asc")],check=True,capture_output=True,timeout=30)
        for manifest_name,signature_name,target,filename,expected in SOURCES:
            manifest=FIXTURES/manifest_name
            checked=subprocess.run(command+["--status-fd","1","--verify",tool_path(FIXTURES/signature_name),tool_path(manifest)],check=True,capture_output=True,text=True,timeout=30)
            signatures=[line.split() for line in checked.stdout.splitlines() if line.startswith("[GNUPG:] VALIDSIG ")]
            if len(signatures)!=1 or signatures[0][2]!=SIGNER or signatures[0][-1]!=PRIMARY or any(word in checked.stdout for word in ["BADSIG","REVKEYSIG","EXPKEYSIG","EXPSIG"]):
                raise ValueError("Signature does not meet the pinned release policy")
            matches=[line.split(" *",1)[0] for line in manifest.read_text("ascii").splitlines() if line.endswith(" *"+filename)]
            image=FIXTURES/filename
            digest=hashlib.sha256(image.read_bytes()).hexdigest()
            if matches!=[expected] or digest!=expected:
                raise ValueError("Fixture does not match the exact signed release entry")
            rows.append({"filename":filename,"url":"https://downloads.openwrt.org/releases/25.12.6/targets/"+target+"/"+filename,
                         "sha256":digest,"bytes":image.stat().st_size,"manifestSha256":hashlib.sha256(manifest.read_bytes()).hexdigest(),"signer":SIGNER,"primary":PRIMARY})
    (FIXTURES/"verified-fixtures.json").write_text(json.dumps(rows,indent=2)+"\n",encoding="utf-8")
    print(json.dumps(rows,indent=2))


if __name__=="__main__":
    main()
