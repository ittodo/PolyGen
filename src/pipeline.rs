//! Compilation pipeline module.
//!
//! This module provides the high-level orchestration for the schema compilation process.
//! It handles the complete workflow from parsing schema files to generating target code.
//!
//! # Pipeline Stages
//!
//! 1. **Preparation**: Create/clean output directory
//! 2. **Parsing**: Parse all schema files (including imports) into ASTs
//! 3. **Validation**: Validate ASTs for correctness
//! 4. **IR Building**: Transform ASTs into the Intermediate Representation
//! 5. **Code Generation**: Generate code for target languages using Rhai templates
//!
//! # Example
//!
//! ```ignore
//! use polygen::{CompilationPipeline, PipelineConfig};
//!
//! let config = PipelineConfig::new(
//!     "schemas/main.poly".into(),
//!     "templates".into(),
//!     "output".into(),
//! )
//! .with_language("csharp");
//!
//! let pipeline = CompilationPipeline::new(config);
//! pipeline.run()?;
//! ```

use anyhow::Result;
use pest::Parser;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast_model::{AstRoot, Definition};
use crate::codegen::{discover_languages, CodeGenerator};
use crate::ir_model::SchemaContext;
use crate::{ast_parser, ir_builder, schema_lint, sources_config, validation, Polygen, Rule};

/// Configuration for the compilation pipeline.
///
/// Specifies the input schema, template directory, output directory,
/// and optional settings like target language and debug output.
pub struct PipelineConfig {
    /// Path to the root schema file (entry point).
    pub schema_path: PathBuf,
    /// Directory containing Rhai templates organized by language.
    pub templates_dir: PathBuf,
    /// Base output directory for generated code.
    pub output_dir: PathBuf,
    /// Optional specific language to generate. If `None`, generates for all languages.
    pub target_lang: Option<String>,
    /// Whether to write debug output files (AST, IR dumps).
    pub debug_output: bool,
    /// Optional baseline schema path for migration diff generation.
    pub baseline_path: Option<PathBuf>,
    /// Optional external sources configuration path.
    pub sources_path: Option<PathBuf>,
    /// Whether to enable preview mode (source marker injection in generated code).
    pub preview_mode: bool,
}

impl PipelineConfig {
    /// Creates a new pipeline configuration with the given paths.
    ///
    /// By default, debug output is enabled and all languages are targeted.
    pub fn new(schema_path: PathBuf, templates_dir: PathBuf, output_dir: PathBuf) -> Self {
        Self {
            schema_path,
            templates_dir,
            output_dir,
            target_lang: None,
            debug_output: true,
            baseline_path: None,
            sources_path: None,
            preview_mode: false,
        }
    }

    /// Sets a specific target language for code generation.
    ///
    /// Use this to generate code for only one language instead of all available.
    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.target_lang = Some(lang.into());
        self
    }

    /// Enables or disables debug output file generation.
    pub fn with_debug_output(mut self, enabled: bool) -> Self {
        self.debug_output = enabled;
        self
    }

    /// Sets a baseline schema path for migration diff generation.
    ///
    /// When set, the pipeline will compare the current schema with the baseline
    /// and generate migration SQL files.
    pub fn with_baseline(mut self, baseline_path: PathBuf) -> Self {
        self.baseline_path = Some(baseline_path);
        self
    }

    /// Sets an external sources configuration path.
    pub fn with_sources(mut self, sources_path: PathBuf) -> Self {
        self.sources_path = Some(sources_path);
        self
    }

    /// Enables preview mode (source marker injection in generated code).
    pub fn with_preview_mode(mut self, enabled: bool) -> Self {
        self.preview_mode = enabled;
        self
    }
}

/// The main compilation pipeline for schema-to-code generation.
///
/// Orchestrates the complete process from parsing schema files to
/// generating code in one or more target languages.
pub struct CompilationPipeline {
    config: PipelineConfig,
}

