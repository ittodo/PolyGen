use crate::ir_model::{
    NamespaceDef, NamespaceItem, RelationDef, SchemaContext, StructDef, StructItem,
};
use heck::ToPascalCase;

/// Represents a pending relation to be added to a target table.
struct PendingRelation {
    /// FQN of the target table that should receive the relation.
    target_table_fqn: String,
    /// The relation definition to add.
    relation: RelationDef,
}

/// Resolves reverse relations from foreign_key definitions with aliases.
/// This must be called after all structs are built, as it needs to find
/// target tables that may be in different files or namespaces.
pub(super) fn resolve_relations(context: &mut SchemaContext) {
    // Step 1: Collect all pending relations from foreign_key ... as fields
    let mut pending_relations: Vec<PendingRelation> = Vec::new();

    for file in &context.files {
        collect_relations_from_namespace(&file.namespaces, &mut pending_relations);
    }

    // Step 2: Apply relations to their target tables
    for pending in pending_relations {
        apply_relation_to_target(context, &pending);
    }
}

/// Recursively collects relations from namespaces.
fn collect_relations_from_namespace(
    namespaces: &[NamespaceDef],
    pending: &mut Vec<PendingRelation>,
) {
    for ns in namespaces {
        for item in &ns.items {
            match item {
                NamespaceItem::Struct(s) => {
                    collect_relations_from_struct(s, pending);
                }
                NamespaceItem::Namespace(nested_ns) => {
                    collect_relations_from_namespace(&[(**nested_ns).clone()], pending);
                }
                _ => {}
            }
        }
    }
}

/// Collects relations from a single struct's fields.
fn collect_relations_from_struct(struct_def: &StructDef, pending: &mut Vec<PendingRelation>) {
    for item in &struct_def.items {
        if let StructItem::Field(field) = item {
            if let Some(fk) = &field.foreign_key {
                if let Some(alias) = &fk.alias {
                    pending.push(PendingRelation {
                        target_table_fqn: fk.target_table_fqn.clone(),
                        relation: RelationDef {
                            name: alias.to_pascal_case(),
                            source_table_fqn: struct_def.fqn.clone(),
                            source_table_name: struct_def.name.clone(),
                            source_field: field.name.clone(),
                        },
                    });
                }
            }
        }
    }
}

/// Applies a pending relation to its target table.
fn apply_relation_to_target(context: &mut SchemaContext, pending: &PendingRelation) {
    for file in &mut context.files {
        if apply_relation_to_namespaces(&mut file.namespaces, pending) {
            return;
        }
    }
}

/// Recursively searches namespaces for the target table and applies the relation.
fn apply_relation_to_namespaces(
    namespaces: &mut [NamespaceDef],
    pending: &PendingRelation,
) -> bool {
    for ns in namespaces {
        for item in &mut ns.items {
            match item {
                NamespaceItem::Struct(s) => {
                    if s.fqn == pending.target_table_fqn {
                        s.relations.push(pending.relation.clone());
                        return true;
                    }
                }
                NamespaceItem::Namespace(nested_ns) => {
                    if apply_relation_to_namespaces(&mut [(**nested_ns).clone()], pending) {
                        // Need to update the boxed namespace
                        // This is a bit awkward due to the Box
                        return true;
                    }
                }
                _ => {}
            }
        }
    }
    false
}
