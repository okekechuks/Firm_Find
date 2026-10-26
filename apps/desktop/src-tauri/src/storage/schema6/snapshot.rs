//! Typed logical values, without text coercion or dependence on SQLite row enumeration.
use super::*;
use rusqlite::types::ValueRef;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Cell {
    Null,
    Integer(i64),
    Real(u64),
    Text(Vec<u8>),
    Blob(Vec<u8>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Table {
    name: String,
    sql: Option<String>,
    columns: Vec<String>,
    rows: Vec<Vec<Cell>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Snapshot {
    version: u32,
    tables: Vec<Table>,
    schema: Vec<(String, String, String, Option<String>)>,
}
fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}
impl Snapshot {
    pub(super) fn capture(connection: &Connection) -> Result<Self, StorageError> {
        Self::read(connection, None)
    }
    fn read(connection: &Connection, original: Option<&Self>) -> Result<Self, StorageError> {
        let version = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let names: Vec<(String, Option<String>)> = if let Some(old) = original {
            old.tables
                .iter()
                .map(|t| (t.name.clone(), t.sql.clone()))
                .collect()
        } else {
            connection
                .prepare("SELECT name,sql FROM sqlite_schema WHERE type='table' ORDER BY name")?
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<_, _>>()?
        };
        let mut tables = vec![];
        let mut total_bytes = 0usize;
        let mut total_rows = 0usize;
        for (name, sql) in names {
            if sql
                .as_deref()
                .is_some_and(|s| s.to_ascii_uppercase().starts_with("CREATE VIRTUAL TABLE"))
            {
                return Err(StorageError::UpgradeRefused(
                    "Virtual tables require a separate upgrade review.",
                ));
            }
            let columns: Vec<String> = if let Some(old) = original {
                old.tables
                    .iter()
                    .find(|t| t.name == name)
                    .ok_or(StorageError::UpgradeRefused("Original table is missing."))?
                    .columns
                    .clone()
            } else {
                connection
                    .prepare("SELECT name FROM pragma_table_info(?1) ORDER BY cid")?
                    .query_map([&name], |r| r.get(0))?
                    .collect::<Result<_, _>>()?
            };
            if columns.is_empty() {
                return Err(StorageError::UpgradeRefused(
                    "Original table has no readable columns.",
                ));
            }
            let query = format!(
                "SELECT {} FROM {}",
                columns
                    .iter()
                    .map(|c| quoted(c))
                    .collect::<Vec<_>>()
                    .join(","),
                quoted(&name)
            );
            let mut statement = connection.prepare(&query)?;
            let mut records = statement.query([])?;
            let mut rows = vec![];
            while let Some(row) = records.next()? {
                total_rows += 1;
                if total_rows > 1_000_000 {
                    return Err(StorageError::UpgradeRefused(
                        "Database exceeds the bounded upgrade snapshot.",
                    ));
                }
                let mut cells = vec![];
                for i in 0..columns.len() {
                    let value = row.get_ref(i)?;
                    let bytes = match value {
                        ValueRef::Text(v) | ValueRef::Blob(v) => v.len(),
                        _ => 8,
                    };
                    total_bytes = total_bytes
                        .checked_add(bytes)
                        .ok_or(StorageError::UpgradeRefused("Snapshot size overflow."))?;
                    if total_bytes > 128 * 1024 * 1024 {
                        return Err(StorageError::UpgradeRefused(
                            "Database exceeds the bounded upgrade snapshot.",
                        ));
                    }
                    cells.push(match value {
                        ValueRef::Null => Cell::Null,
                        ValueRef::Integer(v) => Cell::Integer(v),
                        ValueRef::Real(v) => Cell::Real(v.to_bits()),
                        ValueRef::Text(v) => Cell::Text(v.to_vec()),
                        ValueRef::Blob(v) => Cell::Blob(v.to_vec()),
                    });
                }
                rows.push(cells);
            }
            rows.sort();
            tables.push(Table {
                name,
                sql,
                columns,
                rows,
            });
        }
        let schema = connection
            .prepare("SELECT type,name,tbl_name,sql FROM sqlite_schema ORDER BY type,name")?
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            version,
            tables,
            schema,
        })
    }
    pub(super) fn original_values_unchanged(
        &self,
        connection: &Connection,
    ) -> Result<(), StorageError> {
        let after = Self::read(connection, Some(self))?;
        // Schema 6 is additive to the current schema-5 tables, indexes and triggers.
        // Older sources may need the already reviewed 1..5 ALTER migrations first.
        if self.version == 5
            && self
                .schema
                .iter()
                .any(|object| !after.schema.contains(object))
        {
            return Err(StorageError::UpgradeRefused(
                "Original schema changed during additive migration.",
            ));
        }
        if self.tables != after.tables {
            return Err(StorageError::UpgradeRefused(
                "Original values or IDs changed during migration.",
            ));
        }
        Ok(())
    }
}
