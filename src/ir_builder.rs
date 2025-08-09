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
            .unwrap_or_else(|| "".to_string());

        let mut file_def = FileDef {
            path: file_name, // Use only the file name
            namespaces: Vec::new(),
        };

        let mut current_path: Vec<String> = Vec::new();
        populate_namespaces(
            &mut file_def.namespaces,
            &ast.definitions,
            &mut current_path,
        );
        context.files.push(file_def);
    }

    context
}

/// Recursively traverses AST definitions to populate the namespaces for a given file.
fn populate_namespaces(
    namespaces: &mut Vec<NamespaceDef>,
    definitions: &[ast_model::Definition],
    path: &mut Vec<String>,
) {
    // Ensure the global namespace (with an empty name) always exists.
    if namespaces.is_empty() {
        namespaces.push(NamespaceDef::default());
    }

    for def in definitions {
        match def {
            Definition::Namespace(ns) => {
                path.extend(ns.path.iter().cloned());
                populate_namespaces(namespaces, &ns.definitions, path);
                // Backtrack the path after recursion.
                for _ in 0..ns.path.len() {
                    path.pop();
                }
            }
            _ => {
                // For any other definition (Table, Enum, Embed), find or create the correct namespace based on the current path.
                let fqn_prefix = path.join(".");
                let namespace = if fqn_prefix.is_empty() {
                    // If path is empty, it's the global namespace.
                    &mut namespaces[0]
                } else {
                    // Find if the namespace already exists.
                    if let Some(index) = namespaces.iter().position(|ns| ns.name == fqn_prefix) {
                        &mut namespaces[index]
                    } else {
                        // If not, create a new one.
                        let new_namespace = NamespaceDef {
                            name: fqn_prefix.clone(),
                            items: Vec::new(),
                        };
                        namespaces.push(new_namespace);
                        namespaces.last_mut().unwrap()
                    }
                };

                // Now, add the item to the determined namespace.
                match def {
                    Definition::Table(table) => {
                        //add_metadata_to_namespace(&mut namespace.items, &table.metadata);
                        let (struct_def, mut nested_items) = convert_table_to_struct(table);
                        namespace.items.push(NamespaceItem::Struct(struct_def));
                        namespace.items.append(&mut nested_items);
                    }
                    Definition::Enum(e) => {
                        namespace.items.push(convert_enum(e));
                    }
                    Definition::Embed(embed) => {
                        //add_metadata_to_namespace(&mut namespace.items, &embed.metadata);
                        namespace
                            .items
                            .push(NamespaceItem::Struct(convert_embed_to_struct(embed)));
                    }
                    Definition::Comment(c) => {
                        println!("[ir_builder] comment: {:?} {:?}", c, namespace.name); // [DEBUG]
                        namespace.items.push(NamespaceItem::Comment(c.clone()))
                    }
                    Definition::Annotation(_) => { // Annotations are processed elsewhere or ignored at this level
                    }
                    Definition::Namespace(_) => unreachable!(), // Already handled above.
                }
            }
        }
    }
}

/// Converts an `ast_model::Table` into an `ir_model::StructDef` and any nested types.
fn convert_table_to_struct(table: &ast_model::Table) -> (StructDef, Vec<NamespaceItem>) {
    let mut items = Vec::new();
    let mut nested_items = Vec::new();
    let mut header_items = Vec::new();
    let mut last_doc_comment: Option<String> = None; // To store a preceding doc comment

    // Process metadata for the struct header
    for meta in &table.metadata {
        match meta {
            Metadata::DocComment(c) => {
                last_doc_comment = Some(c.clone());
            }
            Metadata::Annotation(a) => {
                let mut annotation_def = convert_annotations_to_ir(&[a.clone()])[0].clone();
                // Assign the preceding doc comment to the annotation
                annotation_def.comment = last_doc_comment.take(); // .take() clears the Option after moving the value
                header_items.push(StructItem::Annotation(annotation_def));
            }
        }
    }
    // If there's a remaining doc comment not followed by an annotation, add it as a standalone comment
    if let Some(c) = last_doc_comment.take() {
        header_items.push(StructItem::Comment(c));
    }

    for member in &table.members {
        match member {
            TableMember::Field(field) => {
                let (field_def, mut new_nested) = convert_field_to_ir(field);
                items.push(StructItem::Field(field_def));
                nested_items.append(&mut new_nested);
            }
            TableMember::Embed(embed) => {
                // A named embed inside a table is treated as a nested struct.
                add_metadata_to_namespace(&mut nested_items, &embed.metadata);
                nested_items.push(NamespaceItem::Struct(convert_embed_to_struct(embed)));
            }
            TableMember::Comment(c) => items.push(StructItem::Comment(c.clone())),
        }
    }

    let struct_def = StructDef {
        name: table.name.clone(),
        items,
        is_embed: false,
        header: header_items,
    };

    (struct_def, nested_items)
}

