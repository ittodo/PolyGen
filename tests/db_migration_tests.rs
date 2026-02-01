//! Integration tests for DB-based migration.
//!
//! Tests the workflow of reading schema from SQLite database
//! and comparing it with .poly schema definitions.

use polygen::db_introspection::SqliteIntrospector;
use polygen::migration::MigrationDiff;
use polygen::{ir_builder, parse_and_merge_schemas, validation};
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use tempfile::NamedTempFile;

/// Helper to create a test database with given SQL.
fn create_test_db(sql: &str) -> anyhow::Result<NamedTempFile> {
    let file = NamedTempFile::new()?;
    let conn = Connection::open(file.path())?;
    conn.execute_batch(sql)?;
    Ok(file)
}

/// Helper to create a test schema file.
fn create_test_schema(content: &str) -> anyhow::Result<NamedTempFile> {
    let file = NamedTempFile::with_suffix(".poly")?;
    fs::write(file.path(), content)?;
    Ok(file)
}

/// Helper to parse schema and build IR.
fn build_ir_from_schema(schema_path: &std::path::Path) -> anyhow::Result<polygen::SchemaContext> {
    let asts = parse_and_merge_schemas(&PathBuf::from(schema_path), None)?;
    let defs: Vec<_> = asts
        .iter()
        .flat_map(|ast| ast.definitions.clone())
        .collect();
    validation::validate_ast(&defs)?;
    Ok(ir_builder::build_ir(&asts))
}

#[test]
fn test_db_introspection_empty_db() -> anyhow::Result<()> {
    let db_file = create_test_db("")?;
    let introspector = SqliteIntrospector::open(db_file.path())?;
    let schema = introspector.read_schema()?;

    assert!(schema.is_empty());
    assert_eq!(schema.table_count(), 0);
    Ok(())
}

#[test]
fn test_db_introspection_single_table() -> anyhow::Result<()> {
    let db_file = create_test_db(
        "CREATE TABLE Player (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            level INTEGER DEFAULT 1
        )",
    )?;

    let introspector = SqliteIntrospector::open(db_file.path())?;
    let schema = introspector.read_schema()?;

    assert_eq!(schema.table_count(), 1);
    assert!(schema.tables.contains_key("Player"));

    let player = &schema.tables["Player"];
    assert_eq!(player.columns.len(), 3);
    assert!(player.has_column("id"));
    assert!(player.has_column("name"));
    assert!(player.has_column("level"));

    Ok(())
}

#[test]
fn test_migration_new_table_in_poly() -> anyhow::Result<()> {
    // DB has no tables
    let db_file = create_test_db("")?;

    // Schema defines a table
    let schema_file = create_test_schema(
        r#"
        namespace game {
            table Player {
                id: u32 primary_key;
                name: string;
            }
        }
        "#,
    )?;

    let introspector = SqliteIntrospector::open(db_file.path())?;
    let db_schema = introspector.read_schema()?;
    let poly_ir = build_ir_from_schema(schema_file.path())?;

    let diff = MigrationDiff::compare_db(&db_schema, &poly_ir);

    // Should detect one table to add
    assert_eq!(diff.changes.len(), 1);
    match &diff.changes[0] {
        polygen::migration::SchemaChange::TableAdded { table_name, .. } => {
            assert_eq!(table_name, "Player");
        }
        _ => panic!("Expected TableAdded change"),
    }

    // Generate SQL and verify
    let sql = diff.to_sqlite_sql();
    assert!(sql.contains("CREATE TABLE"));
    assert!(sql.contains("Player"));

    Ok(())
}

#[test]
fn test_migration_table_exists_in_db_only() -> anyhow::Result<()> {
    // DB has a table
    let db_file = create_test_db(
        "CREATE TABLE OldTable (
            id INTEGER PRIMARY KEY,
            data TEXT
        )",
    )?;

    // Schema is empty
    let schema_file = create_test_schema(
        r#"
        namespace game {
        }
        "#,
    )?;

    let introspector = SqliteIntrospector::open(db_file.path())?;
    let db_schema = introspector.read_schema()?;
    let poly_ir = build_ir_from_schema(schema_file.path())?;

    let diff = MigrationDiff::compare_db(&db_schema, &poly_ir);

    // Should detect one table to remove
    assert_eq!(diff.changes.len(), 1);
    match &diff.changes[0] {
        polygen::migration::SchemaChange::TableRemoved { table_name, .. } => {
            assert_eq!(table_name, "OldTable");
        }
        _ => panic!("Expected TableRemoved change"),
    }

    // Should have a warning
    assert!(!diff.warnings.is_empty());

    Ok(())
}

