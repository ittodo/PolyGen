//! Schema visualization module.
//!
//! Generates visualization data for schema tables including:
//! - Forward references (tables this table references via foreign keys)
//! - Reverse references (tables that reference this table)
//!
//! Output format is JSON suitable for GUI consumption.

use serde::Serialize;
use std::collections::HashMap;

use crate::ir_model::{NamespaceItem, SchemaContext, StructDef, StructItem};

/// Complete visualization data for the schema.
#[derive(Serialize, Debug)]
pub struct SchemaVisualization {
    /// All tables with their reference information.
    pub tables: Vec<TableVisualization>,
    /// Summary statistics.
    pub stats: SchemaStats,
}

/// Visualization data for a single table.
#[derive(Serialize, Debug)]
pub struct TableVisualization {
    /// Simple table name.
    pub name: String,
    /// Fully qualified name.
    pub fqn: String,
    /// Namespace.
    pub namespace: String,
    /// Fields in this table.
    pub fields: Vec<FieldVisualization>,
    /// Tables that reference this table (for left side of GUI).
    #[serde(rename = "referencedBy")]
    pub referenced_by: Vec<ReferenceInfo>,
    /// Tables this table references (for right side of GUI).
    pub references: Vec<ReferenceInfo>,
    /// Index information.
    pub indexes: Vec<IndexVisualization>,
    /// Annotations/metadata.
    pub metadata: TableMetadata,
}

/// Field visualization data.
#[derive(Serialize, Debug)]
pub struct FieldVisualization {
    /// Field name.
    pub name: String,
    /// Type name (e.g., "u32", "string", "Player").
    #[serde(rename = "type")]
    pub type_name: String,
    /// Whether this field is optional.
    #[serde(rename = "isOptional")]
    pub is_optional: bool,
    /// Whether this field is a list/array.
    #[serde(rename = "isList")]
    pub is_list: bool,
    /// Whether this is a primary key.
    #[serde(rename = "isPrimaryKey")]
    pub is_primary_key: bool,
    /// Whether this field has a unique constraint.
    #[serde(rename = "isUnique")]
    pub is_unique: bool,
    /// Whether this field is a foreign key.
    #[serde(rename = "isForeignKey")]
    pub is_foreign_key: bool,
    /// Constraints on this field.
    pub constraints: Vec<String>,
}

/// Reference information (for both forward and reverse references).
#[derive(Serialize, Debug, Clone)]
pub struct ReferenceInfo {
    /// The other table's simple name.
    #[serde(rename = "tableName")]
    pub table_name: String,
    /// The other table's FQN.
    #[serde(rename = "tableFqn")]
    pub table_fqn: String,
    /// The field that creates this reference.
    #[serde(rename = "fieldName")]
    pub field_name: String,
    /// Relation alias (if specified with `as`).
    pub alias: Option<String>,
}

/// Index visualization data.
#[derive(Serialize, Debug)]
pub struct IndexVisualization {
    /// Index name.
    pub name: String,
    /// Fields in this index.
    pub fields: Vec<String>,
    /// Whether this is a unique index.
    #[serde(rename = "isUnique")]
    pub is_unique: bool,
}

/// Table metadata.
#[derive(Serialize, Debug)]
pub struct TableMetadata {
    /// Data source (sqlite, mysql, etc.).
    pub datasource: Option<String>,
    /// Cache strategy.
    #[serde(rename = "cacheStrategy")]
    pub cache_strategy: Option<String>,
    /// Whether this table is read-only.
    #[serde(rename = "isReadonly")]
    pub is_readonly: bool,
    /// Soft delete field name.
    #[serde(rename = "softDeleteField")]
    pub soft_delete_field: Option<String>,
}

