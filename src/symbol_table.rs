//! Symbol table for "Go to Definition" support.
//!
//! This module extracts symbol definitions and type references from .poly files
//! using the Pest parser, enabling navigation from type references to their definitions.

use pest::Parser;
use crate::{Polygen, Rule};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use serde::Serialize;

/// Represents a source location span (1-indexed line/column)
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Span {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl Span {
    pub fn new(start_line: usize, start_col: usize, end_line: usize, end_col: usize) -> Self {
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    /// Check if a position (1-indexed) is within this span
    pub fn contains(&self, line: usize, col: usize) -> bool {
        if line < self.start_line || line > self.end_line {
            return false;
        }
        if line == self.start_line && col < self.start_col {
            return false;
        }
        if line == self.end_line && col > self.end_col {
            return false;
        }
        true
    }
}

/// Kind of symbol definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum DefinitionKind {
    Namespace,
    Table,
    Enum,
    Embed,
    Field,
}

/// Information about a symbol definition
#[derive(Debug, Clone, Serialize)]
pub struct DefinitionInfo {
    /// Fully qualified name (e.g., "game.character.Player")
    pub fqn: String,
    /// Simple name (e.g., "Player")
    pub name: String,
    /// Kind of definition
    pub kind: DefinitionKind,
    /// Span of the name identifier (for "Go to Definition" target)
    pub name_span: Span,
    /// Source file path (for cross-file navigation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

/// A reference to a type in the source code
#[derive(Debug, Clone, Serialize)]
pub struct TypeReference {
    /// The path as written in source (e.g., "Player", "game.common.Status")
    pub path: String,
    /// Location of this reference in source
    pub span: Span,
    /// Namespace context where this reference appears
    pub context_namespace: String,
    /// Resolved FQN (filled in during resolution)
    pub resolved_fqn: Option<String>,
}

/// Symbol table containing definitions and references for a document
#[derive(Debug, Default)]
pub struct SymbolTable {
    /// Map from FQN to definition info
    pub definitions: HashMap<String, DefinitionInfo>,
    /// All type references found in the document
    pub references: Vec<TypeReference>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Find definition at a given position
    pub fn definition_at(&self, line: usize, col: usize) -> Option<&DefinitionInfo> {
        self.definitions
            .values()
            .find(|def| def.name_span.contains(line, col))
    }

    /// Find type reference at a given position
    pub fn reference_at(&self, line: usize, col: usize) -> Option<&TypeReference> {
        self.references.iter().find(|r| r.span.contains(line, col))
    }

    /// Get definition by FQN
    pub fn get_definition(&self, fqn: &str) -> Option<&DefinitionInfo> {
        self.definitions.get(fqn)
    }

    /// Find all references to a given FQN
    pub fn find_references(&self, fqn: &str) -> Vec<&TypeReference> {
        self.references
            .iter()
            .filter(|r| r.resolved_fqn.as_deref() == Some(fqn))
            .collect()
    }

    /// Find the definition at a position and return all its references (including the definition itself)
    pub fn find_all_references_at(&self, line: usize, col: usize) -> Option<(Option<&DefinitionInfo>, Vec<&TypeReference>)> {
        // First, check if we're on a definition
        if let Some(def) = self.definition_at(line, col) {
            let refs = self.find_references(&def.fqn);
            return Some((Some(def), refs));
        }

        // Otherwise, check if we're on a reference
        if let Some(reference) = self.reference_at(line, col) {
            if let Some(fqn) = &reference.resolved_fqn {
                let def = self.get_definition(fqn);
                let refs = self.find_references(fqn);
                return Some((def, refs));
            }
        }

        None
    }
}

/// Build a symbol table from source content
pub fn build_symbol_table(content: &str) -> Result<SymbolTable, String> {
    let content = content.replace("\r\n", "\n");
    let pairs = Polygen::parse(Rule::main, &content).map_err(|e| e.to_string())?;

    let mut table = SymbolTable::new();
    let mut collector = SymbolCollector::new(&mut table, None);

    for pair in pairs {
        collector.visit_pair(pair, "");
    }

    // Resolve type references
    resolve_references(&mut table);

    Ok(table)
}

/// Parse import statements from content and return relative paths
pub fn parse_import_paths(content: &str) -> Vec<String> {
    let mut imports = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("import ") {
            // Parse: import "path/to/file.poly";
            if let Some(start) = trimmed.find('"') {
                if let Some(end) = trimmed.rfind('"') {
                    if start < end {
                        let import_path = &trimmed[start + 1..end];
                        imports.push(import_path.to_string());
                    }
                }
            }
        }
    }

    imports
}

/// Build a symbol table from source content, including definitions from imported files.
///
/// This function:
/// 1. Parses import statements from the content
/// 2. Recursively loads and parses imported files
/// 3. Merges definitions from imported files (for reference resolution only)
/// 4. Returns references only from the main file (not imported files)
pub fn build_symbol_table_with_imports(
    content: &str,
    file_path: Option<&Path>,
) -> Result<SymbolTable, String> {
    let mut visited = HashSet::new();
    build_symbol_table_recursive(content, file_path, &mut visited, true)
}

fn build_symbol_table_recursive(
    content: &str,
    file_path: Option<&Path>,
    visited: &mut HashSet<std::path::PathBuf>,
    is_main_file: bool,
) -> Result<SymbolTable, String> {
    // Mark current file as visited (if path is available)
    if let Some(path) = file_path {
        if let Ok(canonical) = path.canonicalize() {
            if visited.contains(&canonical) {
                // Already processed this file, return empty table
                return Ok(SymbolTable::new());
            }
            visited.insert(canonical);
        }
    }

    // Build symbol table for current content
    let content_normalized = content.replace("\r\n", "\n");
    let pairs = Polygen::parse(Rule::main, &content_normalized).map_err(|e| e.to_string())?;

    let mut table = SymbolTable::new();
    let file_path_str = file_path.and_then(|p| p.to_str()).map(|s| s.to_string());
    let mut collector = SymbolCollector::new(&mut table, file_path_str);

    for pair in pairs {
        collector.visit_pair(pair, "");
    }

    // Process imports
    if let Some(base_path) = file_path {
        if let Some(base_dir) = base_path.parent() {
            let import_paths = parse_import_paths(&content_normalized);

            for import_path in import_paths {
                let imported_file = base_dir.join(&import_path);

                if imported_file.exists() {
                    if let Ok(imported_content) = std::fs::read_to_string(&imported_file) {
                        // Recursively build symbol table for imported file
                        if let Ok(imported_table) = build_symbol_table_recursive(
                            &imported_content,
                            Some(&imported_file),
                            visited,
                            false, // Not the main file
                        ) {
                            // Merge definitions from imported file (for resolution)
                            for (fqn, def) in imported_table.definitions {
                                table.definitions.entry(fqn).or_insert(def);
                            }

                            // Only keep references from main file
                            // (imported file references are already resolved within that file)
                        }
                    }
                }
            }
        }
    }

    // Resolve type references (now with imported definitions available)
    resolve_references(&mut table);

    // If not main file, clear references (we only care about definitions for imports)
    if !is_main_file {
        table.references.clear();
    }

    Ok(table)
}

/// Internal collector that walks the parse tree
struct SymbolCollector<'a> {
    table: &'a mut SymbolTable,
    file_path: Option<String>,
}

