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
                    field_type = build_type_ref_from_base(&enum_fqn, &generated_enum_name, &rf.field_type.cardinality, true);
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
    let (base_fqn, base_name, is_primitive) = match &t.base_type {
        ast_model::TypeName::Path(p) => (p.join("."), p.join("."), false),
        ast_model::TypeName::Basic(b) => (basic_name(b).to_string(), basic_name(b).to_string(), true),
        ast_model::TypeName::InlineEnum(_) => ("__ANON_ENUM__".to_string(), "__ANON_ENUM__".to_string(), false),
    };
    match t.cardinality {
        Some(ast_model::Cardinality::Optional) => {
            let inner = TypeRef {
                original: base_name.clone(),
                fqn: qualify(&base_fqn, current_ns),
                lang_type: base_name.clone(),
                is_primitive,
                is_option: false,
                is_list: false,
                inner_type: None,
            };
            TypeRef {
                original: format!("Option<{}>", base_name),
                fqn: qualify(&base_fqn, current_ns),
                lang_type: format!("Option<{}>", inner.lang_type),
                is_primitive: is_primitive,
                is_option: true,
                is_list: false,
                inner_type: Some(Box::new(inner)),
            }
        }
        Some(ast_model::Cardinality::Array) => {
            let inner = TypeRef {
                original: base_name.clone(),
                fqn: qualify(&base_fqn, current_ns),
                lang_type: base_name.clone(),
                is_primitive,
                is_option: false,
                is_list: false,
                inner_type: None,
            };
            TypeRef {
                original: format!("List<{}>", base_name),
                fqn: qualify(&base_fqn, current_ns),
                lang_type: format!("List<{}>", inner.lang_type),
                is_primitive: false,
                is_option: false,
                is_list: true,
                inner_type: Some(Box::new(inner)),
            }
        }
        None => TypeRef {
            original: base_name.clone(),
            fqn: qualify(&base_fqn, current_ns),
            lang_type: base_name,
            is_primitive,
            is_option: false,
            is_list: false,
            inner_type: None,
        },
    }
}

fn build_type_ref_from_base(base_fqn: &str, base_name: &str, c: &Option<ast_model::Cardinality>, primitive_hint: bool) -> TypeRef {
    match c {
        Some(ast_model::Cardinality::Optional) => {
            let inner = TypeRef {
                original: base_name.to_string(),
                fqn: base_fqn.to_string(),
                lang_type: base_name.to_string(),
                is_primitive: primitive_hint,
                is_option: false,
                is_list: false,
                inner_type: None,
            };
            TypeRef {
                original: format!("Option<{}>", base_name),
                fqn: base_fqn.to_string(),
                lang_type: format!("Option<{}>", base_name),
                is_primitive: false,
                is_option: true,
                is_list: false,
                inner_type: Some(Box::new(inner)),
            }
        }
        Some(ast_model::Cardinality::Array) => {
            let inner = TypeRef {
                original: base_name.to_string(),
                fqn: base_fqn.to_string(),
                lang_type: base_name.to_string(),
                is_primitive: primitive_hint,
                is_option: false,
                is_list: false,
                inner_type: None,
            };
            TypeRef {
                original: format!("List<{}>", base_name),
                fqn: base_fqn.to_string(),
                lang_type: format!("List<{}>", base_name),
                is_primitive: false,
                is_option: false,
                is_list: true,
                inner_type: Some(Box::new(inner)),
            }
        }
        None => TypeRef {
            original: base_name.to_string(),
            fqn: base_fqn.to_string(),
            lang_type: base_name.to_string(),
            is_primitive: primitive_hint,
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
