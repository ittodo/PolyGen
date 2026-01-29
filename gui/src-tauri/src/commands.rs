use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use pest::Parser;
use polygen::{Polygen, ast_parser, validation, ir_builder, visualize};
use polygen::pipeline::parse_and_merge_schemas;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

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

/// Hover info result
#[derive(Serialize)]
pub struct HoverInfo {
    pub content: String,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

/// Get hover information for the symbol at the given position
#[tauri::command]
pub fn get_hover_info(
    content: String,
    line: usize,
    column: usize,
    file_path: Option<String>,
) -> Option<HoverInfo> {
    use polygen::symbol_table::{build_symbol_table, build_symbol_table_with_imports, DefinitionKind};

    let symbol_table = if let Some(ref path) = file_path {
        build_symbol_table_with_imports(&content, Some(Path::new(path))).ok()?
    } else {
        build_symbol_table(&content).ok()?
    };

    // Check if we're on a reference or definition
    if let Some(def) = symbol_table.symbol_at(line, column) {
        let kind_str = match def.kind {
            DefinitionKind::Namespace => "namespace",
            DefinitionKind::Table => "table",
            DefinitionKind::Enum => "enum",
            DefinitionKind::Embed => "embed",
            DefinitionKind::Field => "field",
        };

        // Get fields for table/embed
        let fields_info = if matches!(def.kind, DefinitionKind::Table | DefinitionKind::Embed) {
            let fields = symbol_table.get_fields_of(&def.fqn);
            if !fields.is_empty() {
                let field_list: Vec<String> = fields
                    .iter()
                    .map(|f| format!("  {}", f.name))
                    .collect();
                format!("\n\n**Fields:**\n{}", field_list.join("\n"))
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let file_info = def.file_path.as_ref()
            .map(|p| format!("\n\n*Defined in: {}*", p))
            .unwrap_or_default();

        let hover_content = format!(
            "**{} {}**\n\nFully qualified name: `{}`{}{}",
            kind_str, def.name, def.fqn, fields_info, file_info
        );

        // Find the span for the word at position
        if let Some(reference) = symbol_table.reference_at(line, column) {
            return Some(HoverInfo {
                content: hover_content,
                start_line: reference.span.start_line,
                start_column: reference.span.start_col,
                end_line: reference.span.end_line,
                end_column: reference.span.end_col,
            });
        } else if let Some(def) = symbol_table.definition_at(line, column) {
            return Some(HoverInfo {
                content: hover_content,
                start_line: def.name_span.start_line,
                start_column: def.name_span.start_col,
                end_line: def.name_span.end_line,
                end_column: def.name_span.end_col,
            });
        }
    }

    // Fallback: check for built-in types/keywords
    let lines: Vec<&str> = content.lines().collect();
    if let Some(line_str) = lines.get(line - 1) {
        let chars: Vec<char> = line_str.chars().collect();
        let col = column - 1;

        // Find word boundaries
        let mut start = col;
        let mut end = col;

        while start > 0 && chars.get(start - 1).map(|c| c.is_alphanumeric() || *c == '_').unwrap_or(false) {
            start -= 1;
        }
        while end < chars.len() && chars.get(end).map(|c| c.is_alphanumeric() || *c == '_').unwrap_or(false) {
            end += 1;
        }

        let word: String = chars[start..end].iter().collect();

        let hover_text = match word.as_str() {
            "namespace" => Some("**namespace**\n\nDefines a namespace to organize types."),
            "table" => Some("**table**\n\nDefines a data structure (class/struct)."),
            "enum" => Some("**enum**\n\nDefines an enumeration type."),
            "embed" => Some("**embed**\n\nDefines a reusable field group."),
            "import" => Some("**import**\n\nImports types from another schema file."),
            "string" => Some("**string**\n\nVariable-length text type."),
            "bool" => Some("**bool**\n\nBoolean type (true/false)."),
            "bytes" => Some("**bytes**\n\nBinary data type."),
            "u8" => Some("**u8**\n\nUnsigned 8-bit integer (0 to 255)."),
            "u16" => Some("**u16**\n\nUnsigned 16-bit integer."),
            "u32" => Some("**u32**\n\nUnsigned 32-bit integer."),
            "u64" => Some("**u64**\n\nUnsigned 64-bit integer."),
            "i8" => Some("**i8**\n\nSigned 8-bit integer."),
            "i16" => Some("**i16**\n\nSigned 16-bit integer."),
            "i32" => Some("**i32**\n\nSigned 32-bit integer."),
            "i64" => Some("**i64**\n\nSigned 64-bit integer."),
            "f32" => Some("**f32**\n\n32-bit floating point."),
            "f64" => Some("**f64**\n\n64-bit floating point."),
            "primary_key" => Some("**primary_key**\n\nMarks this field as the primary key."),
            "unique" => Some("**unique**\n\nEnsures values in this field are unique."),
            "max_length" => Some("**max_length(n)**\n\nSets maximum string length."),
            "default" => Some("**default(value)**\n\nSets a default value."),
            "range" => Some("**range(min, max)**\n\nSets allowed value range."),
            "regex" => Some("**regex(\"pattern\")**\n\nSets a regex validation pattern."),
            "foreign_key" => Some("**foreign_key(Table.field)**\n\nReferences a field in another table."),
            _ => None,
        };

        if let Some(text) = hover_text {
            return Some(HoverInfo {
                content: text.to_string(),
                start_line: line,
                start_column: start + 1,
                end_line: line,
                end_column: end + 1,
            });
        }
    }

    None
}

/// Completion item for Monaco
#[derive(Serialize)]
pub struct CompletionItemResult {
    pub label: String,
    pub kind: String,
    pub detail: String,
    pub insert_text: String,
    pub is_snippet: bool,
}

/// Get completion items for the given position
#[tauri::command]
pub fn get_completions(
    content: String,
    line: usize,
    column: usize,
    file_path: Option<String>,
) -> Vec<CompletionItemResult> {
    use polygen::symbol_table::{build_symbol_table, build_symbol_table_with_imports, DefinitionKind};

    let mut items = Vec::new();

    // Determine context by analyzing current line
    let lines: Vec<&str> = content.lines().collect();
    let current_line = lines.get(line - 1).unwrap_or(&"");
    let col = column.saturating_sub(1);
    let before_cursor = if col <= current_line.len() {
        &current_line[..col]
    } else {
        current_line
    };

    // Check if we're inside a field definition (after ':')
    let in_field_type = before_cursor.contains(':') && !before_cursor.contains(';');
    // Check if we're at the start of a line or after '{' (definition context)
    let in_definition_context = before_cursor.trim().is_empty()
        || before_cursor.trim_end().ends_with('{')
        || before_cursor.trim_end().ends_with('}');
    // Check if triggered by '@'
    let in_annotation = before_cursor.trim_start().starts_with('@');

    // Annotations (only at start of line)
    if in_annotation || before_cursor.trim().is_empty() {
        let annotations = [
            ("@load(csv: \"$1\", json: \"$2\")", "@load", "Specify data loading sources"),
            ("@cache($1)", "@cache", "Enable caching"),
            ("@readonly", "@readonly", "Mark table as read-only"),
            ("@taggable", "@taggable", "Enable row tagging"),
            ("@datasource($1)", "@datasource", "Specify database source"),
            ("@soft_delete($1)", "@soft_delete", "Enable soft delete"),
            ("@link_rows($1)", "@link_rows", "Link rows to another table"),
        ];

        for (insert, label, detail) in annotations {
            items.push(CompletionItemResult {
                label: label.to_string(),
                kind: "snippet".to_string(),
                detail: detail.to_string(),
                insert_text: insert.to_string(),
                is_snippet: true,
            });
        }
    }

    // Keywords (in definition context)
    if in_definition_context {
        let keywords = [
            ("namespace", "namespace $1 {\n\t$0\n}", "Define a namespace"),
            ("table", "table $1 {\n\t$0\n}", "Define a table/struct"),
            ("enum", "enum $1 {\n\t$0\n}", "Define an enumeration"),
            ("embed", "embed $1 {\n\t$0\n}", "Define an embedded type"),
            ("import", "import \"$1\";", "Import another schema file"),
        ];

        for (label, insert, detail) in keywords {
            items.push(CompletionItemResult {
                label: label.to_string(),
                kind: "keyword".to_string(),
                detail: detail.to_string(),
                insert_text: insert.to_string(),
                is_snippet: true,
            });
        }
    }

    // Types (when defining field type)
    if in_field_type || !in_definition_context {
        // Built-in types
        let types = [
            ("string", "Variable-length text"),
            ("bool", "Boolean (true/false)"),
            ("bytes", "Binary data"),
            ("u8", "Unsigned 8-bit integer"),
            ("u16", "Unsigned 16-bit integer"),
            ("u32", "Unsigned 32-bit integer"),
            ("u64", "Unsigned 64-bit integer"),
            ("i8", "Signed 8-bit integer"),
            ("i16", "Signed 16-bit integer"),
            ("i32", "Signed 32-bit integer"),
            ("i64", "Signed 64-bit integer"),
            ("f32", "32-bit floating point"),
            ("f64", "64-bit floating point"),
        ];

        for (type_name, detail) in types {
            items.push(CompletionItemResult {
                label: type_name.to_string(),
                kind: "type".to_string(),
                detail: detail.to_string(),
                insert_text: type_name.to_string(),
                is_snippet: false,
            });
        }

        // User-defined types from symbol table
        let symbol_table = if let Some(ref path) = file_path {
            build_symbol_table_with_imports(&content, Some(Path::new(path))).ok()
        } else {
            build_symbol_table(&content).ok()
        };

        if let Some(table) = symbol_table {
            for (name, fqn, kind) in table.get_all_type_names() {
                let (kind_str, monaco_kind) = match kind {
                    DefinitionKind::Table => ("table", "class"),
                    DefinitionKind::Enum => ("enum", "enum"),
                    DefinitionKind::Embed => ("embed", "struct"),
                    _ => continue,
                };

                // Add simple name
                items.push(CompletionItemResult {
                    label: name.to_string(),
                    kind: monaco_kind.to_string(),
                    detail: format!("{} {}", kind_str, fqn),
                    insert_text: name.to_string(),
                    is_snippet: false,
                });

                // Add fully qualified name if different
                if name != fqn {
                    items.push(CompletionItemResult {
                        label: fqn.to_string(),
                        kind: monaco_kind.to_string(),
                        detail: format!("{} (fully qualified)", kind_str),
                        insert_text: fqn.to_string(),
                        is_snippet: false,
                    });
                }
            }
        }

        // Constraints (after type in field definition)
        let constraints = [
            ("primary_key", "primary_key", "Mark as primary key"),
            ("unique", "unique", "Ensure unique values"),
            ("max_length", "max_length($1)", "Set maximum string length"),
            ("default", "default($1)", "Set default value"),
            ("range", "range($1, $2)", "Set value range"),
            ("regex", "regex(\"$1\")", "Set regex validation pattern"),
            ("foreign_key", "foreign_key($1.$2)", "Reference another table.field"),
        ];

        for (label, insert, detail) in constraints {
            items.push(CompletionItemResult {
                label: label.to_string(),
                kind: "property".to_string(),
                detail: detail.to_string(),
                insert_text: insert.to_string(),
                is_snippet: insert.contains('$'),
            });
        }
    }

    items
}

/// Document symbol for Monaco outline
#[derive(Serialize)]
pub struct DocumentSymbolResult {
    pub name: String,
    pub kind: String,
    pub fqn: String,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub children: Vec<DocumentSymbolResult>,
}

/// Get document symbols for outline view
#[tauri::command]
pub fn get_document_symbols(
    content: String,
    file_path: Option<String>,
) -> Vec<DocumentSymbolResult> {
    use polygen::symbol_table::{build_symbol_table, build_symbol_table_with_imports, DefinitionKind, DefinitionInfo};

    let symbol_table = if let Some(ref path) = file_path {
        match build_symbol_table_with_imports(&content, Some(Path::new(path))) {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        }
    } else {
        match build_symbol_table(&content) {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        }
    };

    fn create_symbol(def: &DefinitionInfo, symbol_table: &polygen::symbol_table::SymbolTable) -> DocumentSymbolResult {
        let kind = match def.kind {
            DefinitionKind::Namespace => "namespace",
            DefinitionKind::Table => "table",
            DefinitionKind::Enum => "enum",
            DefinitionKind::Embed => "embed",
            DefinitionKind::Field => "field",
        };

        // Get children (items with this fqn as prefix)
        let prefix = format!("{}.", def.fqn);
        let children: Vec<DocumentSymbolResult> = symbol_table
            .get_all_definitions()
            .into_iter()
            .filter(|d| {
                d.fqn.starts_with(&prefix)
                    && d.fqn[prefix.len()..].chars().filter(|c| *c == '.').count() == 0
                    && d.file_path == def.file_path // Only include children from same file
            })
            .map(|d| create_symbol(d, symbol_table))
            .collect();

        DocumentSymbolResult {
            name: def.name.clone(),
            kind: kind.to_string(),
            fqn: def.fqn.clone(),
            start_line: def.name_span.start_line,
            start_column: def.name_span.start_col,
            end_line: def.name_span.end_line,
            end_column: def.name_span.end_col,
            children,
        }
    }

    // Get top-level symbols (those without dots in FQN, or namespaces)
    let current_file = file_path.as_deref();
    symbol_table
        .get_all_definitions()
        .into_iter()
        .filter(|d| {
            // Only include definitions from current file
            let is_current_file = match (&d.file_path, current_file) {
                (Some(def_path), Some(cur_path)) => def_path == cur_path,
                (None, _) => true,
                _ => false,
            };

            // Top-level: no parent or is a namespace
            let is_top_level = !d.fqn.contains('.')
                || (matches!(d.kind, DefinitionKind::Namespace) && d.fqn.matches('.').count() == d.name.matches('.').count());

            is_current_file && is_top_level && !matches!(d.kind, DefinitionKind::Field)
        })
        .map(|d| create_symbol(d, &symbol_table))
        .collect()
}

/// Prepare rename result
#[derive(Serialize)]
pub struct PrepareRenameResult {
    pub text: String,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

/// Prepare rename: check if the symbol at position can be renamed
#[tauri::command]
pub fn prepare_rename(
    content: String,
    line: usize,
    column: usize,
    file_path: Option<String>,
) -> Option<PrepareRenameResult> {
    use polygen::symbol_table::{build_symbol_table, build_symbol_table_with_imports, DefinitionKind};

    let symbol_table = if let Some(ref path) = file_path {
        build_symbol_table_with_imports(&content, Some(Path::new(path))).ok()?
    } else {
        build_symbol_table(&content).ok()?
    };

    // Check if we're on a renameable symbol
    if let Some(def) = symbol_table.definition_at(line, column) {
        // Don't allow renaming fields (too complex for now)
        if matches!(def.kind, DefinitionKind::Field) {
            return None;
        }

        return Some(PrepareRenameResult {
            text: def.name.clone(),
            start_line: def.name_span.start_line,
            start_column: def.name_span.start_col,
            end_line: def.name_span.end_line,
            end_column: def.name_span.end_col,
        });
    }

    // Check if we're on a reference
    if let Some(reference) = symbol_table.reference_at(line, column) {
        if reference.resolved_fqn.is_some() {
            // Get the simple name from the path
            let simple_name = reference.path.rsplit('.').next().unwrap_or(&reference.path);
            return Some(PrepareRenameResult {
                text: simple_name.to_string(),
                start_line: reference.span.start_line,
                start_column: reference.span.start_col,
                end_line: reference.span.end_line,
                end_column: reference.span.end_col,
            });
        }
    }

    None
}

/// Rename edit
#[derive(Serialize)]
pub struct RenameEditResult {
    pub file_path: String,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub new_text: String,
}

/// Rename symbol at position
#[tauri::command]
pub fn rename_symbol(
    content: String,
    line: usize,
    column: usize,
    new_name: String,
    file_path: Option<String>,
) -> Vec<RenameEditResult> {
    use polygen::symbol_table::{build_symbol_table, build_symbol_table_with_imports};

    let current_file_path = file_path.as_ref().map(|p| Path::new(p.as_str()));

    let symbol_table = if let Some(path) = current_file_path {
        match build_symbol_table_with_imports(&content, Some(path)) {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        }
    } else {
        match build_symbol_table(&content) {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        }
    };

    // Find the target FQN
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

    let mut edits = Vec::new();
    let default_path = file_path.clone().unwrap_or_default();

    // Rename the definition
    if let Some(def) = symbol_table.get_definition(&target_fqn) {
        let edit_path = def.file_path.clone().unwrap_or_else(|| default_path.clone());
        edits.push(RenameEditResult {
            file_path: edit_path,
            start_line: def.name_span.start_line,
            start_column: def.name_span.start_col,
            end_line: def.name_span.end_line,
            end_column: def.name_span.end_col,
            new_text: new_name.clone(),
        });
    }

    // Rename references in current file
    let simple_name = target_fqn.rsplit('.').next().unwrap_or(&target_fqn);
    for reference in symbol_table.find_references(&target_fqn) {
        let ref_text = &reference.path;

        if ref_text == simple_name || ref_text.ends_with(&format!(".{}", simple_name)) {
            let new_text = if ref_text == simple_name {
                new_name.clone()
            } else {
                let prefix = &ref_text[..ref_text.len() - simple_name.len()];
                format!("{}{}", prefix, new_name)
            };

            edits.push(RenameEditResult {
                file_path: default_path.clone(),
                start_line: reference.span.start_line,
                start_column: reference.span.start_col,
                end_line: reference.span.end_line,
                end_column: reference.span.end_col,
                new_text,
            });
        }
    }

    // Search in files that import the current file
    if let Some(current_path) = current_file_path {
        if let Some(dir) = current_path.parent() {
            let current_filename = current_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.extension().and_then(|e| e.to_str()) == Some("poly") {
                        if entry_path == current_path {
                            continue;
                        }

                        if let Ok(other_content) = fs::read_to_string(&entry_path) {
                            let imports_current = other_content.lines().any(|line| {
                                let trimmed = line.trim();
                                trimmed.starts_with("import ") && trimmed.contains(current_filename)
                            });

                            if imports_current {
                                if let Ok(other_table) = build_symbol_table_with_imports(
                                    &other_content,
                                    Some(&entry_path),
                                ) {
                                    for reference in other_table.find_references(&target_fqn) {
                                        let ref_text = &reference.path;

                                        if ref_text == simple_name || ref_text.ends_with(&format!(".{}", simple_name)) {
                                            let new_text = if ref_text == simple_name {
                                                new_name.clone()
                                            } else {
                                                let prefix = &ref_text[..ref_text.len() - simple_name.len()];
                                                format!("{}{}", prefix, new_name)
                                            };

                                            if let Some(path_str) = entry_path.to_str() {
                                                edits.push(RenameEditResult {
                                                    file_path: path_str.to_string(),
                                                    start_line: reference.span.start_line,
                                                    start_column: reference.span.start_col,
                                                    end_line: reference.span.end_line,
                                                    end_column: reference.span.end_col,
                                                    new_text,
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
        }
    }

    edits
}

/// Get schema visualization data for GUI
#[tauri::command]
pub fn get_schema_visualization(
    schema_path: String,
) -> Result<visualize::SchemaVisualization, String> {
    // Parse schema
    let asts = parse_and_merge_schemas(Path::new(&schema_path), None)
        .map_err(|e| format!("Failed to parse schema: {}", e))?;

    // Validate
    let defs: Vec<_> = asts
        .iter()
        .flat_map(|ast| ast.definitions.clone())
        .collect();
    validation::validate_ast(&defs)
        .map_err(|e| format!("Validation error: {}", e))?;

    // Build IR
    let ir = ir_builder::build_ir(&asts);

    // Build visualization
    Ok(visualize::build_visualization(&ir))
}

// ============================================================================
// Recent Projects Management
// ============================================================================

const MAX_RECENT_PROJECTS: usize = 10;
const RECENT_PROJECTS_FILE: &str = "recent_projects.json";

/// Recent project entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub path: String,
    pub name: String,
    pub timestamp: u64,
    pub output_dir: Option<String>,
    pub languages: Option<Vec<String>>,
}

/// Get the path to the recent projects file
fn get_recent_projects_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    // Create directory if it doesn't exist
    if !app_data_dir.exists() {
        fs::create_dir_all(&app_data_dir)
            .map_err(|e| format!("Failed to create app data directory: {}", e))?;
    }

    Ok(app_data_dir.join(RECENT_PROJECTS_FILE))
}

/// Load recent projects from file
fn load_recent_projects(app: &AppHandle) -> Result<Vec<RecentProject>, String> {
    let path = get_recent_projects_path(app)?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read recent projects file: {}", e))?;

    let projects: Vec<RecentProject> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse recent projects: {}", e))?;

    Ok(projects)
}

/// Save recent projects to file
fn save_recent_projects(app: &AppHandle, projects: &[RecentProject]) -> Result<(), String> {
    let path = get_recent_projects_path(app)?;

    let content = serde_json::to_string_pretty(projects)
        .map_err(|e| format!("Failed to serialize recent projects: {}", e))?;

    fs::write(&path, content)
        .map_err(|e| format!("Failed to write recent projects file: {}", e))?;

    Ok(())
}

/// Get list of recent projects
#[tauri::command]
pub fn get_recent_projects(app: AppHandle) -> Result<Vec<RecentProject>, String> {
    let mut projects = load_recent_projects(&app)?;

    // Filter out projects whose files no longer exist
    projects.retain(|p| Path::new(&p.path).exists());

    // Sort by timestamp (most recent first)
    projects.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(projects)
}

/// Add or update a recent project
#[tauri::command]
pub fn add_recent_project(app: AppHandle, project: RecentProject) -> Result<(), String> {
    let mut projects = load_recent_projects(&app)?;

    // Remove existing entry with same path (if any)
    projects.retain(|p| p.path != project.path);

    // Add new project at the beginning
    projects.insert(0, project);

    // Keep only MAX_RECENT_PROJECTS
    projects.truncate(MAX_RECENT_PROJECTS);

    save_recent_projects(&app, &projects)?;

    Ok(())
}

/// Remove a recent project by path
#[tauri::command]
pub fn remove_recent_project(app: AppHandle, path: String) -> Result<(), String> {
    let mut projects = load_recent_projects(&app)?;

    projects.retain(|p| p.path != path);

    save_recent_projects(&app, &projects)?;

    Ok(())
}

/// Clear all recent projects
#[tauri::command]
pub fn clear_recent_projects(app: AppHandle) -> Result<(), String> {
    save_recent_projects(&app, &[])?;
    Ok(())
}

// ============================================================================
// Template Editor Commands
// ============================================================================

/// Template language info
#[derive(Debug, Clone, Serialize)]
pub struct TemplateLanguageInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub file_count: usize,
}

/// Template file info
#[derive(Debug, Clone, Serialize)]
pub struct TemplateFileInfo {
    pub name: String,
    pub path: String,
    pub relative_path: String,
    pub is_directory: bool,
    pub children: Vec<TemplateFileInfo>,
}

/// Get the default templates directory
fn get_default_templates_dir() -> Result<PathBuf, String> {
    // In development, use the templates directory from the project root
    if cfg!(debug_assertions) {
        Ok(PathBuf::from("../../templates"))
    } else {
        // In production, templates should be bundled with the app
        Ok(PathBuf::from("templates"))
    }
}

/// List all template languages
#[tauri::command]
pub fn list_template_languages(templates_dir: Option<String>) -> Result<Vec<TemplateLanguageInfo>, String> {
    let templates_path = match templates_dir {
        Some(dir) => PathBuf::from(dir),
        None => get_default_templates_dir()?,
    };

    if !templates_path.exists() {
        return Err(format!("Templates directory not found: {:?}", templates_path));
    }

    let mut languages = Vec::new();

    let entries = fs::read_dir(&templates_path)
        .map_err(|e| format!("Failed to read templates directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let dir_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Skip rhai_utils (shared utilities)
            if dir_name == "rhai_utils" {
                continue;
            }

            // Check if there's a .toml config file
            let toml_path = path.join(format!("{}.toml", dir_name));
            let name = if toml_path.exists() {
                // Try to read the name from toml
                if let Ok(content) = fs::read_to_string(&toml_path) {
                    extract_toml_value(&content, "name")
                        .unwrap_or_else(|| dir_name.clone())
                } else {
                    dir_name.clone()
                }
            } else {
                dir_name.clone()
            };

            // Count .rhai files
            let file_count = count_rhai_files(&path);

            languages.push(TemplateLanguageInfo {
                id: dir_name,
                name,
                path: path.to_string_lossy().to_string(),
                file_count,
            });
        }
    }

    // Sort by name
    languages.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(languages)
}

/// Extract a value from TOML content (simple parser)
fn extract_toml_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(&format!("{} ", key)) || trimmed.starts_with(&format!("{}=", key)) {
            if let Some(value_part) = trimmed.split('=').nth(1) {
                let value = value_part.trim().trim_matches('"').trim_matches('\'');
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Count .rhai files in a directory recursively
fn count_rhai_files(path: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                count += count_rhai_files(&entry_path);
            } else if entry_path.extension().and_then(|e| e.to_str()) == Some("rhai") {
                count += 1;
            }
        }
    }
    count
}

/// List template files for a specific language
#[tauri::command]
pub fn list_template_files(
    lang: String,
    templates_dir: Option<String>,
) -> Result<Vec<TemplateFileInfo>, String> {
    let templates_path = match templates_dir {
        Some(dir) => PathBuf::from(dir),
        None => get_default_templates_dir()?,
    };

    let lang_path = templates_path.join(&lang);
    if !lang_path.exists() {
        return Err(format!("Language directory not found: {:?}", lang_path));
    }

    fn build_file_tree(path: &Path, base_path: &Path) -> Vec<TemplateFileInfo> {
        let mut files = Vec::new();

        if let Ok(entries) = fs::read_dir(path) {
            let mut entries: Vec<_> = entries.flatten().collect();
            entries.sort_by(|a, b| {
                let a_is_dir = a.path().is_dir();
                let b_is_dir = b.path().is_dir();
                match (a_is_dir, b_is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.file_name().cmp(&b.file_name()),
                }
            });

            for entry in entries {
                let entry_path = entry.path();
                let name = entry_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                let relative_path = entry_path.strip_prefix(base_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| name.clone());

                if entry_path.is_dir() {
                    let children = build_file_tree(&entry_path, base_path);
                    files.push(TemplateFileInfo {
                        name,
                        path: entry_path.to_string_lossy().to_string(),
                        relative_path,
                        is_directory: true,
                        children,
                    });
                } else {
                    // Only include .rhai and .toml files
                    let ext = entry_path.extension().and_then(|e| e.to_str());
                    if ext == Some("rhai") || ext == Some("toml") {
                        files.push(TemplateFileInfo {
                            name,
                            path: entry_path.to_string_lossy().to_string(),
                            relative_path,
                            is_directory: false,
                            children: Vec::new(),
                        });
                    }
                }
            }
        }

        files
    }

    Ok(build_file_tree(&lang_path, &lang_path))
}

/// Read a template file
#[tauri::command]
pub fn read_template_file(
    lang: String,
    relative_path: String,
    templates_dir: Option<String>,
) -> Result<String, String> {
    let templates_path = match templates_dir {
        Some(dir) => PathBuf::from(dir),
        None => get_default_templates_dir()?,
    };

    let file_path = templates_path.join(&lang).join(&relative_path);

    // Security: ensure path doesn't escape the templates directory
    let canonical_templates = templates_path.canonicalize()
        .map_err(|e| format!("Failed to resolve templates path: {}", e))?;
    let canonical_file = file_path.canonicalize()
        .map_err(|e| format!("Failed to resolve file path: {}", e))?;

    if !canonical_file.starts_with(&canonical_templates) {
        return Err("Invalid path: access denied".to_string());
    }

    fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file: {}", e))
}

/// Write a template file
#[tauri::command]
pub fn write_template_file(
    lang: String,
    relative_path: String,
    content: String,
    templates_dir: Option<String>,
) -> Result<(), String> {
    let templates_path = match templates_dir {
        Some(dir) => PathBuf::from(dir),
        None => get_default_templates_dir()?,
    };

    let file_path = templates_path.join(&lang).join(&relative_path);

    // Security: ensure path doesn't escape the templates directory
    // For new files, check parent directory
    let parent_path = file_path.parent()
        .ok_or("Invalid file path")?;

    if !parent_path.exists() {
        fs::create_dir_all(parent_path)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    let canonical_templates = templates_path.canonicalize()
        .map_err(|e| format!("Failed to resolve templates path: {}", e))?;
    let canonical_parent = parent_path.canonicalize()
        .map_err(|e| format!("Failed to resolve parent path: {}", e))?;

    if !canonical_parent.starts_with(&canonical_templates) {
        return Err("Invalid path: access denied".to_string());
    }

    fs::write(&file_path, content)
        .map_err(|e| format!("Failed to write file: {}", e))
}

/// Create a new language template
#[tauri::command]
pub fn create_new_language(
    lang_id: String,
    lang_name: String,
    templates_dir: Option<String>,
) -> Result<(), String> {
    let templates_path = match templates_dir {
        Some(dir) => PathBuf::from(dir),
        None => get_default_templates_dir()?,
    };

    let lang_path = templates_path.join(&lang_id);

    if lang_path.exists() {
        return Err(format!("Language '{}' already exists", lang_id));
    }

    // Create directory structure
    fs::create_dir_all(&lang_path)
        .map_err(|e| format!("Failed to create language directory: {}", e))?;

    fs::create_dir_all(lang_path.join("rhai_utils"))
        .map_err(|e| format!("Failed to create rhai_utils directory: {}", e))?;

    // Create .toml config file
    let toml_content = format!(r#"[language]
id = "{}"
name = "{}"
version = "1.0"

[code_generation]
extension = "txt"
namespace_separator = "."
indent = "    "
"#, lang_id, lang_name);

    fs::write(lang_path.join(format!("{}.toml", lang_id)), toml_content)
        .map_err(|e| format!("Failed to create config file: {}", e))?;

    // Create main template file
    let main_template = format!(r#"// {lang_name} Code Generator Template
// This is the main entry point for code generation.
//
// Available variables:
//   - file: FileDef with namespaces, imports
//   - config: Language configuration from .toml
//
// Available functions:
//   - to_pascal_case(text), to_snake_case(text), to_camel_case(text)
//   - indent(text, level), indent_lines(text, level)
//   - map_type(type_ref) - use rhai_utils/type_mapping.rhai
//
// See CUSTOMIZATION.md for full documentation.

// Import type mapping utilities
import "rhai_utils/type_mapping" as types;

// File header
`// Generated by PolyGen - {lang_name}`
`// Do not edit manually`
``

// Process each namespace
for ns in file.namespaces {{
    let ns_name = ns.name;

    `// Namespace: ${{ns_name}}`
    ``

    // Generate structs/classes
    for struct_def in ns.structs {{
        let name = struct_def.name;
        let fields = struct_def.fields;

        `struct ${{name}} {{}}`
        for field in fields {{
            let field_name = field.field_name;
            let field_type = types::map_type(field.type_ref);
            `    ${{field_name}}: ${{field_type}}`
        }}
        `}}`
        ``
    }}

    // Generate enums
    for enum_def in ns.enums {{
        let name = enum_def.name;
        let variants = enum_def.variants;

        `enum ${{name}} {{}}`
        for variant in variants {{
            `    ${{variant.name}} = ${{variant.value}}`
        }}
        `}}`
        ``
    }}
}}
"#, lang_name = lang_name);

    fs::write(lang_path.join(format!("{}_file.rhai", lang_id)), main_template)
        .map_err(|e| format!("Failed to create main template: {}", e))?;

    // Create type mapping utility
    let type_mapping = r#"// Type Mapping Utilities
// Map PolyGen types to target language types

fn map_type(type_ref) {
    let base_type = type_ref.base_type;
    let is_optional = type_ref.is_optional;
    let is_array = type_ref.is_array;

    // Map primitive types
    let mapped = switch base_type {
        "string" => "String",
        "bool" => "Bool",
        "bytes" => "Bytes",
        "u8" => "UInt8",
        "u16" => "UInt16",
        "u32" => "UInt32",
        "u64" => "UInt64",
        "i8" => "Int8",
        "i16" => "Int16",
        "i32" => "Int32",
        "i64" => "Int64",
        "f32" => "Float32",
        "f64" => "Float64",
        _ => base_type,  // Custom types pass through
    };

    // Handle array
    if is_array {
        mapped = `Array<${mapped}>`;
    }

    // Handle optional
    if is_optional {
        mapped = `Optional<${mapped}>`;
    }

    mapped
}

fn get_default_value(type_ref) {
    let base_type = type_ref.base_type;

    switch base_type {
        "string" => `""`,
        "bool" => "false",
        "u8" | "u16" | "u32" | "u64" => "0",
        "i8" | "i16" | "i32" | "i64" => "0",
        "f32" | "f64" => "0.0",
        _ => "null",
    }
}
"#;

    fs::write(lang_path.join("rhai_utils/type_mapping.rhai"), type_mapping)
        .map_err(|e| format!("Failed to create type mapping: {}", e))?;

    Ok(())
}

/// Delete a template file or directory
#[tauri::command]
pub fn delete_template_file(
    lang: String,
    relative_path: String,
    templates_dir: Option<String>,
) -> Result<(), String> {
    let templates_path = match templates_dir {
        Some(dir) => PathBuf::from(dir),
        None => get_default_templates_dir()?,
    };

    let file_path = templates_path.join(&lang).join(&relative_path);

    // Security: ensure path doesn't escape the templates directory
    let canonical_templates = templates_path.canonicalize()
        .map_err(|e| format!("Failed to resolve templates path: {}", e))?;
    let canonical_file = file_path.canonicalize()
        .map_err(|e| format!("Failed to resolve file path: {}", e))?;

    if !canonical_file.starts_with(&canonical_templates) {
        return Err("Invalid path: access denied".to_string());
    }

    // Don't allow deleting the language root or critical files
    if relative_path.is_empty() || relative_path == "/" {
        return Err("Cannot delete language root directory".to_string());
    }

    if file_path.is_dir() {
        fs::remove_dir_all(&file_path)
            .map_err(|e| format!("Failed to delete directory: {}", e))
    } else {
        fs::remove_file(&file_path)
            .map_err(|e| format!("Failed to delete file: {}", e))
    }
}

/// Preview template result - generate code using the current template
/// Uses --preview flag to output generated files to stdout
#[tauri::command]
pub fn preview_template(
    schema_path: String,
    lang: String,
    templates_dir: Option<String>,
) -> Result<String, String> {
    let polygen = get_polygen_path();

    // Resolve templates directory to absolute path so polygen can find it
    // regardless of the working directory
    let resolved_templates = match templates_dir {
        Some(dir) => PathBuf::from(dir),
        None => get_default_templates_dir()?,
    };
    let resolved_templates = resolved_templates.canonicalize()
        .map_err(|e| format!("Templates directory not found: {}", e))?;

    // Resolve schema path to absolute as well
    let resolved_schema = PathBuf::from(&schema_path).canonicalize()
        .map_err(|e| format!("Schema file not found: {}: {}", schema_path, e))?;

    // Set working directory to the parent of the templates directory (project root)
    // so that Rhai `import "templates/..."` paths resolve correctly.
    let project_root = resolved_templates.parent()
        .ok_or_else(|| "Cannot determine project root from templates directory".to_string())?;

    let mut cmd = Command::new(&polygen);
    cmd.current_dir(project_root)
        .arg("generate")
        .arg("--schema-path")
        .arg(&resolved_schema)
        .arg("--lang")
        .arg(&lang)
        .arg("--templates-dir")
        .arg(&resolved_templates)
        .arg("--preview");

    let output = cmd.output().map_err(|e| format!("Failed to execute polygen: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(stderr.to_string())
    }
}

/// Validate a Rhai script
#[tauri::command]
pub fn validate_rhai_script(content: String) -> Result<Vec<SchemaError>, String> {
    use rhai::Engine;

    let engine = Engine::new();
    let mut errors = Vec::new();

    match engine.compile(&content) {
        Ok(_) => {
            // Script is valid
        }
        Err(e) => {
            // Extract position from parse error
            let (line, col) = match e.position() {
                rhai::Position::NONE => (1, 1),
                pos => (pos.line().unwrap_or(1), pos.position().unwrap_or(1)),
            };

            errors.push(SchemaError {
                start_line: line,
                start_column: col,
                end_line: line,
                end_column: col + 10,
                message: e.to_string(),
                severity: "error".to_string(),
            });
        }
    }

    Ok(errors)
}
