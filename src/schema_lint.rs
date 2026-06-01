//! Lightweight schema lint checks.

use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::ast_model::{
    AstRoot, Constraint, Definition, Embed, FieldDefinition, InlineEmbedField, Table, TableMember,
    TypeName,
};
use crate::type_registry::{TypeKind, TypeRegistry};

/// Lint report for a schema graph.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct SchemaLintReport {
    /// Warnings produced by lint checks.
    pub warnings: Vec<SchemaLintWarning>,
}

impl SchemaLintReport {
    /// Returns true if there are no lint warnings.
    pub fn is_empty(&self) -> bool {
        self.warnings.is_empty()
    }
}

/// A single schema lint warning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
#[serde(rename_all = "snake_case")]
pub enum SchemaLintWarning {
    /// A file-level import is not used by any type or foreign-key reference in the source file.
    UnusedFileImport {
        /// Source schema file that contains the import.
        source_path: String,
        /// Import path exactly as written in the source schema.
        import_path: String,
        /// Resolved import path, if it can be matched to a parsed AST.
        resolved_path: String,
    },
    /// A value-type table/embed reference graph contains a cycle.
    CircularTypeReference {
        /// Cycle path, with the first type repeated as the final item.
        cycle: Vec<String>,
    },
}

/// Run lightweight lint checks over parsed AST roots.
pub fn lint_asts(asts: &[AstRoot]) -> SchemaLintReport {
    let mut warnings = find_unused_file_imports(asts);
    warnings.extend(find_circular_type_references(asts));
    warnings.sort_by_key(warning_sort_key);
    SchemaLintReport { warnings }
}

/// Render lint warnings as stable text.
pub fn render_lint_text(report: &SchemaLintReport) -> String {
    let mut output = String::new();
    output.push_str("Schema Lint\n");
    output.push_str("===========\n\n");
    output.push_str(&format!("Warnings: {}\n", report.warnings.len()));

    if report.warnings.is_empty() {
        output.push_str("\nNo warnings.\n");
    } else {
        output.push_str("\nWarnings\n");
        output.push_str("--------\n");
        for warning in &report.warnings {
            output.push_str(&format!("- {}\n", format_lint_warning(warning)));
        }
    }

    output
}

/// Find unused file-level imports.
pub fn find_unused_file_imports(asts: &[AstRoot]) -> Vec<SchemaLintWarning> {
    let asts_by_path: HashMap<PathBuf, &AstRoot> = asts
        .iter()
        .map(|ast| (normalize_path(&ast.path), ast))
        .collect();

    let mut warnings = Vec::new();
    for ast in asts {
        let referenced_types = collect_referenced_types(ast);
        let base_dir = ast.path.parent().unwrap_or_else(|| Path::new(""));

        for import_path in &ast.file_imports {
            if import_path.ends_with(".renames") {
                continue;
            }

            let resolved_path = normalize_path(&base_dir.join(import_path));
            let Some(imported_ast) = asts_by_path.get(&resolved_path) else {
                continue;
            };

            let imported_types = collect_defined_types(imported_ast);
            if imported_types.is_empty() || referenced_types.is_disjoint(&imported_types) {
                warnings.push(SchemaLintWarning::UnusedFileImport {
                    source_path: ast.path.display().to_string(),
                    import_path: import_path.clone(),
                    resolved_path: resolved_path.display().to_string(),
                });
            }
        }
    }

    warnings
}

