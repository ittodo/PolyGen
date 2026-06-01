//! Schema statistics for CLI reporting and automation.

use serde::Serialize;
use std::collections::BTreeMap;

use crate::ir_model::{NamespaceDef, NamespaceItem, SchemaContext, StructDef, StructItem};

/// Aggregate schema statistics.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct SchemaStatsReport {
    /// Number of schema files merged into the context.
    pub file_count: usize,
    /// Number of namespaces, including nested namespaces.
    pub namespace_count: usize,
    /// Number of table structs.
    pub table_count: usize,
    /// Number of embed structs.
    pub embed_count: usize,
    /// Number of enums, including inline enums.
    pub enum_count: usize,
    /// Number of table fields.
    pub table_field_count: usize,
    /// Number of embed fields.
    pub embed_field_count: usize,
    /// Number of fields across tables and embeds.
    pub field_count: usize,
    /// Number of declared indexes on tables.
    pub index_count: usize,
    /// Number of foreign-key relationships.
    pub relation_count: usize,
    /// Table counts grouped by namespace.
    pub tables_by_namespace: BTreeMap<String, usize>,
    /// Enum counts grouped by namespace.
    pub enums_by_namespace: BTreeMap<String, usize>,
}

/// Build aggregate statistics from an IR schema context.
pub fn build_schema_stats(schema: &SchemaContext) -> SchemaStatsReport {
    let mut stats = SchemaStatsReport {
        file_count: schema.files.len(),
        ..SchemaStatsReport::default()
    };

    for file in &schema.files {
        for namespace in &file.namespaces {
            collect_namespace_stats(namespace, &mut stats);
        }
    }

    stats.field_count = stats.table_field_count + stats.embed_field_count;
    stats
}

/// Render statistics as a stable, human-readable text report.
pub fn render_stats_text(stats: &SchemaStatsReport) -> String {
    let mut output = String::new();

    output.push_str("Schema Statistics\n");
    output.push_str("=================\n\n");

    output.push_str("Summary\n");
    output.push_str("-------\n");
    output.push_str(&format!("Files: {}\n", stats.file_count));
    output.push_str(&format!("Namespaces: {}\n", stats.namespace_count));
    output.push_str(&format!("Tables: {}\n", stats.table_count));
    output.push_str(&format!("Embeds: {}\n", stats.embed_count));
    output.push_str(&format!("Enums: {}\n", stats.enum_count));
    output.push_str(&format!("Fields: {}\n", stats.field_count));
    output.push_str(&format!("  Table fields: {}\n", stats.table_field_count));
    output.push_str(&format!("  Embed fields: {}\n", stats.embed_field_count));
    output.push_str(&format!("Indexes: {}\n", stats.index_count));
    output.push_str(&format!("Relations: {}\n", stats.relation_count));

    if !stats.tables_by_namespace.is_empty() {
        output.push_str("\nTables by namespace\n");
        output.push_str("-------------------\n");
        for (namespace, count) in &stats.tables_by_namespace {
            output.push_str(&format!("{}: {}\n", namespace, count));
        }
    }

    if !stats.enums_by_namespace.is_empty() {
        output.push_str("\nEnums by namespace\n");
        output.push_str("------------------\n");
        for (namespace, count) in &stats.enums_by_namespace {
            output.push_str(&format!("{}: {}\n", namespace, count));
        }
    }

    output
}

fn collect_namespace_stats(namespace: &NamespaceDef, stats: &mut SchemaStatsReport) {
    stats.namespace_count += 1;

    for item in &namespace.items {
        match item {
            NamespaceItem::Struct(s) => collect_struct_stats(s, &namespace.name, stats),
            NamespaceItem::Enum(_) => {
                stats.enum_count += 1;
                *stats
                    .enums_by_namespace
                    .entry(namespace.name.clone())
                    .or_insert(0) += 1;
            }
            NamespaceItem::Namespace(nested) => collect_namespace_stats(nested, stats),
            NamespaceItem::Comment(_) => {}
        }
    }
}

fn collect_struct_stats(s: &StructDef, namespace: &str, stats: &mut SchemaStatsReport) {
    if s.is_embed {
        stats.embed_count += 1;
        stats.embed_field_count += count_fields(s);
    } else if !s.name.ends_with("__Enum") {
        stats.table_count += 1;
        stats.table_field_count += count_fields(s);
        stats.index_count += s.indexes.len();
        stats.relation_count += count_foreign_keys(s);
        *stats
            .tables_by_namespace
            .entry(namespace.to_string())
            .or_insert(0) += 1;
    }

    for item in &s.items {
        match item {
            StructItem::EmbeddedStruct(nested) => collect_struct_stats(nested, namespace, stats),
            StructItem::InlineEnum(_) => {
                stats.enum_count += 1;
                *stats
                    .enums_by_namespace
                    .entry(namespace.to_string())
                    .or_insert(0) += 1;
            }
            StructItem::Field(_) | StructItem::Comment(_) | StructItem::Annotation(_) => {}
        }
    }
}

fn count_fields(s: &StructDef) -> usize {
    s.items
        .iter()
        .filter(|item| matches!(item, StructItem::Field(_)))
        .count()
}

fn count_foreign_keys(s: &StructDef) -> usize {
    s.items
        .iter()
        .filter(|item| {
            matches!(
                item,
                StructItem::Field(field) if field.foreign_key.is_some()
            )
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ir_builder, parse_and_merge_schemas, validation};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_build_schema_stats_counts_core_items() {
        let temp = TempDir::new().expect("temp dir");
        let schema = temp.path().join("schema.poly");
        fs::write(
            &schema,
            r#"
namespace game {
    enum Kind {
        A = 1;
    }

    embed Point {
        x: f32;
        y: f32;
    }

    table Player {
        id: u32 primary_key;
        point: Point;
        kind: Kind;
    }

    table Item {
        id: u32 primary_key;
        player_id: u32 foreign_key(Player.id);
    }
}
"#,
        )
        .expect("schema");

        let asts = parse_and_merge_schemas(&schema, None).expect("parse schema");
        let defs: Vec<_> = asts
            .iter()
            .flat_map(|ast| ast.definitions.clone())
            .collect();
        validation::validate_ast(&defs).expect("validate schema");
        let ir = ir_builder::build_ir(&asts);

        let stats = build_schema_stats(&ir);

        assert_eq!(stats.file_count, 1);
        assert_eq!(stats.namespace_count, 1);
        assert_eq!(stats.table_count, 2);
        assert_eq!(stats.embed_count, 1);
        assert_eq!(stats.enum_count, 1);
        assert_eq!(stats.table_field_count, 5);
        assert_eq!(stats.embed_field_count, 2);
        assert_eq!(stats.field_count, 7);
        assert_eq!(stats.relation_count, 1);
        assert_eq!(stats.tables_by_namespace.get("game"), Some(&2));
    }

    #[test]
    fn test_render_stats_text_is_stable() {
        let mut stats = SchemaStatsReport {
            file_count: 1,
            namespace_count: 1,
            table_count: 2,
            enum_count: 1,
            field_count: 5,
            table_field_count: 5,
            ..SchemaStatsReport::default()
        };
        stats.tables_by_namespace.insert("game".to_string(), 2);
        stats.enums_by_namespace.insert("game".to_string(), 1);

        let text = render_stats_text(&stats);

        assert!(text.contains("Schema Statistics"));
        assert!(text.contains("Tables: 2"));
        assert!(text.contains("game: 2"));
    }
}
