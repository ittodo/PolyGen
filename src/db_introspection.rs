//! Database introspection module.
//!
//! Reads schema information from actual databases (SQLite, MySQL).
//! Used for migration comparison against .poly schema definitions.

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::Path;

/// Represents a database schema read from an actual database.
#[derive(Debug, Clone, Default)]
pub struct DbSchema {
    /// All tables in the database.
    pub tables: HashMap<String, DbTable>,
}

/// Represents a table in the database.
#[derive(Debug, Clone)]
pub struct DbTable {
    /// Table name.
    pub name: String,
    /// Columns in the table.
    pub columns: HashMap<String, DbColumn>,
    /// Primary key column names.
    pub primary_keys: Vec<String>,
    /// Indexes on this table.
    pub indexes: Vec<DbIndex>,
}

/// Represents a column in a database table.
#[derive(Debug, Clone)]
pub struct DbColumn {
    /// Column name.
    pub name: String,
    /// Column type as stored in DB (e.g., "INTEGER", "TEXT").
    pub db_type: String,
    /// Whether the column allows NULL values.
    pub is_nullable: bool,
    /// Default value if any.
    pub default_value: Option<String>,
    /// Whether this column is part of primary key.
    pub is_primary_key: bool,
}

/// Represents an index on a table.
#[derive(Debug, Clone)]
pub struct DbIndex {
    /// Index name.
    pub name: String,
    /// Column names in the index.
    pub columns: Vec<String>,
    /// Whether this is a unique index.
    pub is_unique: bool,
}

/// SQLite database introspection.
pub struct SqliteIntrospector {
    conn: Connection,
}

impl SqliteIntrospector {
    /// Open a SQLite database for introspection.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path.as_ref())
            .with_context(|| format!("Failed to open SQLite database: {}", path.as_ref().display()))?;
        Ok(Self { conn })
    }

    /// Read the complete schema from the database.
    pub fn read_schema(&self) -> Result<DbSchema> {
        let mut schema = DbSchema::default();

        // Get all table names (excluding internal SQLite tables)
        let table_names = self.get_table_names()?;

        for table_name in table_names {
            let table = self.read_table(&table_name)?;
            schema.tables.insert(table_name, table);
        }

        Ok(schema)
    }

    /// Get all user table names from the database.
    fn get_table_names(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM sqlite_master
             WHERE type='table'
             AND name NOT LIKE 'sqlite_%'
             AND name NOT LIKE '_polygen_%'
             ORDER BY name"
        )?;

        let names = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(names)
    }

    /// Read table schema including columns and indexes.
    fn read_table(&self, table_name: &str) -> Result<DbTable> {
        let columns = self.read_columns(table_name)?;
        let indexes = self.read_indexes(table_name)?;

        // Extract primary keys from columns
        let primary_keys: Vec<String> = columns
            .values()
            .filter(|c| c.is_primary_key)
            .map(|c| c.name.clone())
            .collect();

        Ok(DbTable {
            name: table_name.to_string(),
            columns,
            primary_keys,
            indexes,
        })
    }

    /// Read column information for a table using PRAGMA table_info.
    fn read_columns(&self, table_name: &str) -> Result<HashMap<String, DbColumn>> {
        let mut columns = HashMap::new();

        // PRAGMA table_info returns: cid, name, type, notnull, dflt_value, pk
        let mut stmt = self.conn.prepare(&format!("PRAGMA table_info('{}')", table_name))?;

        let column_iter = stmt.query_map([], |row| {
            Ok(DbColumn {
                name: row.get::<_, String>(1)?,
                db_type: row.get::<_, String>(2)?,
                is_nullable: row.get::<_, i32>(3)? == 0, // notnull=0 means nullable
                default_value: row.get::<_, Option<String>>(4)?,
                is_primary_key: row.get::<_, i32>(5)? > 0,
            })
        })?;

        for col_result in column_iter {
            let col = col_result?;
            columns.insert(col.name.clone(), col);
        }

        Ok(columns)
    }

    /// Read index information for a table.
    fn read_indexes(&self, table_name: &str) -> Result<Vec<DbIndex>> {
        let mut indexes = Vec::new();

        // Get index list
        let mut stmt = self.conn.prepare(&format!("PRAGMA index_list('{}')", table_name))?;

        let index_info: Vec<(String, bool)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(1)?, // name
                    row.get::<_, i32>(2)? == 1, // unique
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        // Get columns for each index
        for (index_name, is_unique) in index_info {
            // Skip auto-generated indexes (sqlite_autoindex_*)
            if index_name.starts_with("sqlite_autoindex_") {
                continue;
            }

            let mut col_stmt = self.conn.prepare(&format!("PRAGMA index_info('{}')", index_name))?;
            let columns: Vec<String> = col_stmt
                .query_map([], |row| row.get::<_, String>(2))? // name is at index 2
                .collect::<std::result::Result<Vec<_>, _>>()?;

            indexes.push(DbIndex {
                name: index_name,
                columns,
                is_unique,
            });
        }

        Ok(indexes)
    }
}