/// Find cycles in table/embed value references.
///
/// Foreign keys are intentionally excluded because they model relational links through IDs,
/// not value containment.
pub fn find_circular_type_references(asts: &[AstRoot]) -> Vec<SchemaLintWarning> {
    let mut registry = TypeRegistry::new();
    let mut containers = HashMap::new();

    for ast in asts {
        collect_registry_and_containers(&ast.definitions, "", &mut registry, &mut containers);
    }

    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    for (fqn, container) in &containers {
        let mut edges = Vec::new();
        for type_path in &container.type_refs {
            if let Some(resolved) = resolve_type_path(type_path, &container.scope, &registry) {
                if matches!(
                    registry.get_kind(&resolved),
                    Some(TypeKind::Struct | TypeKind::Embed)
                ) {
                    edges.push(resolved);
                }
            }
        }
        edges.sort();
        edges.dedup();
        graph.insert(fqn.clone(), edges);
    }

    let mut warnings = Vec::new();
    let mut seen = HashSet::new();
    let mut nodes: Vec<_> = graph.keys().cloned().collect();
    nodes.sort();

    for node in nodes {
        let mut path = Vec::new();
        find_cycles_from(&node, &graph, &mut path, &mut seen, &mut warnings);
    }

    warnings
}

#[derive(Debug, Default)]
struct ContainerRefs {
    scope: Vec<String>,
    type_refs: Vec<Vec<String>>,
}

fn collect_registry_and_containers(
    definitions: &[Definition],
    namespace: &str,
    registry: &mut TypeRegistry,
    containers: &mut HashMap<String, ContainerRefs>,
) {
    for definition in definitions {
        match definition {
            Definition::Namespace(ns) => {
                let ns_name = ns.path.join(".");
                let nested = if namespace.is_empty() {
                    ns_name
                } else {
                    format!("{}.{}", namespace, ns_name)
                };
                collect_registry_and_containers(&ns.definitions, &nested, registry, containers);
            }
            Definition::Table(table) => {
                if let Some(name) = &table.name {
                    let fqn = qualify(namespace, name);
                    registry.register(&fqn, TypeKind::Struct);
                    collect_container(&fqn, &table.members, TypeKind::Struct, registry, containers);
                }
            }
            Definition::Enum(e) => {
                if let Some(name) = &e.name {
                    registry.register(&qualify(namespace, name), TypeKind::Enum);
                }
            }
            Definition::Embed(embed) => {
                if let Some(name) = &embed.name {
                    let fqn = qualify(namespace, name);
                    registry.register(&fqn, TypeKind::Embed);
                    collect_container(&fqn, &embed.members, TypeKind::Embed, registry, containers);
                }
            }
            Definition::Comment(_) | Definition::Annotation(_) => {}
        }
    }
}

fn collect_container(
    fqn: &str,
    members: &[TableMember],
    kind: TypeKind,
    registry: &mut TypeRegistry,
    containers: &mut HashMap<String, ContainerRefs>,
) {
    registry.register(fqn, kind);
    let mut refs = ContainerRefs {
        scope: split_fqn(fqn),
        type_refs: Vec::new(),
    };
    collect_value_type_refs(members, &mut refs.type_refs);
    containers.insert(fqn.to_string(), refs);

    for member in members {
        match member {
            TableMember::Embed(embed) => {
                if let Some(name) = &embed.name {
                    let nested_fqn = qualify(fqn, name);
                    registry.register(&nested_fqn, TypeKind::Embed);
                    collect_container(
                        &nested_fqn,
                        &embed.members,
                        TypeKind::Embed,
                        registry,
                        containers,
                    );
                }
            }
            TableMember::Field(FieldDefinition::InlineEmbed(field)) => {
                collect_inline_embed_container(fqn, field, registry, containers);
            }
            TableMember::Enum(e) => {
                if let Some(name) = &e.name {
                    registry.register(&qualify(fqn, name), TypeKind::Enum);
                }
            }
            TableMember::Field(_) | TableMember::Comment(_) => {}
        }
    }
}

fn collect_inline_embed_container(
    owner_fqn: &str,
    field: &InlineEmbedField,
    registry: &mut TypeRegistry,
    containers: &mut HashMap<String, ContainerRefs>,
) {
    if let Some(name) = &field.name {
        let fqn = qualify(owner_fqn, name);
        registry.register(&fqn, TypeKind::Embed);
        collect_container(&fqn, &field.members, TypeKind::Embed, registry, containers);
    }
}