impl CompilationPipeline {
    /// Creates a new compilation pipeline with the given configuration.
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    /// Runs the complete compilation pipeline.
    ///
    /// This executes all pipeline stages in order:
    /// 1. Prepare output directory
    /// 2. Parse schema files
    /// 3. Validate ASTs
    /// 4. Build IR
    /// 5. Generate migration diff (if baseline provided)
    /// 6. Generate code
    ///
    /// # Errors
    ///
    /// Returns an error if any stage fails (parsing, validation, IO, etc.).
    pub fn run(&self) -> Result<()> {
        self.prepare_output_dir()?;
        let asts = self.parse_schemas()?;
        self.validate_asts(&asts)?;
        let ir_context = self.build_ir(&asts)?;

        // Generate migration diff if baseline is provided
        if let Some(ref baseline_path) = self.config.baseline_path {
            self.generate_migration_diff(baseline_path, &ir_context)?;
        }

        self.generate_code(&ir_context)?;
        Ok(())
    }

    /// Generate migration diff by comparing baseline and current schema.
    fn generate_migration_diff(
        &self,
        baseline_path: &Path,
        current_ir: &SchemaContext,
    ) -> Result<()> {
        use crate::migration::MigrationDiff;

        println!("\n--- 마이그레이션 diff 생성 중 ---");
        println!("  - 기준 스키마: {}", baseline_path.display());
        println!("  - 현재 스키마: {}", self.config.schema_path.display());

        // Parse baseline schema
        let baseline_asts = parse_and_merge_schemas(baseline_path, None)?;

        // Validate baseline
        let baseline_definitions: Vec<Definition> = baseline_asts
            .iter()
            .flat_map(|ast| ast.definitions.clone())
            .collect();
        validation::validate_ast(&baseline_definitions)?;

        // Build baseline IR
        let baseline_ir = ir_builder::build_ir(&baseline_asts);

        // Compare schemas
        let diff = MigrationDiff::compare(&baseline_ir, current_ir);

        // Report changes
        if diff.changes.is_empty() {
            println!("  - 변경사항 없음");
        } else {
            println!("  - {} 개의 변경사항 감지됨", diff.changes.len());
            for warning in &diff.warnings {
                println!("  ⚠️  {}", warning);
            }
        }

        // Generate SQLite migration SQL
        let sql = diff.to_sqlite_sql_with_schema(current_ir);
        let migration_dir = self.config.output_dir.join("sqlite");
        fs::create_dir_all(&migration_dir)?;
        let migration_file = migration_dir.join("migration_diff.sql");
        fs::write(&migration_file, &sql)?;
        println!("  - 마이그레이션 SQL 생성: {}", migration_file.display());

        Ok(())
    }

    /// Prepare the output directory
    fn prepare_output_dir(&self) -> Result<()> {
        if self.config.output_dir.exists() {
            println!(
                "기존 출력 디렉토리 삭제 중: {}",
                self.config.output_dir.display()
            );
            fs::remove_dir_all(&self.config.output_dir)?;
        }
        println!(
            "출력 디렉토리 생성 중: {}",
            self.config.output_dir.display()
        );
        fs::create_dir_all(&self.config.output_dir)?;
        Ok(())
    }

    /// Parse all schema files
    fn parse_schemas(&self) -> Result<Vec<AstRoot>> {
        println!("--- 스키마 처리 시작 ---");
        let asts =
            parse_and_merge_schemas(&self.config.schema_path, Some(&self.config.output_dir))?;

        if self.config.debug_output {
            let ast_debug_path = self.config.output_dir.join("ast_debug.txt");
            fs::write(&ast_debug_path, format!("{:#?}", asts))?;
            println!(
                "AST 디버그 출력이 파일에 저장되었습니다: {}",
                ast_debug_path.display()
            );
        }

        Ok(asts)
    }

    /// Validate the parsed ASTs
    fn validate_asts(&self, asts: &[AstRoot]) -> Result<()> {
        println!("--- AST 유효성 검사 중 ---");
        let all_definitions: Vec<Definition> = asts
            .iter()
            .flat_map(|ast| ast.definitions.clone())
            .collect();
        validation::validate_ast(&all_definitions)?;
        println!("AST 유효성 검사 성공.");

        let lint_report = schema_lint::lint_asts(asts);
        if !lint_report.is_empty() {
            println!("\n--- 스키마 경고 ---");
            for warning in &lint_report.warnings {
                println!("  ⚠️  {}", schema_lint::format_lint_warning(warning));
            }
        }

        Ok(())
    }

