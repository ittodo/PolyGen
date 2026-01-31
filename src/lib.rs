//! # PolyGen - Polyglot Code Generator
//!
//! PolyGen is a code generation tool that transforms custom schema definitions into
//! code for multiple target languages. It uses a pipeline approach:
//!
//! 1. **Parsing**: Schema files (`.poly`) are parsed into an Abstract Syntax Tree (AST)
//! 2. **Validation**: The AST is validated for correctness (duplicate types, undefined references)
//! 3. **IR Building**: The AST is transformed into an Intermediate Representation (IR)
//! 4. **Code Generation**: The IR is passed to Rhai templates to generate target language code
//!
//! ## Quick Start
//!
//! ```ignore
//! use polygen::{CompilationPipeline, PipelineConfig};
//!
//! let config = PipelineConfig::new(
//!     "schemas/main.poly",
//!     "templates",
//!     "output",
//! );
//! let pipeline = CompilationPipeline::new(config);
//! pipeline.run()?;
//! ```
//!
//! ## Modules
//!
//! - [`ast_model`]: AST types representing parsed schema definitions
//! - [`ast_parser`]: Parser for `.poly` schema files
//! - [`ir_model`]: Intermediate Representation types for template consumption
//! - [`ir_builder`]: Transforms AST to IR
//! - [`type_registry`]: Centralized type management for type resolution
//! - [`validation`]: AST validation (duplicate detection, type reference checking)
//! - [`pipeline`]: High-level compilation pipeline orchestration
//! - [`codegen`]: Code generation utilities
//! - [`rhai`] / [`rhai_generator`]: Rhai template engine integration

use anyhow::Result;
use clap::{Parser as ClapParser, Subcommand};
use pest_derive::Parser;
use std::fs;
use std::path::PathBuf;

// All modules are now part of the library
pub mod ast_model;
pub mod ast_parser;
pub mod codegen;
pub mod db_introspection;
pub mod error;
pub mod ir_builder;
pub mod ir_model;
pub mod lang_config;
pub mod migration;
pub mod pipeline;
pub mod rhai;
pub mod rhai_generator;
pub mod symbol_table;
pub mod template;
pub mod type_registry;
pub mod validation;
pub mod visualize;

// Re-exports for convenience
pub use crate::ast_model::AstRoot;
pub use crate::ir_model::SchemaContext;
pub use crate::pipeline::{parse_and_merge_schemas, CompilationPipeline, PipelineConfig};

#[derive(Parser)]
#[grammar = "polygen.pest"]
pub struct Polygen;

/// Polyglot Code Generator from a custom schema language
#[derive(ClapParser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to the root schema file (for default generate command)
    #[arg(short, long)]
    pub schema_path: Option<PathBuf>,

    /// Path to the directory containing templates
    #[arg(short, long, default_value = "templates")]
    pub templates_dir: PathBuf,

    /// Path to the output directory for generated code
    #[arg(short, long, default_value = "output")]
    pub output_dir: PathBuf,

    /// Target language for code generation (e.g., csharp)
    #[arg(short, long)]
    pub lang: Option<String>,

    /// Path to baseline schema file for migration diff
    #[arg(short, long)]
    pub baseline: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate code from schema files (default behavior)
    Generate {
        /// Path to the root schema file
        #[arg(short, long)]
        schema_path: PathBuf,

        /// Path to the directory containing templates
        #[arg(short, long, default_value = "templates")]
        templates_dir: PathBuf,

        /// Path to the output directory for generated code
        #[arg(short, long, default_value = "output")]
        output_dir: PathBuf,

        /// Target language for code generation (e.g., csharp)
        #[arg(short, long)]
        lang: Option<String>,

        /// Path to baseline schema file for migration diff
        #[arg(short, long)]
        baseline: Option<PathBuf>,

        /// Preview mode: generate to temp directory and print results to stdout
        #[arg(long)]
        preview: bool,
    },

    /// Generate migration SQL by comparing schema versions or DB state
    Migrate {
        /// Path to the baseline (old) schema file (for schema-to-schema comparison)
        #[arg(short, long, conflicts_with = "db")]
        baseline: Option<PathBuf>,

        /// Path to SQLite database file (for DB-to-schema comparison)
        /// Example: --db ./game.db
        #[arg(long, conflicts_with = "baseline")]
        db: Option<PathBuf>,

        /// Path to the current (new) schema file
        #[arg(short, long)]
        schema_path: PathBuf,

        /// Path to the output directory for migration SQL
        #[arg(short, long, default_value = "output")]
        output_dir: PathBuf,

        /// Target database type (sqlite, mysql). Defaults to sqlite for DB mode.
        #[arg(short = 't', long)]
        target: Option<String>,
    },

    /// Visualize schema structure with references
    Visualize {
        /// Path to the schema file
        #[arg(short, long)]
        schema_path: PathBuf,

        /// Output format: json (for GUI) or mermaid (for documentation)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Output file path (optional, prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

/// Run the code generation pipeline with the given CLI arguments
pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Some(Commands::Generate {
            schema_path,
            templates_dir,
            output_dir,
            lang,
            baseline,
            preview,
        }) => run_generate(schema_path, templates_dir, output_dir, lang, baseline, preview),

        Some(Commands::Migrate {
            baseline,
            db,
            schema_path,
            output_dir,
            target,
        }) => run_migrate(baseline, db, schema_path, output_dir, target),

        Some(Commands::Visualize {
            schema_path,
            format,
            output,
        }) => run_visualize(schema_path, format, output),

        None => {
            // Default behavior: generate if schema_path is provided
            if let Some(schema_path) = cli.schema_path {
                run_generate(
                    schema_path,
                    cli.templates_dir,
                    cli.output_dir,
                    cli.lang,
                    cli.baseline,
                    false,
                )
            } else {
                anyhow::bail!("스키마 경로가 필요합니다. --schema-path 또는 'polygen generate --schema-path' 를 사용하세요.")
            }
        }
    }
}