/// Schema statistics.
#[derive(Serialize, Debug)]
pub struct SchemaStats {
    /// Total number of tables.
    #[serde(rename = "tableCount")]
    pub table_count: usize,
    /// Total number of fields across all tables.
    #[serde(rename = "fieldCount")]
    pub field_count: usize,
    /// Total number of foreign key relationships.
    #[serde(rename = "relationCount")]
    pub relation_count: usize,
    /// Number of namespaces.
    #[serde(rename = "namespaceCount")]
    pub namespace_count: usize,
    /// Tables grouped by namespace.
    #[serde(rename = "tablesByNamespace")]
    pub tables_by_namespace: HashMap<String, usize>,
}

/// Build visualization data from schema context.
pub fn build_visualization(schema: &SchemaContext) -> SchemaVisualization {
    let mut tables = Vec::new();
    let mut all_structs: Vec<&StructDef> = Vec::new();
    let mut namespaces = std::collections::HashSet::new();

    // First pass: collect all structs
    for file in &schema.files {
        for ns in &file.namespaces {
            collect_structs_from_namespace(ns, &mut all_structs, &mut namespaces);
        }
    }

    // Build a map of FQN -> references to this table (excluding embeds)
    let mut reverse_refs: HashMap<String, Vec<ReferenceInfo>> = HashMap::new();

    for s in &all_structs {
        // Skip embeds for reference calculation
        if s.is_embed {
            continue;
        }
        for item in &s.items {
            if let StructItem::Field(field) = item {
                if let Some(ref fk) = field.foreign_key {
                    let ref_info = ReferenceInfo {
                        table_name: s.name.clone(),
                        table_fqn: s.fqn.clone(),
                        field_name: field.name.clone(),
                        alias: fk.alias.clone(),
                    };
                    reverse_refs
                        .entry(fk.target_table_fqn.clone())
                        .or_default()
                        .push(ref_info);
                }
            }
        }
    }

    // Second pass: build visualization for each struct
    let mut total_fields = 0;
    let mut total_relations = 0;
    let mut tables_by_namespace: HashMap<String, usize> = HashMap::new();

    for s in &all_structs {
        // Skip enum helper structs and embeds
        if s.name.ends_with("__Enum") || s.is_embed {
            continue;
        }

        let namespace = extract_namespace(&s.fqn);
        *tables_by_namespace.entry(namespace.clone()).or_insert(0) += 1;

        let mut fields = Vec::new();
        let mut forward_refs = Vec::new();

        for item in &s.items {
            if let StructItem::Field(field) = item {
                total_fields += 1;

                let mut constraints = Vec::new();
                if field.is_primary_key {
                    constraints.push("primary_key".to_string());
                }
                if field.is_unique {
                    constraints.push("unique".to_string());
                }
                if let Some(max_len) = field.max_length {
                    constraints.push(format!("max_length({})", max_len));
                }
                if let Some(ref range) = field.range {
                    constraints.push(format!("range({}, {})", range.min, range.max));
                }
                if let Some(ref pattern) = field.regex_pattern {
                    constraints.push(format!("regex(\"{}\")", pattern));
                }
                if let Some(ref default) = field.default_value {
                    constraints.push(format!("default({})", default));
                }

                let is_fk = field.foreign_key.is_some();
                if is_fk {
                    total_relations += 1;
                    if let Some(ref fk) = field.foreign_key {
                        forward_refs.push(ReferenceInfo {
                            table_name: extract_type_name(&fk.target_table_fqn),
                            table_fqn: fk.target_table_fqn.clone(),
                            field_name: field.name.clone(),
                            alias: fk.alias.clone(),
                        });
                    }
                }

                fields.push(FieldVisualization {
                    name: field.name.clone(),
                    type_name: field.field_type.type_name.clone(),
                    is_optional: field.field_type.is_option,
                    is_list: field.field_type.is_list,
                    is_primary_key: field.is_primary_key,
                    is_unique: field.is_unique,
                    is_foreign_key: is_fk,
                    constraints,
                });
            }
        }

        let indexes: Vec<IndexVisualization> = s
            .indexes
            .iter()
            .map(|idx| IndexVisualization {
                name: idx.name.clone(),
                fields: idx.fields.iter().map(|f| f.name.clone()).collect(),
                is_unique: idx.is_unique,
            })
            .collect();

        let referenced_by = reverse_refs.get(&s.fqn).cloned().unwrap_or_default();

        tables.push(TableVisualization {
            name: s.name.clone(),
            fqn: s.fqn.clone(),
            namespace: namespace.clone(),
            fields,
            referenced_by,
            references: forward_refs,
            indexes,
            metadata: TableMetadata {
                datasource: s.datasource.clone(),
                cache_strategy: s.cache_strategy.clone(),
                is_readonly: s.is_readonly,
                soft_delete_field: s.soft_delete_field.clone(),
            },
        });
    }

    // Sort tables by namespace, then by name
    tables.sort_by(|a, b| (&a.namespace, &a.name).cmp(&(&b.namespace, &b.name)));

    SchemaVisualization {
        tables,
        stats: SchemaStats {
            table_count: tables_by_namespace.values().sum(),
            field_count: total_fields,
            relation_count: total_relations,
            namespace_count: namespaces.len(),
            tables_by_namespace,
        },
    }
}

