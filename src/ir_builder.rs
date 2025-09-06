use crate::ast_model::{self, Definition, Metadata, TableMember};
use crate::ir_model::{
    self, AnnotationDef, AnnotationParam, EnumDef, EnumItem, FieldDef, FileDef, NamespaceDef,
    NamespaceItem, SchemaContext, StructDef, StructItem, TypeRef,
};
use heck::ToPascalCase;

/// Builds the template-friendly Intermediate Representation (IR) from the AST definitions.
pub fn build_ir(asts: &[ast_model::AstRoot]) -> ir_model::SchemaContext {
    let mut context = SchemaContext::default();

    for ast in asts {
        let file_name = ast
            .path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let mut top_level_namespaces = Vec::new();
        let mut global_items = Vec::new();

        // Process definitions at the root of the file.
        for def in &ast.definitions {
            // Top-level namespace blocks are handled specially to form the root of the hierarchy.
            if let Definition::Namespace(ns_ast) = def {
                let ns_name = ns_ast.path.join(".");
                let mut new_ns = NamespaceDef {
                    name: ns_name.clone(),
                    items: Vec::new(),
                };
                // Recurse into this new top-level namespace
                populate_items_recursively(&mut new_ns.items, &ns_ast.definitions, &ns_name);
                top_level_namespaces.push(new_ns);
            } else {
                // Any other item at the top level belongs to the "global" namespace.
                add_definition_to_items(&mut global_items, def, "");
            }
        }

        // If there were global items, create a global namespace for them (name is empty).
        if !global_items.is_empty() {
            let global_ns = NamespaceDef {
                name: "".to_string(),
                items: global_items,
            };
            top_level_namespaces.insert(0, global_ns);
        }

        let file_def = FileDef {
            path: file_name,
            namespaces: top_level_namespaces,
        };
        context.files.push(file_def);
    }

    // After building the raw IR, resolve TypeRef flags (is_enum/is_struct) using the collected defs
    resolve_type_kinds(&mut context);

    context
}

/// Recursively populates a list of items from AST definitions.
fn populate_items_recursively(
    items: &mut Vec<NamespaceItem>,
    definitions: &[ast_model::Definition],
    current_ns: &str,
) {
    for def in definitions {
        add_definition_to_items(items, def, current_ns);
    }
}

/// Converts a single AST Definition into a NamespaceItem and adds it to a list.
/// This is the core of the recursive build process.
fn add_definition_to_items(items: &mut Vec<NamespaceItem>, def: &Definition, current_ns: &str) {
    match def {
        Definition::Namespace(ns_ast) => {
            let this = ns_ast.path.join(".");
            let next_ns = if current_ns.is_empty() {
                this.clone()
            } else if this.is_empty() {
                current_ns.to_string()
            } else {
                format!("{}.{}", current_ns, this)
            };
            let mut new_ns = NamespaceDef {
                name: next_ns.clone(),
                items: Vec::new(),
            };
            populate_items_recursively(&mut new_ns.items, &ns_ast.definitions, &next_ns);
            items.push(NamespaceItem::Namespace(Box::new(new_ns)));
        }
        Definition::Table(table) => {
            items.push(NamespaceItem::Struct(convert_table_to_struct(table, current_ns)));
        }
        Definition::Enum(e) => {
            items.push(NamespaceItem::Enum(convert_enum_to_enum_def(e, None, current_ns)));
        }
        Definition::Embed(embed) => {
            items.push(NamespaceItem::Struct(convert_embed_to_struct(embed, current_ns)));
        }
        Definition::Comment(c) => {
            items.push(NamespaceItem::Comment(c.clone()));
        }
        Definition::Annotation(_) => { /* Annotations are handled within other items, not as top-level IR items */ }
    }
}

use std::collections::{HashMap, HashSet};

