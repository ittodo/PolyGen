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
//! - [`rhai`]: Rhai scripting engine helpers (used by PolyTemplate `%logic` blocks)
//! - [`template`]: PolyTemplate engine (`.ptpl` template rendering)

use anyhow::Result;
use clap::{Parser as ClapParser, Subcommand};
use notify::{
    Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use pest_derive::Parser;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

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
pub mod schema_diff;
pub mod schema_lint;
pub mod schema_metadata;
pub mod schema_stats;
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

        /// Schema hash mismatch policy for --db mode: warn, fail, or force.
        #[arg(long, default_value = "warn")]
        schema_hash_policy: String,

        /// Destructive migration policy: warn, fail, or allow.
        /// warn comments destructive SQL, fail stops generation, allow emits DROP SQL.
        #[arg(long, default_value = "warn")]
        destructive_policy: String,
    },

    /// Compare two schema files and print a non-SQL diff
    Diff {
        /// Path to the old schema file
        #[arg(long, value_name = "OLD")]
        old: PathBuf,

        /// Path to the new schema file
        #[arg(long = "new", value_name = "NEW")]
        new_schema: PathBuf,

        /// Output format: text or json
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Output file path (optional, prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Run schema lint checks and print warnings
    Lint {
        /// Path to the schema file
        #[arg(short, long)]
        schema_path: PathBuf,

        /// Output format: text or json
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Output file path (optional, prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
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

    /// Generate Markdown schema documentation
    Docs {
        /// Path to the schema file
        #[arg(short, long)]
        schema_path: PathBuf,

        /// Output Markdown file path
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Print aggregate schema statistics
    Stats {
        /// Path to the schema file
        #[arg(short, long)]
        schema_path: PathBuf,

        /// Output format: text or json
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Output file path (optional, prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Watch schema and template files, regenerating code after changes
    Watch {
        /// Path to the root schema file
        #[arg(short, long = "schema-path", visible_alias = "schema")]
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

        /// Debounce interval in milliseconds before regenerating after file changes
        #[arg(long, default_value_t = 300)]
        debounce_ms: u64,
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
        }) => run_generate(
            schema_path,
            templates_dir,
            output_dir,
            lang,
            baseline,
            preview,
        ),

        Some(Commands::Migrate {
            baseline,
            db,
            schema_path,
            output_dir,
            target,
            schema_hash_policy,
            destructive_policy,
        }) => run_migrate(
            baseline,
            db,
            schema_path,
            output_dir,
            target,
            schema_hash_policy,
            destructive_policy,
        ),

        Some(Commands::Diff {
            old,
            new_schema,
            format,
            output,
        }) => run_diff(old, new_schema, format, output),

        Some(Commands::Lint {
            schema_path,
            format,
            output,
        }) => run_lint(schema_path, format, output),

        Some(Commands::Visualize {
            schema_path,
            format,
            output,
        }) => run_visualize(schema_path, format, output),

        Some(Commands::Docs {
            schema_path,
            output,
        }) => run_docs(schema_path, output),

        Some(Commands::Stats {
            schema_path,
            format,
            output,
        }) => run_stats(schema_path, format, output),

        Some(Commands::Watch {
            schema_path,
            templates_dir,
            output_dir,
            lang,
            baseline,
            debounce_ms,
        }) => run_watch(
            schema_path,
            templates_dir,
            output_dir,
            lang,
            baseline,
            debounce_ms,
        ),

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct WatchTarget {
    path: PathBuf,
    recursive: bool,
}

/// Run the watch command.
///
/// The command performs an initial generation, then watches schema, baseline, and template
/// locations for relevant source changes before regenerating.
fn run_watch(
    schema_path: PathBuf,
    templates_dir: PathBuf,
    output_dir: PathBuf,
    lang: Option<String>,
    baseline: Option<PathBuf>,
    debounce_ms: u64,
) -> Result<()> {
    let debounce = Duration::from_millis(debounce_ms);
    let targets = collect_watch_targets(&schema_path, &templates_dir, baseline.as_deref());

    if targets.is_empty() {
        anyhow::bail!("감시할 경로를 찾을 수 없습니다.");
    }

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = RecommendedWatcher::new(tx, NotifyConfig::default())?;

    println!("--- Watch 모드 시작 ---");
    println!("  스키마: {}", schema_path.display());
    println!("  템플릿: {}", templates_dir.display());
    println!("  출력: {}", output_dir.display());
    if let Some(ref lang) = lang {
        println!("  언어: {}", lang);
    }

    for target in &targets {
        let mode = if target.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        watcher.watch(&target.path, mode)?;
        println!("  감시 중: {}", target.path.display());
    }

    run_watch_generation(
        &schema_path,
        &templates_dir,
        &output_dir,
        &lang,
        &baseline,
        true,
    );

    loop {
        let changed_paths = wait_for_relevant_change(&rx, debounce)?;
        println!("\n변경 감지:");
        for path in &changed_paths {
            println!("  - {}", path.display());
        }
        run_watch_generation(
            &schema_path,
            &templates_dir,
            &output_dir,
            &lang,
            &baseline,
            false,
        );
    }
}

fn run_watch_generation(
    schema_path: &Path,
    templates_dir: &Path,
    output_dir: &Path,
    lang: &Option<String>,
    baseline: &Option<PathBuf>,
    initial: bool,
) {
    let label = if initial {
        "초기 생성"
    } else {
        "재생성"
    };
    let started = Instant::now();
    println!("\n--- {} 시작 ---", label);

    match run_generate(
        schema_path.to_path_buf(),
        templates_dir.to_path_buf(),
        output_dir.to_path_buf(),
        lang.clone(),
        baseline.clone(),
        false,
    ) {
        Ok(()) => println!("--- {} 완료 ({:.2?}) ---", label, started.elapsed()),
        Err(err) => eprintln!("--- {} 실패: {} ---", label, err),
    }
}

fn collect_watch_targets(
    schema_path: &Path,
    templates_dir: &Path,
    baseline: Option<&Path>,
) -> Vec<WatchTarget> {
    let mut targets: BTreeMap<PathBuf, bool> = BTreeMap::new();

    add_watch_parent(&mut targets, schema_path, true);
    add_watch_path(&mut targets, templates_dir, true);

    if let Some(baseline) = baseline {
        add_watch_parent(&mut targets, baseline, true);
    }

    targets
        .into_iter()
        .map(|(path, recursive)| WatchTarget { path, recursive })
        .collect()
}

fn add_watch_parent(targets: &mut BTreeMap<PathBuf, bool>, path: &Path, recursive: bool) {
    if let Some(parent) = path.parent() {
        add_watch_path(targets, parent, recursive);
    }
}

fn add_watch_path(targets: &mut BTreeMap<PathBuf, bool>, path: &Path, recursive: bool) {
    let normalized = canonical_or_original(path);
    if !normalized.exists() {
        return;
    }

    targets
        .entry(normalized)
        .and_modify(|existing| *existing |= recursive)
        .or_insert(recursive);
}

fn canonical_or_original(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn wait_for_relevant_change(
    rx: &Receiver<notify::Result<Event>>,
    debounce: Duration,
) -> Result<Vec<PathBuf>> {
    let mut changed_paths = loop {
        match rx.recv()? {
            Ok(event) if is_relevant_event(&event) => break event.paths,
            Ok(_) => continue,
            Err(err) => eprintln!("파일 감시 오류: {}", err),
        }
    };

    let mut deadline = Instant::now() + debounce;
    loop {
        let now = Instant::now();
        if now >= deadline {
            break;
        }

        match rx.recv_timeout(deadline - now) {
            Ok(Ok(event)) if is_relevant_event(&event) => {
                changed_paths.extend(event.paths);
                deadline = Instant::now() + debounce;
            }
            Ok(Ok(_)) => {}
            Ok(Err(err)) => eprintln!("파일 감시 오류: {}", err),
            Err(mpsc::RecvTimeoutError::Timeout) => break,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                anyhow::bail!("파일 감시 채널이 종료되었습니다.");
            }
        }
    }

    changed_paths.retain(|path| is_relevant_path(path));
    changed_paths.sort();
    changed_paths.dedup();
    Ok(changed_paths)
}

fn is_relevant_event(event: &Event) -> bool {
    if matches!(event.kind, EventKind::Access(_)) {
        return false;
    }

    event.paths.iter().any(|path| is_relevant_path(path))
}

fn is_relevant_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("poly" | "renames" | "ptpl" | "rhai" | "toml")
    )
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

    let mut config =
        PipelineConfig::new(schema_path, templates_dir, output_dir.clone()).with_preview_mode(true);

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
    manifest: &codegen::TemplateManifest,
) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?;
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

            let content = fs::read_to_string(&path).unwrap_or_else(|_| "<binary file>".to_string());

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

fn build_ir_from_schema_quiet(schema_path: &Path) -> Result<SchemaContext> {
    let asts = pipeline::parse_and_merge_schemas_quiet(schema_path, None)?;
    let defs: Vec<_> = asts
        .iter()
        .flat_map(|ast| ast.definitions.clone())
        .collect();
    validation::validate_ast(&defs)?;
    Ok(ir_builder::build_ir(&asts))
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
    schema_hash_policy: String,
    destructive_policy: String,
) -> Result<()> {
    use crate::db_introspection::SqliteIntrospector;
    use crate::migration::{DestructiveChangePolicy, MigrationDiff};
    use crate::schema_metadata::build_schema_metadata;

    // Parse current schema
    let current_asts = parse_and_merge_schemas(&schema_path, None)?;
    let current_defs: Vec<_> = current_asts
        .iter()
        .flat_map(|ast| ast.definitions.clone())
        .collect();
    validation::validate_ast(&current_defs)?;
    let current_ir = ir_builder::build_ir(&current_asts);
    let current_metadata = build_schema_metadata(&current_ir)?;
    let hash_policy = SchemaHashPolicy::parse(&schema_hash_policy)?;
    let destructive_policy = DestructiveChangePolicy::parse(&destructive_policy)?;

    let diff = if let Some(db_file) = db_path {
        // DB-to-Schema mode
        println!("--- DB 기반 마이그레이션 생성 모드 ---");
        println!("  데이터베이스: {}", db_file.display());
        println!("  목표 스키마: {}", schema_path.display());

        // Read schema from SQLite database
        let introspector = SqliteIntrospector::open(&db_file)?;
        let stored_metadata = introspector.read_polygen_schema_metadata()?;
        let db_schema = introspector.read_schema()?;

        println!("\n  DB 현재 상태:");
        println!("    - 테이블: {} 개", db_schema.table_count());
        println!("    - 컬럼: {} 개", db_schema.column_count());
        match stored_metadata.and_then(|m| m.schema_hash) {
            Some(stored_hash) if stored_hash == current_metadata.hash => {
                println!("    - 스키마 해시: 일치 ({})", stored_hash);
            }
            Some(stored_hash) => match hash_policy {
                SchemaHashPolicy::Warn => {
                    println!(
                        "    - 스키마 해시: 불일치 (DB: {}, 현재: {})",
                        stored_hash, current_metadata.hash
                    );
                }
                SchemaHashPolicy::Force => {
                    println!(
                        "    - 스키마 해시: 불일치, force로 계속 진행 (DB: {}, 현재: {})",
                        stored_hash, current_metadata.hash
                    );
                }
                SchemaHashPolicy::Fail => {
                    anyhow::bail!(
                            "스키마 해시가 DB와 현재 스키마 간에 일치하지 않습니다. \
                             DB: {}, 현재: {}. 계속하려면 --schema-hash-policy warn 또는 force를 사용하세요.",
                            stored_hash,
                            current_metadata.hash
                        );
                }
            },
            None => {
                println!(
                    "    - 스키마 해시: 기록 없음 (현재: {})",
                    current_metadata.hash
                );
            }
        }

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
    } else {
        println!("\n{} 개의 변경사항 감지됨:", diff.changes.len());
        for change in &diff.changes {
            match change {
                migration::SchemaChange::TableAdded { table_name, .. } => {
                    println!("  + 테이블 추가: {}", table_name);
                }
                migration::SchemaChange::TableRemoved { table_name, .. } => {
                    println!("  - 테이블 삭제: {}", table_name);
                }
                migration::SchemaChange::ColumnAdded {
                    table_name,
                    column_name,
                    ..
                } => {
                    println!("  + 컬럼 추가: {}.{}", table_name, column_name);
                }
                migration::SchemaChange::ColumnRemoved {
                    table_name,
                    column_name,
                    ..
                } => {
                    println!("  - 컬럼 삭제: {}.{}", table_name, column_name);
                }
                migration::SchemaChange::ColumnTypeChanged {
                    table_name,
                    column_name,
                    old_type,
                    new_type,
                    ..
                } => {
                    println!(
                        "  ~ 타입 변경: {}.{} ({} → {})",
                        table_name, column_name, old_type, new_type
                    );
                }
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

    if destructive_policy == DestructiveChangePolicy::Fail && diff.has_destructive_changes() {
        let details = diff.destructive_change_descriptions().join("\n  - ");
        anyhow::bail!(
            "파괴적 마이그레이션 변경이 감지되어 중단합니다.\n  - {}\n\
             계속하려면 --destructive-policy warn 또는 allow를 사용하세요.",
            details
        );
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
        "sqlite" => diff.to_sqlite_sql_with_schema_and_policy(&current_ir, destructive_policy),
        "mysql" => diff.to_mysql_sql_with_policy(destructive_policy),
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

/// Run the diff command.
fn run_diff(
    old_schema: PathBuf,
    new_schema: PathBuf,
    format: String,
    output: Option<PathBuf>,
) -> Result<()> {
    let old_ir = build_ir_from_schema_quiet(&old_schema)?;
    let new_ir = build_ir_from_schema_quiet(&new_schema)?;
    let diff = migration::MigrationDiff::compare(&old_ir, &new_ir);
    let report = schema_diff::build_diff_report(&diff);
    let output_str = match format.to_lowercase().as_str() {
        "text" => schema_diff::render_diff_text(&report),
        "json" => serde_json::to_string_pretty(&report)?,
        _ => anyhow::bail!(
            "지원하지 않는 포맷: {}. 'text' 또는 'json'을 사용하세요.",
            format
        ),
    };

    if let Some(output_path) = output {
        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(&output_path, output_str)?;
        println!("Diff 출력: {}", output_path.display());
    } else {
        println!("{}", output_str);
    }

    Ok(())
}

/// Run the lint command.
fn run_lint(schema_path: PathBuf, format: String, output: Option<PathBuf>) -> Result<()> {
    let asts = pipeline::parse_and_merge_schemas_quiet(&schema_path, None)?;
    let defs: Vec<_> = asts
        .iter()
        .flat_map(|ast| ast.definitions.clone())
        .collect();
    validation::validate_ast(&defs)?;

    let report = schema_lint::lint_asts(&asts);
    let output_str = match format.to_lowercase().as_str() {
        "text" => schema_lint::render_lint_text(&report),
        "json" => serde_json::to_string_pretty(&report)?,
        _ => anyhow::bail!(
            "지원하지 않는 포맷: {}. 'text' 또는 'json'을 사용하세요.",
            format
        ),
    };

    if let Some(output_path) = output {
        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(&output_path, output_str)?;
        println!("Lint 출력: {}", output_path.display());
    } else {
        println!("{}", output_str);
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SchemaHashPolicy {
    Warn,
    Fail,
    Force,
}

impl SchemaHashPolicy {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "warn" => Ok(Self::Warn),
            "fail" => Ok(Self::Fail),
            "force" => Ok(Self::Force),
            _ => anyhow::bail!(
                "지원하지 않는 schema hash policy: {}. warn, fail, force 중 하나를 사용하세요.",
                value
            ),
        }
    }
}

/// Run the visualize command
fn run_visualize(schema_path: PathBuf, format: String, output: Option<PathBuf>) -> Result<()> {
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
        _ => anyhow::bail!(
            "지원하지 않는 포맷: {}. 'json' 또는 'mermaid'를 사용하세요.",
            format
        ),
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

/// Run the docs command.
fn run_docs(schema_path: PathBuf, output: PathBuf) -> Result<()> {
    println!("--- 스키마 문서 생성 ---");
    println!("  스키마: {}", schema_path.display());
    println!("  출력: {}", output.display());

    let asts = parse_and_merge_schemas(&schema_path, None)?;
    let defs: Vec<_> = asts
        .iter()
        .flat_map(|ast| ast.definitions.clone())
        .collect();
    validation::validate_ast(&defs)?;

    let ir = ir_builder::build_ir(&asts);
    let viz = visualize::build_visualization(&ir);
    let markdown = visualize::to_markdown(&viz);

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    fs::write(&output, markdown)?;
    println!("문서 생성 완료: {}", output.display());
    Ok(())
}

/// Run the stats command.
fn run_stats(schema_path: PathBuf, format: String, output: Option<PathBuf>) -> Result<()> {
    let ir = build_ir_from_schema_quiet(&schema_path)?;
    let stats = schema_stats::build_schema_stats(&ir);
    let output_str = match format.to_lowercase().as_str() {
        "text" => schema_stats::render_stats_text(&stats),
        "json" => serde_json::to_string_pretty(&stats)?,
        _ => anyhow::bail!(
            "지원하지 않는 포맷: {}. 'text' 또는 'json'을 사용하세요.",
            format
        ),
    };

    if let Some(output_path) = output {
        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(&output_path, output_str)?;
        println!("통계 출력: {}", output_path.display());
    } else {
        println!("{}", output_str);
    }

    Ok(())
}

/// Build IR from ASTs (convenience function for backward compatibility)
pub fn build_ir_from_asts(asts: &[AstRoot]) -> SchemaContext {
    ir_builder::build_ir(asts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_watch_command_accepts_schema_alias() {
        let cli = Cli::try_parse_from([
            "polygen",
            "watch",
            "--schema",
            "examples/game_schema.poly",
            "--lang",
            "csharp",
        ])
        .expect("watch command should parse --schema alias");

        match cli.command {
            Some(Commands::Watch {
                schema_path, lang, ..
            }) => {
                assert_eq!(schema_path, PathBuf::from("examples/game_schema.poly"));
                assert_eq!(lang, Some("csharp".to_string()));
            }
            other => panic!("expected watch command, got {other:?}"),
        }
    }

    #[test]
    fn test_collect_watch_targets_deduplicates_schema_and_baseline_parent() {
        let temp = TempDir::new().expect("temp dir");
        let schema = temp.path().join("current.poly");
        let baseline = temp.path().join("baseline.poly");
        let templates = temp.path().join("templates");
        fs::write(&schema, "namespace game {}").expect("schema");
        fs::write(&baseline, "namespace game {}").expect("baseline");
        fs::create_dir(&templates).expect("templates");

        let targets = collect_watch_targets(&schema, &templates, Some(&baseline));

        assert_eq!(targets.len(), 2);
        assert!(targets
            .iter()
            .any(|target| target.path == temp.path().canonicalize().unwrap()));
        assert!(targets
            .iter()
            .any(|target| target.path == templates.canonicalize().unwrap()));
        assert!(targets.iter().all(|target| target.recursive));
    }

    #[test]
    fn test_relevant_watch_paths() {
        assert!(is_relevant_path(Path::new("schema.poly")));
        assert!(is_relevant_path(Path::new("renames.renames")));
        assert!(is_relevant_path(Path::new("template.ptpl")));
        assert!(is_relevant_path(Path::new("helper.rhai")));
        assert!(is_relevant_path(Path::new("lang.toml")));
        assert!(!is_relevant_path(Path::new("notes.md")));
    }

    #[test]
    fn test_docs_command_requires_output() {
        let cli = Cli::try_parse_from([
            "polygen",
            "docs",
            "--schema-path",
            "examples/game_schema.poly",
            "--output",
            "docs/schema.md",
        ])
        .expect("docs command should parse");

        match cli.command {
            Some(Commands::Docs {
                schema_path,
                output,
            }) => {
                assert_eq!(schema_path, PathBuf::from("examples/game_schema.poly"));
                assert_eq!(output, PathBuf::from("docs/schema.md"));
            }
            other => panic!("expected docs command, got {other:?}"),
        }
    }

    #[test]
    fn test_stats_command_accepts_json_format() {
        let cli = Cli::try_parse_from([
            "polygen",
            "stats",
            "--schema-path",
            "examples/game_schema.poly",
            "--format",
            "json",
        ])
        .expect("stats command should parse");

        match cli.command {
            Some(Commands::Stats {
                schema_path,
                format,
                output,
            }) => {
                assert_eq!(schema_path, PathBuf::from("examples/game_schema.poly"));
                assert_eq!(format, "json");
                assert_eq!(output, None);
            }
            other => panic!("expected stats command, got {other:?}"),
        }
    }

    #[test]
    fn test_diff_command_accepts_old_new_and_json_format() {
        let cli = Cli::try_parse_from([
            "polygen",
            "diff",
            "--old",
            "schemas/v1.poly",
            "--new",
            "schemas/v2.poly",
            "--format",
            "json",
        ])
        .expect("diff command should parse");

        match cli.command {
            Some(Commands::Diff {
                old,
                new_schema,
                format,
                output,
            }) => {
                assert_eq!(old, PathBuf::from("schemas/v1.poly"));
                assert_eq!(new_schema, PathBuf::from("schemas/v2.poly"));
                assert_eq!(format, "json");
                assert_eq!(output, None);
            }
            other => panic!("expected diff command, got {other:?}"),
        }
    }

    #[test]
    fn test_lint_command_accepts_json_format() {
        let cli = Cli::try_parse_from([
            "polygen",
            "lint",
            "--schema-path",
            "examples/game_schema.poly",
            "--format",
            "json",
        ])
        .expect("lint command should parse");

        match cli.command {
            Some(Commands::Lint {
                schema_path,
                format,
                output,
            }) => {
                assert_eq!(schema_path, PathBuf::from("examples/game_schema.poly"));
                assert_eq!(format, "json");
                assert_eq!(output, None);
            }
            other => panic!("expected lint command, got {other:?}"),
        }
    }
}
