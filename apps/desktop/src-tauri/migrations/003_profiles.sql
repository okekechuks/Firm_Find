CREATE TABLE profile_versions (
    profile_id TEXT NOT NULL,
    profile_version TEXT NOT NULL,
    origin TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    document TEXT NOT NULL,
    installed_at INTEGER NOT NULL,
    PRIMARY KEY (profile_id, profile_version)
);
CREATE TABLE compatibility_evaluations (
    device_id TEXT PRIMARY KEY,
    input_digest TEXT NOT NULL,
    report TEXT NOT NULL,
    prior_report TEXT
);
CREATE TABLE device_identity_observations (
    id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL,
    record TEXT NOT NULL
);
CREATE INDEX identity_observations_by_device ON device_identity_observations(device_id);
