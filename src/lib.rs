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
pub mod error;
pub mod ir_builder;
pub mod ir_model;
pub mod lang_config;
pub mod migration;
pub mod pipeline;
pub mod rhai;
pub mod rhai_generator;
pub mod symbol_table;
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
    },

    /// Generate migration SQL by comparing two schema versions
    Migrate {
        /// Path to the baseline (old) schema file
        #[arg(short, long)]
        baseline: PathBuf,

        /// Path to the current (new) schema file
        #[arg(short, long)]
        schema_path: PathBuf,

        /// Path to the output directory for migration SQL
        #[arg(short, long, default_value = "output")]
        output_dir: PathBuf,

        /// Target database (sqlite, mysql). Defaults to all detected @datasource values.
        #[arg(short = 'd', long)]
        database: Option<String>,
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
        }) => run_generate(schema_path, templates_dir, output_dir, lang, baseline),

        Some(Commands::Migrate {
            baseline,
            schema_path,
            output_dir,
            database,
        }) => run_migrate(baseline, schema_path, output_dir, database),

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
) -> Result<()> {
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

/// Run the migrate command (migration-only mode)
fn run_migrate(
    baseline_path: PathBuf,
    schema_path: PathBuf,
    output_dir: PathBuf,
    database: Option<String>,
) -> Result<()> {
    use crate::migration::MigrationDiff;

    println!("--- 마이그레이션 생성 모드 ---");
    println!("  기준 스키마: {}", baseline_path.display());
    println!("  현재 스키마: {}", schema_path.display());

    // Parse both schemas
    let baseline_asts = parse_and_merge_schemas(&baseline_path, None)?;
    let current_asts = parse_and_merge_schemas(&schema_path, None)?;

    // Validate both
    let baseline_defs: Vec<_> = baseline_asts
        .iter()
        .flat_map(|ast| ast.definitions.clone())
        .collect();
    let current_defs: Vec<_> = current_asts
        .iter()
        .flat_map(|ast| ast.definitions.clone())
        .collect();

    validation::validate_ast(&baseline_defs)?;
    validation::validate_ast(&current_defs)?;

    // Build IR
    let baseline_ir = ir_builder::build_ir(&baseline_asts);
    let current_ir = ir_builder::build_ir(&current_asts);

    // Compare schemas
    let diff = MigrationDiff::compare(&baseline_ir, &current_ir);

    // Report changes
    if diff.changes.is_empty() {
        println!("\n변경사항 없음.");
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

    // Determine which databases to generate for
    let databases = if let Some(db) = database {
        vec![db]
    } else {
        // Detect from @datasource annotations
        let mut dbs = vec![];
        for file in &current_ir.files {
            for ns in &file.namespaces {
                if let Some(ref ds) = ns.datasource {
                    if !dbs.contains(ds) {
                        dbs.push(ds.clone());
                    }
                }
            }
        }
        if dbs.is_empty() {
            vec!["sqlite".to_string()] // Default to SQLite
        } else {
            dbs
        }
    };

    for db in &databases {
        let sql = match db.as_str() {
            "sqlite" => diff.to_sqlite_sql(),
            "mysql" => diff.to_mysql_sql(),
            _ => {
                println!("  (지원하지 않는 DB: {})", db);
                continue;
            }
        };

        let db_dir = output_dir.join(db);
        fs::create_dir_all(&db_dir)?;
        let migration_file = db_dir.join("migration.sql");
        fs::write(&migration_file, &sql)?;
        println!("\n{} 마이그레이션 SQL 생성: {}", db.to_uppercase(), migration_file.display());
    }

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
