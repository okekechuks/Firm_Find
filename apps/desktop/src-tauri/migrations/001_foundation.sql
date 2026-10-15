CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE audit_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    kind TEXT NOT NULL,
    message TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
-- Reserved empty tables for later milestones. No hardware or vault API exists yet.
CREATE TABLE devices (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    connected INTEGER NOT NULL DEFAULT 0 CHECK (connected IN (0, 1))
);
CREATE TABLE firmware_artifacts (
    sha256 TEXT PRIMARY KEY CHECK (length(sha256) = 64),
    size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0)
);
CREATE TABLE jobs (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled'))
);
INSERT INTO settings (key, value) VALUES ('preferences', '{"theme":"system"}');