/// Collect all structs from a namespace (including nested namespaces).
fn collect_structs_from_namespace<'a>(
    ns: &'a crate::ir_model::NamespaceDef,
    structs: &mut Vec<&'a StructDef>,
    namespaces: &mut std::collections::HashSet<String>,
) {
    namespaces.insert(ns.name.clone());

    for item in &ns.items {
        match item {
            NamespaceItem::Struct(s) => {
                structs.push(s);
            }
            NamespaceItem::Namespace(nested) => {
                collect_structs_from_namespace(nested, structs, namespaces);
            }
            _ => {}
        }
    }
}

/// Extract namespace from FQN (e.g., "game.player.Player" -> "game.player").
fn extract_namespace(fqn: &str) -> String {
    if let Some(pos) = fqn.rfind('.') {
        fqn[..pos].to_string()
    } else {
        String::new()
    }
}

/// Extract type name from FQN (e.g., "game.player.Player" -> "Player").
fn extract_type_name(fqn: &str) -> String {
    if let Some(pos) = fqn.rfind('.') {
        fqn[pos + 1..].to_string()
    } else {
        fqn.to_string()
    }
}

/// Generate Mermaid ER diagram from visualization data.
pub fn to_mermaid(viz: &SchemaVisualization) -> String {
    let mut output = String::new();
    output.push_str("erDiagram\n");

    // Generate table definitions
    for table in &viz.tables {
        output.push_str(&format!("    {} {{\n", table.name));
        for field in &table.fields {
            let mut type_str = field.type_name.clone();
            if field.is_optional {
                type_str.push('?');
            }
            if field.is_list {
                type_str = format!("{}[]", type_str);
            }

            let mut markers = Vec::new();
            if field.is_primary_key {
                markers.push("PK");
            }
            if field.is_foreign_key {
                markers.push("FK");
            }
            if field.is_unique && !field.is_primary_key {
                markers.push("UK");
            }

            let marker_str = if markers.is_empty() {
                String::new()
            } else {
                format!(" \"{}\"", markers.join(","))
            };

            output.push_str(&format!("        {} {}{}\n", type_str, field.name, marker_str));
        }
        output.push_str("    }\n");
    }

    output.push('\n');

    // Generate relationships
    for table in &viz.tables {
        for ref_info in &table.references {
            // table ||--o{ ref_table : "field_name"
            let relation_label = if let Some(ref alias) = ref_info.alias {
                alias.clone()
            } else {
                ref_info.field_name.clone()
            };
            output.push_str(&format!(
                "    {} ||--o{{ {} : \"{}\"\n",
                ref_info.table_name, table.name, relation_label
            ));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_namespace() {
        assert_eq!(extract_namespace("game.player.Player"), "game.player");
        assert_eq!(extract_namespace("Player"), "");
        assert_eq!(extract_namespace("game.Player"), "game");
    }

    #[test]
    fn test_extract_type_name() {
        assert_eq!(extract_type_name("game.player.Player"), "Player");
        assert_eq!(extract_type_name("Player"), "Player");
    }
}
