//! Integration tests for DB-based migration.
//!
//! Tests the workflow of reading schema from SQLite database
//! and comparing it with .poly schema definitions.

use polygen::db_introspection::SqliteIntrospector;
use polygen::migration::{DestructiveChangePolicy, MigrationDiff};
use polygen::{ir_builder, parse_and_merge_schemas, validation};
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::{NamedTempFile, TempDir};

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
fn test_cli_migrate_no_changes_writes_schema_metadata_sql() -> anyhow::Result<()> {
    let db_file = create_test_db(
        "CREATE TABLE game_Player (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        )",
    )?;

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

    let output_dir = TempDir::new()?;
    let cli = polygen::Cli {
        command: Some(polygen::Commands::Migrate {
            baseline: None,
            db: Some(db_file.path().to_path_buf()),
            schema_path: schema_file.path().to_path_buf(),
            output_dir: output_dir.path().to_path_buf(),
            target: Some("sqlite".to_string()),
            schema_hash_policy: "warn".to_string(),
            destructive_policy: "warn".to_string(),
        }),
        schema_path: None,
        templates_dir: PathBuf::from("templates"),
        output_dir: output_dir.path().to_path_buf(),
        lang: None,
        baseline: None,
        sources: None,
    };

    polygen::run(cli)?;

    let migration_sql = fs::read_to_string(output_dir.path().join("sqlite/migration.sql"))?;
    assert!(migration_sql.contains("INSERT INTO _polygen_schema"));
    assert!(migration_sql.contains("CREATE TABLE IF NOT EXISTS _polygen_migrations"));
    assert!(migration_sql.contains("INSERT INTO _polygen_migrations"));
    assert!(migration_sql.contains("schema_hash"));
    assert!(migration_sql.contains("-- No changes detected."));

    Ok(())
}

#[test]
fn test_cli_migrate_reports_schema_hash_mismatch() -> anyhow::Result<()> {
    let db_file = create_test_db(
        "CREATE TABLE game_Player (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        );
        CREATE TABLE _polygen_schema (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            schema_hash TEXT,
            schema_json TEXT
        );
        INSERT INTO _polygen_schema (id, schema_hash, schema_json)
        VALUES (1, 'stale_hash', '{}');",
    )?;

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

    let output_dir = TempDir::new()?;
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "migrate",
            "--db",
            &db_file.path().to_string_lossy(),
            "--schema-path",
            &schema_file.path().to_string_lossy(),
            "--output-dir",
            &output_dir.path().to_string_lossy(),
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    assert!(output.status.success(), "migrate failed: {}", combined);
    assert!(combined.contains("스키마 해시"), "output: {}", combined);
    assert!(combined.contains("불일치"), "output: {}", combined);

    Ok(())
}

#[test]
fn test_cli_migrate_schema_hash_policy_fail_stops_on_mismatch() -> anyhow::Result<()> {
    let db_file = create_test_db(
        "CREATE TABLE game_Player (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        );
        CREATE TABLE _polygen_schema (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            schema_hash TEXT,
            schema_json TEXT
        );
        INSERT INTO _polygen_schema (id, schema_hash, schema_json)
        VALUES (1, 'stale_hash', '{}');",
    )?;

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

    let output_dir = TempDir::new()?;
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "migrate",
            "--db",
            &db_file.path().to_string_lossy(),
            "--schema-path",
            &schema_file.path().to_string_lossy(),
            "--output-dir",
            &output_dir.path().to_string_lossy(),
            "--schema-hash-policy",
            "fail",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    assert!(
        !output.status.success(),
        "migrate should fail: {}",
        combined
    );
    assert!(combined.contains("스키마 해시"), "output: {}", combined);
    assert!(!output_dir.path().join("sqlite/migration.sql").exists());

    Ok(())
}

#[test]
fn test_destructive_policy_fail_stops_on_removed_table() -> anyhow::Result<()> {
    let baseline_file = create_test_schema(
        r#"
        namespace game {
            table OldTable {
                id: u32 primary_key;
            }
        }
        "#,
    )?;

    let schema_file = create_test_schema(
        r#"
        namespace game {
        }
        "#,
    )?;

    let output_dir = TempDir::new()?;
    let cli = polygen::Cli {
        command: Some(polygen::Commands::Migrate {
            baseline: Some(baseline_file.path().to_path_buf()),
            db: None,
            schema_path: schema_file.path().to_path_buf(),
            output_dir: output_dir.path().to_path_buf(),
            target: Some("sqlite".to_string()),
            schema_hash_policy: "warn".to_string(),
            destructive_policy: "fail".to_string(),
        }),
        schema_path: None,
        templates_dir: PathBuf::from("templates"),
        output_dir: output_dir.path().to_path_buf(),
        lang: None,
        baseline: None,
        sources: None,
    };

    let result = polygen::run(cli);

    assert!(result.is_err(), "destructive-policy=fail should stop");
    assert!(!output_dir.path().join("sqlite/migration.sql").exists());

    Ok(())
}

#[test]
fn test_destructive_policy_allow_uncomments_drop_sql() -> anyhow::Result<()> {
    let baseline_file = create_test_schema(
        r#"
        namespace game {
            table Player {
                id: u32 primary_key;
                obsolete: string;
            }
            table OldTable {
                id: u32 primary_key;
            }
        }
        "#,
    )?;

    let schema_file = create_test_schema(
        r#"
        namespace game {
            table Player {
                id: u32 primary_key;
            }
        }
        "#,
    )?;

    let baseline_ir = build_ir_from_schema(baseline_file.path())?;
    let current_ir = build_ir_from_schema(schema_file.path())?;
    let diff = MigrationDiff::compare(&baseline_ir, &current_ir);

    assert!(diff.has_destructive_changes());

    let warn_sql = diff.to_sqlite_sql_with_schema(&current_ir);
    assert!(warn_sql.contains("-- DROP TABLE IF EXISTS game_OldTable;"));
    assert!(warn_sql.contains("-- ALTER TABLE game_Player DROP COLUMN obsolete;"));

    let allow_sql =
        diff.to_sqlite_sql_with_schema_and_policy(&current_ir, DestructiveChangePolicy::Allow);
    assert!(allow_sql.contains("DROP TABLE IF EXISTS game_OldTable;"));
    assert!(allow_sql.contains("ALTER TABLE game_Player DROP COLUMN obsolete;"));
    assert!(!allow_sql.contains("-- DROP TABLE IF EXISTS game_OldTable;"));
    assert!(!allow_sql.contains("-- ALTER TABLE game_Player DROP COLUMN obsolete;"));

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
    assert!(
        combined.contains("--schema-hash-policy"),
        "CLI help should show --schema-hash-policy option. Output: {}",
        combined
    );
    assert!(
        combined.contains("--destructive-policy"),
        "CLI help should show --destructive-policy option. Output: {}",
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
