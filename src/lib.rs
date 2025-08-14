use anyhow::Result;
use clap::Parser as ClapParser;
use pest::Parser;
use pest_derive::Parser;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

// All modules are now part of the library
pub mod ast_model;
pub mod ast_parser;
pub mod error;
pub mod ir_builder;
pub mod ir_model;
pub mod rhai_generator;
pub mod validation;

// Re-exporting Definition for use in main.rs
use crate::ast_model::{AstRoot, Definition};

#[derive(Parser)]
#[grammar = "polygen.pest"]
pub struct Polygen;

/// Polyglot Code Generator from a custom schema language
#[derive(ClapParser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to the root schema file
    #[arg(short, long)]
    pub schema_path: PathBuf,

    /// Path to the directory containing templates
    #[arg(short, long, default_value = "templates")]
    pub templates_dir: PathBuf,

    /// Path to the output directory for generated code
    #[arg(short, long, default_value = "output")]
    pub output_dir: PathBuf,

    /// Target language for code generation (e.g., csharp, typescript)
    #[arg(short, long, default_value = "csharp")]
    pub lang: String,
}

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
            if let Some(output_dir) = output_dir { // Added this line
                let debug_dir = output_dir.join("debug");
                fs::create_dir_all(&debug_dir)?;
                let parse_tree_path = debug_dir.join("parse_tree.txt");
                fs::write(&parse_tree_path, format!("{:#?}", main_pair.clone()))?;
                println!(
                    "Pest 파싱 트리 디버그 출력이 파일에 저장되었습니다: {}",
                    parse_tree_path.display()
                );
            } // Added this line
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

pub fn run(cli: Cli) -> Result<()> {
    if cli.output_dir.exists() {
        println!("기존 출력 디렉토리 삭제 중: {}", cli.output_dir.display());
        fs::remove_dir_all(&cli.output_dir)?;
    }
    println!("출력 디렉토리 생성 중: {}", cli.output_dir.display());
    fs::create_dir_all(&cli.output_dir)?;

    println!("--- 스키마 처리 시작 ---");
    let all_asts = parse_and_merge_schemas(&cli.schema_path, Some(&cli.output_dir))?;

    let ast_debug_path = cli.output_dir.join("ast_debug.txt");
    fs::write(&ast_debug_path, format!("{:#?}", all_asts))?;
    println!(
        "AST 디버그 출력이 파일에 저장되었습니다: {}",
        ast_debug_path.display()
    );

    println!("--- AST 유효성 검사 중 ---");
    let all_definitions: Vec<Definition> = all_asts.iter().flat_map(|ast| ast.definitions.clone()).collect();
    validation::validate_ast(&all_definitions)?;
    println!("AST 유효성 검사 성공.");

    println!("\n--- AST를 IR로 변환 중 ---");
    let ir_context = build_ir_from_asts(&all_asts);
    println!("IR 변환 성공.");

    let ir_debug_path = cli.output_dir.join("ir_debug.txt");
    fs::write(&ir_debug_path, format!("{:#?}", ir_context))?;
    println!(
        "IR 디버그 출력이 파일에 저장되었습니다: {}",
        ir_debug_path.display()
    );

    println!("\n--- {} 코드 생성 중 ---", cli.lang.to_uppercase());
    let lang_output_dir = cli.output_dir.join(&cli.lang);

    if cli.lang == "csharp" {
        let static_source_path = Path::new("static/csharp/DataSource.cs");
        let dest_dir = lang_output_dir.join("Common");
        fs::create_dir_all(&dest_dir)?;
        let dest_path = dest_dir.join("DataSource.cs");
        if static_source_path.exists() {
            fs::copy(static_source_path, &dest_path)?;
            println!("Copied static file to {}", dest_path.display());
        }
    }

    println!("Using Rhai template engine.");
    let template_path = cli
        .templates_dir
        .join(&cli.lang)
        .join(format!("{}_file.rhai", cli.lang));
    rhai_generator::generate_code_with_rhai(&ir_context, &template_path)
        .map_err(|e| anyhow::anyhow!(e))?;

    println!("{} 코드 생성이 완료되었습니다.", cli.lang.to_uppercase());

    Ok(())
}

pub fn build_ir_from_asts(asts: &[AstRoot]) -> ir_model::SchemaContext {
    ir_builder::build_ir(asts)
}