impl DbSchema {
    /// Check if the schema is empty (no tables).
    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }

    /// Get table count.
    pub fn table_count(&self) -> usize {
        self.tables.len()
    }

    /// Get total column count across all tables.
    pub fn column_count(&self) -> usize {
        self.tables.values().map(|t| t.columns.len()).sum()
    }
}

impl DbTable {
    /// Check if this table has a column with the given name.
    pub fn has_column(&self, name: &str) -> bool {
        self.columns.contains_key(name)
    }

    /// Get a column by name.
    pub fn get_column(&self, name: &str) -> Option<&DbColumn> {
        self.columns.get(name)
    }
}

impl DbColumn {
    /// Map SQLite type to poly type equivalent.
    pub fn to_poly_type(&self) -> String {
        match self.db_type.to_uppercase().as_str() {
            "INTEGER" | "INT" | "BIGINT" | "SMALLINT" | "TINYINT" => "i64".to_string(),
            "REAL" | "FLOAT" | "DOUBLE" => "f64".to_string(),
            "TEXT" | "VARCHAR" | "CHAR" => "string".to_string(),
            "BLOB" => "bytes".to_string(),
            "BOOLEAN" | "BOOL" => "bool".to_string(),
            _ => "string".to_string(), // Default to string for unknown types
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_db() -> Result<(NamedTempFile, Connection)> {
        let file = NamedTempFile::new()?;
        let conn = Connection::open(file.path())?;
        Ok((file, conn))
    }

    #[test]
    fn test_empty_database() -> Result<()> {
        let (file, _conn) = create_test_db()?;
        let introspector = SqliteIntrospector::open(file.path())?;
        let schema = introspector.read_schema()?;

        assert!(schema.is_empty());
        assert_eq!(schema.table_count(), 0);
        Ok(())
    }

    #[test]
    fn test_simple_table() -> Result<()> {
        let (file, conn) = create_test_db()?;

        conn.execute(
            "CREATE TABLE Player (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                level INTEGER DEFAULT 1
            )",
            [],
        )?;

        let introspector = SqliteIntrospector::open(file.path())?;
        let schema = introspector.read_schema()?;

        assert_eq!(schema.table_count(), 1);
        assert!(schema.tables.contains_key("Player"));

        let player = &schema.tables["Player"];
        assert_eq!(player.columns.len(), 3);
        assert!(player.has_column("id"));
        assert!(player.has_column("name"));
        assert!(player.has_column("level"));

        // Check column properties
        let id_col = player.get_column("id").unwrap();
        assert!(id_col.is_primary_key);
        assert_eq!(id_col.db_type, "INTEGER");

        let name_col = player.get_column("name").unwrap();
        assert!(!name_col.is_nullable); // NOT NULL
        assert_eq!(name_col.db_type, "TEXT");

        let level_col = player.get_column("level").unwrap();
        assert!(level_col.is_nullable); // no NOT NULL constraint
        assert_eq!(level_col.default_value, Some("1".to_string()));

        Ok(())
    }

    #[test]
    fn test_table_with_index() -> Result<()> {
        let (file, conn) = create_test_db()?;

        conn.execute(
            "CREATE TABLE Item (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                category TEXT
            )",
            [],
        )?;
        conn.execute("CREATE INDEX idx_item_name ON Item(name)", [])?;
        conn.execute("CREATE UNIQUE INDEX idx_item_category ON Item(category)", [])?;

        let introspector = SqliteIntrospector::open(file.path())?;
        let schema = introspector.read_schema()?;

        let item = &schema.tables["Item"];
        assert_eq!(item.indexes.len(), 2);

        let name_idx = item.indexes.iter().find(|i| i.name == "idx_item_name").unwrap();
        assert!(!name_idx.is_unique);
        assert_eq!(name_idx.columns, vec!["name"]);

        let cat_idx = item.indexes.iter().find(|i| i.name == "idx_item_category").unwrap();
        assert!(cat_idx.is_unique);
        assert_eq!(cat_idx.columns, vec!["category"]);

        Ok(())
    }