fn collect_value_type_refs(members: &[TableMember], refs: &mut Vec<Vec<String>>) {
    for member in members {
        match member {
            TableMember::Field(FieldDefinition::Regular(field)) => {
                if let TypeName::Path(path) = &field.field_type.base_type {
                    refs.push(path.clone());
                }
            }
            TableMember::Field(FieldDefinition::InlineEmbed(field)) => {
                collect_value_type_refs(&field.members, refs);
            }
            TableMember::Embed(embed) => collect_value_type_refs(&embed.members, refs),
            TableMember::Field(FieldDefinition::InlineEnum(_))
            | TableMember::Enum(_)
            | TableMember::Comment(_) => {}
        }
    }
}

fn resolve_type_path(
    type_path: &[String],
    current_scope: &[String],
    registry: &TypeRegistry,
) -> Option<String> {
    let used_type = type_path.join(".");
    let current_namespace = current_scope.join(".");

    if let Some(fqn) = registry.resolve(&used_type, &current_namespace) {
        return Some(fqn.to_string());
    }

    let mut scope = current_scope.to_vec();
    loop {
        let mut potential_fqn_parts = scope.clone();
        potential_fqn_parts.extend_from_slice(type_path);
        let potential_fqn = potential_fqn_parts.join(".");
        if registry.contains(&potential_fqn) {
            return Some(potential_fqn);
        }

        if scope.is_empty() {
            break;
        }
        scope.pop();
    }

    None
}

fn find_cycles_from(
    node: &str,
    graph: &HashMap<String, Vec<String>>,
    path: &mut Vec<String>,
    seen: &mut HashSet<String>,
    warnings: &mut Vec<SchemaLintWarning>,
) {
    if let Some(index) = path.iter().position(|entry| entry == node) {
        let mut cycle = path[index..].to_vec();
        cycle.push(node.to_string());
        let key = canonical_cycle_key(&cycle);
        if seen.insert(key) {
            warnings.push(SchemaLintWarning::CircularTypeReference { cycle });
        }
        return;
    }

    path.push(node.to_string());
    if let Some(neighbors) = graph.get(node) {
        for neighbor in neighbors {
            find_cycles_from(neighbor, graph, path, seen, warnings);
        }
    }
    path.pop();
}

fn canonical_cycle_key(cycle: &[String]) -> String {
    if cycle.len() <= 1 {
        return String::new();
    }

    let nodes = &cycle[..cycle.len() - 1];
    let mut rotations = Vec::new();
    for index in 0..nodes.len() {
        let mut rotated = Vec::new();
        rotated.extend_from_slice(&nodes[index..]);
        rotated.extend_from_slice(&nodes[..index]);
        rotations.push(rotated.join("->"));
    }

    rotations.into_iter().min().unwrap_or_default()
}

fn collect_defined_types(ast: &AstRoot) -> HashSet<String> {
    let mut types = HashSet::new();
    collect_defined_types_from_definitions(&ast.definitions, "", &mut types);
    types
}

fn collect_defined_types_from_definitions(
    definitions: &[Definition],
    namespace: &str,
    types: &mut HashSet<String>,
) {
    for definition in definitions {
        match definition {
            Definition::Namespace(ns) => {
                let ns_name = ns.path.join(".");
                let nested = if namespace.is_empty() {
                    ns_name
                } else {
                    format!("{}.{}", namespace, ns_name)
                };
                collect_defined_types_from_definitions(&ns.definitions, &nested, types);
            }
            Definition::Table(table) => collect_table_defined_types(table, namespace, types),
            Definition::Enum(e) => {
                if let Some(name) = &e.name {
                    insert_type_keys(types, namespace, name);
                }
            }
            Definition::Embed(embed) => collect_embed_defined_types(embed, namespace, types),
            Definition::Comment(_) | Definition::Annotation(_) => {}
        }
    }
}

fn collect_table_defined_types(table: &Table, namespace: &str, types: &mut HashSet<String>) {
    if let Some(name) = &table.name {
        insert_type_keys(types, namespace, name);
        let owner = qualify(namespace, name);
        for member in &table.members {
            match member {
                TableMember::Embed(embed) => collect_embed_defined_types(embed, &owner, types),
                TableMember::Enum(e) => {
                    if let Some(enum_name) = &e.name {
                        insert_type_keys(types, &owner, enum_name);
                    }
                }
                TableMember::Field(_) | TableMember::Comment(_) => {}
            }
        }
    }
}

