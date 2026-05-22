use std::collections::HashMap;

use crate::ir_model::{IndexDef, IndexFieldDef, StructItem, TypeRef};
use heck::ToPascalCase;

/// Builds index definitions from struct items by examining field constraints.
pub(super) fn build_indexes_from_items(items: &[StructItem]) -> Vec<IndexDef> {
    let mut indexes = Vec::new();

    for item in items {
        if let StructItem::Field(field) = item {
            // Primary key or unique constraint creates a unique index
            if field.is_primary_key || field.is_unique {
                indexes.push(IndexDef {
                    name: format!("By{}", field.name.to_pascal_case()),
                    fields: vec![IndexFieldDef {
                        name: field.name.clone(),
                        field_type: field.field_type.clone(),
                    }],
                    is_unique: true,
                    source: "constraint".to_string(),
                });
            }
            // Index constraint or foreign key creates a group index
            else if field.is_index {
                indexes.push(IndexDef {
                    name: format!("By{}", field.name.to_pascal_case()),
                    fields: vec![IndexFieldDef {
                        name: field.name.clone(),
                        field_type: field.field_type.clone(),
                    }],
                    is_unique: false,
                    source: "constraint".to_string(),
                });
            }
        }
    }

    indexes
}

/// Builds index definitions from @index annotations on a table.
pub(super) fn build_indexes_from_annotations(
    header: &[StructItem],
    items: &[StructItem],
) -> Vec<IndexDef> {
    let mut indexes = Vec::new();

    // Build a map of field names to their types for lookup
    let field_types: HashMap<String, TypeRef> = items
        .iter()
        .filter_map(|item| {
            if let StructItem::Field(field) = item {
                Some((field.name.clone(), field.field_type.clone()))
            } else {
                None
            }
        })
        .collect();

    for item in header {
        if let StructItem::Annotation(ann) = item {
            if ann.name == "index" && !ann.positional_args.is_empty() {
                // Check if "unique: true" is specified in params
                let is_unique = ann
                    .params
                    .iter()
                    .any(|p| p.key == "unique" && (p.value == "true" || p.value == "1"));

                // Build field definitions from positional args
                let fields: Vec<IndexFieldDef> = ann
                    .positional_args
                    .iter()
                    .filter_map(|field_name| {
                        field_types.get(field_name).map(|field_type| IndexFieldDef {
                            name: field_name.clone(),
                            field_type: field_type.clone(),
                        })
                    })
                    .collect();

                if !fields.is_empty() {
                    // Generate index name from field names
                    let name = format!(
                        "By{}",
                        fields
                            .iter()
                            .map(|f| f.name.to_pascal_case())
                            .collect::<Vec<_>>()
                            .join("")
                    );

                    indexes.push(IndexDef {
                        name,
                        fields,
                        is_unique,
                        source: "annotation".to_string(),
                    });
                }
            }
        }
    }

    indexes
}
