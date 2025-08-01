use crate::ast::{self, Constraint, Definition, FieldDefinition, TableMember, TypeName};
use crate::ir_model::{self, FieldDef, SchemaContext, StructDef, TypeDef};
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
        let namespace =
            context
                .namespaces
                .entry(fqn_prefix)
                .or_insert_with(|| ir_model::NamespaceDef {
                    name: path.join("."),
                    types: Vec::new(),
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
                let (struct_def, mut nested_types) = convert_table_to_struct(table);
                namespace.types.push(TypeDef::Struct(struct_def));
                namespace.types.append(&mut nested_types);
            }
            Definition::Enum(e) => {
                namespace.types.push(convert_enum(e));
            }
            Definition::Embed(embed) => {
                // A namespace-level embed is a reusable struct.
                namespace
                    .types
                    .push(TypeDef::Struct(convert_embed_to_struct(embed)));
            }
        }
    }
}

/// Converts an `ast::Table` into an `ir_model::StructDef` and any nested types.
fn convert_table_to_struct(table: &ast::Table) -> (StructDef, Vec<TypeDef>) {
    let mut fields = Vec::new();
    let mut nested_types = Vec::new();

    for member in &table.members {
        match member {
            TableMember::Field(field) => {
                let (field_def, mut new_nested) = convert_field_to_ir(field);
                fields.push(field_def);
                nested_types.append(&mut new_nested);
            }
            TableMember::Embed(embed) => {
                // A named embed inside a table is treated as a nested struct.
                nested_types.push(TypeDef::Struct(convert_embed_to_struct(embed)));
            }
        }
    }

    let struct_def = StructDef {
        name: table.name.clone(),
        comment: table.doc_comment.clone(),
        fields,
        annotations: convert_annotations_to_ir(&table.annotations),
    };

    (struct_def, nested_types)
}

/// Converts an `ast::FieldDefinition` into an `ir_model::FieldDef` and potential nested types (from inline embeds).
fn convert_field_to_ir(field: &ast::FieldDefinition) -> (FieldDef, Vec<TypeDef>) {
    match field {
        FieldDefinition::Regular(rf) => (
            FieldDef {
                name: rf.name.clone(),
                field_type: format_type(&rf.field_type),
                comment: rf.doc_comment.clone(),
                attributes: convert_constraints_to_attributes(&rf.constraints),
            },
            Vec::new(), // Regular fields don't create nested types.
        ),
        FieldDefinition::InlineEmbed(ief) => {
            let struct_name = ief.name.to_pascal_case();
            let (inline_struct, mut nested_types) = convert_table_to_struct(&ast::Table {
                name: struct_name.clone(),
                doc_comment: ief.doc_comment.clone(),
                annotations: vec![],
                members: ief
                    .fields
                    .iter()
                    .map(|f| TableMember::Field(f.clone()))
                    .collect(),
            });

            nested_types.push(TypeDef::Struct(inline_struct));

            let field_def = FieldDef {
                name: ief.name.clone(),
                field_type: format_cardinality(&struct_name, &ief.cardinality),
                comment: ief.doc_comment.clone(),
                attributes: Vec::new(),
            };
            (field_def, nested_types)
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
            Constraint::PrimaryKey => Some("Key".to_string()),
            // `unique` can be mapped to an index attribute.
            Constraint::Unique => Some("Index(IsUnique = true)".to_string()),
            Constraint::MaxLength(len) => Some(format!("MaxLength({})", len)),
            // ForeignKey is a relationship, not a simple attribute, so we ignore it here.
            Constraint::ForeignKey(_, _) => None,
            // Other constraints are not (yet) represented as attributes.
            _ => None,
        })
        .collect()
}

fn convert_enum(e: &ast::Enum) -> TypeDef {
    let members = e
        .variants
        .iter()
        .map(|v| ir_model::EnumMember {
            name: v.name.clone(),
            comment: v.doc_comment.clone(),
        })
        .collect();

    TypeDef::Enum(ir_model::EnumDef {
        name: e.name.clone(),
        comment: e.doc_comment.clone(),
        members,
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
        TypeName::Path(p) => p.join("."), // Keep FQN for now; template filter can shorten it.
        TypeName::Basic(b) => format!("{:?}", b).to_lowercase(),
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

fn convert_annotations_to_ir(annotations: &[ast::Annotation]) -> Vec<ir_model::AnnotationDef> {
    annotations
        .iter()
        .map(|ast_ann| {
            let mut params_map = BTreeMap::new();
            for param in &ast_ann.params {
                params_map.insert(param.key.clone(), param.value.to_string());
            }
            ir_model::AnnotationDef {
                name: ast_ann.name.clone(),
                params: params_map,
            }
        })
        .collect()
}