impl<'a> SymbolCollector<'a> {
    fn new(table: &'a mut SymbolTable, file_path: Option<String>) -> Self {
        Self { table, file_path }
    }

    fn visit_pair(&mut self, pair: pest::iterators::Pair<'_, Rule>, current_namespace: &str) {
        match pair.as_rule() {
            Rule::main => {
                for inner in pair.into_inner() {
                    self.visit_pair(inner, current_namespace);
                }
            }
            Rule::toplevel_item | Rule::definition | Rule::namespace_body_item => {
                for inner in pair.into_inner() {
                    self.visit_pair(inner, current_namespace);
                }
            }
            Rule::namespace => {
                self.visit_namespace(pair, current_namespace);
            }
            Rule::table => {
                self.visit_table(pair, current_namespace);
            }
            Rule::enum_def => {
                self.visit_enum(pair, current_namespace);
            }
            Rule::embed_def => {
                self.visit_embed(pair, current_namespace);
            }
            _ => {}
        }
    }

    fn visit_namespace(
        &mut self,
        pair: pest::iterators::Pair<'_, Rule>,
        parent_namespace: &str,
    ) {
        let mut namespace_path = String::new();
        let mut name_span = None;

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::path => {
                    let (start_line, start_col) = inner.line_col();
                    // Extract path by collecting IDENT tokens (to avoid whitespace issues)
                    let path_parts: Vec<&str> = inner
                        .clone()
                        .into_inner()
                        .filter(|p| p.as_rule() == Rule::IDENT)
                        .map(|p| p.as_str())
                        .collect();
                    let path_str = path_parts.join(".");
                    let end_col = start_col + path_str.len();

                    namespace_path = path_str;
                    name_span = Some(Span::new(start_line, start_col, start_line, end_col));
                }
                Rule::namespace_body_item | Rule::definition => {
                    let full_ns = if parent_namespace.is_empty() {
                        namespace_path.clone()
                    } else {
                        format!("{}.{}", parent_namespace, namespace_path)
                    };
                    self.visit_pair(inner, &full_ns);
                }
                _ => {}
            }
        }

        // Register the namespace definition
        if let Some(span) = name_span {
            let fqn = if parent_namespace.is_empty() {
                namespace_path.clone()
            } else {
                format!("{}.{}", parent_namespace, namespace_path)
            };

            // Extract simple name (last component)
            let simple_name = namespace_path
                .rsplit('.')
                .next()
                .unwrap_or(&namespace_path)
                .to_string();

            self.table.definitions.insert(
                fqn.clone(),
                DefinitionInfo {
                    fqn,
                    name: simple_name,
                    kind: DefinitionKind::Namespace,
                    name_span: span,
                    file_path: self.file_path.clone(),
                },
            );
        }
    }

    fn visit_table(&mut self, pair: pest::iterators::Pair<'_, Rule>, current_namespace: &str) {
        let mut name = String::new();
        let mut name_span = None;
        let mut table_fqn = String::new();

        // First pass: get the table name and FQN
        for inner in pair.clone().into_inner() {
            if inner.as_rule() == Rule::IDENT {
                let (start_line, start_col) = inner.line_col();
                name = inner.as_str().to_string();
                let end_col = start_col + name.len();
                name_span = Some(Span::new(start_line, start_col, start_line, end_col));

                table_fqn = if current_namespace.is_empty() {
                    name.clone()
                } else {
                    format!("{}.{}", current_namespace, name)
                };
                break;
            }
        }

        // Second pass: visit members with the table FQN
        for inner in pair.clone().into_inner() {
            if inner.as_rule() == Rule::table_member {
                self.visit_table_member(inner, current_namespace, &table_fqn);
            }
        }

        if let Some(span) = name_span {
            self.table.definitions.insert(
                table_fqn.clone(),
                DefinitionInfo {
                    fqn: table_fqn,
                    name,
                    kind: DefinitionKind::Table,
                    name_span: span,
                    file_path: self.file_path.clone(),
                },
            );
        }
    }

    fn visit_enum(&mut self, pair: pest::iterators::Pair<'_, Rule>, current_namespace: &str) {
        let mut name = String::new();
        let mut name_span = None;

        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::IDENT {
                let (start_line, start_col) = inner.line_col();
                name = inner.as_str().to_string();
                let end_col = start_col + name.len();
                name_span = Some(Span::new(start_line, start_col, start_line, end_col));
                break;
            }
        }

        if let Some(span) = name_span {
            let fqn = if current_namespace.is_empty() {
                name.clone()
            } else {
                format!("{}.{}", current_namespace, name)
            };

            self.table.definitions.insert(
                fqn.clone(),
                DefinitionInfo {
                    fqn,
                    name,
                    kind: DefinitionKind::Enum,
                    name_span: span,
                    file_path: self.file_path.clone(),
                },
            );
        }
    }

    fn visit_embed(&mut self, pair: pest::iterators::Pair<'_, Rule>, current_namespace: &str) {
        let mut name = String::new();
        let mut name_span = None;
        let mut embed_fqn = String::new();

        // First pass: get the embed name and FQN
        for inner in pair.clone().into_inner() {
            if inner.as_rule() == Rule::IDENT {
                let (start_line, start_col) = inner.line_col();
                name = inner.as_str().to_string();
                let end_col = start_col + name.len();
                name_span = Some(Span::new(start_line, start_col, start_line, end_col));

                embed_fqn = if current_namespace.is_empty() {
                    name.clone()
                } else {
                    format!("{}.{}", current_namespace, name)
                };
                break;
            }
        }

        // Second pass: visit members with the embed FQN
        for inner in pair.clone().into_inner() {
            if inner.as_rule() == Rule::table_member {
                self.visit_table_member(inner, current_namespace, &embed_fqn);
            }
        }

        if let Some(span) = name_span {
            self.table.definitions.insert(
                embed_fqn.clone(),
                DefinitionInfo {
                    fqn: embed_fqn,
                    name,
                    kind: DefinitionKind::Embed,
                    name_span: span,
                    file_path: self.file_path.clone(),
                },
            );
        }
    }

    fn visit_table_member(
        &mut self,
        pair: pest::iterators::Pair<'_, Rule>,
        current_namespace: &str,
        parent_fqn: &str,
    ) {
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::field_definition => {
                    self.visit_field_definition(inner, current_namespace, parent_fqn);
                }
                Rule::enum_def => {
                    self.visit_enum(inner, current_namespace);
                }
                Rule::embed_def => {
                    self.visit_embed(inner, current_namespace);
                }
                _ => {}
            }
        }
    }

    fn visit_field_definition(
        &mut self,
        pair: pest::iterators::Pair<'_, Rule>,
        current_namespace: &str,
        parent_fqn: &str,
    ) {
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::regular_field => {
                    self.visit_regular_field(inner, current_namespace, parent_fqn);
                }
                Rule::inline_embed_field | Rule::inline_enum_field => {
                    // Inline definitions don't create named references
                }
                _ => {}
            }
        }
    }

    fn visit_regular_field(
        &mut self,
        pair: pest::iterators::Pair<'_, Rule>,
        current_namespace: &str,
        parent_fqn: &str,
    ) {
        let mut field_name = String::new();
        let mut field_span = None;

        for inner in pair.clone().into_inner() {
            match inner.as_rule() {
                Rule::IDENT => {
                    // First IDENT is the field name
                    if field_name.is_empty() {
                        let (start_line, start_col) = inner.line_col();
                        field_name = inner.as_str().to_string();
                        let end_col = start_col + field_name.len();
                        field_span = Some(Span::new(start_line, start_col, start_line, end_col));
                    }
                }
                Rule::type_with_cardinality => {
                    self.visit_type_with_cardinality(inner, current_namespace);
                }
                Rule::constraint => {
                    self.visit_constraint(inner, current_namespace);
                }
                _ => {}
            }
        }

        // Register field definition
        if let Some(span) = field_span {
            if !parent_fqn.is_empty() {
                let field_fqn = format!("{}.{}", parent_fqn, field_name);
                self.table.definitions.insert(
                    field_fqn.clone(),
                    DefinitionInfo {
                        fqn: field_fqn,
                        name: field_name,
                        kind: DefinitionKind::Field,
                        name_span: span,
                        file_path: self.file_path.clone(),
                    },
                );
            }
        }
    }

    fn visit_type_with_cardinality(
        &mut self,
        pair: pest::iterators::Pair<'_, Rule>,
        current_namespace: &str,
    ) {
        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::type_name {
                self.visit_type_name(inner, current_namespace);
            }
        }
    }

    fn visit_type_name(&mut self, pair: pest::iterators::Pair<'_, Rule>, current_namespace: &str) {
        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::path => {
                    // This is a user-defined type reference
                    let (start_line, start_col) = inner.line_col();
                    // Extract path by collecting IDENT tokens (to avoid whitespace issues)
                    let path_parts: Vec<&str> = inner
                        .clone()
                        .into_inner()
                        .filter(|p| p.as_rule() == Rule::IDENT)
                        .map(|p| p.as_str())
                        .collect();
                    let path_str = path_parts.join(".");
                    let end_col = start_col + path_str.len();

                    self.table.references.push(TypeReference {
                        path: path_str,
                        span: Span::new(start_line, start_col, start_line, end_col),
                        context_namespace: current_namespace.to_string(),
                        resolved_fqn: None,
                    });
                }
                Rule::basic_type | Rule::anonymous_enum_def => {
                    // Built-in types and anonymous enums don't need resolution
                }
                _ => {}
            }
        }
    }

    fn visit_constraint(
        &mut self,
        pair: pest::iterators::Pair<'_, Rule>,
        current_namespace: &str,
    ) {
        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::foreign_key_val {
                self.visit_foreign_key(inner, current_namespace);
            }
        }
    }

    fn visit_foreign_key(
        &mut self,
        pair: pest::iterators::Pair<'_, Rule>,
        current_namespace: &str,
    ) {
        for inner in pair.into_inner() {
            if inner.as_rule() == Rule::path {
                // Collect IDENT tokens with their positions
                let idents: Vec<_> = inner
                    .clone()
                    .into_inner()
                    .filter(|p| p.as_rule() == Rule::IDENT)
                    .map(|p| {
                        let (line, col) = p.line_col();
                        let name = p.as_str().to_string();
                        let end_col = col + name.len();
                        (name, Span::new(line, col, line, end_col))
                    })
                    .collect();

                if idents.len() < 2 {
                    continue;
                }

                // For foreign_key(game.character.Player.id):
                // - Table path: game.character.Player (all but last)
                // - Field name: id (last part)

                // Build table path from all parts except the last
                let table_parts: Vec<&str> = idents[..idents.len() - 1]
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect();
                let table_path = table_parts.join(".");

                // Get the span for the table reference (from first to second-to-last ident)
                let table_start_span = &idents[0].1;
                let table_end_span = &idents[idents.len() - 2].1;
                let table_span = Span::new(
                    table_start_span.start_line,
                    table_start_span.start_col,
                    table_end_span.end_line,
                    table_end_span.end_col,
                );

                // Add table reference
                self.table.references.push(TypeReference {
                    path: table_path.clone(),
                    span: table_span,
                    context_namespace: current_namespace.to_string(),
                    resolved_fqn: None,
                });

                // Add field reference (table.field for resolution)
                let (field_name, field_span) = &idents[idents.len() - 1];
                let field_path = format!("{}.{}", table_path, field_name);
                self.table.references.push(TypeReference {
                    path: field_path,
                    span: field_span.clone(),
                    context_namespace: current_namespace.to_string(),
                    resolved_fqn: None,
                });
            }
        }
    }
}

