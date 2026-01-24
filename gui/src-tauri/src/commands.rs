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
