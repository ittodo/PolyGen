//! Template context: typed variable bindings wrapping IR values.
//!
//! [`ContextValue`] wraps IR model types so the renderer can resolve property
//! access chains like `struct.name` or `field.field_type.is_option`.

use std::collections::HashMap;

use crate::ir_model::*;

/// A value that can be bound in a template context.
#[derive(Debug, Clone)]
pub enum ContextValue {
    String(String),
    Bool(bool),
    Int(i64),
    Float(f64),
    /// A full schema context.
    Schema(SchemaContext),
    /// A file definition.
    File(FileDef),
    /// A namespace definition.
    Namespace(NamespaceDef),
    /// A namespace item (struct, enum, comment, or nested namespace).
    NamespaceItem(NamespaceItem),
    /// A struct definition.
    Struct(StructDef),
    /// A struct item (field, comment, annotation, embedded struct, inline enum).
    StructItem(StructItem),
    /// A field definition.
    Field(Box<FieldDef>),
    /// A type reference.
    TypeRef(TypeRef),
    /// An enum definition.
    Enum(EnumDef),
    /// An enum item (member or comment).
    EnumItem(EnumItem),
    /// An enum member.
    EnumMember(EnumMember),
    /// An annotation definition.
    Annotation(AnnotationDef),
    /// An annotation parameter (key-value pair).
    AnnotationParam(AnnotationParam),
    /// An index definition.
    Index(IndexDef),
    /// An index field definition (part of a composite index).
    IndexField(IndexFieldDef),
    /// A relation definition.
    Relation(RelationDef),
    /// A foreign key definition.
    ForeignKey(ForeignKeyDef),
    /// A timezone reference.
    Timezone(TimezoneRef),
    /// A range definition.
    Range(RangeDef),
    /// A list of context values.
    List(Vec<ContextValue>),
    /// Null / absent value.
    Null,
}

/// Template rendering context with variable bindings.
#[derive(Debug, Clone)]
pub struct TemplateContext {
    /// Named variable bindings.
    bindings: HashMap<String, ContextValue>,
}

impl TemplateContext {
    /// Creates an empty context.
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Creates a context with a schema binding.
    pub fn with_schema(schema: &SchemaContext) -> Self {
        let mut ctx = Self::new();
        ctx.bindings
            .insert("schema".to_string(), ContextValue::Schema(schema.clone()));
        ctx
    }

    /// Creates a context with a file binding.
    pub fn with_file(file: &FileDef) -> Self {
        let mut ctx = Self::new();
        ctx.bindings
            .insert("file".to_string(), ContextValue::File(file.clone()));
        ctx
    }

    /// Adds a binding to this context.
    pub fn set(&mut self, name: &str, value: ContextValue) {
        self.bindings.insert(name.to_string(), value);
    }

    /// Gets a binding by name.
    pub fn get(&self, name: &str) -> Option<&ContextValue> {
        self.bindings.get(name)
    }

    /// Creates a child context inheriting all current bindings.
    pub fn child(&self) -> Self {
        Self {
            bindings: self.bindings.clone(),
        }
    }

    /// Creates a child context with an additional binding.
    pub fn child_with(&self, name: &str, value: ContextValue) -> Self {
        let mut child = self.child();
        child.set(name, value);
        child
    }
}

impl Default for TemplateContext {
    fn default() -> Self {
        Self::new()
    }
}

// --- Property access ---