/// Resolve type references to their definitions
fn resolve_references(table: &mut SymbolTable) {
    // Collect all definition FQNs and simple names for lookup
    let definitions: Vec<(String, String)> = table
        .definitions
        .values()
        .map(|d| (d.fqn.clone(), d.name.clone()))
        .collect();

    for reference in &mut table.references {
        reference.resolved_fqn = resolve_single_reference(&reference.path, &reference.context_namespace, &definitions);
    }
}

/// Resolve a single type reference
fn resolve_single_reference(
    path: &str,
    context_namespace: &str,
    definitions: &[(String, String)],
) -> Option<String> {
    // 1. If path contains dots, try as FQN first
    if path.contains('.') && definitions.iter().any(|(fqn, _)| fqn == path) {
        return Some(path.to_string());
    }

    // 2. Try qualified with current namespace
    if !context_namespace.is_empty() {
        let qualified = format!("{}.{}", context_namespace, path);
        if definitions.iter().any(|(fqn, _)| fqn == &qualified) {
            return Some(qualified);
        }

        // Try parent namespaces (walk up the hierarchy)
        let mut ns_parts: Vec<&str> = context_namespace.split('.').collect();
        while !ns_parts.is_empty() {
            ns_parts.pop();
            let parent_ns = ns_parts.join(".");
            let qualified = if parent_ns.is_empty() {
                path.to_string()
            } else {
                format!("{}.{}", parent_ns, path)
            };
            if definitions.iter().any(|(fqn, _)| fqn == &qualified) {
                return Some(qualified);
            }
        }
    }

    // 3. Try as simple name (unique match)
    let matches: Vec<&String> = definitions
        .iter()
        .filter(|(_, name)| name == path)
        .map(|(fqn, _)| fqn)
        .collect();

    if matches.len() == 1 {
        return Some(matches[0].clone());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_contains() {
        let span = Span::new(1, 5, 1, 10);
        assert!(span.contains(1, 5));
        assert!(span.contains(1, 7));
        assert!(span.contains(1, 10));
        assert!(!span.contains(1, 4));
        assert!(!span.contains(1, 11));
        assert!(!span.contains(2, 5));
    }

    #[test]
    fn test_simple_table_definition() {
        let content = r#"
namespace game {
    table Player {
        id: u32;
    }
}
"#;
        let table = build_symbol_table(content).unwrap();

        assert!(table.definitions.contains_key("game"));
        assert!(table.definitions.contains_key("game.Player"));

        let player = table.get_definition("game.Player").unwrap();
        assert_eq!(player.name, "Player");
        assert_eq!(player.kind, DefinitionKind::Table);
    }

    #[test]
    fn test_enum_definition() {
        let content = r#"
namespace game {
    enum Status {
        Active = 1;
        Inactive = 2;
    }
}
"#;
        let table = build_symbol_table(content).unwrap();

        assert!(table.definitions.contains_key("game.Status"));
        let status = table.get_definition("game.Status").unwrap();
        assert_eq!(status.name, "Status");
        assert_eq!(status.kind, DefinitionKind::Enum);
    }

    #[test]
    fn test_type_reference_collection() {
        let content = r#"
namespace game {
    enum Status {
        Active = 1;
    }

    table Player {
        id: u32;
        status: Status;
    }
}
"#;
        let table = build_symbol_table(content).unwrap();

        // Should have one type reference to Status
        assert_eq!(table.references.len(), 1);
        let reference = &table.references[0];
        assert_eq!(reference.path, "Status");
        assert_eq!(reference.context_namespace, "game");
        assert_eq!(reference.resolved_fqn, Some("game.Status".to_string()));
    }

    #[test]
    fn test_cross_namespace_reference() {
        let content = r#"
namespace game.common {
    embed Position {
        x: f32;
        y: f32;
    }
}

namespace game.entity {
    table Player {
        id: u32;
        pos: game.common.Position;
    }
}
"#;
        let table = build_symbol_table(content).unwrap();

        // Find the reference to Position
        let pos_ref = table
            .references
            .iter()
            .find(|r| r.path.contains("Position"))
            .unwrap();

        assert_eq!(pos_ref.resolved_fqn, Some("game.common.Position".to_string()));
    }

    #[test]
    fn test_nested_namespace() {
        let content = r#"
namespace game {
    namespace character {
        table Hero {
            id: u32;
        }
    }
}
"#;
        let table = build_symbol_table(content).unwrap();

        assert!(table.definitions.contains_key("game"));
        assert!(table.definitions.contains_key("game.character"));
        assert!(table.definitions.contains_key("game.character.Hero"));
    }

    #[test]
    fn test_field_definitions() {
        let content = r#"
namespace game {
    table Player {
        id: u32 primary_key;
        name: string;
    }
}
"#;
        let table = build_symbol_table(content).unwrap();

        // Fields should be registered with their FQN
        assert!(table.definitions.contains_key("game.Player.id"));
        assert!(table.definitions.contains_key("game.Player.name"));

        let id_field = table.get_definition("game.Player.id").unwrap();
        assert_eq!(id_field.name, "id");
        assert_eq!(id_field.kind, DefinitionKind::Field);
    }

    #[test]
    fn test_foreign_key_field_reference() {
        let content = r#"
namespace game {
    table Item {
        id: u32 primary_key;
        name: string;
    }

    table Inventory {
        id: u32 primary_key;
        item_id: u32 foreign_key(Item.id);
    }
}
"#;
        let table = build_symbol_table(content).unwrap();

        // Should have references to both Item (table) and Item.id (field)
        let item_ref = table.references.iter().find(|r| r.path == "Item").unwrap();
        assert_eq!(item_ref.resolved_fqn, Some("game.Item".to_string()));

        let field_ref = table.references.iter().find(|r| r.path == "Item.id").unwrap();
        assert_eq!(field_ref.resolved_fqn, Some("game.Item.id".to_string()));

        // The field definition should exist
        assert!(table.definitions.contains_key("game.Item.id"));
    }

    #[test]
    fn test_foreign_key_full_path() {
        let content = r#"
namespace game.character {
    table Player {
        id: u32 primary_key;
        name: string;
    }
}

namespace game.junction {
    table PlayerSkill {
        player_id: u32 foreign_key(game.character.Player.id);
        skill_id: u32;
    }
}
"#;
        let table = build_symbol_table(content).unwrap();

        // Should have reference to game.character.Player (table path)
        let table_ref = table.references.iter().find(|r| r.path == "game.character.Player");
        assert!(table_ref.is_some(), "Should have table reference");
        assert_eq!(table_ref.unwrap().resolved_fqn, Some("game.character.Player".to_string()));

        // Should have reference to game.character.Player.id (field path)
        let field_ref = table.references.iter().find(|r| r.path == "game.character.Player.id");
        assert!(field_ref.is_some(), "Should have field reference");
        assert_eq!(field_ref.unwrap().resolved_fqn, Some("game.character.Player.id".to_string()));
    }
}
