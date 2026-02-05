use rusqlite::params;
use std::collections::HashMap;

use super::{validate_identifier, Database};

impl Database {
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

#[cfg(test)]
mod tests {
    use super::super::ColumnDef;
    use super::*;

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
