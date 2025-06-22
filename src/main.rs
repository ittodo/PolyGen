use pest::Parser;
use pest_derive::Parser;
use std::env;
use std::fs;
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 명령줄 인자에서 스키마 파일 경로를 가져옵니다.
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("오류: 스키마 파일 경로가 필요합니다.");
        eprintln!("사용법: {} <스키마_파일_경로>", args[0]);
        return Err("스키마 파일이 지정되지 않았습니다.".into());
    }
    let schema_path = &args[1];

    // 2. 스키마 파일을 읽고 파싱합니다.
    println!("--- 스키마 파싱 중: {} ---", schema_path);
    let unparsed_file = fs::read_to_string(schema_path)?.replace("\r\n", "\n");
    let main_pair = Polygen::parse(Rule::main, &unparsed_file)?
        .next()
        .ok_or("스키마 파일에서 main 규칙을 찾을 수 없습니다.")?;

    // 3. 파싱된 결과로부터 AST(추상 구문 트리)를 빌드합니다.
    println!("--- AST 빌드 중 ---");
    let ast_root = ast::build_ast_from_pairs(main_pair)?;

    // 4. AST의 유효성을 검사합니다. (예: 타입 정의 중복, 존재하지 않는 타입 사용 등)
    println!("--- AST 유효성 검사 중 ---");
    validation::validate_ast(&ast_root)?;
    println!("AST 유효성 검사 성공.");

    // --- C# 코드 생성 ---
    println!("\n--- C# 코드 생성 중 ---");
    let output_dir = std::path::Path::new("output/csharp");
    fs::create_dir_all(output_dir)?;
    let output_file_path = output_dir.join("GeneratedFromTemplate.cs");

    // TODO: 이 부분도 나중에는 AST를 입력으로 받도록 수정해야 합니다.
    let csharp_code = csharp_generator::generate_csharp_with_askama();
    fs::write(&output_file_path, csharp_code)?;
    println!(
        "C# 코드가 성공적으로 생성되었습니다: {}",
        output_file_path.display()
    );

    // --- Mermaid 다이어그램 생성 ---
    println!("\n--- Mermaid 다이어그램 생성 중 ---");
    let mermaid_output_dir = std::path::Path::new("output/diagram");
    fs::create_dir_all(mermaid_output_dir)?;
    let mermaid_output_file_path = mermaid_output_dir.join("class_diagram.md");

    let mermaid_code = mermaid_generator::generate_mermaid_diagram(&ast_root);
    // GitHub에서 렌더링되도록 마크다운 코드 블록으로 감싸줍니다.
    let mermaid_content = format!("```mermaid\n{}\n```", mermaid_code);
    fs::write(&mermaid_output_file_path, mermaid_content)?;
    println!(
        "Mermaid 다이어그램이 성공적으로 생성되었습니다: {}",
        mermaid_output_file_path.display()
    );

    Ok(())
}
