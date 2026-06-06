use std::collections::HashMap;

use crate::ir_model::{
    IndexDef, NamespaceDef, NamespaceItem, RefDef, RefFieldDef, SchemaContext, SearchIndexDef,
    StructDef, StructItem, TypeRef,
};
use heck::ToPascalCase;

use super::type_names::{last_segment_owned, qualify};

#[derive(Clone)]
struct RefTargetInfo {
    table_name: String,
    indexes: Vec<IndexDef>,
    searches: Vec<(String, SearchIndexDef)>,
}

/// Builds unresolved row refs from table-level @ref annotations.
pub(super) fn build_refs_from_annotations(
    header: &[StructItem],
    items: &[StructItem],
    current_ns: &str,
    source_fqn: &str,
) -> Vec<RefDef> {
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

    let mut refs = Vec::new();
    for item in header {
        let StructItem::Annotation(ann) = item else {
            continue;
        };
        if ann.name != "ref" {
            continue;
        }

        let Some(name) = param_value(ann, "name") else {
            continue;
        };
        let Some(target) = param_value(ann, "target") else {
            continue;
        };
        let Some(fields_value) = param_value(ann, "fields") else {
            continue;
        };

        let target_parts = target.split('.').collect::<Vec<_>>();
        if target_parts.len() < 2 {
            continue;
        }
        let target_name = target_parts[target_parts.len() - 1].to_string();
        let target_table_fqn = qualify(
            &target_parts[..target_parts.len() - 1].join("."),
            current_ns,
        );
        let target_table_name = last_segment_owned(&target_table_fqn);
        let fields = parse_tuple_fields(&fields_value)
            .into_iter()
            .filter_map(|field_name| {
                field_types.get(&field_name).map(|field_type| RefFieldDef {
                    local_field: field_name,
                    target_field: String::new(),
                    field_type: field_type.clone(),
                })
            })
            .collect::<Vec<_>>();
        let local_key_expr = local_key_expr(&fields);

        refs.push(RefDef {
            name: name.to_pascal_case(),
            source_table_fqn: source_fqn.to_string(),
            target_table_fqn,
            target_table_name,
            target_kind: "unknown".to_string(),
            target_name,
            target_method_suffix: String::new(),
            local_key_expr,
            fields,
            is_unique: false,
            reverse: param_value(ann, "reverse").map(|value| value.to_pascal_case()),
        });
    }

    refs
}

/// Resolves @ref targets against named @index aliases and @search names.
pub(super) fn resolve_refs(context: &mut SchemaContext) {
    let targets = collect_ref_targets(context);
    for file in &mut context.files {
        resolve_refs_in_namespaces(&mut file.namespaces, &targets);
    }
}

fn collect_ref_targets(context: &SchemaContext) -> HashMap<String, RefTargetInfo> {
    let mut targets = HashMap::new();
    for file in &context.files {
        collect_ref_targets_from_namespaces(&file.namespaces, &mut targets);
    }
    targets
}

fn collect_ref_targets_from_namespaces(
    namespaces: &[NamespaceDef],
    targets: &mut HashMap<String, RefTargetInfo>,
) {
    for ns in namespaces {
        for item in &ns.items {
            match item {
                NamespaceItem::Struct(s) => {
                    let searches = collect_searches(s);
                    targets.insert(
                        s.fqn.clone(),
                        RefTargetInfo {
                            table_name: s.name.clone(),
                            indexes: s.indexes.clone(),
                            searches,
                        },
                    );
                }
                NamespaceItem::Namespace(nested) => {
                    collect_ref_targets_from_namespaces(std::slice::from_ref(nested), targets);
                }
                NamespaceItem::Enum(_) | NamespaceItem::Comment(_) => {}
            }
        }
    }
}

fn collect_searches(s: &StructDef) -> Vec<(String, SearchIndexDef)> {
    let mut searches = Vec::new();
    for item in &s.items {
        if let StructItem::Field(field) = item {
            if let Some(search) = &field.search_index {
                searches.push((field.name.clone(), search.clone()));
            }
        }
    }
    searches
}

fn resolve_refs_in_namespaces(
    namespaces: &mut [NamespaceDef],
    targets: &HashMap<String, RefTargetInfo>,
) {
    for ns in namespaces {
        for item in &mut ns.items {
            match item {
                NamespaceItem::Struct(s) => resolve_refs_in_struct(s, targets),
                NamespaceItem::Namespace(nested) => {
                    resolve_refs_in_namespaces(std::slice::from_mut(nested), targets);
                }
                NamespaceItem::Enum(_) | NamespaceItem::Comment(_) => {}
            }
        }
    }
}

fn resolve_refs_in_struct(s: &mut StructDef, targets: &HashMap<String, RefTargetInfo>) {
    for row_ref in &mut s.refs {
        let Some(target) = targets.get(&row_ref.target_table_fqn) else {
            continue;
        };
        row_ref.target_table_name = target.table_name.clone();

        if let Some(index) = target
            .indexes
            .iter()
            .find(|idx| idx.schema_name.as_deref() == Some(row_ref.target_name.as_str()))
        {
            row_ref.target_kind = "index".to_string();
            row_ref.target_method_suffix = index.name.clone();
            row_ref.is_unique = index.is_unique;
            for (field, target_field) in row_ref.fields.iter_mut().zip(index.fields.iter()) {
                field.target_field = target_field.name.clone();
            }
            continue;
        }

        if let Some((_, search)) = target
            .searches
            .iter()
            .find(|(_, search)| search.schema_name == row_ref.target_name)
        {
            row_ref.target_kind = "search".to_string();
            row_ref.target_method_suffix = search.name.clone();
            row_ref.is_unique = false;
        }
    }
}

fn param_value(ann: &crate::ir_model::AnnotationDef, key: &str) -> Option<String> {
    ann.params
        .iter()
        .find(|param| param.key == key)
        .map(|param| param.value.clone())
}

fn parse_tuple_fields(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    let inner = trimmed
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
        .unwrap_or(trimmed);
    inner
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn local_key_expr(fields: &[RefFieldDef]) -> String {
    if fields.len() == 1 {
        return fields[0].local_field.clone();
    }

    format!(
        "({})",
        fields
            .iter()
            .map(|field| field.local_field.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )
}
