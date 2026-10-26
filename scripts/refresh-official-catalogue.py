"""Maintainer-only: build pinned catalogue from independently fingerprinted OpenWrt metadata.

Input is previously downloaded public metadata. No firmware is fetched or executed.
Runtime downloads use the compiled catalogue, never an arbitrary supplied URL/hash.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

PRIMARY = "8A8BC12F46B836C0F9CDB36F1D53D1877742E911"
SIGNER = "92C561DE55AE6552F3C736B82B0151090606D1D9"
VERSION = "25.12.6"
BASE = f"https://downloads.openwrt.org/releases/{VERSION}/targets/ath79/generic/"
PROFILES = ["tplink_archer-c7-v5", "tplink_archer-c7-v2", "ubnt_unifiac-lite"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--upstream", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    upstream = args.upstream.resolve()
    gpg = shutil.which("gpg") or "C:/Program Files/Git/usr/bin/gpg.exe"
    def gpg_path(path):
        text = Path(path).as_posix()
        if "/git/" in gpg.replace("\\", "/").lower() and Path(path).drive:
            return "/" + text[0].lower() + text[2:]
        return text
    with tempfile.TemporaryDirectory(prefix="obtainer-catalogue-gpg-") as home:
        command = [gpg, "--batch", "--no-options", "--homedir", gpg_path(home)]
        subprocess.run(command + ["--import", gpg_path(upstream / "openwrt-release-key.asc")], check=True, capture_output=True)
        checked = subprocess.run(command + ["--status-fd", "1", "--verify", gpg_path(upstream / "sha256sums.asc"), gpg_path(upstream / "sha256sums")], check=True, capture_output=True, text=True)
        valid = [line.split() for line in checked.stdout.splitlines() if line.startswith("[GNUPG:] VALIDSIG ")]
        if len(valid) != 1 or valid[0][2] != SIGNER or valid[0][-1] != PRIMARY:
            raise RuntimeError("Release signature does not match pinned OpenWrt fingerprints")
        if any(word in checked.stdout for word in ["BADSIG", "REVKEYSIG", "EXPKEYSIG", "EXPSIG"]):
            raise RuntimeError("Signature is invalid, revoked or expired")
    manifest = (upstream / "sha256sums").read_bytes()
    checksums = {}
    for line in manifest.decode("ascii").splitlines():
        digest, name = line.split(" *", 1)
        if name in checksums or len(digest) != 64:
            raise RuntimeError("Ambiguous checksum manifest")
        checksums[name] = digest
    raw_profiles = (upstream / "profiles.json").read_bytes()
    profile_hash = hashlib.sha256(raw_profiles).hexdigest()
    if checksums.get("profiles.json") != profile_hash:
        raise RuntimeError("profiles.json does not match signed checksum manifest")
    data = json.loads(raw_profiles)
    if data["version_number"] != VERSION or data["target"] != "ath79/generic":
        raise RuntimeError("Wrong release or target")
    entries = []
    for profile in PROFILES:
        item = data["profiles"][profile]
        images = [image for image in item["images"] if image["type"] == "sysupgrade" and image["filesystem"] == "squashfs"]
        if len(images) != 1:
            raise RuntimeError("No unique selected image")
        image = images[0]
        if image["sha256"] != checksums.get(image["name"]):
            raise RuntimeError("Image metadata disagrees with signed manifest")
        title = item["titles"][0]
        entries.append({
            "id": f"openwrt-{VERSION}-{profile}", "vendor": title["vendor"], "model": title["model"],
            "revision": title.get("variant", "board-profile"), "profile": profile, "target": data["target"],
            "version": VERSION, "filename": image["name"], "sha256": image["sha256"], "sizeBytes": image["size"],
            "url": BASE + image["name"], "manifestUrl": BASE + "sha256sums",
            "manifestSha256": hashlib.sha256(manifest).hexdigest(), "profilesSha256": profile_hash,
            "signingFingerprint": SIGNER, "primaryFingerprint": PRIMARY,
            "verification": "pinned_catalogue_from_verified_release_signature",
            "installability": "unknown",
        })
    output = root / "apps/desktop/src-tauri/assets/official"
    if args.check:
        if json.loads((output / "catalogue.json").read_text(encoding="utf-8")) != entries:
            raise RuntimeError("Compiled catalogue differs from signed release metadata")
        print(json.dumps({"signer": SIGNER, "manifestSha256": hashlib.sha256(manifest).hexdigest(), "profilesSha256": profile_hash, "entries": len(entries)}))
        return
    output.mkdir(parents=True, exist_ok=True)
    (output / "catalogue.json").write_text(json.dumps(entries, indent=2) + "\n", encoding="utf-8", newline="\n")
    for name in ["sha256sums", "sha256sums.asc", "openwrt-release-key.asc", "profiles.json"]:
        (output / name).write_bytes((upstream / name).read_bytes())
    print(json.dumps({"signer": SIGNER, "manifestSha256": hashlib.sha256(manifest).hexdigest(), "profilesSha256": profile_hash, "entries": len(entries)}))


if __name__ == "__main__":
    main()