// Traverse the built IR and fix TypeRef.is_enum / is_struct flags based on declared enums
fn resolve_type_kinds(ctx: &mut SchemaContext) {
    let mut enum_fqns: HashSet<String> = HashSet::new();
    let mut enum_by_ns_and_name: HashSet<(String, String)> = HashSet::new();
    let mut enum_by_name: HashMap<String, HashSet<String>> = HashMap::new();

    // Collect all enum FQNs from all files/namespaces (including inline enums)
    for file in &ctx.files {
        for ns in &file.namespaces {
            collect_enum_fqns_from_namespace_items(&ns.items, &mut enum_fqns, &mut enum_by_ns_and_name, &mut enum_by_name);
        }
    }

    // Adjust all TypeRefs in struct fields recursively
    for file in &mut ctx.files {
        for ns in &mut file.namespaces {
            adjust_namespace_items_typerefs(&mut ns.items, &enum_fqns, &enum_by_ns_and_name, &enum_by_name);
        }
    }
}

fn collect_enum_fqns_from_namespace_items(
    items: &Vec<NamespaceItem>,
    set: &mut HashSet<String>,
    by_ns_and_name: &mut HashSet<(String, String)>,
    by_name: &mut HashMap<String, HashSet<String>>,
) {
    for it in items {
        match it {
            NamespaceItem::Enum(e) => {
                set.insert(e.fqn.clone());
                let ns = namespace_of_owned(&e.fqn);
                by_ns_and_name.insert((ns.clone(), last_segment_owned(&e.fqn)));
                by_name.entry(last_segment_owned(&e.fqn)).or_default().insert(e.fqn.clone());
            }
            NamespaceItem::Struct(s) => {
                collect_enum_fqns_from_struct(s, set, by_ns_and_name, by_name);
            }
            NamespaceItem::Namespace(inner) => {
                collect_enum_fqns_from_namespace_items(&inner.items, set, by_ns_and_name, by_name);
            }
            NamespaceItem::Comment(_) => {}
        }
    }
}

fn collect_enum_fqns_from_struct(
    s: &StructDef,
    set: &mut HashSet<String>,
    by_ns_and_name: &mut HashSet<(String, String)>,
    by_name: &mut HashMap<String, HashSet<String>>,
) {
    for item in &s.items {
        match item {
            StructItem::InlineEnum(e) => {
                set.insert(e.fqn.clone());
                let ns = namespace_of_owned(&e.fqn);
                by_ns_and_name.insert((ns.clone(), last_segment_owned(&e.fqn)));
                by_name.entry(last_segment_owned(&e.fqn)).or_default().insert(e.fqn.clone());
            }
            StructItem::EmbeddedStruct(sub) => collect_enum_fqns_from_struct(sub, set, by_ns_and_name, by_name),
            StructItem::Field(_) | StructItem::Annotation(_) | StructItem::Comment(_) => {}
        }
    }
}

fn adjust_namespace_items_typerefs(
    items: &mut Vec<NamespaceItem>,
    enum_fqns: &HashSet<String>,
    by_ns_and_name: &HashSet<(String, String)>,
    by_name: &HashMap<String, HashSet<String>>,
) {
    for it in items {
        match it {
            NamespaceItem::Struct(ref mut s) => adjust_struct_typerefs(s, enum_fqns, by_ns_and_name, by_name),
            NamespaceItem::Namespace(ref mut inner) => adjust_namespace_items_typerefs(&mut inner.items, enum_fqns, by_ns_and_name, by_name),
            NamespaceItem::Enum(_) | NamespaceItem::Comment(_) => {}
        }
    }
}

fn adjust_struct_typerefs(
    s: &mut StructDef,
    enum_fqns: &HashSet<String>,
    by_ns_and_name: &HashSet<(String, String)>,
    by_name: &HashMap<String, HashSet<String>>,
) {
    for item in &mut s.items {
        match item {
            StructItem::Field(ref mut f) => {
                adjust_typeref(&mut f.field_type, enum_fqns, by_ns_and_name, by_name);
            }
            StructItem::EmbeddedStruct(ref mut sub) => adjust_struct_typerefs(sub, enum_fqns, by_ns_and_name, by_name),
            StructItem::InlineEnum(_) | StructItem::Annotation(_) | StructItem::Comment(_) => {}
        }
    }
}

