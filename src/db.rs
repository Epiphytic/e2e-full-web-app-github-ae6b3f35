use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
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

    pub fn connection(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("Database mutex poisoned")
    }
}

/// Validate a SQL identifier (table name, column name) against a strict allowlist.
/// Only allows ASCII letters, digits, and underscores, starting with a letter or underscore.
pub fn validate_identifier(name: &str) -> Result<&str, String> {
    let re = regex_lite::Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap();
    if name.is_empty() {
        return Err("Identifier cannot be empty".to_string());
    }
    if !re.is_match(name) {
        return Err(format!(
            "Invalid identifier '{}': must match ^[a-zA-Z_][a-zA-Z0-9_]*$",
            name
        ));
    }
    Ok(name)
}