    /// Build the IR from ASTs
    fn build_ir(&self, asts: &[AstRoot]) -> Result<SchemaContext> {
        println!("\n--- AST를 IR로 변환 중 ---");
        let mut ir_context = ir_builder::build_ir(asts);
        self.apply_sources_config(&mut ir_context)?;
        println!("IR 변환 성공.");

        if self.config.debug_output {
            let ir_debug_path = self.config.output_dir.join("ir_debug.txt");
            if let Err(e) = fs::write(&ir_debug_path, format!("{:#?}", ir_context)) {
                eprintln!("IR 디버그 파일 쓰기 실패: {}", e);
            } else {
                println!(
                    "IR 디버그 출력이 파일에 저장되었습니다: {}",
                    ir_debug_path.display()
                );
            }
        }

        Ok(ir_context)
    }

    fn apply_sources_config(&self, ir_context: &mut SchemaContext) -> Result<()> {
        let loaded = sources_config::load_sources_config(
            &self.config.schema_path,
            self.config.sources_path.as_deref(),
        )?;

        let Some((path, config)) = loaded else {
            return Ok(());
        };

        println!("  - sources config: {}", path.display());
        sources_config::apply_sources_config(ir_context, &config)?;
        Ok(())
    }

    /// Generate code for all target languages
    fn generate_code(&self, ir_context: &SchemaContext) -> Result<()> {
        let languages = self.get_target_languages();

        for lang in languages {
            self.generate_for_language(&lang, ir_context)?;
        }

        Ok(())
    }

    /// Get the list of target languages
    fn get_target_languages(&self) -> Vec<String> {
        if let Some(ref lang) = self.config.target_lang {
            vec![lang.clone()]
        } else {
            discover_languages(&self.config.templates_dir)
        }
    }

    /// Generate code for a specific language.
    ///
    /// This method:
    /// 1. Creates a CodeGenerator which loads the language config if available
    /// 2. Copies static files (from config or defaults)
    /// 3. Runs the main template
    /// 4. Runs any extra templates defined in the config
    /// 5. Generates datasource outputs for any @datasource annotations found
    fn generate_for_language(&self, lang: &str, ir_context: &SchemaContext) -> Result<()> {
        println!("\n--- {} 코드 생성 중 ---", lang.to_uppercase());

        let generator = CodeGenerator::new(
            lang,
            self.config.templates_dir.clone(),
            self.config.output_dir.clone(),
        )
        .with_preview_mode(self.config.preview_mode);

        // Get the current working directory for resolving static file paths
        let base_dir = std::env::current_dir()?;

        // Copy static files from language configuration
        generator.copy_configured_static_files(&base_dir)?;

        // Generate main code
        generator.generate(ir_context)?;

        // Generate extra templates from configuration
        generator.generate_extras(ir_context)?;

        // Generate datasource outputs for @datasource annotations (if not already generating one).
        if !is_db_language(lang) {
            self.generate_datasource_outputs(ir_context)?;
        }

        println!("{} 코드 생성이 완료되었습니다.", lang.to_uppercase());
        Ok(())
    }

    /// Generate outputs for all @datasource annotations found in the schema.
    ///
    /// Scans the IR for @datasource annotations and generates corresponding
    /// files using the appropriate datasource templates.
    fn generate_datasource_outputs(&self, ir_context: &SchemaContext) -> Result<()> {
        let datasources = collect_datasource_output_languages(ir_context);

        if datasources.is_empty() {
            return Ok(());
        }

        println!("\n--- @datasource 산출물 생성 중 ---");

        for datasource in datasources {
            println!("  - {} 산출물 생성", datasource);

            let generator = CodeGenerator::new(
                &datasource,
                self.config.templates_dir.clone(),
                self.config.output_dir.clone(),
            )
            .with_preview_mode(self.config.preview_mode);

            // Check if template exists for this datasource
            if generator.has_template(&format!("{}_file.ptpl", datasource)) {
                generator.generate(ir_context)?;
                generator.generate_extras(ir_context)?;
            } else {
                println!("    (템플릿 없음: {}_file.ptpl)", datasource);
            }
        }

        Ok(())
    }
}