/// Run the generate command
fn run_generate(
    schema_path: PathBuf,
    templates_dir: PathBuf,
    output_dir: PathBuf,
    lang: Option<String>,
    baseline: Option<PathBuf>,
    preview: bool,
) -> Result<()> {
    if preview {
        return run_generate_preview(schema_path, templates_dir, lang);
    }

    let mut config = PipelineConfig::new(schema_path, templates_dir, output_dir);

    if let Some(lang) = lang {
        config = config.with_language(lang);
    }

    if let Some(baseline) = baseline {
        config = config.with_baseline(baseline);
    }

    let pipeline = CompilationPipeline::new(config);
    pipeline.run()
}

/// Preview mode: generate to temp directory and print all generated files to stdout
fn run_generate_preview(
    schema_path: PathBuf,
    templates_dir: PathBuf,
    lang: Option<String>,
) -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let output_dir = temp_dir.path().to_path_buf();

    let mut config = PipelineConfig::new(schema_path, templates_dir, output_dir.clone())
        .with_preview_mode(true);

    if let Some(lang) = lang {
        config = config.with_language(lang);
    }

    let pipeline = CompilationPipeline::new(config);
    pipeline.run()?;

    // Load manifest to include template source info in output
    let manifest = codegen::load_manifest(&output_dir);

    // Read and print all generated files
    print_directory_contents(&output_dir, &output_dir, &manifest)?;

    Ok(())
}

/// Recursively print all file contents in a directory.
///
/// The manifest maps relative file paths to the Rhai template that generated them.
/// Output format: `=== relative/path [template_name.rhai] ===`
fn print_directory_contents(
    dir: &std::path::Path,
    base: &std::path::Path,
    manifest: &std::collections::HashMap<String, String>,
) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            // Skip debug directory
            if path.file_name().and_then(|n| n.to_str()) == Some("debug") {
                continue;
            }
            print_directory_contents(&path, base, manifest)?;
        } else {
            let relative = path.strip_prefix(base).unwrap_or(&path);
            let relative_str = relative.to_string_lossy().replace('\\', "/");

            // Skip manifest file itself
            if relative_str == codegen::MANIFEST_FILENAME {
                continue;
            }

            let content = fs::read_to_string(&path)
                .unwrap_or_else(|_| "<binary file>".to_string());

            // Include template source if available
            if let Some(template) = manifest.get(&relative_str) {
                println!("=== {} [{}] ===", relative.display(), template);
            } else {
                println!("=== {} ===", relative.display());
            }
            println!("{}", content);
            println!();
        }
    }

    Ok(())
}

