use pest::Parser;
use pest_derive::Parser;
use std::collections::{HashSet, VecDeque};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{AstRoot, Definition};

mod ast;
mod csharp_generator;
mod csharp_model;
mod error; // error 모듈을 추가합니다.
mod mermaid_generator;
mod mermaid_model;
mod validation;

// `polygen.pest` 파일에 정의된 문법 규칙을 사용하기 위한 파서 구조체입니다.
#[derive(Parser)]
#[grammar = "polygen.pest"] // `src` 폴더에 있는 문법 파일을 지정합니다.
pub struct Polygen;

/// Recursively parses the initial schema file and all its imports.
/// It returns a vector of `AstRoot`, one for each parsed file.
fn parse_and_merge_schemas(initial_path: &str) -> Result<Vec<AstRoot>, Box<dyn std::error::Error>> {
    let mut files_to_process: VecDeque<PathBuf> = VecDeque::new();
    let mut processed_files: HashSet<PathBuf> = HashSet::new();
    let mut all_asts: Vec<AstRoot> = Vec::new();

    let initial_path_buf = PathBuf::from(initial_path).canonicalize()?;
    files_to_process.push_back(initial_path_buf);

    while let Some(current_path) = files_to_process.pop_front() {
        if !processed_files.insert(current_path.clone()) {
            continue; // Already processed, skip to avoid cycles
        }

        println!("--- 스키마 파싱 중: {} ---", current_path.display());
        let unparsed_file = fs::read_to_string(&current_path)?.replace("\r\n", "\n");
        let main_pair = Polygen::parse(Rule::main, &unparsed_file)?
            .next()
            .ok_or_else(|| {
                format!(
                    "스키마 파일에서 main 규칙을 찾을 수 없습니다: {}",
                    current_path.display()
                )
            })?;

        let ast_root = ast::build_ast_from_pairs(main_pair, current_path.clone())?;
        let file_imports = ast_root.file_imports.clone(); // Clone imports before moving ast_root
        all_asts.push(ast_root);

        let base_dir = current_path
            .parent()
            .ok_or("파일의 부모 디렉토리를 찾을 수 없습니다.")?;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 명령줄 인자에서 스키마 파일 경로를 가져옵니다.
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("오류: 스키마 파일 경로가 필요합니다.");
        eprintln!("사용법: {} <스키마_파일_경로>", args[0]);
        return Err("스키마 파일이 지정되지 않았습니다.".into());
    }
    let schema_path = &args[1];

    // 2. 스키마 파일과 모든 import를 재귀적으로 파싱하고 하나의 AST로 합칩니다.
    println!("--- 스키마 처리 시작 ---");
    let all_asts = parse_and_merge_schemas(schema_path)?;

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

    // --- C# 코드 생성 ---
    println!("\n--- C# 코드 생성 중 ---");
    let csharp_output_dir = Path::new("output/csharp");
    csharp_generator::generate_csharp_code(&all_asts, csharp_output_dir)?;
    println!("C# 코드 생성이 완료되었습니다.");

    // --- Mermaid 다이어그램 생성 ---
    println!("\n--- Mermaid 다이어그램 생성 중 ---");
    let mermaid_output_dir = Path::new("output/diagram");
    fs::create_dir_all(mermaid_output_dir)?;
    let mermaid_output_file_path = mermaid_output_dir.join("class_diagram.md");

    let mermaid_code = mermaid_generator::generate_mermaid_diagram(&all_definitions);
    // GitHub에서 렌더링되도록 마크다운 코드 블록으로 감싸줍니다.
    let mermaid_content = format!("```mermaid\n{}\n```", mermaid_code);
    fs::write(&mermaid_output_file_path, mermaid_content)?;
    println!(
        "Mermaid 다이어그램이 성공적으로 생성되었습니다: {}",
        mermaid_output_file_path.display()
    );

    Ok(())
}
