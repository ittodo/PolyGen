//! Compilation pipeline module
//!
//! Handles the complete schema compilation process from parsing to code generation.

use anyhow::Result;
use pest::Parser;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast_model::{AstRoot, Definition};
use crate::codegen::{csharp_static_files, discover_languages, CodeGenerator};
use crate::ir_model::SchemaContext;
use crate::{ast_parser, ir_builder, validation, Polygen, Rule};

/// Configuration for the compilation pipeline
pub struct PipelineConfig {
    /// Path to the root schema file
    pub schema_path: PathBuf,
    /// Directory containing templates
    pub templates_dir: PathBuf,
    /// Base output directory
    pub output_dir: PathBuf,
    /// Optional specific language to generate (None = all languages)
    pub target_lang: Option<String>,
    /// Whether to write debug output files
    pub debug_output: bool,
}

impl PipelineConfig {
    pub fn new(schema_path: PathBuf, templates_dir: PathBuf, output_dir: PathBuf) -> Self {
        Self {
            schema_path,
            templates_dir,
            output_dir,
            target_lang: None,
            debug_output: true,
        }
    }

    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.target_lang = Some(lang.into());
        self
    }

    pub fn with_debug_output(mut self, enabled: bool) -> Self {
        self.debug_output = enabled;
        self
    }
}

/// The compilation pipeline handles the complete schema-to-code process
pub struct CompilationPipeline {
    config: PipelineConfig,
}

impl CompilationPipeline {
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    /// Run the complete compilation pipeline
    pub fn run(&self) -> Result<()> {
        self.prepare_output_dir()?;
        let asts = self.parse_schemas()?;
        self.validate_asts(&asts)?;
        let ir_context = self.build_ir(&asts);
        self.generate_code(&ir_context)?;
        Ok(())
    }

    /// Prepare the output directory
    fn prepare_output_dir(&self) -> Result<()> {
        if self.config.output_dir.exists() {
            println!("기존 출력 디렉토리 삭제 중: {}", self.config.output_dir.display());
            fs::remove_dir_all(&self.config.output_dir)?;
        }
        println!("출력 디렉토리 생성 중: {}", self.config.output_dir.display());
        fs::create_dir_all(&self.config.output_dir)?;
        Ok(())
    }

    /// Parse all schema files
    fn parse_schemas(&self) -> Result<Vec<AstRoot>> {
        println!("--- 스키마 처리 시작 ---");
        let asts = parse_and_merge_schemas(&self.config.schema_path, Some(&self.config.output_dir))?;

        if self.config.debug_output {
            let ast_debug_path = self.config.output_dir.join("ast_debug.txt");
            fs::write(&ast_debug_path, format!("{:#?}", asts))?;
            println!("AST 디버그 출력이 파일에 저장되었습니다: {}", ast_debug_path.display());
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
        Ok(())
    }

    /// Build the IR from ASTs
    fn build_ir(&self, asts: &[AstRoot]) -> SchemaContext {
        println!("\n--- AST를 IR로 변환 중 ---");
        let ir_context = ir_builder::build_ir(asts);
        println!("IR 변환 성공.");

        if self.config.debug_output {
            let ir_debug_path = self.config.output_dir.join("ir_debug.txt");
            if let Err(e) = fs::write(&ir_debug_path, format!("{:#?}", ir_context)) {
                eprintln!("IR 디버그 파일 쓰기 실패: {}", e);
            } else {
                println!("IR 디버그 출력이 파일에 저장되었습니다: {}", ir_debug_path.display());
            }
        }

        ir_context
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

    /// Generate code for a specific language
    fn generate_for_language(&self, lang: &str, ir_context: &SchemaContext) -> Result<()> {
        println!("\n--- {} 코드 생성 중 ---", lang.to_uppercase());

        let generator = CodeGenerator::new(
            lang,
            self.config.templates_dir.clone(),
            self.config.output_dir.clone(),
        );

        // Copy static files for language-specific assets
        if lang == "csharp" {
            generator.copy_static_files(&csharp_static_files())?;
        }

        // Generate main code
        generator.generate(ir_context)?;

        // Generate additional templates for C#
        if lang == "csharp" {
            self.generate_csharp_extras(&generator, ir_context)?;
        }

        println!("{} 코드 생성이 완료되었습니다.", lang.to_uppercase());
        Ok(())
    }

    /// Generate additional C# code (readers, writers, CSV mappers)
    fn generate_csharp_extras(&self, generator: &CodeGenerator, ir_context: &SchemaContext) -> Result<()> {
        let extras = [
            ("csharp_binary_readers_file.rhai", "C# Binary Readers"),
            ("csharp_binary_writers_file.rhai", "C# Binary Writers"),
            ("csharp_csv_columns_file.rhai", "C# CSV Columns"),
            ("csharp_csv_mappers_file.rhai", "C# CSV Mappers"),
        ];

        for (template, name) in extras {
            if generator.has_template(template) {
                println!("\n--- {} generation ---", name);
                generator.generate_with_template(ir_context, template)?;
            }
        }

        Ok(())
    }
}

/// Parse and merge all schema files starting from the initial path
pub fn parse_and_merge_schemas(initial_path: &Path, output_dir: Option<&Path>) -> Result<Vec<AstRoot>> {
    let mut files_to_process: VecDeque<PathBuf> = VecDeque::new();
    let mut processed_files: HashSet<PathBuf> = HashSet::new();
    let mut all_asts: Vec<AstRoot> = Vec::new();

    let initial_path_buf = initial_path.to_path_buf();
    files_to_process.push_back(initial_path_buf.clone());
    processed_files.insert(initial_path_buf.canonicalize()?);

    let mut is_first_file = true;

    while let Some(current_path) = files_to_process.pop_front() {
        println!("--- 스키마 파싱 중: {} ---", current_path.display());
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
                println!(
                    "Pest 파싱 트리 디버그 출력이 파일에 저장되었습니다: {}",
                    parse_tree_path.display()
                );
            }
            is_first_file = false;
        }

        let ast_root = ast_parser::build_ast_from_pairs(main_pair, current_path.clone())?;
        let file_imports = ast_root.file_imports.clone();
        all_asts.push(ast_root);

        let base_dir = current_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("파일의 부모 디렉토리를 찾을 수 없습니다."))?;

        for import_path_str in file_imports {
            let import_path = base_dir.join(import_path_str);
            let canonical_import_path = import_path.canonicalize()?;
            if !processed_files.contains(&canonical_import_path) {
                processed_files.insert(canonical_import_path);
                files_to_process.push_back(import_path);
            }
        }
    }

    Ok(all_asts)
}
