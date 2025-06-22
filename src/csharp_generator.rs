use crate::ast::{
    BasicType, Cardinality, Constraint, Definition, Embed, Enum, FieldDefinition, RegularField,
    Table, TableMember, TypeName, TypeWithCardinality,
};
use heck::ToUpperCamelCase;
use serde::Serialize;
use tera::{Context, Tera};

// --- View Models for Tera ---
// These structs define the data structure passed to the template for rendering.
#[derive(Serialize, Default, Debug)]
struct NamespaceView {
    name: String,
    items: Vec<ItemView>,
    children: Vec<NamespaceView>,
}

#[derive(Serialize, Debug, Clone)]
struct ItemView {
    name: String,
    r#type: String, // "class", "enum", "struct"
    fields: Vec<FieldView>,
    variants: Vec<String>,
    nested_items: Vec<ItemView>,
}

#[derive(Serialize, Debug, Clone)]
struct FieldView {
    name: String,
    r#type: String,
    attributes: Vec<String>,
}

// --- Main Generation Function ---
pub fn generate_code(
    ast_root: &[Definition],
    template_str: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut tera = Tera::default();
    tera.add_raw_template("csharp", template_str)?;

    // Build the view model directly from the AST, preserving the schema's structure.
    let root_namespaces = build_namespace_views(ast_root)?;

    // 3. Create a Tera context and render the template.
    let mut context = Context::new();
    context.insert("namespaces", &root_namespaces);

    let rendered = tera.render("csharp", &context)?;
    Ok(rendered)
}

// --- AST to View Model Conversion ---

/// Recursively builds a `Vec<NamespaceView>` that mirrors the structure of the input `definitions`.
fn build_namespace_views(
    definitions: &[Definition],
) -> Result<Vec<NamespaceView>, Box<dyn std::error::Error>> {
    let mut views = Vec::new();

    for def in definitions {
        if let Definition::Namespace(ns_def) = def {
            // The name for the template is the path defined in this specific block.
            let name = ns_def.path.join(".");

            // Recursively build views for any nested namespaces.
            let children = build_namespace_views(&ns_def.definitions)?;

            // Collect items (tables, enums, etc.) defined directly in this namespace.
            let mut items = Vec::new();
            for inner_def in &ns_def.definitions {
                match inner_def {
                    Definition::Table(t) => items.push(convert_table_to_view(t)?),
                    Definition::Enum(e) => items.push(convert_enum_to_view(e)),
                    Definition::Embed(e) => items.push(convert_embed_to_view(e)?),
                    // Namespaces are handled by the recursive call above, so we ignore them here.
                    Definition::Namespace(_) => {}
                }
            }

            views.push(NamespaceView {
                name,
                items,
                children,
            });
        }
    }

    Ok(views)
}

fn convert_table_to_view(table: &Table) -> Result<ItemView, Box<dyn std::error::Error>> {
    let mut fields = Vec::new();
    let mut nested_items = Vec::new();

    for member in &table.members {
        match member {
            TableMember::Field(field_def) => match field_def {
                FieldDefinition::Regular(reg_field) => {
                    fields.push(convert_regular_field_to_view(reg_field)?);
                }
                FieldDefinition::InlineEmbed(inline_embed) => {
                    let mut nested_fields = Vec::new();
                    for f in &inline_embed.fields {
                        if let FieldDefinition::Regular(rf) = f {
                            nested_fields.push(convert_regular_field_to_view(rf)?);
                        }
                    }

                    let nested_view = ItemView {
                        name: inline_embed.name.to_upper_camel_case(),
                        r#type: "class".to_string(),
                        fields: nested_fields,
                        variants: Vec::new(),
                        nested_items: Vec::new(),
                    };
                    nested_items.push(nested_view);

                    let field_type = map_cardinality_to_csharp(
                        &inline_embed.cardinality,
                        inline_embed.name.to_upper_camel_case(),
                    );
                    fields.push(FieldView {
                        name: inline_embed.name.to_upper_camel_case(),
                        r#type: field_type,
                        attributes: vec![],
                    });
                }
            },
            TableMember::Embed(embed_def) => {
                nested_items.push(convert_embed_to_view(embed_def)?);
            }
        }
    }

    Ok(ItemView {
        name: table.name.to_upper_camel_case(),
        r#type: "class".to_string(),
        fields,
        variants: Vec::new(),
        nested_items,
    })
}

fn convert_enum_to_view(enm: &Enum) -> ItemView {
    ItemView {
        name: enm.name.to_upper_camel_case(),
        r#type: "enum".to_string(),
        fields: Vec::new(),
        variants: enm.variants.clone(),
        nested_items: Vec::new(),
    }
}

fn convert_embed_to_view(embed: &Embed) -> Result<ItemView, Box<dyn std::error::Error>> {
    let mut fields = Vec::new();
    for field_def in &embed.fields {
        if let FieldDefinition::Regular(reg_field) = field_def {
            fields.push(convert_regular_field_to_view(reg_field)?);
        }
    }

    Ok(ItemView {
        name: embed.name.to_upper_camel_case(),
        r#type: "struct".to_string(),
        fields,
        variants: Vec::new(),
        nested_items: Vec::new(),
    })
}

fn convert_regular_field_to_view(
    field: &RegularField,
) -> Result<FieldView, Box<dyn std::error::Error>> {
    let mut attributes = Vec::new();
    for constraint in &field.constraints {
        if let Constraint::PrimaryKey = constraint {
            attributes.push("[Key]".to_string());
        }
        // TODO: Add more attribute logic here (unique, max_length, etc.)
    }

    let type_name = map_type_to_csharp(&field.field_type)?;

    Ok(FieldView {
        name: field.name.to_upper_camel_case(),
        r#type: type_name,
        attributes,
    })
}

fn map_type_to_csharp(t: &TypeWithCardinality) -> Result<String, Box<dyn std::error::Error>> {
    let base_type = match &t.base_type {
        TypeName::Basic(bt) => match bt {
            BasicType::U8 => "byte".to_string(),
            BasicType::I8 => "sbyte".to_string(),
            BasicType::U16 => "ushort".to_string(),
            BasicType::I16 => "short".to_string(),
            BasicType::U32 => "uint".to_string(),
            BasicType::I32 => "int".to_string(),
            BasicType::U64 => "ulong".to_string(),
            BasicType::I64 => "long".to_string(),
            BasicType::F32 => "float".to_string(),
            BasicType::F64 => "double".to_string(),
            BasicType::Bool => "bool".to_string(),
            BasicType::String => "string".to_string(),
            BasicType::Bytes => "byte[]".to_string(), // Special case
        },
        TypeName::Path(path) => path.last().unwrap().to_upper_camel_case(),
    };

    // `bytes` is already an array type in C#, so handle it before other cardinality.
    if matches!(t.base_type, TypeName::Basic(BasicType::Bytes)) {
        return Ok(base_type);
    }

    Ok(map_cardinality_to_csharp(&t.cardinality, base_type))
}

fn map_cardinality_to_csharp(cardinality: &Option<Cardinality>, base_type: String) -> String {
    match cardinality {
        Some(Cardinality::Optional) => {
            // In C#, value types get `?`, reference types are nullable by default.
            if &base_type != "string" {
                format!("{}?", base_type)
            } else {
                base_type
            }
        }
        Some(Cardinality::Array) => format!("List<{}>", base_type),
        None => base_type,
    }
}
