use super::{validate_identifier, ColumnDef, ColumnInfo, Database};

impl Database {
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