    #[test]
    fn test_multiple_tables() -> Result<()> {
        let (file, conn) = create_test_db()?;

        conn.execute("CREATE TABLE Users (id INTEGER PRIMARY KEY, name TEXT)", [])?;
        conn.execute("CREATE TABLE Items (id INTEGER PRIMARY KEY, title TEXT)", [])?;
        conn.execute("CREATE TABLE Orders (id INTEGER PRIMARY KEY, user_id INTEGER)", [])?;

        let introspector = SqliteIntrospector::open(file.path())?;
        let schema = introspector.read_schema()?;

        assert_eq!(schema.table_count(), 3);
        assert!(schema.tables.contains_key("Users"));
        assert!(schema.tables.contains_key("Items"));
        assert!(schema.tables.contains_key("Orders"));

        Ok(())
    }

    #[test]
    fn test_ignores_internal_tables() -> Result<()> {
        let (file, conn) = create_test_db()?;

        conn.execute("CREATE TABLE MyTable (id INTEGER PRIMARY KEY)", [])?;
        conn.execute("CREATE TABLE _polygen_migrations (version INTEGER)", [])?;

        let introspector = SqliteIntrospector::open(file.path())?;
        let schema = introspector.read_schema()?;

        // Should only see MyTable, not _polygen_migrations
        assert_eq!(schema.table_count(), 1);
        assert!(schema.tables.contains_key("MyTable"));
        assert!(!schema.tables.contains_key("_polygen_migrations"));

        Ok(())
    }

    #[test]
    fn test_poly_type_mapping() {
        let col = DbColumn {
            name: "test".to_string(),
            db_type: "INTEGER".to_string(),
            is_nullable: false,
            default_value: None,
            is_primary_key: false,
        };
        assert_eq!(col.to_poly_type(), "i64");

        let col = DbColumn {
            name: "test".to_string(),
            db_type: "TEXT".to_string(),
            is_nullable: false,
            default_value: None,
            is_primary_key: false,
        };
        assert_eq!(col.to_poly_type(), "string");

        let col = DbColumn {
            name: "test".to_string(),
            db_type: "REAL".to_string(),
            is_nullable: false,
            default_value: None,
            is_primary_key: false,
        };
        assert_eq!(col.to_poly_type(), "f64");

        let col = DbColumn {
            name: "test".to_string(),
            db_type: "BLOB".to_string(),
            is_nullable: false,
            default_value: None,
            is_primary_key: false,
        };
        assert_eq!(col.to_poly_type(), "bytes");
    }
}
