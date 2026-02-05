use super::{validate_identifier, ColumnDef, Database};

impl Database {
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
        let remaining_cols: Vec<&super::ColumnInfo> =
            schema.iter().filter(|c| c.name != column).collect();

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
}

#[cfg(test)]
mod tests {
    use super::super::ColumnDef;
    use super::*;

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
}
