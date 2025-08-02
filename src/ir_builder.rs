use crate::ast::{self, Definition, TableMember};
use crate::ir_model::{
    self, AnnotationDef, EnumDef, EnumItem, FieldDef, NamespaceDef, NamespaceItem, SchemaContext,
    StructDef, StructItem,
};
use heck::ToPascalCase;
use std::collections::BTreeMap;

/// Builds the template-friendly Intermediate Representation (IR) from the AST definitions.
pub fn build_ir(definitions: &[ast::Definition]) -> ir_model::SchemaContext {
    let mut context = SchemaContext::default();
    let mut current_path: Vec<String> = Vec::new();
    populate_context(&mut context, definitions, &mut current_path);
    context
}

/// Recursively traverses AST definitions to populate the SchemaContext.
fn populate_context(
    context: &mut SchemaContext,
    definitions: &[ast::Definition],
    path: &mut Vec<String>,
) {
    for def in definitions {
        let fqn_prefix = path.join(".");
        let namespace = context
            .namespaces
            .entry(fqn_prefix)
            .or_insert_with(|| NamespaceDef {
                name: path.join("."),
                items: Vec::new(),
            });

        match def {
            Definition::Namespace(ns) => {
                path.extend(ns.path.iter().cloned());
                populate_context(context, &ns.definitions, path);
                for _ in 0..ns.path.len() {
                    path.pop();
                }
            }
            Definition::Table(table) => {
                if let Some(comment) = &table.doc_comment {
                    namespace
                        .items
                        .push(NamespaceItem::Comment(comment.clone()));
                }
                let (struct_def, mut nested_items) = convert_table_to_struct(table);
                namespace.items.push(NamespaceItem::Struct(struct_def));
                namespace.items.append(&mut nested_items);
            }
            Definition::Enum(e) => {
                if let Some(comment) = &e.doc_comment {
                    namespace.items.push(NamespaceItem::Comment(comment.clone()));
                }
                namespace.items.push(convert_enum(e));
            }
            Definition::Embed(embed) => {
                // A namespace-level embed is a reusable struct.
                if let Some(comment) = &embed.doc_comment {
                    namespace
                        .items
                        .push(NamespaceItem::Comment(comment.clone()));
                }
                namespace
                    .items
                    .push(NamespaceItem::Struct(convert_embed_to_struct(embed)));
            }
            Definition::Comment(c) => namespace.items.push(NamespaceItem::Comment(c.clone())),
        }
    }
}

/// Converts an `ast::Table` into an `ir_model::StructDef` and any nested types.
fn convert_table_to_struct(table: &ast::Table) -> (StructDef, Vec<NamespaceItem>) {
    let mut items = Vec::new();
    let mut nested_items = Vec::new();

    for annotation in convert_annotations_to_ir(&table.annotations) {
        items.push(StructItem::Annotation(annotation));
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
                if let Some(comment) = &embed.doc_comment {
                    nested_items.push(NamespaceItem::Comment(comment.clone()));
                }
                nested_items.push(NamespaceItem::Struct(convert_embed_to_struct(embed)));
            }
            TableMember::Comment(c) => items.push(StructItem::Comment(c.clone())),
        }
    }

    let struct_def = StructDef {
        name: table.name.clone(),
        items,
    };

    (struct_def, nested_items)
}