impl ContextValue {
    /// Resolve a single property access on this value.
    ///
    /// Returns `Null` if the property doesn't exist.
    pub fn get_property(&self, name: &str) -> ContextValue {
        match self {
            ContextValue::Schema(s) => match name {
                "files" => ContextValue::List(
                    s.files.iter().map(|f| ContextValue::File(f.clone())).collect(),
                ),
                _ => ContextValue::Null,
            },
            ContextValue::File(f) => match name {
                "path" | "file_name" => ContextValue::String(f.path.clone()),
                "namespaces" => ContextValue::List(
                    f.namespaces
                        .iter()
                        .map(|ns| ContextValue::Namespace(ns.clone()))
                        .collect(),
                ),
                "all_tables" => {
                    // Collect all non-embed, non-__Enum structs from all namespaces (flat)
                    let mut tables = Vec::new();
                    collect_tables_from_namespaces(&f.namespaces, &mut tables);
                    ContextValue::List(tables)
                }
                "container_name" => {
                    // Derive PascalCase container name from file path:
                    // "examples/game_schema.poly" -> "GameSchema"
                    let base = f.path.rsplit('/').next().unwrap_or(&f.path);
                    let base = base.strip_suffix(".poly").unwrap_or(base);
                    let pascal = base
                        .split('_')
                        .map(|part| {
                            let mut chars = part.chars();
                            match chars.next() {
                                None => String::new(),
                                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                            }
                        })
                        .collect::<String>();
                    ContextValue::String(pascal)
                }
                "all_tables_interface_list" => {
                    // Generate ", IHas{Name}Table" for each table
                    // e.g., ", IHasPlayerTable, IHasMonsterTable"
                    let mut tables = Vec::new();
                    collect_tables_from_namespaces(&f.namespaces, &mut tables);
                    let names: Vec<String> = tables.iter().filter_map(|t| {
                        if let ContextValue::Struct(s) = t {
                            Some(format!(", IHas{}Table", s.name))
                        } else {
                            None
                        }
                    }).collect();
                    ContextValue::String(names.join(""))
                }
                _ => ContextValue::Null,
            },
            ContextValue::Namespace(ns) => match name {
                "name" => ContextValue::String(ns.name.clone()),
                "datasource" => match &ns.datasource {
                    Some(ds) => ContextValue::String(ds.clone()),
                    None => ContextValue::Null,
                },
                "items" => ContextValue::List(
                    ns.items
                        .iter()
                        .map(|item| ContextValue::NamespaceItem(item.clone()))
                        .collect(),
                ),
                "has_structs" => {
                    let has = ns.items.iter().any(|item| matches!(item, NamespaceItem::Struct(_)));
                    ContextValue::Bool(has)
                }
                "structs" => {
                    let structs: Vec<ContextValue> = ns
                        .items
                        .iter()
                        .filter_map(|item| match item {
                            NamespaceItem::Struct(s) => Some(ContextValue::Struct(s.clone())),
                            _ => None,
                        })
                        .collect();
                    ContextValue::List(structs)
                }
                "fqn" => {
                    // Namespace FQN (e.g., "game.character")
                    ContextValue::String(ns.name.clone())
                }
                _ => ContextValue::Null,
            },
            ContextValue::NamespaceItem(item) => match name {
                "is_struct" => ContextValue::Bool(matches!(item, NamespaceItem::Struct(_))),
                "is_enum" => ContextValue::Bool(matches!(item, NamespaceItem::Enum(_))),
                "is_comment" => ContextValue::Bool(matches!(item, NamespaceItem::Comment(_))),
                "is_namespace" => ContextValue::Bool(matches!(item, NamespaceItem::Namespace(_))),
                "as_struct" => match item {
                    NamespaceItem::Struct(s) => ContextValue::Struct(s.clone()),
                    _ => ContextValue::Null,
                },
                "as_enum" => match item {
                    NamespaceItem::Enum(e) => ContextValue::Enum(e.clone()),
                    _ => ContextValue::Null,
                },
                "as_comment" => match item {
                    NamespaceItem::Comment(c) => ContextValue::String(c.clone()),
                    _ => ContextValue::Null,
                },
                "as_namespace" => match item {
                    NamespaceItem::Namespace(ns) => ContextValue::Namespace(*ns.clone()),
                    _ => ContextValue::Null,
                },
                _ => ContextValue::Null,
            },
            ContextValue::Struct(s) => match name {
                "name" => ContextValue::String(s.name.clone()),
                "fqn" => ContextValue::String(s.fqn.clone()),
                "is_embed" => ContextValue::Bool(s.is_embed),
                "is_readonly" => ContextValue::Bool(s.is_readonly),
                "datasource" => match &s.datasource {
                    Some(ds) => ContextValue::String(ds.clone()),
                    None => ContextValue::Null,
                },
                "cache_strategy" => match &s.cache_strategy {
                    Some(cs) => ContextValue::String(cs.clone()),
                    None => ContextValue::Null,
                },
                "soft_delete_field" => match &s.soft_delete_field {
                    Some(f) => ContextValue::String(f.clone()),
                    None => ContextValue::Null,
                },
                "pack_separator" => match &s.pack_separator {
                    Some(sep) => ContextValue::String(sep.clone()),
                    None => ContextValue::Null,
                },
                "header" => ContextValue::List(
                    s.header
                        .iter()
                        .map(|item| ContextValue::StructItem(item.clone()))
                        .collect(),
                ),
                "items" => ContextValue::List(
                    s.items
                        .iter()
                        .map(|item| ContextValue::StructItem(item.clone()))
                        .collect(),
                ),
                "indexes" => ContextValue::List(
                    s.indexes
                        .iter()
                        .map(|idx| ContextValue::Index(idx.clone()))
                        .collect(),
                ),
                "relations" => ContextValue::List(
                    s.relations
                        .iter()
                        .map(|rel| ContextValue::Relation(rel.clone()))
                        .collect(),
                ),
                "doc_comments" => {
                    let comments: Vec<ContextValue> = s
                        .header
                        .iter()
                        .filter_map(|item| match item {
                            StructItem::Comment(c) => Some(ContextValue::String(c.clone())),
                            _ => None,
                        })
                        .collect();
                    ContextValue::List(comments)
                }
                "fields" => {
                    let fields: Vec<ContextValue> = s
                        .items
                        .iter()
                        .filter_map(|item| match item {
                            StructItem::Field(f) => Some(ContextValue::Field(f.clone())),
                            _ => None,
                        })
                        .collect();
                    ContextValue::List(fields)
                }
                "default_fields" => {
                    let fields: Vec<ContextValue> = s
                        .items
                        .iter()
                        .filter_map(|item| match item {
                            StructItem::Field(f) if f.default_value.is_some() => {
                                Some(ContextValue::Field(f.clone()))
                            }
                            _ => None,
                        })
                        .collect();
                    ContextValue::List(fields)
                }
                "has_defaults" => {
                    let has = s.items.iter().any(|item| matches!(item, StructItem::Field(f) if f.default_value.is_some()));
                    ContextValue::Bool(has)
                }
                "has_foreign_keys" => {
                    let has = s.items.iter().any(|item| matches!(item, StructItem::Field(f) if f.foreign_key.is_some()));
                    ContextValue::Bool(has)
                }
                "fk_fields" => {
                    let fields: Vec<ContextValue> = s
                        .items
                        .iter()
                        .filter_map(|item| match item {
                            StructItem::Field(f) if f.foreign_key.is_some() => {
                                Some(ContextValue::Field(f.clone()))
                            }
                            _ => None,
                        })
                        .collect();
                    ContextValue::List(fields)
                }
                "pk_field_name" => {
                    // Find primary key field name (defaults to "Id")
                    let pk = s.items.iter().find_map(|item| match item {
                        StructItem::Field(f) if f.is_primary_key => {
                            Some(f.name.clone())
                        }
                        _ => None,
                    });
                    match pk {
                        Some(name) => ContextValue::String(name),
                        None => ContextValue::String("Id".to_string()),
                    }
                }
                "has_primary_key" => {
                    let has = s.items.iter().any(|item| matches!(item, StructItem::Field(f) if f.is_primary_key));
                    ContextValue::Bool(has)
                }
                "namespace_fqn" => {
                    // Extract namespace from FQN: "game.character.Player" -> "game.character"
                    if let Some(pos) = s.fqn.rfind('.') {
                        ContextValue::String(s.fqn[..pos].to_string())
                    } else {
                        ContextValue::String(String::new())
                    }
                }
                "has_validations" => {
                    // Check if any field has max_length, range, or regex_pattern
                    let has = s.items.iter().any(|item| match item {
                        StructItem::Field(f) => {
                            f.max_length.is_some() || f.range.is_some() || f.regex_pattern.is_some()
                        }
                        _ => false,
                    });
                    ContextValue::Bool(has)
                }
                "validation_fields" => {
                    // Fields that have max_length, range, or regex_pattern
                    let fields: Vec<ContextValue> = s
                        .items
                        .iter()
                        .filter_map(|item| match item {
                            StructItem::Field(f) if f.max_length.is_some() || f.range.is_some() || f.regex_pattern.is_some() => {
                                Some(ContextValue::Field(f.clone()))
                            }
                            _ => None,
                        })
                        .collect();
                    ContextValue::List(fields)
                }
                "auto_create_field" => {
                    // Find the field with auto_create timestamp
                    s.items.iter().find_map(|item| match item {
                        StructItem::Field(f) if f.auto_create.is_some() => {
                            Some(ContextValue::Field(f.clone()))
                        }
                        _ => None,
                    }).unwrap_or(ContextValue::Null)
                }
                _ => ContextValue::Null,
            },
            ContextValue::StructItem(item) => match name {
                "is_field" => ContextValue::Bool(matches!(item, StructItem::Field(_))),
                "is_comment" => ContextValue::Bool(matches!(item, StructItem::Comment(_))),
                "is_annotation" => ContextValue::Bool(matches!(item, StructItem::Annotation(_))),
                "is_embedded_struct" => {
                    ContextValue::Bool(matches!(item, StructItem::EmbeddedStruct(_)))
                }
                "is_inline_enum" => {
                    ContextValue::Bool(matches!(item, StructItem::InlineEnum(_)))
                }
                "as_field" => match item {
                    StructItem::Field(f) => ContextValue::Field(f.clone()),
                    _ => ContextValue::Null,
                },
                "as_comment" => match item {
                    StructItem::Comment(c) => ContextValue::String(c.clone()),
                    _ => ContextValue::Null,
                },
                "as_annotation" => match item {
                    StructItem::Annotation(a) => ContextValue::Annotation(a.clone()),
                    _ => ContextValue::Null,
                },
                "as_embedded_struct" => match item {
                    StructItem::EmbeddedStruct(s) => ContextValue::Struct(s.clone()),
                    _ => ContextValue::Null,
                },
                "as_inline_enum" => match item {
                    StructItem::InlineEnum(e) => ContextValue::Enum(e.clone()),
                    _ => ContextValue::Null,
                },
                _ => ContextValue::Null,
            },
            ContextValue::Field(f) => match name {
                "name" | "field_name" => ContextValue::String(f.name.clone()),
                "field_type" => ContextValue::TypeRef(f.field_type.clone()),
                "attributes" => ContextValue::List(
                    f.attributes
                        .iter()
                        .map(|a| ContextValue::String(a.clone()))
                        .collect(),
                ),
                "is_primary_key" => ContextValue::Bool(f.is_primary_key),
                "is_unique" => ContextValue::Bool(f.is_unique),
                "is_index" => ContextValue::Bool(f.is_index),
                "foreign_key" => match &f.foreign_key {
                    Some(fk) => ContextValue::ForeignKey(fk.clone()),
                    None => ContextValue::Null,
                },
                "has_foreign_key" => ContextValue::Bool(f.foreign_key.is_some()),
                "max_length" => match f.max_length {
                    Some(ml) => ContextValue::Int(ml as i64),
                    None => ContextValue::Null,
                },
                "default_value" => match &f.default_value {
                    Some(dv) => ContextValue::String(dv.clone()),
                    None => ContextValue::Null,
                },
                "has_default_value" => ContextValue::Bool(f.default_value.is_some()),
                "range" => match &f.range {
                    Some(r) => ContextValue::Range(r.clone()),
                    None => ContextValue::Null,
                },
                "regex_pattern" => match &f.regex_pattern {
                    Some(p) => ContextValue::String(p.clone()),
                    None => ContextValue::Null,
                },
                "auto_create" => match &f.auto_create {
                    Some(tz) => ContextValue::Timezone(tz.clone()),
                    None => ContextValue::Null,
                },
                "auto_update" => match &f.auto_update {
                    Some(tz) => ContextValue::Timezone(tz.clone()),
                    None => ContextValue::Null,
                },
                "has_auto_update" => ContextValue::Bool(f.auto_update.is_some()),
                "has_max_length" => ContextValue::Bool(f.max_length.is_some()),
                "has_range" => ContextValue::Bool(f.range.is_some()),
                "has_regex_pattern" => ContextValue::Bool(f.regex_pattern.is_some()),
                "nav_name" => {
                    // FK navigation property name: strip _id/Id suffix from field name
                    if let Some(fk) = &f.foreign_key {
                        let target_name = fk.target_table_fqn
                            .rsplit('.')
                            .next()
                            .unwrap_or(&fk.target_table_fqn);
                        let name = if f.name.ends_with("_id") {
                            f.name[..f.name.len() - 3].to_string()
                        } else if f.name.ends_with("Id") {
                            f.name[..f.name.len() - 2].to_string()
                        } else {
                            target_name.to_string()
                        };
                        ContextValue::String(name)
                    } else {
                        ContextValue::Null
                    }
                }
                "name_pascal" => {
                    // Convert field name to PascalCase
                    let pascal = f.name
                        .split('_')
                        .map(|part| {
                            let mut chars = part.chars();
                            match chars.next() {
                                None => String::new(),
                                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                            }
                        })
                        .collect::<String>();
                    ContextValue::String(pascal)
                }
                _ => ContextValue::Null,
            },
            ContextValue::TypeRef(t) => match name {
                "original" => ContextValue::String(t.original.clone()),
                "fqn" => ContextValue::String(t.fqn.clone()),
                "namespace_fqn" => ContextValue::String(t.namespace_fqn.clone()),
                "type_name" => ContextValue::String(t.type_name.clone()),
                "parent_type_path" => ContextValue::String(t.parent_type_path.clone()),
                "lang_type" => ContextValue::String(t.lang_type.clone()),
                "is_primitive" => ContextValue::Bool(t.is_primitive),
                "is_struct" => ContextValue::Bool(t.is_struct),
                "is_enum" => ContextValue::Bool(t.is_enum),
                "is_option" => ContextValue::Bool(t.is_option),
                "is_list" => ContextValue::Bool(t.is_list),
                "is_float" => ContextValue::Bool(
                    t.type_name == "f32" || t.type_name == "f64"
                ),
                "is_unsigned" => ContextValue::Bool(
                    t.type_name.starts_with('u') && t.is_primitive
                ),
                "is_string" => ContextValue::Bool(t.type_name == "string"),
                "inner_type" | "inner" => match &t.inner_type {
                    Some(inner) => ContextValue::TypeRef(*inner.clone()),
                    None => ContextValue::Null,
                },
                _ => ContextValue::Null,
            },
            ContextValue::Enum(e) => match name {
                "name" => ContextValue::String(e.name.clone()),
                "fqn" => ContextValue::String(e.fqn.clone()),
                "items" => ContextValue::List(
                    e.items
                        .iter()
                        .map(|item| ContextValue::EnumItem(item.clone()))
                        .collect(),
                ),
                "members" => {
                    let members: Vec<ContextValue> = e
                        .items
                        .iter()
                        .filter_map(|item| match item {
                            EnumItem::Member(m) => Some(ContextValue::EnumMember(m.clone())),
                            _ => None,
                        })
                        .collect();
                    ContextValue::List(members)
                }
                "use_iota" => {
                    // Check if all member values are sequential starting from 0
                    let mut expected = 0i64;
                    let mut sequential = true;
                    for item in &e.items {
                        if let EnumItem::Member(m) = item {
                            match m.value {
                                Some(v) if v != expected => {
                                    sequential = false;
                                    break;
                                }
                                _ => {}
                            }
                            expected += 1;
                        }
                    }
                    ContextValue::Bool(sequential)
                }
                "first_member_name" => e
                    .items
                    .iter()
                    .find_map(|item| match item {
                        EnumItem::Member(m) => Some(ContextValue::String(m.name.clone())),
                        _ => None,
                    })
                    .unwrap_or(ContextValue::Null),
                // Items before the first member (typically doc comments)
                "items_before_first_member" => {
                    let items: Vec<ContextValue> = e
                        .items
                        .iter()
                        .take_while(|item| !matches!(item, EnumItem::Member(_)))
                        .map(|item| ContextValue::EnumItem(item.clone()))
                        .collect();
                    ContextValue::List(items)
                }
                // Items after the first member (remaining members + comments)
                "items_after_first_member" => {
                    let mut found_first = false;
                    let items: Vec<ContextValue> = e
                        .items
                        .iter()
                        .filter(|item| {
                            if found_first {
                                return true;
                            }
                            if matches!(item, EnumItem::Member(_)) {
                                found_first = true;
                                return false; // skip the first member
                            }
                            false // skip items before first member
                        })
                        .map(|item| ContextValue::EnumItem(item.clone()))
                        .collect();
                    ContextValue::List(items)
                }
                _ => ContextValue::Null,
            },
            ContextValue::EnumItem(item) => match name {
                "is_member" => ContextValue::Bool(matches!(item, EnumItem::Member(_))),
                "is_comment" => ContextValue::Bool(matches!(item, EnumItem::Comment(_))),
                "as_member" => match item {
                    EnumItem::Member(m) => ContextValue::EnumMember(m.clone()),
                    _ => ContextValue::Null,
                },
                "as_comment" => match item {
                    EnumItem::Comment(c) => ContextValue::String(c.clone()),
                    _ => ContextValue::Null,
                },
                _ => ContextValue::Null,
            },
            ContextValue::EnumMember(m) => match name {
                "name" => ContextValue::String(m.name.clone()),
                "value" => match m.value {
                    Some(v) => ContextValue::Int(v),
                    None => ContextValue::Null,
                },
                "has_value" => ContextValue::Bool(m.value.is_some()),
                _ => ContextValue::Null,
            },
            ContextValue::Annotation(a) => match name {
                "name" => ContextValue::String(a.name.clone()),
                "positional_args" => ContextValue::List(
                    a.positional_args
                        .iter()
                        .map(|arg| ContextValue::String(arg.clone()))
                        .collect(),
                ),
                "params" => ContextValue::List(
                    a.params
                        .iter()
                        .map(|p| ContextValue::AnnotationParam(p.clone()))
                        .collect(),
                ),
                "has_params" => ContextValue::Bool(!a.params.is_empty()),
                _ => ContextValue::Null,
            },
            ContextValue::AnnotationParam(p) => match name {
                "key" => ContextValue::String(p.key.clone()),
                "value" => ContextValue::String(p.value.clone()),
                _ => ContextValue::Null,
            },
            ContextValue::Index(idx) => match name {
                "name" => ContextValue::String(idx.name.clone()),
                "is_unique" => ContextValue::Bool(idx.is_unique),
                "is_composite" => ContextValue::Bool(idx.is_composite()),
                "field_count" => ContextValue::Int(idx.field_count() as i64),
                "source" => ContextValue::String(idx.source.clone()),
                "field_name" => ContextValue::String(idx.field_name().to_string()),
                "field_type" => match idx.field_type() {
                    Some(t) => ContextValue::TypeRef(t.clone()),
                    None => ContextValue::Null,
                },
                "fields" => ContextValue::List(
                    idx.fields.iter().map(|f| ContextValue::IndexField(f.clone())).collect(),
                ),
                _ => ContextValue::Null,
            },
            ContextValue::IndexField(ifd) => match name {
                "name" => ContextValue::String(ifd.name.clone()),
                "field_type" => ContextValue::TypeRef(ifd.field_type.clone()),
                _ => ContextValue::Null,
            },
            ContextValue::Relation(rel) => match name {
                "name" => ContextValue::String(rel.name.clone()),
                "source_table_fqn" => ContextValue::String(rel.source_table_fqn.clone()),
                "source_table_name" => ContextValue::String(rel.source_table_name.clone()),
                "source_field" => ContextValue::String(rel.source_field.clone()),
                "source_field_pascal" => {
                    // Convert snake_case to PascalCase: "player_id" → "PlayerId"
                    let pascal = rel.source_field
                        .split('_')
                        .map(|part| {
                            let mut chars = part.chars();
                            match chars.next() {
                                None => String::new(),
                                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                            }
                        })
                        .collect::<String>();
                    ContextValue::String(pascal)
                }
                _ => ContextValue::Null,
            },
            ContextValue::ForeignKey(fk) => match name {
                "target_table_fqn" => ContextValue::String(fk.target_table_fqn.clone()),
                "target_field" => ContextValue::String(fk.target_field.clone()),
                "target_table_name" => {
                    // Extract last segment of FQN: "game.character.Player" → "Player"
                    let table_name = fk.target_table_fqn
                        .rsplit('.')
                        .next()
                        .unwrap_or(&fk.target_table_fqn)
                        .to_string();
                    ContextValue::String(table_name)
                }
                "alias" => match &fk.alias {
                    Some(a) => ContextValue::String(a.clone()),
                    None => ContextValue::Null,
                },
                _ => ContextValue::Null,
            },
            ContextValue::Range(r) => match name {
                "min" => ContextValue::String(r.min.clone()),
                "max" => ContextValue::String(r.max.clone()),
                "literal_type" => ContextValue::String(r.literal_type.clone()),
                _ => ContextValue::Null,
            },
            ContextValue::Timezone(tz) => match name {
                "kind" => ContextValue::String(tz.kind.clone()),
                "name" => match &tz.name {
                    Some(n) => ContextValue::String(n.clone()),
                    None => ContextValue::Null,
                },
                "offset_hours" => match tz.offset_hours {
                    Some(h) => ContextValue::Int(h as i64),
                    None => ContextValue::Int(0),
                },
                "offset_minutes" => match tz.offset_minutes {
                    Some(m) => ContextValue::Int(m as i64),
                    None => ContextValue::Int(0),
                },
                _ => ContextValue::Null,
            },
            _ => ContextValue::Null,
        }
    }