fn adjust_typeref(
    t: &mut TypeRef,
    enum_fqns: &HashSet<String>,
    by_ns_and_name: &HashSet<(String, String)>,
    by_name: &HashMap<String, HashSet<String>>,
) {
    if let Some(inner) = &mut t.inner_type {
        adjust_typeref(inner.as_mut(), enum_fqns, by_ns_and_name, by_name);
    }

    if !t.is_primitive {
        if enum_fqns.contains(&t.fqn) {
            t.is_enum = true;
            t.is_struct = false;
            return;
        }

        // Fallback 1: same-namespace + type name match
        if by_ns_and_name.contains(&(t.namespace_fqn.clone(), t.type_name.clone())) {
            let fqn = if t.namespace_fqn.is_empty() {
                t.type_name.clone()
            } else {
                format!("{}.{}", t.namespace_fqn, t.type_name)
            };
            if enum_fqns.contains(&fqn) {
                t.fqn = fqn;
                t.is_enum = true;
                t.is_struct = false;
                return;
            }
        }

        // Fallback 2: unique type name across all enums
        if let Some(set) = by_name.get(&t.type_name) {
            if set.len() == 1 {
                if let Some(only_fqn) = set.iter().next() {
                    t.fqn = only_fqn.clone();
                    t.namespace_fqn = namespace_of_owned(only_fqn);
                    t.is_enum = true;
                    t.is_struct = false;
                    return;
                }
            }
        }

        // Default: treat as struct
        t.is_enum = false;
        t.is_struct = true;
    }
}

fn convert_table_to_struct(table: &ast_model::Table, current_ns: &str) -> StructDef {
    let mut items = Vec::new();
    let mut header_items = Vec::new();
    let name = table.name.clone().unwrap();
    let fqn = if current_ns.is_empty() {
        name.clone()
    } else {
        format!("{}.{}", current_ns, name)
    };

    // Process metadata for the struct header
    for meta in &table.metadata {
        match meta {
            Metadata::DocComment(c) => {
                header_items.push(StructItem::Comment(c.clone()));
            }
            Metadata::Annotation(a) => {
                let annotation_def = convert_annotations_to_ir(&[a.clone()])[0].clone();
                header_items.push(StructItem::Annotation(annotation_def));
            }
        }
    }

    for member in &table.members {
        match member {
            TableMember::Field(field) => {
                // Handle metadata (comments, annotations) associated with the field first
                let field_metadata = match field {
                    ast_model::FieldDefinition::Regular(rf) => &rf.metadata,
                    ast_model::FieldDefinition::InlineEmbed(ief) => &ief.metadata,
                    ast_model::FieldDefinition::InlineEnum(e) => &e.metadata,
                };
                for meta in field_metadata {
                    match meta {
                        Metadata::DocComment(c) => items.push(StructItem::Comment(c.clone())),
                        Metadata::Annotation(a) => {
                            let annotation_def = convert_annotations_to_ir(&[a.clone()])[0].clone();
                            items.push(StructItem::Annotation(annotation_def));
                        }
                    }
                }

                // Then handle the field itself
                let (field_def, mut new_nested_structs, mut new_nested_enums) =
                    convert_field_to_ir(field, current_ns, &fqn);
                items.push(StructItem::Field(field_def));
                // Add the new nested types to the items list
                items.extend(new_nested_structs.into_iter().map(StructItem::EmbeddedStruct));
                items.extend(new_nested_enums.into_iter().map(StructItem::InlineEnum));
            }
            TableMember::Embed(embed) => {
                items.push(StructItem::EmbeddedStruct(convert_embed_to_struct(embed, &fqn)));
            }
            TableMember::Enum(e) => {
                items.push(StructItem::InlineEnum(convert_enum_to_enum_def(e, None, &fqn)));
            }
            TableMember::Comment(c) => items.push(StructItem::Comment(c.clone())),
        }
    }

    StructDef {
        name,
        fqn,
        items,
        header: header_items,
    }
}

