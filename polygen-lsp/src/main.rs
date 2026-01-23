use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use pest::Parser;
use polygen::{ast_parser, validation, Polygen, Rule};

#[derive(Debug)]
struct Backend {
    client: Client,
    documents: Arc<RwLock<std::collections::HashMap<Url, String>>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    async fn validate_document(&self, uri: &Url, content: &str) {
        let diagnostics = self.get_diagnostics(content);
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }

    fn get_diagnostics(&self, content: &str) -> Vec<Diagnostic> {
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
                            // Step 3: Validate AST
                            if let Err(e) = validation::validate_ast(&ast.definitions) {
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

    fn get_completions(&self, _content: &str, _position: Position) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Keywords
        let keywords = [
            ("namespace", "Define a namespace", CompletionItemKind::KEYWORD),
            ("table", "Define a table/struct", CompletionItemKind::KEYWORD),
            ("enum", "Define an enumeration", CompletionItemKind::KEYWORD),
            ("embed", "Define an embedded type", CompletionItemKind::KEYWORD),
            ("import", "Import another schema file", CompletionItemKind::KEYWORD),
        ];

        for (label, detail, kind) in keywords {
            items.push(CompletionItem {
                label: label.to_string(),
                kind: Some(kind),
                detail: Some(detail.to_string()),
                insert_text: Some(label.to_string()),
                ..Default::default()
            });
        }

        // Types
        let types = [
            "string", "bool", "bytes",
            "u8", "u16", "u32", "u64",
            "i8", "i16", "i32", "i64",
            "f32", "f64",
        ];

        for type_name in types {
            items.push(CompletionItem {
                label: type_name.to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                detail: Some(format!("Built-in type: {}", type_name)),
                insert_text: Some(type_name.to_string()),
                ..Default::default()
            });
        }

        // Constraints
        let constraints = [
            ("primary_key", "Mark as primary key"),
            ("unique", "Mark as unique"),
            ("max_length($1)", "Set maximum length"),
            ("default($1)", "Set default value"),
            ("range($1, $2)", "Set value range"),
            ("regex(\"$1\")", "Set regex pattern"),
            ("foreign_key($1)", "Reference another table"),
        ];

        for (label, detail) in constraints {
            items.push(CompletionItem {
                label: label.replace("($1)", "").replace("($1, $2)", "").replace("(\"$1\")", ""),
                kind: Some(CompletionItemKind::PROPERTY),
                detail: Some(detail.to_string()),
                insert_text: Some(label.to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            });
        }

        // Annotations
        let annotations = [
            ("@load", "Specify data loading source"),
            ("@cache", "Enable caching"),
            ("@readonly", "Mark as read-only"),
            ("@taggable", "Enable tagging"),
            ("@datasource", "Specify database source"),
        ];

        for (label, detail) in annotations {
            items.push(CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some(detail.to_string()),
                insert_text: Some(label.to_string()),
                ..Default::default()
            });
        }

        items
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
        self.documents
            .write()
            .await
            .remove(&params.text_document.uri);
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

        let items = self.get_completions(&content, position);
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

            // Provide hover info for keywords and types
            let hover_text = match word.as_str() {
                "namespace" => Some("**namespace**\n\nDefines a namespace to organize types.\n\n```poly\nnamespace game.data {\n    table Player { }\n}\n```"),
                "table" => Some("**table**\n\nDefines a data structure (class/struct).\n\n```poly\ntable Player {\n    id: u32 primary_key;\n    name: string;\n}\n```"),
                "enum" => Some("**enum**\n\nDefines an enumeration type.\n\n```poly\nenum Status {\n    Active = 1\n    Inactive = 2\n}\n```"),
                "embed" => Some("**embed**\n\nDefines a reusable field group.\n\n```poly\nembed Address {\n    street: string;\n    city: string;\n}\n```"),
                "string" => Some("**string**\n\nVariable-length text type."),
                "bool" => Some("**bool**\n\nBoolean type (true/false)."),
                "u8" | "u16" | "u32" | "u64" => Some("**unsigned integer**\n\nUnsigned integer type."),
                "i8" | "i16" | "i32" | "i64" => Some("**signed integer**\n\nSigned integer type."),
                "f32" | "f64" => Some("**floating point**\n\nFloating-point number type."),
                "primary_key" => Some("**primary_key**\n\nMarks this field as the primary key."),
                "unique" => Some("**unique**\n\nEnsures values in this field are unique."),
                "max_length" => Some("**max_length(n)**\n\nSets maximum string length.\n\n```poly\nname: string max_length(100);\n```"),
                "default" => Some("**default(value)**\n\nSets a default value.\n\n```poly\nlevel: u16 default(1);\n```"),
                "range" => Some("**range(min, max)**\n\nSets allowed value range.\n\n```poly\nhp: u32 range(0, 9999);\n```"),
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
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
