use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

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

    // --- Table listing ---

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

    // --- Table-level schema operations ---

    pub fn create_table(&self, name: &str, columns: &[ColumnDef]) -> Result<(), String> {
        validate_identifier(name)?;
        if columns.is_empty() {
            return Err("At least one column is required".to_string());
        }

        let col_defs: Vec<String> = columns
            .iter()
            .map(|col| {
                validate_identifier(&col.name)?;
                let mut def = format!("\"{}\" {}", col.name, col.col_type);
                if !col.nullable {
                    def.push_str(" NOT NULL");
                }
                if let Some(ref default) = col.default_value {
                    def.push_str(&format!(" DEFAULT {}", default));
                }
                Ok(def)
            })
            .collect::<Result<Vec<_>, String>>()?;

        let sql = format!("CREATE TABLE \"{}\" ({})", name, col_defs.join(", "));

        self.connection()
            .execute(&sql, [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn drop_table(&self, name: &str) -> Result<(), String> {
        validate_identifier(name)?;
        let sql = format!("DROP TABLE \"{}\"", name);
        self.connection()
            .execute(&sql, [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_table_schema(&self, name: &str) -> Result<Vec<ColumnInfo>, String> {
        validate_identifier(name)?;
        let conn = self.connection();
        let sql = format!("PRAGMA table_info(\"{}\")", name);
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let columns = stmt
            .query_map([], |row| {
                Ok(ColumnInfo {
                    cid: row.get(0)?,
                    name: row.get(1)?,
                    col_type: row.get(2)?,
                    notnull: row.get(3)?,
                    default_value: row.get(4)?,
                    pk: row.get::<_, i64>(5)? != 0,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(columns)
    }

    // --- Column-level schema operations ---

    pub fn add_column(&self, table: &str, column: &ColumnDef) -> Result<(), String> {
        validate_identifier(table)?;
        validate_identifier(&column.name)?;

        let mut sql = format!(
            "ALTER TABLE \"{}\" ADD COLUMN \"{}\" {}",
            table, column.name, column.col_type
        );
        if !column.nullable {
            sql.push_str(" NOT NULL");
            if let Some(ref default) = column.default_value {
                sql.push_str(&format!(" DEFAULT {}", default));
            } else {
                sql.push_str(" DEFAULT ''");
            }
        } else if let Some(ref default) = column.default_value {
            sql.push_str(&format!(" DEFAULT {}", default));
        }

        self.connection()
            .execute(&sql, [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn remove_column(&self, table: &str, column: &str) -> Result<(), String> {
        validate_identifier(table)?;
        validate_identifier(column)?;

        // Try ALTER TABLE DROP COLUMN first (SQLite >= 3.35.0)
        let sql = format!("ALTER TABLE \"{}\" DROP COLUMN \"{}\"", table, column);
        let result = self.connection().execute(&sql, []);

        match result {
            Ok(_) => Ok(()),
            Err(_) => self.remove_column_via_recreation(table, column),
        }
    }

    fn remove_column_via_recreation(&self, table: &str, column: &str) -> Result<(), String> {
        let schema = self.get_table_schema(table)?;
        let remaining_cols: Vec<&ColumnInfo> = schema.iter().filter(|c| c.name != column).collect();

        if remaining_cols.is_empty() {
            return Err("Cannot remove the last column".to_string());
        }

        let col_names: Vec<String> = remaining_cols
            .iter()
            .map(|c| format!("\"{}\"", c.name))
            .collect();
        let col_defs: Vec<String> = remaining_cols
            .iter()
            .map(|c| {
                let mut def = format!("\"{}\" {}", c.name, c.col_type);
                if c.notnull {
                    def.push_str(" NOT NULL");
                }
                if let Some(ref d) = c.default_value {
                    def.push_str(&format!(" DEFAULT {}", d));
                }
                if c.pk {
                    def.push_str(" PRIMARY KEY");
                }
                def
            })
            .collect();

        let temp_name = format!("{}_migration_tmp", table);
        let conn = self.connection();

        conn.execute_batch("BEGIN TRANSACTION;")
            .map_err(|e| e.to_string())?;

        let create_sql = format!("CREATE TABLE \"{}\" ({})", temp_name, col_defs.join(", "));
        if let Err(e) = conn.execute(&create_sql, []) {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(e.to_string());
        }

        let copy_sql = format!(
            "INSERT INTO \"{}\" SELECT {} FROM \"{}\"",
            temp_name,
            col_names.join(", "),
            table
        );
        if let Err(e) = conn.execute(&copy_sql, []) {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(e.to_string());
        }

        let drop_sql = format!("DROP TABLE \"{}\"", table);
        if let Err(e) = conn.execute(&drop_sql, []) {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(e.to_string());
        }

        let rename_sql = format!("ALTER TABLE \"{}\" RENAME TO \"{}\"", temp_name, table);
        if let Err(e) = conn.execute(&rename_sql, []) {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(e.to_string());
        }

        conn.execute_batch("COMMIT;").map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn rename_column(&self, table: &str, old_name: &str, new_name: &str) -> Result<(), String> {
        validate_identifier(table)?;
        validate_identifier(old_name)?;
        validate_identifier(new_name)?;

        let sql = format!(
            "ALTER TABLE \"{}\" RENAME COLUMN \"{}\" TO \"{}\"",
            table, old_name, new_name
        );
        self.connection()
            .execute(&sql, [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // --- Row-level CRUD operations ---

    pub fn list_rows(
        &self,
        table: &str,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<String>, Vec<Vec<Option<String>>>), String> {
        validate_identifier(table)?;
        let conn = self.connection();

        let sql = format!("SELECT rowid, * FROM \"{}\" LIMIT ?1 OFFSET ?2", table);
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

        let col_count = stmt.column_count();
        let col_names: Vec<String> = (0..col_count)
            .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
            .collect();

        let rows = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                let mut values = Vec::new();
                for i in 0..col_count {
                    let val: Option<String> = row
                        .get::<_, rusqlite::types::Value>(i)
                        .ok()
                        .map(|v| match v {
                            rusqlite::types::Value::Null => None,
                            rusqlite::types::Value::Integer(i) => Some(i.to_string()),
                            rusqlite::types::Value::Real(f) => Some(f.to_string()),
                            rusqlite::types::Value::Text(s) => Some(s),
                            rusqlite::types::Value::Blob(b) => {
                                Some(format!("[blob: {} bytes]", b.len()))
                            }
                        })
                        .unwrap_or(None);
                    values.push(val);
                }
                Ok(values)
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        Ok((col_names, rows))
    }

    pub fn insert_row(&self, table: &str, values: &HashMap<String, String>) -> Result<i64, String> {
        validate_identifier(table)?;
        for key in values.keys() {
            validate_identifier(key)?;
        }

        if values.is_empty() {
            return Err("No values provided".to_string());
        }

        let col_names: Vec<String> = values.keys().map(|k| format!("\"{}\"", k)).collect();
        let placeholders: Vec<String> = (1..=values.len()).map(|i| format!("?{}", i)).collect();
        let vals: Vec<&String> = values.values().collect();

        let sql = format!(
            "INSERT INTO \"{}\" ({}) VALUES ({})",
            table,
            col_names.join(", "),
            placeholders.join(", ")
        );

        let conn = self.connection();
        let params: Vec<&dyn rusqlite::types::ToSql> = vals
            .iter()
            .map(|v| v as &dyn rusqlite::types::ToSql)
            .collect();
        conn.execute(&sql, params.as_slice())
            .map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_row(
        &self,
        table: &str,
        rowid: i64,
        values: &HashMap<String, String>,
    ) -> Result<(), String> {
        validate_identifier(table)?;
        for key in values.keys() {
            validate_identifier(key)?;
        }

        if values.is_empty() {
            return Err("No values provided".to_string());
        }

        let set_clauses: Vec<String> = values
            .keys()
            .enumerate()
            .map(|(i, k)| format!("\"{}\" = ?{}", k, i + 1))
            .collect();
        let vals: Vec<&String> = values.values().collect();

        let sql = format!(
            "UPDATE \"{}\" SET {} WHERE rowid = ?{}",
            table,
            set_clauses.join(", "),
            vals.len() + 1
        );

        let conn = self.connection();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vals
            .iter()
            .map(|v| Box::new(v.to_string()) as Box<dyn rusqlite::types::ToSql>)
            .collect();
        params.push(Box::new(rowid));
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_row(&self, table: &str, rowid: i64) -> Result<(), String> {
        validate_identifier(table)?;
        let sql = format!("DELETE FROM \"{}\" WHERE rowid = ?1", table);
        self.connection()
            .execute(&sql, params![rowid])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn count_rows(&self, table: &str) -> Result<i64, String> {
        validate_identifier(table)?;
        let conn = self.connection();
        let sql = format!("SELECT COUNT(*) FROM \"{}\"", table);
        conn.query_row(&sql, [], |row| row.get(0))
            .map_err(|e| e.to_string())
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

    #[test]
    fn test_create_and_list_tables() {
        let db = Database::new_in_memory().unwrap();
        let cols = vec![ColumnDef {
            name: "id".to_string(),
            col_type: "INTEGER".to_string(),
            nullable: false,
            default_value: None,
        }];
        db.create_table("test_table", &cols).unwrap();
        let tables = db.list_tables().unwrap();
        assert_eq!(tables, vec!["test_table"]);
    }

    #[test]
    fn test_create_table_rejects_bad_name() {
        let db = Database::new_in_memory().unwrap();
        let cols = vec![ColumnDef {
            name: "id".to_string(),
            col_type: "INTEGER".to_string(),
            nullable: false,
            default_value: None,
        }];
        let result = db.create_table("users; DROP TABLE", &cols);
        assert!(result.is_err());
    }

    #[test]
    fn test_drop_table() {
        let db = Database::new_in_memory().unwrap();
        let cols = vec![ColumnDef {
            name: "id".to_string(),
            col_type: "INTEGER".to_string(),
            nullable: false,
            default_value: None,
        }];
        db.create_table("to_drop", &cols).unwrap();
        db.drop_table("to_drop").unwrap();
        let tables = db.list_tables().unwrap();
        assert!(tables.is_empty());
    }

    #[test]
    fn test_get_table_schema() {
        let db = Database::new_in_memory().unwrap();
        let cols = vec![
            ColumnDef {
                name: "id".to_string(),
                col_type: "INTEGER".to_string(),
                nullable: false,
                default_value: None,
            },
            ColumnDef {
                name: "name".to_string(),
                col_type: "TEXT".to_string(),
                nullable: true,
                default_value: None,
            },
        ];
        db.create_table("schema_test", &cols).unwrap();
        let schema = db.get_table_schema("schema_test").unwrap();
        assert_eq!(schema.len(), 2);
        assert_eq!(schema[0].name, "id");
        assert_eq!(schema[1].name, "name");
    }

    #[test]
    fn test_add_column() {
        let db = Database::new_in_memory().unwrap();
        let cols = vec![ColumnDef {
            name: "id".to_string(),
            col_type: "INTEGER".to_string(),
            nullable: false,
            default_value: None,
        }];
        db.create_table("add_col_test", &cols).unwrap();
        db.add_column(
            "add_col_test",
            &ColumnDef {
                name: "email".to_string(),
                col_type: "TEXT".to_string(),
                nullable: true,
                default_value: None,
            },
        )
        .unwrap();
        let schema = db.get_table_schema("add_col_test").unwrap();
        assert_eq!(schema.len(), 2);
        assert_eq!(schema[1].name, "email");
    }

    #[test]
    fn test_remove_column() {
        let db = Database::new_in_memory().unwrap();
        let cols = vec![
            ColumnDef {
                name: "id".to_string(),
                col_type: "INTEGER".to_string(),
                nullable: false,
                default_value: None,
            },
            ColumnDef {
                name: "to_remove".to_string(),
                col_type: "TEXT".to_string(),
                nullable: true,
                default_value: None,
            },
        ];
        db.create_table("remove_col_test", &cols).unwrap();
        db.remove_column("remove_col_test", "to_remove").unwrap();
        let schema = db.get_table_schema("remove_col_test").unwrap();
        assert_eq!(schema.len(), 1);
        assert_eq!(schema[0].name, "id");
    }

    #[test]
    fn test_rename_column() {
        let db = Database::new_in_memory().unwrap();
        let cols = vec![ColumnDef {
            name: "old_name".to_string(),
            col_type: "TEXT".to_string(),
            nullable: true,
            default_value: None,
        }];
        db.create_table("rename_test", &cols).unwrap();
        db.rename_column("rename_test", "old_name", "new_name")
            .unwrap();
        let schema = db.get_table_schema("rename_test").unwrap();
        assert_eq!(schema[0].name, "new_name");
    }

    #[test]
    fn test_row_crud() {
        let db = Database::new_in_memory().unwrap();
        let cols = vec![
            ColumnDef {
                name: "name".to_string(),
                col_type: "TEXT".to_string(),
                nullable: true,
                default_value: None,
            },
            ColumnDef {
                name: "age".to_string(),
                col_type: "INTEGER".to_string(),
                nullable: true,
                default_value: None,
            },
        ];
        db.create_table("crud_test", &cols).unwrap();

        let mut values = HashMap::new();
        values.insert("name".to_string(), "Alice".to_string());
        values.insert("age".to_string(), "30".to_string());
        let rowid = db.insert_row("crud_test", &values).unwrap();
        assert!(rowid > 0);

        let (col_names, rows) = db.list_rows("crud_test", 100, 0).unwrap();
        assert!(col_names.contains(&"name".to_string()));
        assert_eq!(rows.len(), 1);

        let mut new_values = HashMap::new();
        new_values.insert("name".to_string(), "Bob".to_string());
        db.update_row("crud_test", rowid, &new_values).unwrap();
        let (_, rows) = db.list_rows("crud_test", 100, 0).unwrap();
        let name_idx = col_names.iter().position(|n| n == "name").unwrap();
        assert_eq!(rows[0][name_idx], Some("Bob".to_string()));

        db.delete_row("crud_test", rowid).unwrap();
        let (_, rows) = db.list_rows("crud_test", 100, 0).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn test_row_crud_rejects_bad_identifiers() {
        let db = Database::new_in_memory().unwrap();
        let cols = vec![ColumnDef {
            name: "id".to_string(),
            col_type: "INTEGER".to_string(),
            nullable: false,
            default_value: None,
        }];
        db.create_table("safe_table", &cols).unwrap();

        let mut bad_values = HashMap::new();
        bad_values.insert("col; DROP TABLE".to_string(), "value".to_string());
        assert!(db.insert_row("safe_table", &bad_values).is_err());

        assert!(db.list_rows("bad table!", 10, 0).is_err());
        assert!(db.delete_row("bad'table", 1).is_err());
    }

    #[test]
    fn test_count_rows() {
        let db = Database::new_in_memory().unwrap();
        let cols = vec![ColumnDef {
            name: "val".to_string(),
            col_type: "TEXT".to_string(),
            nullable: true,
            default_value: None,
        }];
        db.create_table("count_test", &cols).unwrap();

        assert_eq!(db.count_rows("count_test").unwrap(), 0);

        let mut values = HashMap::new();
        values.insert("val".to_string(), "a".to_string());
        db.insert_row("count_test", &values).unwrap();
        db.insert_row("count_test", &values).unwrap();

        assert_eq!(db.count_rows("count_test").unwrap(), 2);
    }

    #[test]
    fn test_pagination() {
        let db = Database::new_in_memory().unwrap();
        let cols = vec![ColumnDef {
            name: "val".to_string(),
            col_type: "INTEGER".to_string(),
            nullable: true,
            default_value: None,
        }];
        db.create_table("page_test", &cols).unwrap();

        for i in 0..10 {
            let mut values = HashMap::new();
            values.insert("val".to_string(), i.to_string());
            db.insert_row("page_test", &values).unwrap();
        }

        let (_, rows) = db.list_rows("page_test", 3, 0).unwrap();
        assert_eq!(rows.len(), 3);

        let (_, rows) = db.list_rows("page_test", 3, 8).unwrap();
        assert_eq!(rows.len(), 2);
    }
}