    /// Resolve a property chain (e.g. `["field", "field_type", "is_option"]`).
    pub fn resolve_path(&self, path: &[String]) -> ContextValue {
        let mut current = self.clone();
        for segment in path {
            current = current.get_property(segment);
            if matches!(current, ContextValue::Null) {
                return ContextValue::Null;
            }
        }
        current
    }

    /// Convert this value to a display string.
    pub fn to_display_string(&self) -> String {
        match self {
            ContextValue::String(s) => s.clone(),
            ContextValue::Bool(b) => b.to_string(),
            ContextValue::Int(i) => i.to_string(),
            ContextValue::Float(f) => f.to_string(),
            ContextValue::AnnotationParam(p) => format!("{} = \"{}\"", p.key, p.value),
            ContextValue::Null => String::new(),
            _ => format!("[{:?}]", std::mem::discriminant(self)),
        }
    }

    /// Evaluate this value as a boolean (for conditions).
    pub fn is_truthy(&self) -> bool {
        match self {
            ContextValue::Bool(b) => *b,
            ContextValue::String(s) => !s.is_empty(),
            ContextValue::Int(i) => *i != 0,
            ContextValue::Float(f) => *f != 0.0,
            ContextValue::List(l) => !l.is_empty(),
            ContextValue::Null => false,
            // IR objects are truthy (they exist)
            _ => true,
        }
    }

