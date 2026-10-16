-- Additive migration: preserve settings, inventory, audit, jobs and firmware tables.
CREATE TABLE manual_routers (
    id TEXT PRIMARY KEY,
    address TEXT NOT NULL,
    label TEXT NOT NULL,
    interface_id TEXT,
    device_id TEXT,
    created_at INTEGER NOT NULL,
    last_result TEXT
);
CREATE TABLE discovery_metadata (
    device_id TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    confidence TEXT NOT NULL,
    last_seen_at INTEGER NOT NULL
);
