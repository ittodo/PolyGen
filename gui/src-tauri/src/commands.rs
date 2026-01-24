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

/// Result of go to definition lookup
#[derive(Serialize)]
pub struct DefinitionLocation {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub name: String,
    pub kind: String, // "table", "enum", "embed", "namespace"
    pub file_path: Option<String>, // For cross-file navigation
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
/// If file_path is provided, imports will be resolved relative to that path
#[tauri::command]
pub fn validate_schema(content: String, file_path: Option<String>) -> Result<Vec<SchemaError>, String> {
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
                    // Step 3: Validate AST (skip TypeNotFound when we have file_path for import support)
                    if let Err(e) = validation::validate_ast(&ast.definitions) {
                        // Skip TypeNotFound errors when file_path is provided,
                        // as the symbol table validation handles imports properly
                        let is_type_not_found = matches!(&e, polygen::error::ValidationError::TypeNotFound(_));
                        if !is_type_not_found || file_path.is_none() {
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

                    // Step 4: Check for unresolved type references using symbol table
                    // Use build_symbol_table_with_imports if file_path is provided
                    let symbol_table_result = if let Some(ref path) = file_path {
                        polygen::symbol_table::build_symbol_table_with_imports(
                            &content,
                            Some(Path::new(path)),
                        )
                    } else {
                        polygen::symbol_table::build_symbol_table(&content)
                    };

                    if let Ok(symbol_table) = symbol_table_result {
                        for reference in &symbol_table.references {
                            if reference.resolved_fqn.is_none() {
                                errors.push(SchemaError {
                                    start_line: reference.span.start_line,
                                    start_column: reference.span.start_col,
                                    end_line: reference.span.end_line,
                                    end_column: reference.span.end_col,
                                    message: format!("Unresolved reference: '{}'", reference.path),
                                    severity: "error".to_string(),
                                });
                            }
                        }
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

/// Go to definition: find the definition location for a symbol at the given position
/// If file_path is provided, imports will be resolved relative to that path
#[tauri::command]
pub fn goto_definition(content: String, line: usize, column: usize, file_path: Option<String>) -> Option<DefinitionLocation> {
    use polygen::symbol_table::{build_symbol_table, build_symbol_table_with_imports, DefinitionKind};

    let symbol_table = if let Some(ref path) = file_path {
        build_symbol_table_with_imports(&content, Some(Path::new(path))).ok()?
    } else {
        build_symbol_table(&content).ok()?
    };

    // First, check if we're on a type reference
    if let Some(reference) = symbol_table.reference_at(line, column) {
        if let Some(fqn) = &reference.resolved_fqn {
            if let Some(definition) = symbol_table.get_definition(fqn) {
                return Some(DefinitionLocation {
                    start_line: definition.name_span.start_line,
                    start_column: definition.name_span.start_col,
                    end_line: definition.name_span.end_line,
                    end_column: definition.name_span.end_col,
                    name: definition.name.clone(),
                    kind: match definition.kind {
                        DefinitionKind::Table => "table".to_string(),
                        DefinitionKind::Enum => "enum".to_string(),
                        DefinitionKind::Embed => "embed".to_string(),
                        DefinitionKind::Namespace => "namespace".to_string(),
                        DefinitionKind::Field => "field".to_string(),
                    },
                    file_path: definition.file_path.clone(),
                });
            }
        }
    }

    // Also check if we're on a definition itself
    if let Some(definition) = symbol_table.definition_at(line, column) {
        return Some(DefinitionLocation {
            start_line: definition.name_span.start_line,
            start_column: definition.name_span.start_col,
            end_line: definition.name_span.end_line,
            end_column: definition.name_span.end_col,
            name: definition.name.clone(),
            kind: match definition.kind {
                DefinitionKind::Table => "table".to_string(),
                DefinitionKind::Enum => "enum".to_string(),
                DefinitionKind::Embed => "embed".to_string(),
                DefinitionKind::Namespace => "namespace".to_string(),
                DefinitionKind::Field => "field".to_string(),
            },
            file_path: definition.file_path.clone(),
        });
    }

    None
}

/// Reference location for Find All References
#[derive(Serialize)]
pub struct ReferenceLocation {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub is_definition: bool,
    pub file_path: Option<String>,
}

/// Find all references: find all locations that reference the symbol at the given position
/// If file_path is provided, also searches in files that import the current file
#[tauri::command]
pub fn find_references(
    content: String,
    line: usize,
    column: usize,
    include_definition: bool,
    file_path: Option<String>,
) -> Vec<ReferenceLocation> {
    use polygen::symbol_table::{build_symbol_table, build_symbol_table_with_imports};

    let current_file_path = file_path.as_ref().map(|p| Path::new(p));

    // Build symbol table for current file (with imports for type resolution)
    let symbol_table = if let Some(path) = current_file_path {
        match build_symbol_table_with_imports(&content, Some(path)) {
            Ok(table) => table,
            Err(_) => return Vec::new(),
        }
    } else {
        match build_symbol_table(&content) {
            Ok(table) => table,
            Err(_) => return Vec::new(),
        }
    };

    // Find what we're looking for (definition or reference at position)
    let target_fqn = if let Some(def) = symbol_table.definition_at(line, column) {
        def.fqn.clone()
    } else if let Some(reference) = symbol_table.reference_at(line, column) {
        match &reference.resolved_fqn {
            Some(fqn) => fqn.clone(),
            None => return Vec::new(),
        }
    } else {
        return Vec::new();
    };

    let mut locations = Vec::new();

    // Include the definition itself if requested
    if include_definition {
        if let Some(def) = symbol_table.get_definition(&target_fqn) {
            locations.push(ReferenceLocation {
                start_line: def.name_span.start_line,
                start_column: def.name_span.start_col,
                end_line: def.name_span.end_line,
                end_column: def.name_span.end_col,
                is_definition: true,
                file_path: def.file_path.clone(),
            });
        }
    }

    // Include references from current file
    for reference in symbol_table.find_references(&target_fqn) {
        locations.push(ReferenceLocation {
            start_line: reference.span.start_line,
            start_column: reference.span.start_col,
            end_line: reference.span.end_line,
            end_column: reference.span.end_col,
            is_definition: false,
            file_path: file_path.clone(),
        });
    }

    // Search in files that might import the current file
    if let Some(current_path) = current_file_path {
        if let Some(dir) = current_path.parent() {
            // Find all .poly files in the same directory
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.extension().and_then(|e| e.to_str()) == Some("poly") {
                        // Skip the current file
                        if entry_path == current_path {
                            continue;
                        }

                        // Read and check if this file imports our file
                        if let Ok(other_content) = fs::read_to_string(&entry_path) {
                            let current_filename = current_path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("");

                            // Check if this file imports the current file
                            let imports_current = other_content.lines().any(|line| {
                                let trimmed = line.trim();
                                trimmed.starts_with("import ") && trimmed.contains(current_filename)
                            });

                            if imports_current {
                                // Build symbol table for this file (with imports)
                                if let Ok(other_table) = build_symbol_table_with_imports(
                                    &other_content,
                                    Some(&entry_path),
                                ) {
                                    // Find references to our target FQN
                                    for reference in other_table.find_references(&target_fqn) {
                                        locations.push(ReferenceLocation {
                                            start_line: reference.span.start_line,
                                            start_column: reference.span.start_col,
                                            end_line: reference.span.end_line,
                                            end_column: reference.span.end_col,
                                            is_definition: false,
                                            file_path: entry_path.to_str().map(|s| s.to_string()),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    locations
}