#[test]
fn test_migration_new_column_in_poly() -> anyhow::Result<()> {
    // DB has Player with id and name
    let db_file = create_test_db(
        "CREATE TABLE game_Player (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        )",
    )?;

    // Schema adds 'level' column
    let schema_file = create_test_schema(
        r#"
        namespace game {
            table Player {
                id: u32 primary_key;
                name: string;
                level: u16;
            }
        }
        "#,
    )?;

    let introspector = SqliteIntrospector::open(db_file.path())?;
    let db_schema = introspector.read_schema()?;
    let poly_ir = build_ir_from_schema(schema_file.path())?;

    let diff = MigrationDiff::compare_db(&db_schema, &poly_ir);

    // Should detect one column to add
    assert_eq!(diff.changes.len(), 1);
    match &diff.changes[0] {
        polygen::migration::SchemaChange::ColumnAdded {
            table_name,
            column_name,
            ..
        } => {
            assert_eq!(table_name, "Player");
            assert_eq!(column_name, "level");
        }
        _ => panic!("Expected ColumnAdded change"),
    }

    // Generate SQL and verify
    let sql = diff.to_sqlite_sql();
    assert!(sql.contains("ALTER TABLE"));
    assert!(sql.contains("ADD COLUMN"));
    assert!(sql.contains("level"));

    Ok(())
}

#[test]
fn test_migration_column_removed_in_poly() -> anyhow::Result<()> {
    // DB has Player with id, name, and obsolete_field
    let db_file = create_test_db(
        "CREATE TABLE game_Player (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            obsolete_field TEXT
        )",
    )?;

    // Schema doesn't have obsolete_field
    let schema_file = create_test_schema(
        r#"
        namespace game {
            table Player {
                id: u32 primary_key;
                name: string;
            }
        }
        "#,
    )?;

    let introspector = SqliteIntrospector::open(db_file.path())?;
    let db_schema = introspector.read_schema()?;
    let poly_ir = build_ir_from_schema(schema_file.path())?;

    let diff = MigrationDiff::compare_db(&db_schema, &poly_ir);

    // Should detect one column to remove
    assert_eq!(diff.changes.len(), 1);
    match &diff.changes[0] {
        polygen::migration::SchemaChange::ColumnRemoved {
            table_name,
            column_name,
            ..
        } => {
            assert_eq!(table_name, "Player");
            assert_eq!(column_name, "obsolete_field");
        }
        _ => panic!("Expected ColumnRemoved change"),
    }

    // Should have a warning
    assert!(!diff.warnings.is_empty());

    Ok(())
}

#[test]
fn test_migration_column_type_changed() -> anyhow::Result<()> {
    // DB has level as TEXT
    let db_file = create_test_db(
        "CREATE TABLE game_Player (
            id INTEGER PRIMARY KEY,
            level TEXT
        )",
    )?;

    // Schema has level as u16 (INTEGER)
    let schema_file = create_test_schema(
        r#"
        namespace game {
            table Player {
                id: u32 primary_key;
                level: u16;
            }
        }
        "#,
    )?;

    let introspector = SqliteIntrospector::open(db_file.path())?;
    let db_schema = introspector.read_schema()?;
    let poly_ir = build_ir_from_schema(schema_file.path())?;

    let diff = MigrationDiff::compare_db(&db_schema, &poly_ir);

    // Should detect type change
    assert_eq!(diff.changes.len(), 1);
    match &diff.changes[0] {
        polygen::migration::SchemaChange::ColumnTypeChanged {
            table_name,
            column_name,
            old_type,
            new_type,
            ..
        } => {
            assert_eq!(table_name, "Player");
            assert_eq!(column_name, "level");
            assert_eq!(old_type, "TEXT");
            assert_eq!(new_type, "INTEGER");
        }
        _ => panic!("Expected ColumnTypeChanged change"),
    }

    Ok(())
}

