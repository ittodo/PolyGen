use pest::Parser;
use pest_derive::Parser;
use std::env;
use std::fs;
mod ast; // ast 모듈을 가져옵니다.
mod error; // error 모듈을 추가합니다.

// `polygen.pest` 파일에 정의된 문법 규칙을 사용하기 위한 파서 구조체입니다.
#[derive(Parser)]
#[grammar = "polygen.pest"] // `src` 폴더에 있는 문법 파일을 지정합니다.
pub struct Polygen;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 명령줄 인자를 수집합니다.
    let args: Vec<String> = env::args().collect();

    // 파일 경로가 인자로 주어졌는지 확인합니다.
    if args.len() < 2 {
        eprintln!("오류: 스키마 파일 경로가 필요합니다.");
        eprintln!("사용법: {} <스키마_파일_경로>", args[0]);
        return Err("스키마 파일이 지정되지 않았습니다.".into());
    }
    let schema_path = &args[1];

    // 스키마 파일의 내용을 읽어옵니다.
    let unparsed_file = fs::read_to_string(schema_path).map_err(|e| {
        format!(
            "스키마 파일을 읽는 데 실패했습니다 ({}): {}",
            schema_path, e
        )
    })?;

    // `main` 규칙을 사용하여 파일 내용을 파싱합니다.
    let parse_result = Polygen::parse(Rule::main, &unparsed_file);

    match parse_result {
        Ok(pairs) => {
            println!("Successfully parsed the schema!");

            // `main` 규칙에 해당하는 단일 Pair를 가져옵니다.
            // `main = { SOI ~ (definition)* ~ EOI }` 이므로, `pairs`는 하나의 `Rule::main` Pair를 포함합니다.
            let main_pair = pairs.into_iter().next().expect("Expected a main rule pair");

            // AST를 빌드합니다.
            match ast::build_ast_from_pairs(main_pair) {
                Ok(ast_root) => {
                    // 빌드된 AST를 출력합니다. (디버깅 목적)
                    println!("\n--- Abstract Syntax Tree (AST) ---");
                    for def in ast_root {
                        println!("{:#?}", def); // Debug 출력으로 AST 구조를 확인합니다.
                    }
                    println!("----------------------------------");
                }
                Err(e) => {
                    eprintln!("\n--- AST Build Failed ---");
                    eprintln!("{}", e);
                    eprintln!("------------------------");
                }
            }
        }
        Err(e) => eprintln!("Failed to parse the schema:\n{}", e),
    }

    Ok(())
}
