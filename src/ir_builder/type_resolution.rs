use crate::ir_model::{NamespaceItem, SchemaContext, StructDef, StructItem, TypeRef};
use crate::type_registry::{TypeKind, TypeRegistry};

use super::type_names::namespace_of_owned;

// Traverse the built IR and fix TypeRef.is_enum / is_struct flags based on declared types
pub(super) fn resolve_type_kinds(ctx: &mut SchemaContext) {
    let mut registry = TypeRegistry::new();

    // Pass 1: Collect all types (enums and structs) from all files/namespaces
    for file in &ctx.files {
        for ns in &file.namespaces {
            collect_types_from_namespace_items(&ns.items, &mut registry);
        }
    }

    // Pass 2: Adjust all TypeRefs in struct fields recursively
    for file in &mut ctx.files {
        for ns in &mut file.namespaces {
            adjust_namespace_items_typerefs(&mut ns.items, &registry);
        }
    }
}

/// Recursively collects all type definitions from namespace items into the registry.
fn collect_types_from_namespace_items(items: &[NamespaceItem], registry: &mut TypeRegistry) {
    for item in items {
        match item {
            NamespaceItem::Enum(e) => {
                registry.register(&e.fqn, TypeKind::Enum);
            }
            NamespaceItem::Struct(s) => {
                registry.register(&s.fqn, TypeKind::Struct);
                collect_types_from_struct(s, registry);
            }
            NamespaceItem::Namespace(inner) => {
                collect_types_from_namespace_items(&inner.items, registry);
            }
            NamespaceItem::Comment(_) => {}
        }
    }
}

/// Recursively collects types from within a struct (inline enums, embedded structs).
fn collect_types_from_struct(s: &StructDef, registry: &mut TypeRegistry) {
    for item in &s.items {
        match item {
            StructItem::InlineEnum(e) => {
                registry.register(&e.fqn, TypeKind::Enum);
            }
            StructItem::EmbeddedStruct(sub) => {
                registry.register(&sub.fqn, TypeKind::Embed);
                collect_types_from_struct(sub, registry);
            }
            StructItem::Field(_) | StructItem::Annotation(_) | StructItem::Comment(_) => {}
        }
    }
}

/// Recursively adjusts TypeRef flags in namespace items using the registry.
fn adjust_namespace_items_typerefs(items: &mut [NamespaceItem], registry: &TypeRegistry) {
    for item in items {
        match item {
            NamespaceItem::Struct(ref mut s) => {
                let struct_fqn = s.fqn.clone();
                adjust_struct_typerefs(s, registry, &struct_fqn);
            }
            NamespaceItem::Namespace(ref mut inner) => {
                adjust_namespace_items_typerefs(&mut inner.items, registry)
            }
            NamespaceItem::Enum(_) | NamespaceItem::Comment(_) => {}
        }
    }
}

/// Recursively adjusts TypeRef flags in a struct's fields.
/// `parent_fqn` is the FQN of the containing struct (for resolving inline embedded types).
fn adjust_struct_typerefs(s: &mut StructDef, registry: &TypeRegistry, parent_fqn: &str) {
    for item in &mut s.items {
        match item {
            StructItem::Field(ref mut f) => {
                adjust_typeref(&mut f.field_type, registry, parent_fqn);
            }
            StructItem::EmbeddedStruct(ref mut sub) => {
                let sub_fqn = sub.fqn.clone();
                adjust_struct_typerefs(sub, registry, &sub_fqn);
            }
            StructItem::InlineEnum(_) | StructItem::Annotation(_) | StructItem::Comment(_) => {}
        }
    }
}

/// Adjusts a single TypeRef's is_enum/is_struct flags using the registry.
/// `parent_fqn` is the FQN of the containing struct (for resolving inline embedded types).
fn adjust_typeref(t: &mut TypeRef, registry: &TypeRegistry, parent_fqn: &str) {
    // First, recursively process inner types (for Option<T> or List<T>)
    if let Some(inner) = &mut t.inner_type {
        adjust_typeref(inner.as_mut(), registry, parent_fqn);
    }

    // Skip primitives - they don't need resolution
    if t.is_primitive {
        return;
    }

    // Try to resolve the type using the registry
    // Strategy 1: Direct FQN match
    if registry.is_enum(&t.fqn) {
        t.is_enum = true;
        t.is_struct = false;
        return;
    }

    // Strategy 2: Check if this is an inline embedded struct within the parent struct
    // Try parent_fqn.TypeName (e.g., "test.embed.Person.Details")
    if !parent_fqn.is_empty() {
        let embedded_fqn = format!("{}.{}", parent_fqn, t.type_name);
        if registry.is_embed(&embedded_fqn) {
            t.fqn = embedded_fqn.clone();
            t.namespace_fqn = parent_fqn.to_string();
            t.is_enum = false;
            t.is_struct = true;
            return;
        }
    }

    // Strategy 3: Resolve using namespace context
    if let Some(resolved_fqn) = registry.resolve(&t.type_name, &t.namespace_fqn) {
        if registry.is_enum(resolved_fqn) {
            t.fqn = resolved_fqn.to_string();
            t.namespace_fqn = namespace_of_owned(resolved_fqn);
            t.is_enum = true;
            t.is_struct = false;
            return;
        }
    }

    // Default: treat as struct (could be a struct reference or external type)
    t.is_enum = false;
    t.is_struct = true;
}