    /// Try to get this value as a list.
    pub fn as_list(&self) -> Option<&Vec<ContextValue>> {
        match self {
            ContextValue::List(l) => Some(l),
            _ => None,
        }
    }
}

/// Recursively collects all non-__Enum structs from namespaces.
///
/// Returns them as ContextValue::Struct entries in a flat list.
/// Note: embeds ARE included (they may be used as container tables).
fn collect_tables_from_namespaces(namespaces: &[NamespaceDef], out: &mut Vec<ContextValue>) {
    for ns in namespaces {
        for item in &ns.items {
            match item {
                NamespaceItem::Struct(s) => {
                    if !s.name.ends_with("__Enum") {
                        out.push(ContextValue::Struct(s.clone()));
                    }
                }
                NamespaceItem::Namespace(child_ns) => {
                    collect_tables_from_namespaces(&[*child_ns.clone()], out);
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_struct() -> StructDef {
        StructDef {
            name: "Player".to_string(),
            fqn: "game.Player".to_string(),
            is_embed: false,
            datasource: None,
            cache_strategy: None,
            is_readonly: false,
            soft_delete_field: None,
            pack_separator: None,
            header: vec![StructItem::Comment("A player".to_string())],
            items: vec![StructItem::Field(Box::new(FieldDef {
                name: "id".to_string(),
                field_type: TypeRef {
                    original: "u32".to_string(),
                    fqn: "u32".to_string(),
                    namespace_fqn: String::new(),
                    type_name: "u32".to_string(),
                    parent_type_path: String::new(),
                    lang_type: "uint".to_string(),
                    is_primitive: true,
                    is_struct: false,
                    is_enum: false,
                    is_option: false,
                    is_list: false,
                    inner_type: None,
                },
                attributes: vec!["Key".to_string()],
                is_primary_key: true,
                is_unique: false,
                is_index: false,
                foreign_key: None,
                max_length: None,
                default_value: None,
                range: None,
                regex_pattern: None,
                auto_create: None,
                auto_update: None,
            }))],
            indexes: vec![],
            relations: vec![],
        }
    }

    #[test]
    fn test_struct_property_access() {
        let s = make_test_struct();
        let cv = ContextValue::Struct(s);

        assert_eq!(cv.get_property("name").to_display_string(), "Player");
        assert_eq!(cv.get_property("fqn").to_display_string(), "game.Player");
        assert_eq!(cv.get_property("is_embed").is_truthy(), false);
    }

    #[test]
    fn test_resolve_path() {
        let s = make_test_struct();
        let cv = ContextValue::Struct(s);

        // struct.name
        let result = cv.resolve_path(&["name".to_string()]);
        assert_eq!(result.to_display_string(), "Player");

        // struct.doc_comments (filtered from header)
        let result = cv.resolve_path(&["doc_comments".to_string()]);
        let list = result.as_list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].to_display_string(), "A player");
    }

    #[test]
    fn test_context_bindings() {
        let mut ctx = TemplateContext::new();
        ctx.set("name", ContextValue::String("Test".to_string()));

        assert_eq!(ctx.get("name").unwrap().to_display_string(), "Test");
        assert!(ctx.get("missing").is_none());

        let child = ctx.child_with("age", ContextValue::Int(42));
        assert_eq!(child.get("name").unwrap().to_display_string(), "Test");
        assert_eq!(child.get("age").unwrap().to_display_string(), "42");
    }

    #[test]
    fn test_truthiness() {
        assert!(ContextValue::Bool(true).is_truthy());
        assert!(!ContextValue::Bool(false).is_truthy());
        assert!(ContextValue::String("hello".to_string()).is_truthy());
        assert!(!ContextValue::String(String::new()).is_truthy());
        assert!(!ContextValue::Null.is_truthy());
        assert!(ContextValue::Int(1).is_truthy());
        assert!(!ContextValue::Int(0).is_truthy());
    }
}