/// Converts an `ast::FieldDefinition` into an `ir_model::FieldDef` and potential nested types (from inline embeds).
fn convert_field_to_ir(field: &ast::FieldDefinition) -> (FieldDef, Vec<NamespaceItem>) {
    match field {
        ast::FieldDefinition::Regular(rf) => (
            FieldDef {
                name: rf.name.clone(),
                field_type: format_type(&rf.field_type),
                comment: rf.doc_comment.clone(),
                attributes: convert_constraints_to_attributes(&rf.constraints),
            },
            Vec::new(), // Regular fields don't create nested types.
        ),
        ast::FieldDefinition::InlineEmbed(ief) => {
            let struct_name = ief.name.to_pascal_case();
            let (inline_struct, mut nested_items) = convert_table_to_struct(&ast::Table {
                name: struct_name.clone(),
                doc_comment: ief.doc_comment.clone(),
                annotations: vec![],
                members: ief
                    .fields
                    .iter()
                    .map(|f| TableMember::Field(f.clone()))
                    .collect(),
            });

            nested_items.push(NamespaceItem::Struct(inline_struct));

            let field_def = FieldDef {
                name: ief.name.clone(),
                field_type: format_cardinality(&struct_name, &ief.cardinality),
                comment: ief.doc_comment.clone(),
                attributes: Vec::new(),
            };
            (field_def, nested_items)
        }
    }
}

/// Converts field constraints from the AST into a vector of strings
/// suitable for C# attributes.
fn convert_constraints_to_attributes(constraints: &[ast::Constraint]) -> Vec<String> {
    constraints
        .iter()
        .filter_map(|c| match c {
            // `primary_key` is mapped to the `[Key]` attribute, common in ORMs like EF Core.
            ast::Constraint::PrimaryKey => Some("Key".to_string()),
            // `unique` can be mapped to an index attribute.
            ast::Constraint::Unique => Some("Index(IsUnique = true)".to_string()),
            ast::Constraint::MaxLength(len) => Some(format!("MaxLength({})", len)),
            // ForeignKey is a relationship, not a simple attribute, so we ignore it here.
            ast::Constraint::ForeignKey(_, _) => None,
            // Other constraints are not (yet) represented as attributes.
            _ => None,
        })
        .collect()
}

fn convert_enum(e: &ast::Enum) -> NamespaceItem {
    let mut items = Vec::new();
    for variant in &e.variants {
        if let Some(comment) = &variant.doc_comment {
            items.push(EnumItem::Comment(comment.clone()));
        }
        items.push(EnumItem::Member(ir_model::EnumMember {
            name: variant.name.clone(),
            comment: None, // Line-end comments are handled differently if needed
        }));
    }

    NamespaceItem::Enum(EnumDef {
        name: e.name.clone(),
        items,
    })
}

fn convert_embed_to_struct(embed: &ast::Embed) -> StructDef {
    let (struct_def, _nested_types) = convert_table_to_struct(&ast::Table {
        name: embed.name.clone(),
        doc_comment: embed.doc_comment.clone(),
        annotations: embed.annotations.clone(),
        members: embed
            .fields
            .iter()
            .map(|f| TableMember::Field(f.clone()))
            .collect(),
    });
    // For now, we assume named embeds don't contain inline embeds that would create more nested types.
    struct_def
}

// Helper to format the abstract type into a language-agnostic string for the template.
fn format_type(t: &ast::TypeWithCardinality) -> String {
    let base = match &t.base_type {
        ast::TypeName::Path(p) => p.join("."), // Keep FQN for now; template filter can shorten it.
        ast::TypeName::Basic(b) => format!("{:?}", b).to_lowercase(),
    };
    format_cardinality(&base, &t.cardinality)
}

fn format_cardinality(base: &str, c: &Option<ast::Cardinality>) -> String {
    match c {
        // Use a generic representation that templates can interpret.
        Some(ast::Cardinality::Optional) => format!("Option<{}>", base),
        Some(ast::Cardinality::Array) => format!("List<{}>", base),
        None => base.to_string(),
    }
}

fn convert_annotations_to_ir(annotations: &[ast::Annotation]) -> Vec<AnnotationDef> {
    annotations
        .iter()
        .map(|ast_ann| {
            let mut params_map = BTreeMap::new();
            for param in &ast_ann.params {
                params_map.insert(param.key.clone(), param.value.to_string());
            }
            AnnotationDef {
                name: ast_ann.name.clone(),
                params: params_map,
            }
        })
        .collect()
}