fn collect_embed_defined_types(embed: &Embed, namespace: &str, types: &mut HashSet<String>) {
    if let Some(name) = &embed.name {
        insert_type_keys(types, namespace, name);
        let owner = qualify(namespace, name);
        for member in &embed.members {
            match member {
                TableMember::Embed(nested) => collect_embed_defined_types(nested, &owner, types),
                TableMember::Enum(e) => {
                    if let Some(enum_name) = &e.name {
                        insert_type_keys(types, &owner, enum_name);
                    }
                }
                TableMember::Field(_) | TableMember::Comment(_) => {}
            }
        }
    }
}

fn collect_referenced_types(ast: &AstRoot) -> HashSet<String> {
    let mut refs = HashSet::new();
    collect_references_from_definitions(&ast.definitions, &mut refs);
    refs
}

fn collect_references_from_definitions(definitions: &[Definition], refs: &mut HashSet<String>) {
    for definition in definitions {
        match definition {
            Definition::Namespace(ns) => collect_references_from_definitions(&ns.definitions, refs),
            Definition::Table(table) => collect_references_from_members(&table.members, refs),
            Definition::Embed(embed) => collect_references_from_members(&embed.members, refs),
            Definition::Enum(_) | Definition::Comment(_) | Definition::Annotation(_) => {}
        }
    }
}

fn collect_references_from_members(members: &[TableMember], refs: &mut HashSet<String>) {
    for member in members {
        match member {
            TableMember::Field(FieldDefinition::Regular(field)) => {
                collect_type_name_reference(&field.field_type.base_type, refs);
                for constraint in &field.constraints {
                    if let Constraint::ForeignKey(path, _) = constraint {
                        insert_reference_path(path, refs);
                        if path.len() > 1 {
                            insert_reference_path(&path[..path.len() - 1], refs);
                        }
                    }
                }
            }
            TableMember::Field(FieldDefinition::InlineEmbed(field)) => {
                collect_references_from_members(&field.members, refs);
            }
            TableMember::Field(FieldDefinition::InlineEnum(_)) => {}
            TableMember::Embed(embed) => collect_references_from_members(&embed.members, refs),
            TableMember::Enum(_) | TableMember::Comment(_) => {}
        }
    }
}

fn collect_type_name_reference(type_name: &TypeName, refs: &mut HashSet<String>) {
    if let TypeName::Path(path) = type_name {
        insert_reference_path(path, refs);
    }
}

fn insert_reference_path(path: &[String], refs: &mut HashSet<String>) {
    if path.is_empty() {
        return;
    }

    refs.insert(path.join("."));
    if let Some(last) = path.last() {
        refs.insert(last.clone());
    }
}

fn insert_type_keys(types: &mut HashSet<String>, namespace: &str, name: &str) {
    types.insert(name.to_string());
    types.insert(qualify(namespace, name));
}

fn qualify(namespace: &str, name: &str) -> String {
    if namespace.is_empty() {
        name.to_string()
    } else {
        format!("{}.{}", namespace, name)
    }
}

fn split_fqn(fqn: &str) -> Vec<String> {
    fqn.split('.').map(String::from).collect()
}

fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn warning_sort_key(warning: &SchemaLintWarning) -> String {
    match warning {
        SchemaLintWarning::UnusedFileImport {
            source_path,
            import_path,
            resolved_path,
        } => format!("unused:{source_path}:{import_path}:{resolved_path}"),
        SchemaLintWarning::CircularTypeReference { cycle } => format!("cycle:{}", cycle.join("->")),
    }
}

