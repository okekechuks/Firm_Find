-- GUI state complements existing records; no original history or vault row is deleted.
ALTER TABLE artifact_details ADD COLUMN removed_at INTEGER;
CREATE TABLE artifact_presentation (
    sha256 TEXT PRIMARY KEY REFERENCES firmware_artifacts(sha256),
    display_name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('imported_original','extracted','customized','restored_default','build_output','unknown'))
);
CREATE TABLE firmware_backups (
    id TEXT PRIMARY KEY,
    artifact_sha256 TEXT NOT NULL REFERENCES firmware_artifacts(sha256),
    filename TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    verified_at INTEGER NOT NULL,
    source_kind TEXT NOT NULL,
    removed_at INTEGER
);
CREATE TABLE desktop_preferences (
    id INTEGER PRIMARY KEY CHECK(id=1),
    value TEXT NOT NULL
);
INSERT INTO desktop_preferences VALUES(1,'{"autoRefresh":true,"notifyCompleted":true,"notifyFailed":true,"notifyDevices":false,"libraryDirectory":null}');
