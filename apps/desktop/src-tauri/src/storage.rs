use crate::connection::{EndpointResult, SavedRouter};
use crate::discovery::DiscoverySnapshot;
use crate::domain::{AuditEvent, Counts, Settings, Snapshot};
use rusqlite::OptionalExtension;
use rusqlite::{params, Connection, OpenFlags};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Database operation failed")]
    Database(#[from] rusqlite::Error),
    #[error("Stored settings are invalid")]
    Settings(#[from] serde_json::Error),
    #[error("This database was created by a newer OBTAINER version")]
    NewerSchema,
}

pub struct Repository {
    pub(crate) connection: Connection,
}

impl Repository {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        // Read-only preflight must precede any WAL or migration write.
        // If SQLite cannot read the file, retain the existing read-write error path.
        if path.exists() {
            if let Ok(preflight) =
                Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            {
                if let Ok(version) =
                    preflight.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                {
                    if version > 5 {
                        return Err(StorageError::NewerSchema);
                    }
                }
            }
        }
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > 5 {
            return Err(StorageError::NewerSchema);
        }
        if version == 0 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/001_foundation.sql"))?;
            transaction.pragma_update(None, "user_version", 1)?;
            transaction.commit()?;
        }
        if version < 2 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/002_discovery.sql"))?;
            transaction.pragma_update(None, "user_version", 2)?;
            transaction.commit()?;
        }
        if version < 3 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/003_profiles.sql"))?;
            transaction.pragma_update(None, "user_version", 3)?;
            transaction.commit()?;
        }
        if version < 4 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/004_firmware_library.sql"))?;
            transaction.pragma_update(None, "user_version", 4)?;
            transaction.commit()?;
        }
        if version < 5 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/005_desktop_workspace.sql"))?;
            transaction.pragma_update(None, "user_version", 5)?;
            transaction.commit()?;
        }
        Ok(Self { connection })
    }

    pub fn record_startup(&mut self) -> Result<(), StorageError> {
        let transaction = self.connection.transaction()?;
        transaction.execute("UPDATE devices SET connected = 0", [])?;
        transaction.execute(
            "INSERT INTO audit_events (kind, message) VALUES ('application_started', 'Desktop workspace initialized')",
            [],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn save_discovery(&mut self, inventory: &DiscoverySnapshot) -> Result<(), StorageError> {
        use std::collections::BTreeMap;
        let before: BTreeMap<String, (String, bool)> = self
            .connection
            .prepare("SELECT id, display_name, connected FROM devices")?
            .query_map([], |row| Ok((row.get(0)?, (row.get(1)?, row.get(2)?))))?
            .collect::<Result<_, _>>()?;
        let after: BTreeMap<_, _> = inventory
            .devices
            .iter()
            .map(|device| {
                (
                    device.id.clone(),
                    (device.display_name.clone(), device.connected),
                )
            })
            .collect();
        let metadata = self.connection.transaction()?;
        for device in &inventory.devices {
            metadata.execute("INSERT INTO discovery_metadata (device_id, category, confidence, last_seen_at) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(device_id) DO UPDATE SET category=excluded.category, confidence=excluded.confidence, last_seen_at=excluded.last_seen_at",
                params![device.id, serde_json::to_string(&device.category)?, device.identity.confidence, device.last_seen_at])?;
        }
        metadata.commit()?;
        if before == after {
            return Ok(());
        }
        let arrived = after
            .iter()
            .filter(|(id, (_, connected))| {
                *connected && !before.get(*id).is_some_and(|(_, connected)| *connected)
            })
            .count();
        let removed = before
            .iter()
            .filter(|(id, (_, connected))| {
                *connected && !after.get(*id).is_some_and(|(_, connected)| *connected)
            })
            .count();
        let transaction = self.connection.transaction()?;
        transaction.execute("UPDATE devices SET connected = 0", [])?;
        for device in &inventory.devices {
            transaction.execute(
                "INSERT INTO devices (id, display_name, connected) VALUES (?1, ?2, ?3) ON CONFLICT(id) DO UPDATE SET display_name=excluded.display_name, connected=excluded.connected",
                params![device.id, device.display_name, device.connected],
            )?;
        }
        transaction.execute(
            "INSERT INTO audit_events (kind, message) VALUES ('device_inventory_changed', ?1)",
            params![format!(
                "Passive inventory updated: {arrived} added, {removed} removed"
            )],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn discovery_history(&self) -> Result<Vec<crate::discovery::Device>, StorageError> {
        let mut query = self.connection.prepare("SELECT d.id,d.display_name,COALESCE(m.last_seen_at,0),m.category FROM devices d LEFT JOIN discovery_metadata m ON m.device_id=d.id ORDER BY m.last_seen_at DESC LIMIT 512")?;
        let records = query.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, u64>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })?;
        records
            .map(|r| {
                let (id, name, seen, category) = r?;
                Ok(crate::discovery::history_device(
                    id,
                    name,
                    seen,
                    category
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or_default(),
                ))
            })
            .collect()
    }

    pub fn routers(&self) -> Result<Vec<SavedRouter>, StorageError> {
        let mut query = self.connection.prepare("SELECT id,address,label,interface_id,device_id,created_at,last_result FROM manual_routers ORDER BY created_at,id LIMIT 100")?;
        let rows = query.query_map([], |row| {
            Ok((
                SavedRouter {
                    id: row.get(0)?,
                    address: row.get(1)?,
                    label: row.get(2)?,
                    interface_id: row.get(3)?,
                    device_id: row.get(4)?,
                    created_at: row.get(5)?,
                    last_result: None,
                    // A historical response never means management is reachable in this session.
                    state: "discovered".into(),
                },
                row.get::<_, Option<String>>(6)?,
            ))
        })?;
        rows.map(|r| {
            let (mut router, result) = r?;
            router.last_result = result.map(|s| serde_json::from_str(&s)).transpose()?;
            Ok(router)
        })
        .collect()
    }
    pub fn router(&self, id: &str) -> Result<Option<SavedRouter>, StorageError> {
        Ok(self.routers()?.into_iter().find(|r| r.id == id))
    }
    pub fn save_router(&mut self, router: &SavedRouter) -> Result<(), StorageError> {
        let count: i64 =
            self.connection
                .query_row("SELECT COUNT(*) FROM manual_routers", [], |r| r.get(0))?;
        let exists = self
            .connection
            .query_row(
                "SELECT id FROM manual_routers WHERE id=?1",
                [&router.id],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .is_some();
        if count >= 100 && !exists {
            return Err(rusqlite::Error::InvalidQuery.into());
        }
        self.connection.execute("INSERT INTO manual_routers (id,address,label,interface_id,device_id,created_at) VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET label=excluded.label, device_id=excluded.device_id",
            params![router.id,router.address,router.label,router.interface_id,router.device_id,router.created_at])?;
        Ok(())
    }
    pub fn remove_router(&mut self, id: &str) -> Result<(), StorageError> {
        self.connection
            .execute("DELETE FROM manual_routers WHERE id=?1", [id])?;
        Ok(())
    }
    pub fn save_router_result(
        &mut self,
        id: &str,
        result: &EndpointResult,
    ) -> Result<bool, StorageError> {
        Ok(self.connection.execute(
            "UPDATE manual_routers SET last_result=?1 WHERE id=?2",
            params![serde_json::to_string(result)?, id],
        )? == 1)
    }

    pub fn install_registry(
        &mut self,
        registry: &mut crate::profiles::Registry,
    ) -> Result<(), StorageError> {
        let tx = self.connection.transaction()?;
        let mut accepted = vec![];
        for entry in registry.profiles.drain(..) {
            let existing: Option<String> = tx.query_row("SELECT content_hash FROM profile_versions WHERE profile_id=?1 AND profile_version=?2", params![entry.profile.profile_id,entry.profile.profile_version], |r| r.get(0)).optional()?;
            if existing
                .as_ref()
                .is_some_and(|hash| hash != &entry.content_hash)
            {
                registry.diagnostics.push(format!(
                    "{} {} changed without a content-version increment; profile quarantined.",
                    entry.profile.profile_id, entry.profile.profile_version
                ));
                continue;
            }
            tx.execute(
                "INSERT OR IGNORE INTO profile_versions VALUES(?1,?2,?3,?4,?5,?6)",
                params![
                    entry.profile.profile_id,
                    entry.profile.profile_version,
                    entry.origin,
                    entry.content_hash,
                    serde_json::to_string(&entry.profile)?,
                    crate::connection::now()
                ],
            )?;
            accepted.push(entry);
        }
        tx.commit()?;
        registry.profiles = accepted;
        registry.digest = crate::profiles::digest(&serde_json::to_vec(&registry.profiles)?);
        Ok(())
    }
    pub fn prior_compatibility(
        &self,
        id: &str,
    ) -> Result<Option<crate::profiles::CompatibilityReport>, StorageError> {
        let value: Option<String> = self
            .connection
            .query_row(
                "SELECT report FROM compatibility_evaluations WHERE device_id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()?;
        value
            .map(|v| serde_json::from_str(&v).map_err(StorageError::from))
            .transpose()
    }
    pub fn save_compatibility(
        &mut self,
        report: &crate::profiles::CompatibilityReport,
    ) -> Result<(), StorageError> {
        self.connection.execute("INSERT INTO compatibility_evaluations(device_id,input_digest,report) VALUES(?1,?2,?3) ON CONFLICT(device_id) DO UPDATE SET prior_report=CASE WHEN input_digest<>excluded.input_digest THEN report ELSE prior_report END,input_digest=excluded.input_digest,report=excluded.report WHERE input_digest<>excluded.input_digest", params![report.device_id,report.input_digest,serde_json::to_string(report)?])?;
        Ok(())
    }

    pub fn identity_observations(
        &self,
        device_id: &str,
    ) -> Result<Vec<crate::profiles::EvidenceRecord>, StorageError> {
        let mut statement = self.connection.prepare("SELECT record FROM device_identity_observations WHERE device_id=?1 ORDER BY id LIMIT 32")?;
        let rows = statement.query_map([device_id], |r| r.get::<_, String>(0))?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
    pub fn save_identity_observation(
        &mut self,
        record: &crate::profiles::EvidenceRecord,
    ) -> Result<bool, StorageError> {
        // Keep bounded, append-only user evidence; duplicates retain the original observation time.
        let count: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM device_identity_observations WHERE device_id=?1",
            [&record.subject_id],
            |r| r.get(0),
        )?;
        if count >= 32 {
            return Ok(self.connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM device_identity_observations WHERE id=?1)",
                [&record.id],
                |r| r.get(0),
            )?);
        }
        self.connection.execute(
            "INSERT OR IGNORE INTO device_identity_observations VALUES(?1,?2,?3)",
            params![record.id, record.subject_id, serde_json::to_string(record)?],
        )?;
        Ok(true)
    }

    pub fn settings(&self) -> Result<Settings, StorageError> {
        let json: String = self.connection.query_row(
            "SELECT value FROM settings WHERE key = 'preferences'",
            [],
            |row| row.get(0),
        )?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn save_settings(&mut self, settings: &Settings) -> Result<(), StorageError> {
        let value = serde_json::to_string(settings)?;
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "UPDATE settings SET value = ?1 WHERE key = 'preferences'",
            params![value],
        )?;
        transaction.execute(
            "INSERT INTO audit_events (kind, message) VALUES ('settings_saved', 'Appearance preferences saved')",
            [],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn snapshot(&self, directory: &Path) -> Result<Snapshot, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, kind, message, created_at FROM audit_events ORDER BY id DESC LIMIT 20",
        )?;
        let events = statement.query_map([], |row| {
            Ok(AuditEvent {
                id: row.get(0)?,
                kind: row.get(1)?,
                message: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        let counts = self.connection.query_row(
            "SELECT (SELECT COUNT(*) FROM devices WHERE connected = 1), (SELECT COUNT(*) FROM firmware_artifacts a JOIN artifact_details d USING(sha256) WHERE d.removed_at IS NULL), (SELECT COUNT(*) FROM jobs WHERE status IN ('pending','running'))",
            [],
            |row| Ok(Counts { devices: row.get(0)?, artifacts: row.get(1)?, running_jobs: row.get(2)? }),
        )?;
        Ok(Snapshot {
            version: env!("CARGO_PKG_VERSION").to_string(),
            database_version: self
                .connection
                .query_row("PRAGMA user_version", [], |row| row.get(0))?,
            data_directory: directory.to_string_lossy().into_owned(),
            settings: self.settings()?,
            counts,
            recent_events: events.collect::<Result<Vec<_>, _>>()?,
            test_profile: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Theme;

    #[test]
    fn schema_two_upgrade_preserves_data_and_phase_three_history_across_restarts(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("phase2.db");
        let connection = Connection::open(&path)?;
        connection.execute_batch(include_str!("../migrations/001_foundation.sql"))?;
        connection.execute_batch(include_str!("../migrations/002_discovery.sql"))?;
        connection.execute_batch("PRAGMA user_version=2; UPDATE settings SET value='{\"theme\":\"dark\"}'; INSERT INTO devices VALUES('preserved-device','Existing MiFi',1); INSERT INTO manual_routers VALUES('preserved-router','http://192.168.100.1/','Owner label',NULL,'preserved-device',1,NULL);")?;
        drop(connection);
        let mut repo = Repository::open(&path)?;
        let settings = repo.settings()?;
        let routers = repo.routers()?;
        let mut registry = crate::profiles::Registry::bundled();
        repo.install_registry(&mut registry)?;
        let observation = crate::profiles::EvidenceRecord::user_reported(
            "preserved-device".into(),
            crate::profiles::Attribute::ReportedIdentifier,
            "LT800/".into(),
            123,
        )
        .unwrap();
        assert!(repo.save_identity_observation(&observation)?);
        assert!(repo.save_identity_observation(&observation)?);
        assert_eq!(repo.identity_observations("preserved-device")?.len(), 1);
        let d =
            crate::discovery::replay_fixture(include_str!("../fixtures/discovery-lifecycle.json"))
                .unwrap()[1]
                .devices[0]
                .clone();
        let report = crate::profiles::evaluate(
            &registry,
            &d,
            crate::profiles::gather_evidence(&d, &[]),
            123,
            1,
        );
        repo.save_compatibility(&report)?;
        let mut changed = report.clone();
        changed.input_digest = "changed".into();
        crate::profiles::record_change(&mut changed, &report);
        repo.save_compatibility(&changed)?;
        // Same inputs do not rewrite stored history on every render.
        repo.save_compatibility(&changed)?;
        drop(repo);
        let mut repo = Repository::open(&path)?;
        assert_eq!(
            serde_json::to_string(&repo.settings()?)?,
            serde_json::to_string(&settings)?
        );
        assert_eq!(
            serde_json::to_string(&repo.routers()?)?,
            serde_json::to_string(&routers)?
        );
        assert_eq!(
            repo.identity_observations("preserved-device")?[0],
            observation
        );
        assert!(repo.prior_compatibility(&d.id)?.unwrap().change.is_some());
        let previous: String = repo.connection.query_row(
            "SELECT prior_report FROM compatibility_evaluations WHERE device_id=?1",
            [&d.id],
            |r| r.get(0),
        )?;
        assert_eq!(
            serde_json::from_str::<crate::profiles::CompatibilityReport>(&previous)?.input_digest,
            report.input_digest
        );
        let mut changed_registry = registry.clone();
        changed_registry.profiles[0].content_hash = "changed-without-version".into();
        repo.install_registry(&mut changed_registry)?;
        assert_eq!(changed_registry.profiles.len(), 1);
        assert!(changed_registry
            .diagnostics
            .iter()
            .any(|d| d.contains("quarantined")));
        assert_eq!(
            repo.connection
                .query_row("SELECT COUNT(*) FROM profile_versions", [], |r| r
                    .get::<_, i64>(0))?,
            2
        );
        assert_eq!(
            repo.connection
                .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))?,
            "ok"
        );
        Ok(())
    }

    #[test]
    fn additive_migration_preserves_existing_nonempty_tables(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("old.db");
        let connection = Connection::open(&path)?;
        connection.execute_batch(include_str!("../migrations/001_foundation.sql"))?;
        connection.execute_batch("PRAGMA user_version=1; UPDATE settings SET value='{\"theme\":\"dark\"}'; INSERT INTO devices VALUES ('legacy','Existing device',1); INSERT INTO audit_events(kind,message) VALUES('existing','Preserve'); INSERT INTO jobs VALUES('job','existing','completed');")?;
        connection.execute(
            "INSERT INTO firmware_artifacts VALUES(?1,123)",
            ["a".repeat(64)],
        )?;
        drop(connection);
        let mut repository = Repository::open(&path)?;
        assert_eq!(repository.settings()?.theme, Theme::Dark);
        repository.record_startup()?;
        repository.save_discovery(&DiscoverySnapshot::default())?;
        let snapshot = repository.snapshot(temporary.path())?;
        assert_eq!(snapshot.database_version, 5);
        assert_eq!(snapshot.counts.artifacts, 1);
        assert!(snapshot.recent_events.iter().any(|e| e.kind == "existing"));
        assert_eq!(
            repository.connection.query_row(
                "SELECT COUNT(*) FROM devices WHERE id='legacy'",
                [],
                |r| r.get::<_, i64>(0)
            )?,
            1
        );
        assert_eq!(
            repository
                .connection
                .query_row("SELECT COUNT(*) FROM jobs", [], |r| r.get::<_, i64>(0))?,
            1
        );
        Ok(())
    }

    #[test]
    fn migration_and_settings_survive_reopening() -> Result<(), Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("obtainer.db");
        {
            let mut repository = Repository::open(&path)?;
            repository.record_startup()?;
            repository.save_settings(&Settings { theme: Theme::Dark })?;
        }
        let repository = Repository::open(&path)?;
        let snapshot = repository.snapshot(temporary.path())?;
        assert_eq!(snapshot.database_version, 5);
        assert_eq!(snapshot.settings.theme, Theme::Dark);
        assert_eq!(snapshot.recent_events.len(), 2);
        assert_eq!(snapshot.counts.devices, 0);
        assert_eq!(snapshot.counts.artifacts, 0);
        assert_eq!(snapshot.counts.running_jobs, 0);
        Ok(())
    }

    #[test]
    fn newer_database_is_rejected_without_changing_its_schema_version(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("newer.db");
        let connection = Connection::open(&path)?;
        connection.pragma_update(None, "user_version", 99)?;
        drop(connection);
        assert!(matches!(
            Repository::open(&path),
            Err(StorageError::NewerSchema)
        ));
        let connection = Connection::open(path)?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        assert_eq!(version, 99);
        Ok(())
    }

    #[test]
    fn d6_newer_schema_is_refused_without_mutation() -> Result<(), Box<dyn std::error::Error>> {
        use rusqlite::OpenFlags;
        use std::fs;
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("schema-six.db");
        let connection = Connection::open(&path)?;
        connection.pragma_update(None, "user_version", 6)?;
        drop(connection);

        let before_bytes = fs::read(&path)?;
        let before_mode = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)?
            .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))?;
        assert!(matches!(
            Repository::open(&path),
            Err(StorageError::NewerSchema)
        ));
        let after_bytes = fs::read(&path)?;
        let after_mode = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)?
            .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))?;

        assert_eq!(before_mode, "delete");
        assert_eq!(after_mode, "delete");
        assert_eq!(before_bytes, after_bytes);
        assert!(!Path::new(&format!("{}-wal", path.display())).exists());
        assert!(!Path::new(&format!("{}-shm", path.display())).exists());
        Ok(())
    }
    #[test]
    fn d6_supported_database_keeps_wal_and_existing_rows() -> Result<(), Box<dyn std::error::Error>>
    {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("supported.db");
        let mut repo = Repository::open(&path)?;
        repo.save_settings(&Settings { theme: Theme::Dark })?;
        drop(repo);
        let c = Connection::open(&path)?;
        c.execute_batch("PRAGMA journal_mode=DELETE;")?;
        drop(c);
        let repo = Repository::open(&path)?;
        assert_eq!(repo.settings()?.theme, Theme::Dark);
        assert_eq!(
            repo.connection
                .query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))?,
            "wal"
        );
        assert_eq!(
            repo.connection
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))?,
            5
        );
        Ok(())
    }
    #[test]
    fn d6_invalid_file_preserves_existing_error_mapping() -> Result<(), Box<dyn std::error::Error>>
    {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("invalid.db");
        std::fs::write(&path, b"this is not SQLite")?;
        assert!(
            matches!(Repository::open(&path), Err(StorageError::Database(rusqlite::Error::SqliteFailure(e, _))) if e.code == rusqlite::ErrorCode::NotADatabase)
        );
        Ok(())
    }
    #[test]
    fn d6_locked_file_preserves_existing_error_mapping() -> Result<(), Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("locked.db");
        let c = Connection::open(&path)?;
        c.execute_batch("CREATE TABLE retained(value); BEGIN EXCLUSIVE;")?;
        assert!(
            matches!(Repository::open(&path), Err(StorageError::Database(rusqlite::Error::SqliteFailure(e, _))) if e.code == rusqlite::ErrorCode::DatabaseBusy)
        );
        c.execute_batch("ROLLBACK;")?;
        Ok(())
    }
}
