"""Harmless schema-5 fixture for a fresh, marked native-test run. Never normal data."""
import argparse
import ctypes
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import sqlite3
import sys

sys.dont_write_bytecode = True
REPO = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("byte_audit", REPO / "scripts/audit-data-bytes.py")
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=["create", "snapshot", "inspect"])
    parser.add_argument("run_id")
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{32}", args.run_id):
        parser.error("native-test run ID required")
    buffer = ctypes.create_unicode_buffer(32768)
    if ctypes.windll.shell32.SHGetFolderPathW(None, 28, None, 0, buffer) != 0:
        raise RuntimeError("LocalAppData unavailable")
    root = Path(buffer.value) / "OBTAINER-NativeTests" / args.run_id
    for parent in reversed([root, *root.parents]):
        if parent.exists():
            audit.reject_reparse(parent)
    data = root / "data"
    marker_path = root / ".obtainer-native-test.json"
    marker = {"version": 1, "kind": "OBTAINER_NATIVE_TEST", "run_id": args.run_id, "state": "active"}
    if args.operation == "create":
        root.mkdir(parents=True, exist_ok=False)
        marker_path.write_text(json.dumps(marker), encoding="utf-8")
        for child in ["data/logs", "data/vault", "data/workspaces", "data/reports", "webview", "fixtures", "exports"]:
            (root / child).mkdir(parents=True, exist_ok=False)
        payload = b"OBTAINER harmless migration fixture\n"
        digest = hashlib.sha256(payload).hexdigest()
        storage = f"sha256/{digest[:2]}/{digest}.bin"
        blob = data / "vault" / storage
        blob.parent.mkdir(parents=True)
        blob.write_bytes(payload)
        connection = sqlite3.connect(data / "obtainer.db")
        try:
            connection.execute("PRAGMA foreign_keys=ON")
            for name in ["001_foundation.sql", "002_discovery.sql", "003_profiles.sql", "004_firmware_library.sql", "005_desktop_workspace.sql"]:
                connection.executescript((REPO / "apps/desktop/src-tauri/migrations" / name).read_text(encoding="utf-8"))
            connection.execute("UPDATE settings SET value=?", (json.dumps({"theme": "dark"}),))
            connection.execute("INSERT INTO devices VALUES('fixture-device','Synthetic migration fixture',1)")
            connection.execute("INSERT INTO jobs VALUES('fixture-job','firmware_import','completed')")
            connection.execute("INSERT INTO firmware_artifacts VALUES(?,?)", (digest, len(payload)))
            connection.execute("INSERT INTO artifact_details(sha256,storage_key,integrity_status,first_imported_at,last_verified_at) VALUES(?,?,'verified',123,456)", (digest, storage))
            connection.execute("INSERT INTO artifact_imports VALUES('fixture-import',?,'harmless-migration-fixture.bin',123,'user_import','unverified',NULL,'fixture-device','fixture-job')", (digest,))
            connection.execute("PRAGMA user_version=5")
            connection.commit()
        finally:
            connection.close()
    audit.reject_reparse(marker_path)
    if json.loads(marker_path.read_text(encoding="utf-8")) != marker:
        raise RuntimeError("native-test ownership mismatch")
    if args.operation in ["create", "snapshot"]:
        result = audit.snapshot(data)
        result["directories"] = sorted(str(p.relative_to(data)) for p in data.rglob("*") if p.is_dir())
    else:
        audit.snapshot(data)  # Refuse any linked/reparse item before opening fixture SQLite.
        connection = sqlite3.connect((data / "obtainer.db").as_uri() + "?mode=ro", uri=True)
        try:
            version = connection.execute("PRAGMA user_version").fetchone()[0]
            theme = json.loads(connection.execute("SELECT value FROM settings WHERE key='preferences'").fetchone()[0])["theme"]
            artifact = connection.execute("SELECT a.sha256,a.size_bytes,f.artifact_class FROM firmware_artifacts a JOIN firmware_admissions f USING(sha256)").fetchone()
            receipt = connection.execute("SELECT backup_filename,backup_sha256,backup_bytes FROM artifact_schema_receipt WHERE schema_version=6").fetchone()
            if not re.fullmatch(r"schema-before-6-[0-9a-f]{32}\.sqlite", receipt[0]):
                raise RuntimeError("unexpected fixture backup name")
            backup = data / "backups" / receipt[0]
            audit.reject_reparse(backup)
            backup_hash = hashlib.sha256(backup.read_bytes()).hexdigest()
            if (backup_hash, backup.stat().st_size) != (receipt[1], receipt[2]):
                raise RuntimeError("backup differs from migration receipt")
            copy = sqlite3.connect(backup.as_uri() + "?mode=ro&immutable=1", uri=True)
            try:
                backup_version = copy.execute("PRAGMA user_version").fetchone()[0]
                backup_artifact = copy.execute("SELECT sha256,size_bytes FROM firmware_artifacts").fetchone()
                backup_integrity = copy.execute("PRAGMA integrity_check").fetchall()
                backup_fk = copy.execute("PRAGMA foreign_key_check").fetchall()
            finally:
                copy.close()
            blob = data / "vault" / f"sha256/{artifact[0][:2]}/{artifact[0]}.bin"
            audit.reject_reparse(blob)
            result = {"version": version, "theme": theme, "artifact": artifact, "backupArtifact": backup_artifact,
                      "backupVersion": backup_version, "backupIntegrity": backup_integrity, "backupForeignKeys": backup_fk,
                      "backupSha256": backup_hash, "blobSha256": hashlib.sha256(blob.read_bytes()).hexdigest()}
        finally:
            connection.close()
    print(json.dumps(result))


if __name__ == "__main__":
    main()