/// Converts an `ast_model::FieldDefinition` into an `ir_model::FieldDef` and potential nested types (from inline embeds).
fn convert_field_to_ir(
    field: &ast_model::FieldDefinition,
    current_ns: &str,
    owner_fqn: &str,
) -> (FieldDef, Vec<StructDef>, Vec<EnumDef>) {
    match field {
        ast_model::FieldDefinition::Regular(rf) => {
            let attributes = convert_constraints_to_attributes(&rf.constraints);
            let mut inline_enums = Vec::new();
            let field_type: TypeRef;

            match &rf.field_type.base_type {
                ast_model::TypeName::InlineEnum(e) => {
                    // Generate a unique name for the inline enum
                    // For now, let's use FieldName_Enum. We'll need table context later for better names.
                    let generated_enum_name = format!("{}_Enum", rf.name.clone().expect("Regular field name must be present").to_pascal_case());
                    
                    // Create the EnumDef using the generated name
                    let enum_fqn = if owner_fqn.is_empty() { generated_enum_name.clone() } else { format!("{}.{}", owner_fqn, generated_enum_name) };
                    let enum_def = convert_enum_to_enum_def(e, Some(generated_enum_name.clone()), owner_fqn);
                    inline_enums.push(enum_def);
                    field_type = build_type_ref_from_base(&enum_fqn, &generated_enum_name, &rf.field_type.cardinality, false);
                },
                _ => {
                    field_type = build_type_ref(&rf.field_type, current_ns);
                },
            };

            (
                FieldDef {
                    name: rf.name.clone().expect("Regular field name must be present"),
                    field_type,
                    attributes,
                },
                Vec::new(),
                inline_enums,
            )
        }
        ast_model::FieldDefinition::InlineEmbed(ief) => {
            let struct_name = ief.name.clone().expect("Inline embed field name must be present").to_pascal_case();
            let inline_struct = convert_table_to_struct(&ast_model::Table {
                name: Some(struct_name.clone()),
                metadata: ief.metadata.clone(),
                members: ief.members.clone(),
            }, owner_fqn);

            let nested_items = vec![inline_struct];

            let field_def = FieldDef {
                name: ief.name.clone().expect("Inline embed field name must be present"),
                field_type: build_type_ref_from_base(
                    &format!("{}.{}", owner_fqn, struct_name),
                    &struct_name,
                    &ief.cardinality,
                    false,
                ),
                attributes: Vec::new(),
            };
            (field_def, nested_items, Vec::new())
        }
        ast_model::FieldDefinition::InlineEnum(e) => {
            let generated_enum_name = format!("{}__Enum", e.name.clone().expect("Inline enum name must be present").to_pascal_case());
            
            // Create a temporary Enum from InlineEnumField
            let temp_enum = ast_model::Enum {
                metadata: e.metadata.clone(),
                name: e.name.clone(),
                variants: e.variants.clone(),
            };

            let enum_def = convert_enum_to_enum_def(&temp_enum, Some(generated_enum_name.clone()), owner_fqn);
            
            let field_def = FieldDef {
                name: e.name.clone().expect("Inline enum name must be present"),
                field_type: build_type_ref_from_base(
                    &format!("{}.{}", owner_fqn, generated_enum_name),
                    &generated_enum_name,
                    &e.cardinality,
                    false,
                ),
                attributes: Vec::new(),
            };
            (field_def, Vec::new(), vec![enum_def])
        }
    }
}

/// Converts field constraints from the AST into a vector of strings
/// suitable for C# attributes.
fn convert_constraints_to_attributes(constraints: &[ast_model::Constraint]) -> Vec<String> {
    constraints
        .iter()
        .filter_map(|c| match c {
            // `primary_key` is mapped to the `[Key]` attribute, common in ORMs like EF Core.
            ast_model::Constraint::PrimaryKey => Some("Key".to_string()),
            // `unique` can be mapped to an index attribute.
            ast_model::Constraint::Unique => Some("Index(IsUnique = true)".to_string()),
            ast_model::Constraint::MaxLength(len) => Some(format!("MaxLength({})", len)),
            // ForeignKey is a relationship, not a simple attribute, so we ignore it here.
            ast_model::Constraint::ForeignKey(_, _) => None,
            // Other constraints are not (yet) represented as attributes.
            _ => None,
        })
        .collect()
}