/// Check if a language is a datasource output generator.
fn is_db_language(lang: &str) -> bool {
    matches!(lang, "sqlite" | "mysql" | "postgresql" | "redis")
}

/// Maps user-facing datasource names to template language directories.
fn datasource_output_language(datasource: &str) -> &str {
    match datasource {
        "mariadb" => "mysql",
        "postgres" => "postgresql",
        "cache" => "redis",
        other => other,
    }
}

/// Collect all unique datasource output languages from the IR.
fn collect_datasource_output_languages(ir_context: &SchemaContext) -> Vec<String> {
    use std::collections::HashSet;

    let mut datasources: HashSet<String> = HashSet::new();

    for file in &ir_context.files {
        for ns in &file.namespaces {
            collect_datasources_from_namespace(ns, &mut datasources);
        }
    }

    let mut result: Vec<String> = datasources
        .into_iter()
        .map(|ds| datasource_output_language(&ds).to_string())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    result.sort();
    result
}

/// Recursively collect datasources from a namespace and its items
fn collect_datasources_from_namespace(
    ns: &crate::ir_model::NamespaceDef,
    datasources: &mut std::collections::HashSet<String>,
) {
    // Check namespace-level datasource
    if let Some(ref ds) = ns.datasource {
        datasources.insert(ds.clone());
    }

    // Check items
    for item in &ns.items {
        match item {
            crate::ir_model::NamespaceItem::Struct(s) => {
                if let Some(ref ds) = s.datasource {
                    datasources.insert(ds.clone());
                }
            }
            crate::ir_model::NamespaceItem::Namespace(nested_ns) => {
                collect_datasources_from_namespace(nested_ns, datasources);
            }
            _ => {}
        }
    }
}

/// Parses and merges all schema files starting from an initial entry point.
///
/// This function performs a breadth-first traversal of schema files, following
/// `import` statements to discover and parse all dependent schemas.
///
/// # Arguments
///
/// * `initial_path` - The entry point schema file
/// * `output_dir` - Optional output directory for debug files (parse tree dump)
///
/// # Returns
///
/// A vector of parsed AST roots, one for each schema file processed.
///
/// # Errors
///
/// Returns an error if any file cannot be read or parsed.
pub fn parse_and_merge_schemas(
    initial_path: &Path,
    output_dir: Option<&Path>,
) -> Result<Vec<AstRoot>> {
    parse_and_merge_schemas_inner(initial_path, output_dir, true)
}

/// Parses and merges schema files without progress logging.
pub fn parse_and_merge_schemas_quiet(
    initial_path: &Path,
    output_dir: Option<&Path>,
) -> Result<Vec<AstRoot>> {
    parse_and_merge_schemas_inner(initial_path, output_dir, false)
}