/// Format a single lint warning for CLI output.
pub fn format_lint_warning(warning: &SchemaLintWarning) -> String {
    match warning {
        SchemaLintWarning::UnusedFileImport {
            source_path,
            import_path,
            ..
        } => format!("unused import in {}: {}", source_path, import_path),
        SchemaLintWarning::CircularTypeReference { cycle } => {
            format!("circular type reference: {}", cycle.join(" -> "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_and_merge_schemas;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_lint_unused_file_import_detects_unused_import() {
        let temp = TempDir::new().expect("temp dir");
        let main = temp.path().join("main.poly");
        let unused = temp.path().join("unused.poly");
        fs::write(
            &main,
            r#"
import "unused.poly";

namespace game {
    table Player {
        id: u32 primary_key;
    }
}
"#,
        )
        .expect("main schema");
        fs::write(
            &unused,
            r#"
namespace game.common {
    embed StatBlock {
        hp: u32;
    }
}
"#,
        )
        .expect("unused schema");

        let asts = parse_and_merge_schemas(&main, None).expect("parse schemas");
        let report = lint_asts(&asts);

        assert_eq!(report.warnings.len(), 1);
        assert!(matches!(
            report.warnings[0],
            SchemaLintWarning::UnusedFileImport { .. }
        ));
    }

    #[test]
    fn test_lint_unused_file_import_accepts_fqn_reference() {
        let temp = TempDir::new().expect("temp dir");
        let main = temp.path().join("main.poly");
        let common = temp.path().join("common.poly");
        fs::write(
            &main,
            r#"
import "common.poly";

namespace game {
    table Player {
        id: u32 primary_key;
        stats: game.common.StatBlock;
    }
}
"#,
        )
        .expect("main schema");
        fs::write(
            &common,
            r#"
namespace game.common {
    embed StatBlock {
        hp: u32;
    }
}
"#,
        )
        .expect("common schema");

        let asts = parse_and_merge_schemas(&main, None).expect("parse schemas");
        let report = lint_asts(&asts);

        assert!(report.is_empty(), "{report:#?}");
    }

    #[test]
    fn test_lint_circular_type_reference_detects_value_cycle() {
        let temp = TempDir::new().expect("temp dir");
        let main = temp.path().join("main.poly");
        fs::write(
            &main,
            r#"
namespace game {
    table A {
        id: u32 primary_key;
        b: B;
    }

    table B {
        id: u32 primary_key;
        a: A;
    }
}
"#,
        )
        .expect("main schema");

        let asts = parse_and_merge_schemas(&main, None).expect("parse schemas");
        let report = lint_asts(&asts);

        assert!(report.warnings.iter().any(|warning| {
            matches!(
                warning,
                SchemaLintWarning::CircularTypeReference { cycle }
                    if cycle == &vec![
                        "game.A".to_string(),
                        "game.B".to_string(),
                        "game.A".to_string()
                    ]
            )
        }));
    }

    #[test]
    fn test_lint_circular_type_reference_ignores_foreign_key_cycle() {
        let temp = TempDir::new().expect("temp dir");
        let main = temp.path().join("main.poly");
        fs::write(
            &main,
            r#"
namespace game {
    table A {
        id: u32 primary_key;
        b_id: u32 foreign_key(B.id);
    }

    table B {
        id: u32 primary_key;
        a_id: u32 foreign_key(A.id);
    }
}
"#,
        )
        .expect("main schema");

        let asts = parse_and_merge_schemas(&main, None).expect("parse schemas");
        let report = lint_asts(&asts);

        assert!(
            report
                .warnings
                .iter()
                .all(|warning| !matches!(warning, SchemaLintWarning::CircularTypeReference { .. })),
            "{report:#?}"
        );
    }

    #[test]
    fn test_render_lint_text_includes_warning_count() {
        let report = SchemaLintReport {
            warnings: vec![SchemaLintWarning::UnusedFileImport {
                source_path: "main.poly".to_string(),
                import_path: "unused.poly".to_string(),
                resolved_path: "unused.poly".to_string(),
            }],
        };

        let text = render_lint_text(&report);

        assert!(text.contains("Schema Lint"));
        assert!(text.contains("Warnings: 1"));
        assert!(text.contains("unused import"));
    }
}