fn convert_enum_to_enum_def(e: &ast_model::Enum, name_override: Option<String>, ns_or_owner_fqn: &str) -> EnumDef {
    let mut items = Vec::new();
    let mut current_value: i64 = 0; // Initialize counter for sequential numbering

    // Extract enum's own comment and add to items
    for meta in &e.metadata {
        if let Metadata::DocComment(c) = meta {
            items.push(EnumItem::Comment(c.clone()));
            break; // Assuming only one doc comment for the enum itself
        }
    }

    for variant in &e.variants {
        for meta in &variant.metadata {
            if let Metadata::DocComment(c) = meta {
                items.push(EnumItem::Comment(c.clone()));
                println!("[ir_builder] comment 2: {:?}", c);
            }
        }

        let member_value = if let Some(explicit_value) = variant.value {
            // If an explicit value is provided, use it and update the counter
            current_value = explicit_value;
            Some(explicit_value)
        } else {
            // If no explicit value, use the current sequential value
            let value_to_assign = current_value;
            current_value += 1; // Increment for the next member
            Some(value_to_assign)
        };

        items.push(EnumItem::Member(ir_model::EnumMember {
            name: variant.name.clone().unwrap(),
            value: member_value,
        }));
    }

    let name = name_override.unwrap_or_else(|| e.name.clone().expect("Named enum must have a name"));
    let fqn = if ns_or_owner_fqn.is_empty() { name.clone() } else { format!("{}.{}", ns_or_owner_fqn, name) };
    EnumDef { name, fqn, items }
}

fn convert_embed_to_struct(embed: &ast_model::Embed, owner_fqn: &str) -> StructDef {
    convert_table_to_struct(&ast_model::Table {
        name: embed.name.clone(),
        metadata: embed.metadata.clone(),
        members: embed.members.clone(),
    }, owner_fqn)
}

// Build TypeRef from an AST type in the given namespace context
fn build_type_ref(t: &ast_model::TypeWithCardinality, current_ns: &str) -> TypeRef {
    let (base_fqn, base_name, is_primitive, is_enum) = match &t.base_type {
        ast_model::TypeName::Path(p) => (p.join("."), p.join("."), false, false),
        ast_model::TypeName::Basic(b) => (basic_name(b).to_string(), basic_name(b).to_string(), true, false),
        ast_model::TypeName::InlineEnum(_) => ("__ANON_ENUM__".to_string(), "__ANON_ENUM__".to_string(), false, true),
    };
    match t.cardinality {
        Some(ast_model::Cardinality::Optional) => {
            let inner_fqn = if is_primitive { base_fqn.clone() } else { qualify(&base_fqn, current_ns) };
            let inner = TypeRef {
                original: base_name.clone(),
                fqn: inner_fqn.clone(),
                namespace_fqn: namespace_of_owned(&inner_fqn),
                type_name: last_segment_owned(&inner_fqn),
                lang_type: base_name.clone(),
                is_primitive,
                is_struct: !(is_primitive || is_enum),
                is_enum,
                is_option: false,
                is_list: false,
                inner_type: None,
            };
            TypeRef {
                original: format!("Option<{}>", base_name),
                fqn: inner_fqn.clone(),
                namespace_fqn: namespace_of_owned(&inner_fqn),
                type_name: last_segment_owned(&inner_fqn),
                lang_type: format!("Option<{}>", inner.lang_type),
                is_primitive: false,
                is_struct: !(is_primitive || is_enum),
                is_enum,
                is_option: true,
                is_list: false,
                inner_type: Some(Box::new(inner)),
            }
        }
        Some(ast_model::Cardinality::Array) => {
            let inner_fqn = if is_primitive { base_fqn.clone() } else { qualify(&base_fqn, current_ns) };
            let inner = TypeRef {
                original: base_name.clone(),
                fqn: inner_fqn.clone(),
                namespace_fqn: namespace_of_owned(&inner_fqn),
                type_name: last_segment_owned(&inner_fqn),
                lang_type: base_name.clone(),
                is_primitive,
                is_struct: !(is_primitive || is_enum),
                is_enum,
                is_option: false,
                is_list: false,
                inner_type: None,
            };
            TypeRef {
                original: format!("List<{}>", base_name),
                fqn: inner_fqn.clone(),
                namespace_fqn: namespace_of_owned(&inner_fqn),
                type_name: last_segment_owned(&inner_fqn),
                lang_type: format!("List<{}>", inner.lang_type),
                is_primitive: false,
                is_struct: !(is_primitive || is_enum),
                is_enum,
                is_option: false,
                is_list: true,
                inner_type: Some(Box::new(inner)),
            }
        }
        None => {
            let core_fqn = if is_primitive { base_fqn.clone() } else { qualify(&base_fqn, current_ns) };
            TypeRef {
                original: base_name.clone(),
                fqn: core_fqn.clone(),
                namespace_fqn: namespace_of_owned(&core_fqn),
                type_name: last_segment_owned(&core_fqn),
                lang_type: base_name,
                is_primitive,
                is_struct: !(is_primitive || is_enum),
                is_enum,
                is_option: false,
                is_list: false,
                inner_type: None,
            }
        }
    }
}

