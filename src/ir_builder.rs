use crate::ast_model::{self, Definition, Metadata, TableMember};
use crate::ir_model::{
    self, AnnotationDef, AnnotationParam, EnumDef, EnumItem, FieldDef, FileDef, NamespaceDef,
    NamespaceItem, SchemaContext, StructDef, StructItem,
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
                let mut new_ns = NamespaceDef {
                    name: ns_ast.path.join("."),
                    items: Vec::new(),
                };
                // Recurse into this new top-level namespace
                populate_items_recursively(&mut new_ns.items, &ns_ast.definitions);
                top_level_namespaces.push(new_ns);
            } else {
                // Any other item at the top level belongs to the "global" namespace.
                add_definition_to_items(&mut global_items, def);
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
fn populate_items_recursively(items: &mut Vec<NamespaceItem>, definitions: &[ast_model::Definition]) {
    for def in definitions {
        add_definition_to_items(items, def);
    }
}

/// Converts a single AST Definition into a NamespaceItem and adds it to a list.
/// This is the core of the recursive build process.
fn add_definition_to_items(items: &mut Vec<NamespaceItem>, def: &Definition) {
    match def {
        Definition::Namespace(ns_ast) => {
            let mut new_ns = NamespaceDef {
                name: ns_ast.path.join("."),
                items: Vec::new(),
            };
            populate_items_recursively(&mut new_ns.items, &ns_ast.definitions);
            items.push(NamespaceItem::Namespace(Box::new(new_ns)));
        }
        Definition::Table(table) => {
            items.push(NamespaceItem::Struct(convert_table_to_struct(table)));
        }
        Definition::Enum(e) => {
            items.push(NamespaceItem::Enum(convert_enum_to_enum_def(e, None)));
        }
        Definition::Embed(embed) => {
            items.push(NamespaceItem::Struct(convert_embed_to_struct(embed)));
        }
        Definition::Comment(c) => {
            items.push(NamespaceItem::Comment(c.clone()));
        }
        Definition::Annotation(_) => { /* Annotations are handled within other items, not as top-level IR items */ }
    }
}

fn convert_table_to_struct(table: &ast_model::Table) -> StructDef {
    let mut items = Vec::new();
    let mut header_items = Vec::new();
    let mut embedded_structs = Vec::new();
    let mut embedded_enums = Vec::new(); // New line

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
                // 필드에 연결된 메타데이터(주석, 어노테이션)를 먼저 처리
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

                // 그 다음에 필드 자체를 처리
                let (field_def, mut new_nested_structs, mut new_nested_enums) = convert_field_to_ir(field);
                items.push(StructItem::Field(field_def));
                embedded_structs.append(&mut new_nested_structs);
                embedded_enums.append(&mut new_nested_enums);
            }
            TableMember::Embed(embed) => {
                embedded_structs.push(convert_embed_to_struct(embed));
            }
            TableMember::Enum(e) => {
                embedded_enums.push(convert_enum_to_enum_def(e, None));
            }
            TableMember::Comment(c) => items.push(StructItem::Comment(c.clone())),
        }
    }

    StructDef {
        name: table.name.clone().unwrap(),
        items,
        is_embed: false,
        header: header_items,
        embedded_structs,
        inline_enums: embedded_enums, // New line
    }
}

/// Converts an `ast_model::FieldDefinition` into an `ir_model::FieldDef` and potential nested types (from inline embeds).
fn convert_field_to_ir(field: &ast_model::FieldDefinition) -> (FieldDef, Vec<StructDef>, Vec<EnumDef>) {
    match field {
        ast_model::FieldDefinition::Regular(rf) => {
            let attributes = convert_constraints_to_attributes(&rf.constraints);
            let mut inline_enums = Vec::new();
            let field_type_str: String;

            match &rf.field_type.base_type {
                ast_model::TypeName::InlineEnum(e) => {
                    // Generate a unique name for the inline enum
                    // For now, let's use FieldName_Enum. We'll need table context later for better names.
                    let generated_enum_name = format!("{}_Enum", rf.name.clone().expect("Regular field name must be present").to_pascal_case());
                    
                    // Create the EnumDef using the generated name
                    let enum_def = convert_enum_to_enum_def(e, Some(generated_enum_name.clone()));
                    inline_enums.push(enum_def);
                    field_type_str = format_cardinality(&generated_enum_name, &rf.field_type.cardinality);
                },
                _ => {
                    field_type_str = format_type(&rf.field_type); // Use existing format_type for other types
                },
            };

            (
                FieldDef {
                    name: rf.name.clone().expect("Regular field name must be present"),
                    field_type: field_type_str,
                    attributes,
                },
                Vec::new(),
                inline_enums,
            )
        }
        ast_model::FieldDefinition::InlineEmbed(ief) => {
            let struct_name = ief.name.clone().expect("Inline embed field name must be present").to_pascal_case();
            let mut inline_struct = convert_table_to_struct(&ast_model::Table {
                name: Some(struct_name.clone()),
                metadata: ief.metadata.clone(),
                members: ief.members.clone(),
            });
            inline_struct.is_embed = true;

            let nested_items = vec![inline_struct];

            let field_def = FieldDef {
                name: ief.name.clone().expect("Inline embed field name must be present"),
                field_type: format_cardinality(&struct_name, &ief.cardinality),
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

            let enum_def = convert_enum_to_enum_def(&temp_enum, Some(generated_enum_name.clone()));
            
            let field_def = FieldDef {
                name: e.name.clone().expect("Inline enum name must be present"),
                field_type: format_cardinality(&generated_enum_name, &e.cardinality),
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

fn convert_enum_to_enum_def(e: &ast_model::Enum, name_override: Option<String>) -> EnumDef {
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

    EnumDef {
        name: name_override.unwrap_or_else(|| e.name.clone().expect("Named enum must have a name")),
        items,
    }
}

fn convert_embed_to_struct(embed: &ast_model::Embed) -> StructDef {
    let mut struct_def = convert_table_to_struct(&ast_model::Table {
        name: embed.name.clone(),
        metadata: embed.metadata.clone(),
        members: embed.members.clone(),
    });
    struct_def.is_embed = true;
    // For now, we assume named embeds don't contain inline embeds that would create more nested types.
    struct_def
}

// Helper to format the abstract type into a language-agnostic string for the template.
fn format_type(t: &ast_model::TypeWithCardinality) -> String {
    let base = match &t.base_type {
        ast_model::TypeName::Path(p) => p.join("."), // Keep FQN for now; template filter can shorten it.
        ast_model::TypeName::Basic(b) => format!("{:?}", b).to_lowercase(),
        ast_model::TypeName::InlineEnum(_) => {
            // TODO: This needs context from the field/table to generate a good name,
            // e.g., TableName_FieldName_Enum.
            "__ANONYMOUS_ENUM__".to_string()
        }
    };
    format_cardinality(&base, &t.cardinality)
}

fn format_cardinality(base: &str, c: &Option<ast_model::Cardinality>) -> String {
    match c {
        // Use a generic representation that templates can interpret.
        Some(ast_model::Cardinality::Optional) => format!("Option<{}>", base),
        Some(ast_model::Cardinality::Array) => format!("List<{}>", base),
        None => base.to_string(),
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