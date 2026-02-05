mod columns;
mod rows;
mod tables;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

pub use columns::*;
pub use rows::*;
pub use tables::*;

pub struct Database {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub col_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColumnInfo {
    pub cid: i64,
    pub name: String,
    pub col_type: String,
    pub notnull: bool,
    pub default_value: Option<String>,
    pub pk: bool,
}

impl Database {
    pub fn new(path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn new_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn connection(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("Database mutex poisoned")
    }

    pub fn list_tables(&self) -> Result<Vec<String>, rusqlite::Error> {
        let conn = self.connection();
        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )?;
        let tables = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tables)
    }
}

/// Validate a SQL identifier (table name, column name) against a strict allowlist.
/// Only allows ASCII letters, digits, and underscores, starting with a letter or underscore.
pub fn validate_identifier(name: &str) -> Result<&str, String> {
    if name.is_empty() {
        return Err("Identifier cannot be empty".to_string());
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => {
            return Err(format!(
                "Invalid identifier '{}': must start with a letter or underscore",
                name
            ));
        }
    }
    for c in chars {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return Err(format!(
                "Invalid identifier '{}': must match ^[a-zA-Z_][a-zA-Z0-9_]*$",
                name
            ));
        }
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_identifier_valid() {
        assert!(validate_identifier("users").is_ok());
        assert!(validate_identifier("_private").is_ok());
        assert!(validate_identifier("Table123").is_ok());
        assert!(validate_identifier("a").is_ok());
        assert!(validate_identifier("my_table_name").is_ok());
    }

    #[test]
    fn test_validate_identifier_empty() {
        assert!(validate_identifier("").is_err());
    }

    #[test]
    fn test_validate_identifier_sql_injection() {
        assert!(validate_identifier("users; DROP TABLE").is_err());
        assert!(validate_identifier("col\"name").is_err());
        assert!(validate_identifier("col'name").is_err());
        assert!(validate_identifier("table--comment").is_err());
        assert!(validate_identifier("1starts_with_number").is_err());
        assert!(validate_identifier("col name").is_err());
        assert!(validate_identifier("col.name").is_err());
    }

    #[test]
    fn test_validate_identifier_unicode() {
        assert!(validate_identifier("tëst").is_err());
        assert!(validate_identifier("\u{540d}\u{524d}").is_err());
        assert!(validate_identifier("col\u{200B}name").is_err());
    }

    #[test]
    fn test_database_init_wal_mode() {
        let db = Database::new_in_memory().unwrap();
        let conn = db.connection();
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert!(mode == "wal" || mode == "memory");
    }

    #[test]
    fn test_list_tables_empty() {
        let db = Database::new_in_memory().unwrap();
        let tables = db.list_tables().unwrap();
        assert!(tables.is_empty());
    }
}