fn build_type_ref_from_base(base_fqn: &str, base_name: &str, c: &Option<ast_model::Cardinality>, primitive_hint: bool) -> TypeRef {
    match c {
        Some(ast_model::Cardinality::Optional) => {
            let inner = TypeRef {
                original: base_name.to_string(),
                fqn: base_fqn.to_string(),
                namespace_fqn: namespace_of_owned(base_fqn),
                type_name: last_segment_owned(base_fqn),
                lang_type: base_name.to_string(),
                is_primitive: primitive_hint,
                is_struct: !primitive_hint,
                is_enum: false,
                is_option: false,
                is_list: false,
                inner_type: None,
            };
            TypeRef {
                original: format!("Option<{}>", base_name),
                fqn: base_fqn.to_string(),
                namespace_fqn: namespace_of_owned(base_fqn),
                type_name: last_segment_owned(base_fqn),
                lang_type: format!("Option<{}>", base_name),
                is_primitive: false,
                is_struct: !primitive_hint,
                is_enum: false,
                is_option: true,
                is_list: false,
                inner_type: Some(Box::new(inner)),
            }
        }
        Some(ast_model::Cardinality::Array) => {
            let inner = TypeRef {
                original: base_name.to_string(),
                fqn: base_fqn.to_string(),
                namespace_fqn: namespace_of_owned(base_fqn),
                type_name: last_segment_owned(base_fqn),
                lang_type: base_name.to_string(),
                is_primitive: primitive_hint,
                is_struct: !primitive_hint,
                is_enum: false,
                is_option: false,
                is_list: false,
                inner_type: None,
            };
            TypeRef {
                original: format!("List<{}>", base_name),
                fqn: base_fqn.to_string(),
                namespace_fqn: namespace_of_owned(base_fqn),
                type_name: last_segment_owned(base_fqn),
                lang_type: format!("List<{}>", base_name),
                is_primitive: false,
                is_struct: !primitive_hint,
                is_enum: false,
                is_option: false,
                is_list: true,
                inner_type: Some(Box::new(inner)),
            }
        }
        None => TypeRef {
            original: base_name.to_string(),
            fqn: base_fqn.to_string(),
            namespace_fqn: namespace_of_owned(base_fqn),
            type_name: last_segment_owned(base_fqn),
            lang_type: base_name.to_string(),
            is_primitive: primitive_hint,
            is_struct: !primitive_hint,
            is_enum: false,
            is_option: false,
            is_list: false,
            inner_type: None,
        },
    }
}

fn qualify(base_fqn: &str, current_ns: &str) -> String {
    if base_fqn.contains('.') || current_ns.is_empty() {
        base_fqn.to_string()
    } else {
        format!("{}.{}", current_ns, base_fqn)
    }
}

fn namespace_of_owned(fqn: &str) -> String {
    match fqn.rfind('.') {
        Some(i) => fqn[..i].to_string(),
        None => String::new(),
    }
}

fn last_segment_owned(fqn: &str) -> String {
    match fqn.rfind('.') {
        Some(i) => fqn[i + 1..].to_string(),
        None => fqn.to_string(),
    }
}

fn basic_name(b: &ast_model::BasicType) -> &'static str {
    use ast_model::BasicType::*;
    match b {
        String => "string",
        I8 => "i8",
        I16 => "i16",
        I32 => "i32",
        I64 => "i64",
        U8 => "u8",
        U16 => "u16",
        U32 => "u32",
        U64 => "u64",
        F32 => "f32",
        F64 => "f64",
        Bool => "bool",
        Bytes => "bytes",
    }
}

fn convert_annotations_to_ir(annotations: &[ast_model::Annotation]) -> Vec<AnnotationDef> {
    annotations
        .iter()
        .map(|ast_ann| AnnotationDef {
            name: ast_ann.name.clone().unwrap(),
            params: ast_ann
                .params
                .iter()
                .map(|p| AnnotationParam {
                    key: p.key.clone(),
                    value: p.value.to_string(), // Assuming value is a simple literal
                })
                .collect(),
        })
        .collect()
}