/// Run the migrate command (migration-only mode)
///
/// Supports two modes:
/// 1. Schema-to-Schema: --baseline old.poly --schema-path new.poly
/// 2. DB-to-Schema: --db game.db --schema-path schema.poly
fn run_migrate(
    baseline_path: Option<PathBuf>,
    db_path: Option<PathBuf>,
    schema_path: PathBuf,
    output_dir: PathBuf,
    target: Option<String>,
) -> Result<()> {
    use crate::db_introspection::SqliteIntrospector;
    use crate::migration::MigrationDiff;

    // Parse current schema
    let current_asts = parse_and_merge_schemas(&schema_path, None)?;
    let current_defs: Vec<_> = current_asts
        .iter()
        .flat_map(|ast| ast.definitions.clone())
        .collect();
    validation::validate_ast(&current_defs)?;
    let current_ir = ir_builder::build_ir(&current_asts);

    let diff = if let Some(db_file) = db_path {
        // DB-to-Schema mode
        println!("--- DB 기반 마이그레이션 생성 모드 ---");
        println!("  데이터베이스: {}", db_file.display());
        println!("  목표 스키마: {}", schema_path.display());

        // Read schema from SQLite database
        let introspector = SqliteIntrospector::open(&db_file)?;
        let db_schema = introspector.read_schema()?;

        println!("\n  DB 현재 상태:");
        println!("    - 테이블: {} 개", db_schema.table_count());
        println!("    - 컬럼: {} 개", db_schema.column_count());

        // Compare DB with poly schema
        MigrationDiff::compare_db(&db_schema, &current_ir)
    } else if let Some(baseline) = baseline_path {
        // Schema-to-Schema mode
        println!("--- 스키마 비교 마이그레이션 생성 모드 ---");
        println!("  기준 스키마: {}", baseline.display());
        println!("  현재 스키마: {}", schema_path.display());

        let baseline_asts = parse_and_merge_schemas(&baseline, None)?;
        let baseline_defs: Vec<_> = baseline_asts
            .iter()
            .flat_map(|ast| ast.definitions.clone())
            .collect();
        validation::validate_ast(&baseline_defs)?;
        let baseline_ir = ir_builder::build_ir(&baseline_asts);

        MigrationDiff::compare(&baseline_ir, &current_ir)
    } else {
        anyhow::bail!(
            "마이그레이션 소스가 필요합니다.\n\
             --baseline <스키마파일> 또는 --db <SQLite파일> 중 하나를 지정하세요."
        );
    };

    // Report changes
    if diff.changes.is_empty() {
        println!("\n변경사항 없음. 스키마가 동기화되어 있습니다.");
        return Ok(());
    }

    println!("\n{} 개의 변경사항 감지됨:", diff.changes.len());
    for change in &diff.changes {
        match change {
            migration::SchemaChange::TableAdded { table_name, .. } => {
                println!("  + 테이블 추가: {}", table_name);
            }
            migration::SchemaChange::TableRemoved { table_name, .. } => {
                println!("  - 테이블 삭제: {}", table_name);
            }
            migration::SchemaChange::ColumnAdded { table_name, column_name, .. } => {
                println!("  + 컬럼 추가: {}.{}", table_name, column_name);
            }
            migration::SchemaChange::ColumnRemoved { table_name, column_name, .. } => {
                println!("  - 컬럼 삭제: {}.{}", table_name, column_name);
            }
            migration::SchemaChange::ColumnTypeChanged { table_name, column_name, old_type, new_type, .. } => {
                println!("  ~ 타입 변경: {}.{} ({} → {})", table_name, column_name, old_type, new_type);
            }
        }
    }

    // Print warnings
    if !diff.warnings.is_empty() {
        println!("\n경고:");
        for warning in &diff.warnings {
            println!("  ⚠️  {}", warning);
        }
    }

    // Generate migration SQL
    fs::create_dir_all(&output_dir)?;

    // Determine target database type
    let target_db = target.unwrap_or_else(|| {
        // Detect from @datasource annotations or default to sqlite
        for file in &current_ir.files {
            for ns in &file.namespaces {
                if let Some(ref ds) = ns.datasource {
                    return ds.clone();
                }
            }
        }
        "sqlite".to_string()
    });

    let sql = match target_db.as_str() {
        "sqlite" => diff.to_sqlite_sql(),
        "mysql" => diff.to_mysql_sql(),
        _ => {
            anyhow::bail!("지원하지 않는 DB 타입: {}", target_db);
        }
    };

    let db_dir = output_dir.join(&target_db);
    fs::create_dir_all(&db_dir)?;
    let migration_file = db_dir.join("migration.sql");
    fs::write(&migration_file, &sql)?;
    println!(
        "\n{} 마이그레이션 SQL 생성 완료: {}",
        target_db.to_uppercase(),
        migration_file.display()
    );

    Ok(())
}

/// Run the visualize command
fn run_visualize(
    schema_path: PathBuf,
    format: String,
    output: Option<PathBuf>,
) -> Result<()> {
    println!("--- 스키마 시각화 ---");
    println!("  스키마: {}", schema_path.display());
    println!("  포맷: {}", format);

    // Parse schema
    let asts = parse_and_merge_schemas(&schema_path, None)?;

    // Validate
    let defs: Vec<_> = asts
        .iter()
        .flat_map(|ast| ast.definitions.clone())
        .collect();
    validation::validate_ast(&defs)?;

    // Build IR
    let ir = ir_builder::build_ir(&asts);

    // Build visualization
    let viz = visualize::build_visualization(&ir);

    // Generate output
    let output_str = match format.to_lowercase().as_str() {
        "json" => serde_json::to_string_pretty(&viz)?,
        "mermaid" => visualize::to_mermaid(&viz),
        _ => anyhow::bail!("지원하지 않는 포맷: {}. 'json' 또는 'mermaid'를 사용하세요.", format),
    };

    // Write or print output
    if let Some(output_path) = output {
        fs::write(&output_path, &output_str)?;
        println!("\n시각화 출력: {}", output_path.display());
    } else {
        println!("\n{}", output_str);
    }

    // Print stats
    println!("\n--- 통계 ---");
    println!("  테이블: {} 개", viz.stats.table_count);
    println!("  필드: {} 개", viz.stats.field_count);
    println!("  관계: {} 개", viz.stats.relation_count);
    println!("  네임스페이스: {} 개", viz.stats.namespace_count);

    Ok(())
}

/// Build IR from ASTs (convenience function for backward compatibility)
pub fn build_ir_from_asts(asts: &[AstRoot]) -> SchemaContext {
    ir_builder::build_ir(asts)
}