fn parse_and_merge_schemas_inner(
    initial_path: &Path,
    output_dir: Option<&Path>,
    verbose: bool,
) -> Result<Vec<AstRoot>> {
    let mut files_to_process: VecDeque<PathBuf> = VecDeque::new();
    let mut processed_files: HashSet<PathBuf> = HashSet::new();
    let mut all_asts: Vec<AstRoot> = Vec::new();

    let initial_path_buf = initial_path.to_path_buf();
    files_to_process.push_back(initial_path_buf.clone());
    processed_files.insert(initial_path_buf.canonicalize()?);

    let mut is_first_file = true;

    while let Some(current_path) = files_to_process.pop_front() {
        if verbose {
            println!("--- 스키마 파싱 중: {} ---", current_path.display());
        }
        let unparsed_file = fs::read_to_string(&current_path)?.replace("\r\n", "\n");
        let main_pair = Polygen::parse(Rule::main, &unparsed_file)?
            .next()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "스키마 파일에서 main 규칙을 찾을 수 없습니다: {}",
                    current_path.display()
                )
            })?;

        if is_first_file {
            if let Some(output_dir) = output_dir {
                let debug_dir = output_dir.join("debug");
                fs::create_dir_all(&debug_dir)?;
                let parse_tree_path = debug_dir.join("parse_tree.txt");
                fs::write(&parse_tree_path, format!("{:#?}", main_pair.clone()))?;
                if verbose {
                    println!(
                        "Pest 파싱 트리 디버그 출력이 파일에 저장되었습니다: {}",
                        parse_tree_path.display()
                    );
                }
            }
            is_first_file = false;
        }

        let mut ast_root = ast_parser::build_ast_from_pairs(main_pair, current_path.clone())?;
        let file_imports = ast_root.file_imports.clone();

        let base_dir = current_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("파일의 부모 디렉토리를 찾을 수 없습니다."))?;

        // Process imports - separate .renames from .poly files
        for import_path_str in file_imports {
            let import_path = base_dir.join(&import_path_str);

            // Handle .renames files specially
            if import_path_str.ends_with(".renames") {
                let renames_content = fs::read_to_string(&import_path)?.replace("\r\n", "\n");
                let renames_pair = Polygen::parse(Rule::renames_file, &renames_content)?
                    .next()
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            ".renames 파일에서 renames_file 규칙을 찾을 수 없습니다: {}",
                            import_path.display()
                        )
                    })?;
                let renames = ast_parser::parse_renames_file(renames_pair)?;
                let count = renames.len();
                ast_root.renames.extend(renames);
                if verbose {
                    println!("  - Loaded {} rename rules from {}", count, import_path_str);
                }
            } else {
                // Regular .poly file - add to processing queue
                let canonical_import_path = import_path.canonicalize()?;
                if !processed_files.contains(&canonical_import_path) {
                    processed_files.insert(canonical_import_path);
                    files_to_process.push_back(import_path);
                }
            }
        }

        all_asts.push(ast_root);
    }

    Ok(all_asts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir_model::{FileDef, NamespaceDef, NamespaceItem, StructDef};

    fn table(name: &str, datasource: Option<&str>) -> NamespaceItem {
        NamespaceItem::Struct(Box::new(StructDef {
            name: name.to_string(),
            fqn: format!("game.{name}"),
            is_embed: false,
            datasource: datasource.map(String::from),
            cache_strategy: None,
            load: None,
            is_readonly: false,
            soft_delete_field: None,
            pack_separator: None,
            header: Vec::new(),
            items: Vec::new(),
            indexes: Vec::new(),
            relations: Vec::new(),
        }))
    }

    #[test]
    fn datasource_output_language_maps_cache_to_redis() {
        assert_eq!(datasource_output_language("cache"), "redis");
        assert_eq!(datasource_output_language("mariadb"), "mysql");
        assert_eq!(datasource_output_language("postgres"), "postgresql");
        assert_eq!(datasource_output_language("sqlite"), "sqlite");
    }

    #[test]
    fn collect_datasource_output_languages_deduplicates_cache_and_redis() {
        let context = SchemaContext {
            files: vec![FileDef {
                path: "schema.poly".to_string(),
                namespaces: vec![NamespaceDef {
                    name: "game".to_string(),
                    datasource: Some("cache".to_string()),
                    items: vec![table("Session", None), table("HotData", Some("redis"))],
                }],
                renames: Vec::new(),
            }],
        };

        assert_eq!(collect_datasource_output_languages(&context), vec!["redis"]);
    }

    #[test]
    fn collect_datasource_output_languages_normalizes_aliases() {
        let context = SchemaContext {
            files: vec![FileDef {
                path: "schema.poly".to_string(),
                namespaces: vec![NamespaceDef {
                    name: "game".to_string(),
                    datasource: Some("mariadb".to_string()),
                    items: vec![
                        table("Audit", Some("postgres")),
                        table("Session", Some("cache")),
                    ],
                }],
                renames: Vec::new(),
            }],
        };

        assert_eq!(
            collect_datasource_output_languages(&context),
            vec!["mysql", "postgresql", "redis"]
        );
    }
}