#[test]
fn test_migration_no_changes() -> anyhow::Result<()> {
    // DB matches schema exactly
    let db_file = create_test_db(
        "CREATE TABLE game_Player (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            level INTEGER
        )",
    )?;

    let schema_file = create_test_schema(
        r#"
        namespace game {
            table Player {
                id: u32 primary_key;
                name: string;
                level: u16;
            }
        }
        "#,
    )?;

    let introspector = SqliteIntrospector::open(db_file.path())?;
    let db_schema = introspector.read_schema()?;
    let poly_ir = build_ir_from_schema(schema_file.path())?;

    let diff = MigrationDiff::compare_db(&db_schema, &poly_ir);

    // Should detect no changes
    assert!(diff.changes.is_empty());
    assert!(diff.warnings.is_empty());

    Ok(())
}

#[test]
fn test_migration_multiple_tables() -> anyhow::Result<()> {
    // DB has Player but not Item
    let db_file = create_test_db(
        "CREATE TABLE game_Player (
            id INTEGER PRIMARY KEY,
            name TEXT
        )",
    )?;

    // Schema has both Player and Item
    let schema_file = create_test_schema(
        r#"
        namespace game {
            table Player {
                id: u32 primary_key;
                name: string;
            }
            table Item {
                id: u32 primary_key;
                title: string;
            }
        }
        "#,
    )?;

    let introspector = SqliteIntrospector::open(db_file.path())?;
    let db_schema = introspector.read_schema()?;
    let poly_ir = build_ir_from_schema(schema_file.path())?;

    let diff = MigrationDiff::compare_db(&db_schema, &poly_ir);

    // Should detect Item table to add
    assert_eq!(diff.changes.len(), 1);
    match &diff.changes[0] {
        polygen::migration::SchemaChange::TableAdded { table_name, .. } => {
            assert_eq!(table_name, "Item");
        }
        _ => panic!("Expected TableAdded change for Item"),
    }

    Ok(())
}

#[test]
fn test_migration_nested_namespace() -> anyhow::Result<()> {
    // DB has no tables
    let db_file = create_test_db("")?;

    // Schema has nested namespace
    let schema_file = create_test_schema(
        r#"
        namespace game.data {
            table ItemTable {
                id: u32 primary_key;
                name: string;
            }
        }
        "#,
    )?;

    let introspector = SqliteIntrospector::open(db_file.path())?;
    let db_schema = introspector.read_schema()?;
    let poly_ir = build_ir_from_schema(schema_file.path())?;

    let diff = MigrationDiff::compare_db(&db_schema, &poly_ir);

    // Should create table with namespace prefix
    let sql = diff.to_sqlite_sql();
    assert!(sql.contains("game_data_ItemTable"));

    Ok(())
}

#[test]
fn test_cli_help_shows_db_option() -> anyhow::Result<()> {
    use std::process::Command;

    let output = Command::new("cargo")
        .args(["run", "--", "migrate", "--help"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Should show --db option
    assert!(
        combined.contains("--db"),
        "CLI help should show --db option. Output: {}",
        combined
    );

    Ok(())
}

#[test]
fn test_end_to_end_db_migration() -> anyhow::Result<()> {
    // Create a realistic scenario
    let db_file = create_test_db(
        r#"
        CREATE TABLE game_Player (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            old_score INTEGER
        );
        CREATE TABLE game_Guild (
            id INTEGER PRIMARY KEY,
            name TEXT
        );
        "#,
    )?;

    // Schema with changes:
    // - Player: remove old_score, add level
    // - Guild: no changes
    // - Item: new table
    let schema_file = create_test_schema(
        r#"
        namespace game {
            table Player {
                id: u32 primary_key;
                name: string;
                level: u16;
            }
            table Guild {
                id: u32 primary_key;
                name: string;
            }
            table Item {
                id: u32 primary_key;
                title: string;
                price: u32;
            }
        }
        "#,
    )?;

    let introspector = SqliteIntrospector::open(db_file.path())?;
    let db_schema = introspector.read_schema()?;
    let poly_ir = build_ir_from_schema(schema_file.path())?;

    let diff = MigrationDiff::compare_db(&db_schema, &poly_ir);

    // Expected changes:
    // 1. ColumnRemoved: Player.old_score
    // 2. ColumnAdded: Player.level
    // 3. TableAdded: Item
    assert_eq!(diff.changes.len(), 3);

    // Generate SQL
    let sql = diff.to_sqlite_sql();

    // Verify SQL contains expected statements
    assert!(sql.contains("ALTER TABLE") || sql.contains("CREATE TABLE"));
    assert!(sql.contains("Item")); // New table
    assert!(sql.contains("level")); // New column

    println!("Generated SQL:\n{}", sql);

    Ok(())
}
