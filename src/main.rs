use anyhow::Result;
use clap::Parser as ClapParser;
use pest::Parser;
use pest_derive::Parser;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{AstRoot, Definition};

mod ast;
mod error; // error 모듈을 추가합니다.
mod generator;
mod ir_builder;
mod ir_model;
mod validation;

// `polygen.pest` 파일에 정의된 문법 규칙을 사용하기 위한 파서 구조체입니다.
#[derive(Parser)]
#[grammar = "polygen.pest"] // `src` 폴더에 있는 문법 파일을 지정합니다.
pub struct Polygen;

/// Polyglot Code Generator from a custom schema language
#[derive(ClapParser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the root schema file
    schema_path: PathBuf,

    /// Path to the directory containing templates
    #[arg(short, long, default_value = "templates")]
    templates_dir: PathBuf,

    /// Path to the output directory for generated code
    #[arg(short, long, default_value = "output")]
    output_dir: PathBuf,

    /// Target language for code generation (e.g., csharp, typescript)
    #[arg(short, long, default_value = "csharp")]
    lang: String,
}

// ... parse_and_merge_schemas function remains the same ...
fn parse_and_merge_schemas(initial_path: &Path, output_dir: &Path) -> Result<Vec<AstRoot>> {
    let mut files_to_process: VecDeque<PathBuf> = VecDeque::new();
    let mut processed_files: HashSet<PathBuf> = HashSet::new();
    let mut all_asts: Vec<AstRoot> = Vec::new();

    let initial_path_buf = PathBuf::from(initial_path).canonicalize()?;
    files_to_process.push_back(initial_path_buf);

    // 디버그 출력을 위해 첫 번째 파일인지 확인하는 플래그
    let mut is_first_file = true;

    while let Some(current_path) = files_to_process.pop_front() {
        if !processed_files.insert(current_path.clone()) {
            continue; // Already processed, skip to avoid cycles
        }

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

        // --- [디버깅] Pest 파싱 결과(Parse Tree)를 파일로 출력합니다. ---
        // 첫 번째 (최상위) 스키마 파일의 파싱 트리만 저장합니다.
        if is_first_file {
            let debug_dir = output_dir.join("debug");
            fs::create_dir_all(&debug_dir)?;
            let parse_tree_path = debug_dir.join("parse_tree.txt");
            // main_pair를 복제하여 디버그 출력을 생성합니다.
            fs::write(&parse_tree_path, format!("{:#?}", main_pair.clone()))?;
            println!(
                "Pest 파싱 트리 디버그 출력이 파일에 저장되었습니다: {}",
                parse_tree_path.display()
            );
            is_first_file = false;
        }

        let ast_root = ast::build_ast_from_pairs(main_pair, current_path.clone())?;
        let file_imports = ast_root.file_imports.clone(); // Clone imports before moving ast_root
        all_asts.push(ast_root);

        let base_dir = current_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("파일의 부모 디렉토리를 찾을 수 없습니다."))?;

        for import_path_str in file_imports {
            let import_path = base_dir.join(import_path_str);
            let canonical_import_path = import_path.canonicalize()?;
            if !processed_files.contains(&canonical_import_path) {
                files_to_process.push_back(canonical_import_path);
            }
        }
    }
    Ok(all_asts)
}

fn main() -> Result<()> {
    // 1. 명령줄 인자를 파싱합니다.
    let cli = Cli::parse();

    // 출력 디렉토리가 없으면 생성합니다.
    fs::create_dir_all(&cli.output_dir)?;

    // 2. 스키마 파일과 모든 import를 재귀적으로 파싱하고 하나의 AST로 합칩니다.
    println!("--- 스키마 처리 시작 ---");
    let all_asts = parse_and_merge_schemas(&cli.schema_path, &cli.output_dir)?;

    // --- [디버깅] 파싱된 AST의 전체 구조를 파일로 출력합니다. ---
    // 이 코드를 통해 doc_comment 필드에 주석이 올바르게 채워졌는지 확인할 수 있습니다.
    let ast_debug_path = cli.output_dir.join("ast_debug.txt");
    fs::write(&ast_debug_path, format!("{:#?}", all_asts))?;
    println!(
        "AST 디버그 출력이 파일에 저장되었습니다: {}",
        ast_debug_path.display()
    );

    // 3. 모든 AST에서 정의(definition)만 추출하여 하나의 리스트로 합칩니다.
    //    `all_asts`는 나중에 C# 파일 생성 시 다시 사용하기 위해 원본을 유지합니다.
    let all_definitions: Vec<Definition> = all_asts
        .iter()
        .flat_map(|ast| ast.definitions.clone())
        .collect();

    // 4. 합쳐진 AST의 유효성을 검사합니다.
    println!("--- AST 유효성 검사 중 ---");
    validation::validate_ast(&all_definitions)?;
    println!("AST 유효성 검사 성공.");

    // 5. AST를 템플릿 엔진이 사용하기 좋은 IR(Intermediate Representation)로 변환합니다.
    println!("\n--- AST를 IR로 변환 중 ---");
    let ir_context = ir_builder::build_ir(&all_definitions);
    println!("IR 변환 성공.");

    // --- 코드 생성 (설정 기반) ---
    println!("\n--- {} 코드 생성 중 ---", cli.lang.to_uppercase());
    let lang_output_dir = cli.output_dir.join(&cli.lang);

    // C#의 경우, 정적 유틸리티 파일을 출력 폴더로 복사합니다.
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

    let template_generator = generator::Generator::new(&cli.templates_dir)?;
    template_generator.generate(&ir_context, &cli.lang, &lang_output_dir)?;
    println!("{} 코드 생성이 완료되었습니다.", cli.lang.to_uppercase());

    // --- Mermaid 다이어그램 생성 ---
    println!("\n--- Mermaid 다이어그램 생성 중 ---");
    let mermaid_output_dir = Path::new("output/diagram");
    fs::create_dir_all(mermaid_output_dir)?;
    let mermaid_output_file_path = mermaid_output_dir.join("class_diagram.md");

    let mermaid_code = template_generator.generate_mermaid_diagram(&all_definitions)?;
    // GitHub에서 렌더링되도록 마크다운 코드 블록으로 감싸줍니다.
    let mermaid_content = format!("```mermaid\n{}\n```", mermaid_code);
    fs::write(&mermaid_output_file_path, mermaid_content)?;
    println!(
        "Mermaid 다이어그램이 성공적으로 생성되었습니다: {}",
        mermaid_output_file_path.display()
    );

    Ok(())
}