/// Converts an `ast_model::FieldDefinition` into an `ir_model::FieldDef` and potential nested types (from inline embeds).
fn convert_field_to_ir(field: &ast_model::FieldDefinition) -> (FieldDef, Vec<NamespaceItem>) {
    match field {
        ast_model::FieldDefinition::Regular(rf) => {
            let (comment, attributes) = get_field_metadata(rf);
            (
                FieldDef {
                    name: rf.name.clone(),
                    field_type: format_type(&rf.field_type),
                    comment,
                    attributes,
                },
                Vec::new(), // Regular fields don't create nested types.
            )
        }
        ast_model::FieldDefinition::InlineEmbed(ief) => {
            let struct_name = ief.name.to_pascal_case();
            let (mut inline_struct, mut nested_items) =
                convert_table_to_struct(&ast_model::Table {
                    name: struct_name.clone(),
                    metadata: ief.metadata.clone(),
                    members: ief
                        .fields
                        .iter()
                        .map(|f| TableMember::Field(f.clone()))
                        .collect(),
                });
            inline_struct.is_embed = true;

            nested_items.push(NamespaceItem::Struct(inline_struct));

            let (comment, _) = get_field_metadata_from_vec(&ief.metadata);

            let field_def = FieldDef {
                name: ief.name.clone(),
                field_type: format_cardinality(&struct_name, &ief.cardinality),
                comment,
                attributes: Vec::new(),
            };
            (field_def, nested_items)
        }
    }
}

fn get_field_metadata(rf: &ast_model::RegularField) -> (Option<String>, Vec<String>) {
    let comment = rf.metadata.iter().find_map(|m| match m {
        Metadata::DocComment(c) => Some(c.clone()),
        _ => None,
    });

    let attributes = convert_constraints_to_attributes(&rf.constraints);
    (comment, attributes)
}

fn get_field_metadata_from_vec(
    metadata: &[ast_model::Metadata],
) -> (Option<String>, Vec<AnnotationDef>) {
    let comment = metadata.iter().find_map(|m| match m {
        Metadata::DocComment(c) => Some(c.clone()),
        _ => None,
    });

    let annotations: Vec<ast_model::Annotation> = metadata
        .iter()
        .filter_map(|m| match m {
            Metadata::Annotation(a) => Some(a.clone()),
            _ => None,
        })
        .collect();

    (comment, convert_annotations_to_ir(&annotations))
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

fn convert_enum(e: &ast_model::Enum) -> NamespaceItem {
    let mut items = Vec::new();
    let mut enum_comment: Option<String> = None; // To store the enum's own comment

    // Extract enum's own comment
    for meta in &e.metadata {
        if let Metadata::DocComment(c) = meta {
            enum_comment = Some(c.clone());
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
        items.push(EnumItem::Member(ir_model::EnumMember {
            name: variant.name.clone(),
            comment: None, // Line-end comments are handled differently if needed
        }));
    }

    NamespaceItem::Enum(EnumDef {
        name: e.name.clone(),
        items,
        comment: enum_comment, // Assign the extracted comment here
    })
}

fn convert_embed_to_struct(embed: &ast_model::Embed) -> StructDef {
    let (mut struct_def, _nested_types) = convert_table_to_struct(&ast_model::Table {
        name: embed.name.clone(),
        metadata: embed.metadata.clone(),
        members: embed
            .fields
            .iter()
            .map(|f| TableMember::Field(f.clone()))
            .collect(),
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

fn add_metadata_to_namespace(namespace_items: &mut Vec<NamespaceItem>, metadata: &[Metadata]) {
    for meta in metadata {
        match meta {
            Metadata::DocComment(c) => namespace_items.push(NamespaceItem::Comment(c.clone())),
            Metadata::Annotation(_) => {
                // Annotations on namespaces/tables are handled at a higher level if needed
            }
        }
    }
}

fn convert_annotations_to_ir(annotations: &[ast_model::Annotation]) -> Vec<AnnotationDef> {
    annotations
        .iter()
        .map(|ast_ann| AnnotationDef {
            name: ast_ann.name.clone(),
            params: ast_ann
                .params
                .iter()
                .map(|p| AnnotationParam {
                    key: p.key.clone(),
                    value: p.value.to_string(), // Assuming value is a simple literal
                })
                .collect(),
            comment: None, // Initialize the new comment field
        })
        .collect()
}
