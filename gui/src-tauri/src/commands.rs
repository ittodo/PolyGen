use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use pest::Parser;
use polygen::{Polygen, ast_parser, validation};
use serde::Serialize;

fn get_polygen_path() -> String {
    // In bundled app, the binary is in the resources folder
    // During development, use the built binary from parent project
    if cfg!(debug_assertions) {
        // Development: use the binary built by cargo
        // Path is relative from gui/src-tauri/ to project root
        if cfg!(target_os = "windows") {
            "../../target/release/polygen.exe".to_string()
        } else {
            "../../target/release/polygen".to_string()
        }
    } else {
        // Production: bundled binary (in resources folder)
        "polygen".to_string()
    }
}

#[tauri::command]
pub fn run_generate(
    schema_path: String,
    lang: String,
    output_dir: String,
    templates_dir: Option<String>,
) -> Result<String, String> {
    let polygen = get_polygen_path();

    let mut cmd = Command::new(&polygen);
    cmd.arg("generate")
        .arg("--schema-path")
        .arg(&schema_path)
        .arg("--lang")
        .arg(&lang)
        .arg("--output-dir")
        .arg(&output_dir);

    if let Some(templates) = templates_dir {
        cmd.arg("--templates-dir").arg(&templates);
    }

    let output = cmd.output().map_err(|e| format!("Failed to execute polygen: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(format!("Generated successfully. {}", stdout.trim()))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!("{}{}", stdout, stderr))
    }
}

#[tauri::command]
pub fn run_migrate(
    baseline_path: String,
    schema_path: String,
    output_dir: String,
) -> Result<String, String> {
    let polygen = get_polygen_path();

    let output = Command::new(&polygen)
        .arg("migrate")
        .arg("--baseline")
        .arg(&baseline_path)
        .arg("--schema-path")
        .arg(&schema_path)
        .arg("--output-dir")
        .arg(&output_dir)
        .output()
        .map_err(|e| format!("Failed to execute polygen: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(format!("Migration generated. {}", stdout.trim()))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(stderr.to_string())
    }
}

#[tauri::command]
pub fn get_polygen_version() -> Result<String, String> {
    let polygen = get_polygen_path();

    let output = Command::new(&polygen)
        .arg("--version")
        .output()
        .map_err(|e| format!("Failed to execute polygen: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().to_string())
    } else {
        Err("Failed to get version".to_string())
    }
}

#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))
}

#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, &content).map_err(|e| format!("Failed to write file: {}", e))
}

/// Parse import statements from a .poly file and return absolute paths
#[tauri::command]
pub fn parse_imports(file_path: String) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let base_dir = Path::new(&file_path)
        .parent()
        .ok_or("Invalid file path")?;

    let mut imports = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("import ") {
            // Parse: import "path/to/file.poly";
            if let Some(start) = trimmed.find('"') {
                if let Some(end) = trimmed.rfind('"') {
                    if start < end {
                        let import_path = &trimmed[start + 1..end];
                        let absolute_path = base_dir.join(import_path);
                        if absolute_path.exists() {
                            if let Some(path_str) = absolute_path.to_str() {
                                imports.push(path_str.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(imports)
}

/// Schema validation error with position info for Monaco
#[derive(Serialize)]
pub struct SchemaError {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub message: String,
    pub severity: String, // "error", "warning", "info"
}

/// Validate schema content and return errors
#[tauri::command]
pub fn validate_schema(content: String) -> Result<Vec<SchemaError>, String> {
    use polygen::Rule;

    let mut errors = Vec::new();
    let content = content.replace("\r\n", "\n");

    // Step 1: Parse with Pest
    let parse_result = Polygen::parse(Rule::main, &content);

    match parse_result {
        Ok(mut pairs) => {
            // Step 2: Build AST
            let main_pair = match pairs.next() {
                Some(p) => p,
                None => {
                    errors.push(SchemaError {
                        start_line: 1,
                        start_column: 1,
                        end_line: 1,
                        end_column: 1,
                        message: "Empty schema".to_string(),
                        severity: "warning".to_string(),
                    });
                    return Ok(errors);
                }
            };

            match ast_parser::build_ast_from_pairs(main_pair, PathBuf::from("editor.poly")) {
                Ok(ast) => {
                    // Step 3: Validate AST
                    if let Err(e) = validation::validate_ast(&ast.definitions) {
                        // Validation errors don't have line info, show at start
                        errors.push(SchemaError {
                            start_line: 1,
                            start_column: 1,
                            end_line: 1,
                            end_column: 100,
                            message: e.to_string(),
                            severity: "error".to_string(),
                        });
                    }
                }
                Err(e) => {
                    // AST build errors have line/col info
                    let (line, col, msg) = match &e {
                        polygen::error::AstBuildError::InvalidValue { line, col, .. } => (*line, *col, e.to_string()),
                        polygen::error::AstBuildError::UnexpectedRule { line, col, .. } => (*line, *col, e.to_string()),
                        polygen::error::AstBuildError::MissingElement { line, col, .. } => (*line, *col, e.to_string()),
                    };
                    errors.push(SchemaError {
                        start_line: line,
                        start_column: col,
                        end_line: line,
                        end_column: col + 10,
                        message: msg,
                        severity: "error".to_string(),
                    });
                }
            }
        }
        Err(e) => {
            // Parse errors from pest
            let (line, col) = match e.line_col {
                pest::error::LineColLocation::Pos((l, c)) => (l, c),
                pest::error::LineColLocation::Span((l, c), _) => (l, c),
            };
            errors.push(SchemaError {
                start_line: line,
                start_column: col,
                end_line: line,
                end_column: col + 10,
                message: format!("Syntax error: {}", e),
                severity: "error".to_string(),
            });
        }
    }

    Ok(errors)
}
