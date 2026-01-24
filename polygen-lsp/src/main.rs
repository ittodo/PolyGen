mod symbol_table;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use pest::Parser;
use polygen::{ast_parser, validation, Polygen, Rule};

use symbol_table::SymbolTable;

#[derive(Debug)]
struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, String>>>,
    symbol_tables: Arc<RwLock<HashMap<Url, SymbolTable>>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
            symbol_tables: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn validate_document(&self, uri: &Url, content: &str) {
        // Get file path from URI for import resolution
        let file_path = uri.to_file_path().ok();

        let diagnostics = self.get_diagnostics(content, file_path.as_deref());
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;

        // Build symbol table for Go to Definition support (with imports)
        let symbol_table_result = if let Some(ref path) = file_path {
            symbol_table::build_symbol_table_with_imports(content, Some(path))
        } else {
            symbol_table::build_symbol_table(content)
        };

        if let Ok(symbol_table) = symbol_table_result {
            self.symbol_tables
                .write()
                .await
                .insert(uri.clone(), symbol_table);
        }
    }

    fn get_diagnostics(&self, content: &str, file_path: Option<&std::path::Path>) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let content = content.replace("\r\n", "\n");

        // Step 1: Parse with Pest
        match Polygen::parse(Rule::main, &content) {
            Ok(mut pairs) => {
                if let Some(main_pair) = pairs.next() {
                    // Step 2: Build AST
                    match ast_parser::build_ast_from_pairs(main_pair, PathBuf::from("editor.poly"))
                    {
                        Ok(ast) => {
                            // Step 3: Validate AST (skip TypeNotFound when we have file_path for import support)
                            if let Err(e) = validation::validate_ast(&ast.definitions) {
                                // Skip TypeNotFound errors when file_path is provided,
                                // as the symbol table validation handles imports properly
                                let is_type_not_found = matches!(&e, polygen::error::ValidationError::TypeNotFound(_));
                                if !is_type_not_found || file_path.is_none() {
                                    diagnostics.push(Diagnostic {
                                        range: Range {
                                            start: Position {
                                                line: 0,
                                                character: 0,
                                            },
                                            end: Position {
                                                line: 0,
                                                character: 100,
                                            },
                                        },
                                        severity: Some(DiagnosticSeverity::ERROR),
                                        message: e.to_string(),
                                        source: Some("polygen".to_string()),
                                        ..Default::default()
                                    });
                                }
                            }

                            // Step 4: Check for unresolved type references (with imports)
                            let symbol_table_result = if let Some(path) = file_path {
                                symbol_table::build_symbol_table_with_imports(&content, Some(path))
                            } else {
                                symbol_table::build_symbol_table(&content)
                            };
                            if let Ok(symbol_table) = symbol_table_result {
                                for reference in &symbol_table.references {
                                    if reference.resolved_fqn.is_none() {
                                        diagnostics.push(Diagnostic {
                                            range: Range {
                                                start: Position {
                                                    line: (reference.span.start_line - 1) as u32,
                                                    character: (reference.span.start_col - 1) as u32,
                                                },
                                                end: Position {
                                                    line: (reference.span.end_line - 1) as u32,
                                                    character: (reference.span.end_col - 1) as u32,
                                                },
                                            },
                                            severity: Some(DiagnosticSeverity::ERROR),
                                            message: format!("Unresolved reference: '{}'", reference.path),
                                            source: Some("polygen".to_string()),
                                            ..Default::default()
                                        });
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let (line, col, msg) = match &e {
                                polygen::error::AstBuildError::InvalidValue { line, col, .. } => {
                                    (*line, *col, e.to_string())
                                }
                                polygen::error::AstBuildError::UnexpectedRule { line, col, .. } => {
                                    (*line, *col, e.to_string())
                                }
                                polygen::error::AstBuildError::MissingElement { line, col, .. } => {
                                    (*line, *col, e.to_string())
                                }
                            };
                            diagnostics.push(Diagnostic {
                                range: Range {
                                    start: Position {
                                        line: (line - 1) as u32,
                                        character: (col - 1) as u32,
                                    },
                                    end: Position {
                                        line: (line - 1) as u32,
                                        character: (col + 10) as u32,
                                    },
                                },
                                severity: Some(DiagnosticSeverity::ERROR),
                                message: msg,
                                source: Some("polygen".to_string()),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
            Err(e) => {
                let (line, col) = match e.line_col {
                    pest::error::LineColLocation::Pos((l, c)) => (l, c),
                    pest::error::LineColLocation::Span((l, c), _) => (l, c),
                };
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: (line - 1) as u32,
                            character: (col - 1) as u32,
                        },
                        end: Position {
                            line: (line - 1) as u32,
                            character: (col + 10) as u32,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: format!("Syntax error: {}", e),
                    source: Some("polygen".to_string()),
                    ..Default::default()
                });
            }
        }

        diagnostics
    }

    fn get_completions(&self, content: &str, position: Position, symbol_table: Option<&SymbolTable>) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Determine context by analyzing current line
        let lines: Vec<&str> = content.lines().collect();
        let current_line = lines.get(position.line as usize).unwrap_or(&"");
        let col = position.character as usize;
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
                ("@cache($1)", "@cache", "Enable caching (full_load, on_demand, write_through)"),
                ("@readonly", "@readonly", "Mark table as read-only"),
                ("@taggable", "@taggable", "Enable row tagging"),
                ("@datasource($1)", "@datasource", "Specify database source (sqlite, mysql)"),
                ("@soft_delete($1)", "@soft_delete", "Enable soft delete with specified field"),
                ("@link_rows($1)", "@link_rows", "Link rows to another table"),
            ];

            for (insert, label, detail) in annotations {
                items.push(CompletionItem {
                    label: label.to_string(),
                    kind: Some(CompletionItemKind::SNIPPET),
                    detail: Some(detail.to_string()),
                    insert_text: Some(insert.to_string()),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    ..Default::default()
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
                items.push(CompletionItem {
                    label: label.to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    detail: Some(detail.to_string()),
                    insert_text: Some(insert.to_string()),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    ..Default::default()
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
                ("u8", "Unsigned 8-bit integer (0-255)"),
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
                items.push(CompletionItem {
                    label: type_name.to_string(),
                    kind: Some(CompletionItemKind::TYPE_PARAMETER),
                    detail: Some(detail.to_string()),
                    insert_text: Some(type_name.to_string()),
                    ..Default::default()
                });
            }

            // User-defined types from symbol table
            if let Some(table) = symbol_table {
                for (name, fqn, kind) in table.get_all_type_names() {
                    let kind_str = match kind {
                        symbol_table::DefinitionKind::Table => "table",
                        symbol_table::DefinitionKind::Enum => "enum",
                        symbol_table::DefinitionKind::Embed => "embed",
                        _ => continue,
                    };

                    // Add simple name
                    items.push(CompletionItem {
                        label: name.to_string(),
                        kind: Some(match kind {
                            symbol_table::DefinitionKind::Table => CompletionItemKind::CLASS,
                            symbol_table::DefinitionKind::Enum => CompletionItemKind::ENUM,
                            symbol_table::DefinitionKind::Embed => CompletionItemKind::STRUCT,
                            _ => CompletionItemKind::TYPE_PARAMETER,
                        }),
                        detail: Some(format!("{} {}", kind_str, fqn)),
                        insert_text: Some(name.to_string()),
                        ..Default::default()
                    });

                    // Add fully qualified name if different
                    if name != fqn {
                        items.push(CompletionItem {
                            label: fqn.to_string(),
                            kind: Some(match kind {
                                symbol_table::DefinitionKind::Table => CompletionItemKind::CLASS,
                                symbol_table::DefinitionKind::Enum => CompletionItemKind::ENUM,
                                symbol_table::DefinitionKind::Embed => CompletionItemKind::STRUCT,
                                _ => CompletionItemKind::TYPE_PARAMETER,
                            }),
                            detail: Some(format!("{} (fully qualified)", kind_str)),
                            insert_text: Some(fqn.to_string()),
                            ..Default::default()
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
                ("range", "range($1, $2)", "Set value range (min, max)"),
                ("regex", "regex(\"$1\")", "Set regex validation pattern"),
                ("foreign_key", "foreign_key($1.$2)", "Reference another table.field"),
            ];

            for (label, insert, detail) in constraints {
                items.push(CompletionItem {
                    label: label.to_string(),
                    kind: Some(CompletionItemKind::PROPERTY),
                    detail: Some(detail.to_string()),
                    insert_text: Some(insert.to_string()),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    ..Default::default()
                });
            }
        }

        items
    }

    /// Get document symbols for outline view
    fn get_document_symbols(&self, symbol_table: &SymbolTable) -> Vec<DocumentSymbol> {
        use symbol_table::DefinitionKind;

        let mut symbols: Vec<DocumentSymbol> = Vec::new();

        // Get top-level namespaces first
        let namespaces: Vec<_> = symbol_table
            .get_definitions_by_kind(DefinitionKind::Namespace)
            .into_iter()
            .filter(|d| !d.fqn.contains('.') || d.fqn.matches('.').count() == d.name.matches('.').count())
            .collect();

        for ns in namespaces {
            let ns_symbol = self.create_document_symbol(ns, symbol_table);
            symbols.push(ns_symbol);
        }

        // Get top-level types (not in namespace)
        for def in symbol_table.get_all_definitions() {
            if !def.fqn.contains('.') && !matches!(def.kind, DefinitionKind::Namespace | DefinitionKind::Field) {
                symbols.push(self.create_document_symbol(def, symbol_table));
            }
        }

        symbols
    }

    fn create_document_symbol(&self, def: &symbol_table::DefinitionInfo, symbol_table: &SymbolTable) -> DocumentSymbol {
        use symbol_table::DefinitionKind;

        let kind = match def.kind {
            DefinitionKind::Namespace => SymbolKind::NAMESPACE,
            DefinitionKind::Table => SymbolKind::CLASS,
            DefinitionKind::Enum => SymbolKind::ENUM,
            DefinitionKind::Embed => SymbolKind::STRUCT,
            DefinitionKind::Field => SymbolKind::FIELD,
        };

        let range = Range {
            start: Position {
                line: (def.name_span.start_line - 1) as u32,
                character: (def.name_span.start_col - 1) as u32,
            },
            end: Position {
                line: (def.name_span.end_line - 1) as u32,
                character: (def.name_span.end_col - 1) as u32,
            },
        };

        // Get children
        let prefix = format!("{}.", def.fqn);
        let children: Vec<DocumentSymbol> = symbol_table
            .get_all_definitions()
            .into_iter()
            .filter(|d| {
                d.fqn.starts_with(&prefix)
                    && d.fqn[prefix.len()..].chars().filter(|c| *c == '.').count() == 0
            })
            .map(|d| self.create_document_symbol(d, symbol_table))
            .collect();

        #[allow(deprecated)]
        DocumentSymbol {
            name: def.name.clone(),
            detail: Some(def.fqn.clone()),
            kind,
            tags: None,
            deprecated: None,
            range,
            selection_range: range,
            children: if children.is_empty() { None } else { Some(children) },
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        ".".to_string(),
                        ":".to_string(),
                        "@".to_string(),
                    ]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "polygen-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "PolyGen LSP server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params.text_document.text;

        self.documents
            .write()
            .await
            .insert(uri.clone(), content.clone());
        self.validate_document(&uri, &content).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().next() {
            let content = change.text;
            self.documents
                .write()
                .await
                .insert(uri.clone(), content.clone());
            self.validate_document(&uri, &content).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = &params.text_document.uri;
        self.documents.write().await.remove(uri);
        self.symbol_tables.write().await.remove(uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let content = self
            .documents
            .read()
            .await
            .get(uri)
            .cloned()
            .unwrap_or_default();

        let symbol_tables = self.symbol_tables.read().await;
        let symbol_table = symbol_tables.get(uri);

        let items = self.get_completions(&content, position, symbol_table);
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let content = self
            .documents
            .read()
            .await
            .get(uri)
            .cloned()
            .unwrap_or_default();

        // Get word at position
        let lines: Vec<&str> = content.lines().collect();
        if let Some(line) = lines.get(position.line as usize) {
            let col = position.character as usize;
            let chars: Vec<char> = line.chars().collect();

            // Find word boundaries (including dots for qualified names)
            let mut start = col;
            let mut end = col;

            while start > 0 && chars.get(start - 1).map(|c| c.is_alphanumeric() || *c == '_' || *c == '.').unwrap_or(false) {
                start -= 1;
            }
            while end < chars.len() && chars.get(end).map(|c| c.is_alphanumeric() || *c == '_' || *c == '.').unwrap_or(false) {
                end += 1;
            }

            let word: String = chars[start..end].iter().collect();

            // First, try to find user-defined type from symbol table
            let symbol_tables = self.symbol_tables.read().await;
            if let Some(symbol_table) = symbol_tables.get(uri) {
                // Check if cursor is on a reference or definition
                let line_1indexed = position.line as usize + 1;
                let col_1indexed = position.character as usize + 1;

                if let Some(def) = symbol_table.symbol_at(line_1indexed, col_1indexed) {
                    let kind_str = match def.kind {
                        symbol_table::DefinitionKind::Namespace => "namespace",
                        symbol_table::DefinitionKind::Table => "table",
                        symbol_table::DefinitionKind::Enum => "enum",
                        symbol_table::DefinitionKind::Embed => "embed",
                        symbol_table::DefinitionKind::Field => "field",
                    };

                    // Get fields for table/embed
                    let fields_info = if matches!(def.kind, symbol_table::DefinitionKind::Table | symbol_table::DefinitionKind::Embed) {
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

                    let hover_text = format!(
                        "**{} {}**\n\nFully qualified name: `{}`{}{}",
                        kind_str, def.name, def.fqn, fields_info, file_info
                    );

                    return Ok(Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: hover_text,
                        }),
                        range: Some(Range {
                            start: Position { line: position.line, character: start as u32 },
                            end: Position { line: position.line, character: end as u32 },
                        }),
                    }));
                }
            }

            // Fallback to built-in keywords and types
            let simple_word = word.split('.').last().unwrap_or(&word);
            let hover_text = match simple_word {
                "namespace" => Some("**namespace**\n\nDefines a namespace to organize types.\n\n```poly\nnamespace game.data {\n    table Player { }\n}\n```"),
                "table" => Some("**table**\n\nDefines a data structure (class/struct).\n\n```poly\ntable Player {\n    id: u32 primary_key;\n    name: string;\n}\n```"),
                "enum" => Some("**enum**\n\nDefines an enumeration type.\n\n```poly\nenum Status {\n    Active = 1\n    Inactive = 2\n}\n```"),
                "embed" => Some("**embed**\n\nDefines a reusable field group.\n\n```poly\nembed Address {\n    street: string;\n    city: string;\n}\n```"),
                "import" => Some("**import**\n\nImports types from another schema file.\n\n```poly\nimport \"common_types.poly\";\n```"),
                "string" => Some("**string**\n\nVariable-length text type.\n\nMaps to: C# `string`, C++ `std::string`, Rust `String`, TS `string`"),
                "bool" => Some("**bool**\n\nBoolean type (true/false).\n\nMaps to: C# `bool`, C++ `bool`, Rust `bool`, TS `boolean`"),
                "bytes" => Some("**bytes**\n\nBinary data type.\n\nMaps to: C# `byte[]`, C++ `std::vector<uint8_t>`, Rust `Vec<u8>`, TS `Uint8Array`"),
                "u8" => Some("**u8**\n\nUnsigned 8-bit integer (0 to 255).\n\nMaps to: C# `byte`, C++ `uint8_t`, Rust `u8`, TS `number`"),
                "u16" => Some("**u16**\n\nUnsigned 16-bit integer (0 to 65,535).\n\nMaps to: C# `ushort`, C++ `uint16_t`, Rust `u16`, TS `number`"),
                "u32" => Some("**u32**\n\nUnsigned 32-bit integer (0 to 4,294,967,295).\n\nMaps to: C# `uint`, C++ `uint32_t`, Rust `u32`, TS `number`"),
                "u64" => Some("**u64**\n\nUnsigned 64-bit integer.\n\nMaps to: C# `ulong`, C++ `uint64_t`, Rust `u64`, TS `bigint`"),
                "i8" => Some("**i8**\n\nSigned 8-bit integer (-128 to 127).\n\nMaps to: C# `sbyte`, C++ `int8_t`, Rust `i8`, TS `number`"),
                "i16" => Some("**i16**\n\nSigned 16-bit integer.\n\nMaps to: C# `short`, C++ `int16_t`, Rust `i16`, TS `number`"),
                "i32" => Some("**i32**\n\nSigned 32-bit integer.\n\nMaps to: C# `int`, C++ `int32_t`, Rust `i32`, TS `number`"),
                "i64" => Some("**i64**\n\nSigned 64-bit integer.\n\nMaps to: C# `long`, C++ `int64_t`, Rust `i64`, TS `bigint`"),
                "f32" => Some("**f32**\n\n32-bit floating point.\n\nMaps to: C# `float`, C++ `float`, Rust `f32`, TS `number`"),
                "f64" => Some("**f64**\n\n64-bit floating point.\n\nMaps to: C# `double`, C++ `double`, Rust `f64`, TS `number`"),
                "primary_key" => Some("**primary_key**\n\nMarks this field as the primary key.\n\n```poly\nid: u32 primary_key;\n```"),
                "unique" => Some("**unique**\n\nEnsures values in this field are unique.\n\n```poly\nemail: string unique;\n```"),
                "max_length" => Some("**max_length(n)**\n\nSets maximum string length.\n\n```poly\nname: string max_length(100);\n```"),
                "default" => Some("**default(value)**\n\nSets a default value.\n\n```poly\nlevel: u16 default(1);\nenabled: bool default(true);\n```"),
                "range" => Some("**range(min, max)**\n\nSets allowed value range.\n\n```poly\nhp: u32 range(0, 9999);\n```"),
                "regex" => Some("**regex(\"pattern\")**\n\nSets a regex validation pattern.\n\n```poly\nemail: string regex(\"^[^@]+@[^@]+$\");\n```"),
                "foreign_key" => Some("**foreign_key(Table.field)**\n\nReferences a field in another table.\n\n```poly\nuser_id: u32 foreign_key(User.id);\n```"),
                _ => None,
            };

            if let Some(text) = hover_text {
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: text.to_string(),
                    }),
                    range: Some(Range {
                        start: Position { line: position.line, character: start as u32 },
                        end: Position { line: position.line, character: end as u32 },
                    }),
                }));
            }
        }

        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;

        let symbol_tables = self.symbol_tables.read().await;
        let Some(symbol_table) = symbol_tables.get(uri) else {
            return Ok(None);
        };

        let symbols = self.get_document_symbols(symbol_table);
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = &params.text_document.uri;
        let position = params.position;

        let line = position.line as usize + 1;
        let col = position.character as usize + 1;

        let symbol_tables = self.symbol_tables.read().await;
        let Some(symbol_table) = symbol_tables.get(uri) else {
            return Ok(None);
        };

        // Check if we're on a renameable symbol
        if let Some(def) = symbol_table.definition_at(line, col) {
            // Don't allow renaming fields (too complex for now)
            if matches!(def.kind, symbol_table::DefinitionKind::Field) {
                return Ok(None);
            }

            return Ok(Some(PrepareRenameResponse::Range(Range {
                start: Position {
                    line: (def.name_span.start_line - 1) as u32,
                    character: (def.name_span.start_col - 1) as u32,
                },
                end: Position {
                    line: (def.name_span.end_line - 1) as u32,
                    character: (def.name_span.end_col - 1) as u32,
                },
            })));
        }

        // Check if we're on a reference
        if let Some(reference) = symbol_table.reference_at(line, col) {
            if reference.resolved_fqn.is_some() {
                return Ok(Some(PrepareRenameResponse::Range(Range {
                    start: Position {
                        line: (reference.span.start_line - 1) as u32,
                        character: (reference.span.start_col - 1) as u32,
                    },
                    end: Position {
                        line: (reference.span.end_line - 1) as u32,
                        character: (reference.span.end_col - 1) as u32,
                    },
                })));
            }
        }

        Ok(None)
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = &params.new_name;

        let line = position.line as usize + 1;
        let col = position.character as usize + 1;

        let symbol_tables = self.symbol_tables.read().await;
        let Some(symbol_table) = symbol_tables.get(uri) else {
            return Ok(None);
        };

        // Find the target FQN
        let target_fqn = if let Some(def) = symbol_table.definition_at(line, col) {
            def.fqn.clone()
        } else if let Some(reference) = symbol_table.reference_at(line, col) {
            match &reference.resolved_fqn {
                Some(fqn) => fqn.clone(),
                None => return Ok(None),
            }
        } else {
            return Ok(None);
        };

        let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();

        // Rename the definition
        if let Some(def) = symbol_table.get_definition(&target_fqn) {
            let def_uri = if let Some(ref file_path) = def.file_path {
                Url::from_file_path(file_path).unwrap_or_else(|_| uri.clone())
            } else {
                uri.clone()
            };

            changes.entry(def_uri).or_default().push(TextEdit {
                range: Range {
                    start: Position {
                        line: (def.name_span.start_line - 1) as u32,
                        character: (def.name_span.start_col - 1) as u32,
                    },
                    end: Position {
                        line: (def.name_span.end_line - 1) as u32,
                        character: (def.name_span.end_col - 1) as u32,
                    },
                },
                new_text: new_name.clone(),
            });
        }

        // Rename references in current file
        for reference in symbol_table.find_references(&target_fqn) {
            // Only rename simple references (not qualified paths)
            // For qualified paths, we'd need more complex logic
            let ref_text = &reference.path;
            let simple_name = target_fqn.rsplit('.').next().unwrap_or(&target_fqn);

            if ref_text == simple_name || ref_text.ends_with(&format!(".{}", simple_name)) {
                let new_text = if ref_text == simple_name {
                    new_name.clone()
                } else {
                    // Replace only the last part of qualified name
                    let prefix = &ref_text[..ref_text.len() - simple_name.len()];
                    format!("{}{}", prefix, new_name)
                };

                changes.entry(uri.clone()).or_default().push(TextEdit {
                    range: Range {
                        start: Position {
                            line: (reference.span.start_line - 1) as u32,
                            character: (reference.span.start_col - 1) as u32,
                        },
                        end: Position {
                            line: (reference.span.end_line - 1) as u32,
                            character: (reference.span.end_col - 1) as u32,
                        },
                    },
                    new_text,
                });
            }
        }

        // Search in files that import the current file
        if let Ok(current_path) = uri.to_file_path() {
            if let Some(dir) = current_path.parent() {
                let current_filename = current_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let entry_path = entry.path();
                        if entry_path.extension().and_then(|e| e.to_str()) == Some("poly") {
                            if entry_path == current_path {
                                continue;
                            }

                            if let Ok(other_content) = std::fs::read_to_string(&entry_path) {
                                let imports_current = other_content.lines().any(|line| {
                                    let trimmed = line.trim();
                                    trimmed.starts_with("import ") && trimmed.contains(current_filename)
                                });

                                if imports_current {
                                    if let Ok(other_table) = symbol_table::build_symbol_table_with_imports(
                                        &other_content,
                                        Some(&entry_path),
                                    ) {
                                        let simple_name = target_fqn.rsplit('.').next().unwrap_or(&target_fqn);

                                        for reference in other_table.find_references(&target_fqn) {
                                            let ref_text = &reference.path;

                                            if ref_text == simple_name || ref_text.ends_with(&format!(".{}", simple_name)) {
                                                let new_text = if ref_text == simple_name {
                                                    new_name.clone()
                                                } else {
                                                    let prefix = &ref_text[..ref_text.len() - simple_name.len()];
                                                    format!("{}{}", prefix, new_name)
                                                };

                                                if let Ok(ref_uri) = Url::from_file_path(&entry_path) {
                                                    changes.entry(ref_uri).or_default().push(TextEdit {
                                                        range: Range {
                                                            start: Position {
                                                                line: (reference.span.start_line - 1) as u32,
                                                                character: (reference.span.start_col - 1) as u32,
                                                            },
                                                            end: Position {
                                                                line: (reference.span.end_line - 1) as u32,
                                                                character: (reference.span.end_col - 1) as u32,
                                                            },
                                                        },
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

        if changes.is_empty() {
            Ok(None)
        } else {
            Ok(Some(WorkspaceEdit {
                changes: Some(changes),
                ..Default::default()
            }))
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // LSP position is 0-indexed, but our spans are 1-indexed
        let line = position.line as usize + 1;
        let col = position.character as usize + 1;

        let symbol_tables = self.symbol_tables.read().await;
        let Some(symbol_table) = symbol_tables.get(uri) else {
            return Ok(None);
        };

        // First, check if we're clicking on a type reference
        if let Some(reference) = symbol_table.reference_at(line, col) {
            if let Some(fqn) = &reference.resolved_fqn {
                if let Some(definition) = symbol_table.get_definition(fqn) {
                    let target_range = Range {
                        start: Position {
                            line: (definition.name_span.start_line - 1) as u32,
                            character: (definition.name_span.start_col - 1) as u32,
                        },
                        end: Position {
                            line: (definition.name_span.end_line - 1) as u32,
                            character: (definition.name_span.end_col - 1) as u32,
                        },
                    };

                    return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                        uri: uri.clone(),
                        range: target_range,
                    })));
                }
            }
        }

        // Also check if we're clicking on a definition itself (to support F12 on definitions)
        if let Some(definition) = symbol_table.definition_at(line, col) {
            let target_range = Range {
                start: Position {
                    line: (definition.name_span.start_line - 1) as u32,
                    character: (definition.name_span.start_col - 1) as u32,
                },
                end: Position {
                    line: (definition.name_span.end_line - 1) as u32,
                    character: (definition.name_span.end_col - 1) as u32,
                },
            };

            return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                uri: uri.clone(),
                range: target_range,
            })));
        }

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        // LSP position is 0-indexed, but our spans are 1-indexed
        let line = position.line as usize + 1;
        let col = position.character as usize + 1;

        let symbol_tables = self.symbol_tables.read().await;
        let Some(symbol_table) = symbol_tables.get(uri) else {
            return Ok(None);
        };

        // Find what we're looking for (definition or reference at position)
        let target_fqn = if let Some(def) = symbol_table.definition_at(line, col) {
            def.fqn.clone()
        } else if let Some(reference) = symbol_table.reference_at(line, col) {
            match &reference.resolved_fqn {
                Some(fqn) => fqn.clone(),
                None => return Ok(None),
            }
        } else {
            return Ok(None);
        };

        let mut locations: Vec<Location> = Vec::new();

        // Include the definition itself if include_declaration is true
        if params.context.include_declaration {
            if let Some(def) = symbol_table.get_definition(&target_fqn) {
                let def_uri = if let Some(ref file_path) = def.file_path {
                    Url::from_file_path(file_path).unwrap_or_else(|_| uri.clone())
                } else {
                    uri.clone()
                };
                locations.push(Location {
                    uri: def_uri,
                    range: Range {
                        start: Position {
                            line: (def.name_span.start_line - 1) as u32,
                            character: (def.name_span.start_col - 1) as u32,
                        },
                        end: Position {
                            line: (def.name_span.end_line - 1) as u32,
                            character: (def.name_span.end_col - 1) as u32,
                        },
                    },
                });
            }
        }

        // Include references from current file
        for reference in symbol_table.find_references(&target_fqn) {
            locations.push(Location {
                uri: uri.clone(),
                range: Range {
                    start: Position {
                        line: (reference.span.start_line - 1) as u32,
                        character: (reference.span.start_col - 1) as u32,
                    },
                    end: Position {
                        line: (reference.span.end_line - 1) as u32,
                        character: (reference.span.end_col - 1) as u32,
                    },
                },
            });
        }

        // Search in files that might import the current file
        if let Ok(current_path) = uri.to_file_path() {
            if let Some(dir) = current_path.parent() {
                let current_filename = current_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let entry_path = entry.path();
                        if entry_path.extension().and_then(|e| e.to_str()) == Some("poly") {
                            // Skip the current file
                            if entry_path == current_path {
                                continue;
                            }

                            // Read and check if this file imports our file
                            if let Ok(other_content) = std::fs::read_to_string(&entry_path) {
                                let imports_current = other_content.lines().any(|line| {
                                    let trimmed = line.trim();
                                    trimmed.starts_with("import ") && trimmed.contains(current_filename)
                                });

                                if imports_current {
                                    // Build symbol table for this file (with imports)
                                    if let Ok(other_table) = symbol_table::build_symbol_table_with_imports(
                                        &other_content,
                                        Some(&entry_path),
                                    ) {
                                        // Find references to our target FQN
                                        for reference in other_table.find_references(&target_fqn) {
                                            if let Ok(ref_uri) = Url::from_file_path(&entry_path) {
                                                locations.push(Location {
                                                    uri: ref_uri,
                                                    range: Range {
                                                        start: Position {
                                                            line: (reference.span.start_line - 1) as u32,
                                                            character: (reference.span.start_col - 1) as u32,
                                                        },
                                                        end: Position {
                                                            line: (reference.span.end_line - 1) as u32,
                                                            character: (reference.span.end_col - 1) as u32,
                                                        },
                                                    },
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

        if locations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(locations))
        }
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
