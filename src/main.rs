use pest_derive::Parser;
use std::fs;
mod ast;
mod csharp_generator;
mod csharp_model;
mod error; // error 모듈을 추가합니다.
mod validation; // validation 모듈을 추가합니다.

// `polygen.pest` 파일에 정의된 문법 규칙을 사용하기 위한 파서 구조체입니다.
#[derive(Parser)]
#[grammar = "polygen.pest"] // `src` 폴더에 있는 문법 파일을 지정합니다.
pub struct Polygen;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- C# 코드 생성 ---
    println!("\n--- C# 코드 생성 중 ---");
    let output_dir = std::path::Path::new("output/csharp");
    fs::create_dir_all(output_dir)?;
    let output_file_path = output_dir.join("GeneratedFromTemplate.cs");

    // Askama 템플릿 엔진을 사용하여 코드 생성
    let csharp_code = csharp_generator::generate_csharp_with_askama();
    fs::write(&output_file_path, csharp_code)?;
    println!(
        "C# 코드가 성공적으로 생성되었습니다: {}",
        output_file_path.display()
    );

    // TODO: 다음 단계로, `.poly` 파일 파싱 -> AST 변환 -> C# 모델 변환 -> 코드 생성 파이프라인을 연결해야 합니다.

    Ok(())
}
