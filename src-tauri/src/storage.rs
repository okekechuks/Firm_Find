use crate::discovery::DiscoverySnapshot;
use crate::domain::{AuditEvent, Counts, Settings, Snapshot};
use rusqlite::{params, Connection};
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
    connection: Connection,
}

impl Repository {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > 1 {
            return Err(StorageError::NewerSchema);
        }
        if version == 0 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/001_foundation.sql"))?;
            transaction.pragma_update(None, "user_version", 1)?;
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
        transaction.execute("DELETE FROM devices", [])?;
        for device in &inventory.devices {
            transaction.execute(
                "INSERT INTO devices (id, display_name, connected) VALUES (?1, ?2, ?3)",
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
            "SELECT (SELECT COUNT(*) FROM devices WHERE connected = 1), (SELECT COUNT(*) FROM firmware_artifacts), (SELECT COUNT(*) FROM jobs WHERE status = 'running')",
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
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Theme;

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
        assert_eq!(snapshot.database_version, 1);
        assert_eq!(snapshot.settings.theme, Theme::Dark);
        assert_eq!(snapshot.recent_events.len(), 2);
        assert_eq!(snapshot.counts.devices, 0);
        assert_eq!(snapshot.counts.artifacts, 0);
        assert_eq!(snapshot.counts.running_jobs, 0);
        Ok(())
    }

    #[test]
    fn newer_database_is_rejected_without_rewriting_it() -> Result<(), Box<dyn std::error::Error>> {
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
}
