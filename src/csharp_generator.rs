use crate::ast::*;
use heck::{ToPascalCase, ToUpperCamelCase};
use serde::Serialize;
use std::fmt;
use tera::{Context, Tera};
use thiserror::Error;

/// 코드 생성 중에 발생할 수 있는 오류를 나타냅니다.
#[derive(Error, Debug)]
pub enum GenError {
    #[error(transparent)]
    Tera(#[from] tera::Error),
    #[error("Code generation error: {0}")]
    Custom(String),
}

// --- Template Context Structs ---
// AST를 템플릿에서 사용하기 쉬운 직렬화 가능한 구조체로 변환합니다.

#[derive(Debug, Serialize, Default)]
struct TemplateContext {
    namespaces: Vec<TemplateNamespace>,
}

#[derive(Debug, Serialize, Default)]
struct TemplateNamespace {
    name: String,
    items: Vec<TemplateItem>,
}

#[derive(Debug, Serialize, Default)]
struct TemplateItem {
    #[serde(rename = "type")]
    item_type: String, // "class", "enum", "struct"
    name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    fields: Vec<TemplateField>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    variants: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    nested_items: Vec<TemplateItem>,
}

#[derive(Debug, Serialize)]
struct TemplateField {
    name: String,
    #[serde(rename = "type")]
    field_type: String,
    attributes: Vec<String>,
}

/// AST를 템플릿 컨텍스트로 변환합니다.
fn build_context(ast: &[Definition]) -> Result<TemplateContext, GenError> {
    let mut namespaces = Vec::new();
    for def in ast {
        if let Definition::Namespace(ns) = def {
            let mut items = Vec::new();
            for ns_def in &ns.definitions {
                let item = match ns_def {
                    Definition::Table(table) => {
                        let mut table_fields = Vec::new();
                        let mut nested_items = Vec::new();

                        // First pass: define nested types from named `embed` definitions
                        for member in &table.members {
                            if let TableMember::Embed(embed_def) = member {
                                let nested_fields = embed_def
                                    .fields
                                    .iter()
                                    .map(build_template_field)
                                    .collect::<Result<_, _>>()?;

                                nested_items.push(TemplateItem {
                                    item_type: "class".to_string(), // Nested types as classes
                                    name: embed_def.name.clone(),
                                    fields: nested_fields,
                                    ..Default::default()
                                });
                            }
                        }

                        // Second pass: define fields, including those from inline embeds
                        for member in &table.members {
                            if let TableMember::Field(field_def) = member {
                                match field_def {
                                    FieldDefinition::Regular(_) => {
                                        table_fields.push(build_template_field(field_def)?);
                                    }
                                    FieldDefinition::InlineEmbed(inline_embed) => {
                                        let nested_item_name = inline_embed.name.to_pascal_case();
                                        let nested_fields = inline_embed
                                            .fields
                                            .iter()
                                            .map(build_template_field)
                                            .collect::<Result<_, _>>()?;

                                        nested_items.push(TemplateItem {
                                            item_type: "class".to_string(),
                                            name: nested_item_name.clone(),
                                            fields: nested_fields,
                                            ..Default::default()
                                        });

                                        table_fields.push(build_template_field_for_inline_embed(
                                            inline_embed,
                                        ));
                                    }
                                }
                            }
                        }

                        TemplateItem {
                            item_type: "class".to_string(),
                            name: table.name.clone(),
                            fields: table_fields,
                            nested_items,
                            ..Default::default()
                        }
                    }
                    Definition::Enum(enum_def) => TemplateItem {
                        item_type: "enum".to_string(),
                        name: enum_def.name.clone(),
                        fields: vec![],
                        variants: enum_def
                            .variants
                            .iter()
                            .map(|v| v.to_upper_camel_case())
                            .collect(),
                        ..Default::default()
                    },
                    Definition::Embed(embed) => {
                        let fields = embed
                            .fields
                            .iter()
                            .map(build_template_field)
                            .collect::<Result<_, _>>()?;
                        TemplateItem {
                            item_type: "struct".to_string(),
                            name: embed.name.clone(),
                            fields,
                            ..Default::default()
                        }
                    }
                    _ => continue, // Nested namespaces are not handled in this simple version
                };
                items.push(item);
            }
            namespaces.push(TemplateNamespace {
                name: ns.path.join("."),
                items,
            });
        }
    }
    Ok(TemplateContext { namespaces })
}

fn build_template_field(field_def: &FieldDefinition) -> Result<TemplateField, GenError> {
    match field_def {
        FieldDefinition::Regular(field) => {
            let attributes = field
                .constraints
                .iter()
                .map(map_constraint_to_attribute)
                .filter_map(Result::transpose)
                .collect::<Result<Vec<_>, _>>()?;

            Ok(TemplateField {
                name: field.name.to_pascal_case(),
                field_type: map_type(&field.field_type),
                attributes,
            })
        }
        FieldDefinition::InlineEmbed(_) => {
            // This case is handled in `build_context` directly, as it needs to create
            // both a nested type and a field. This function should only handle regular fields.
            unreachable!("build_template_field should not be called with InlineEmbedField");
        }
    }
}

/// Creates a TemplateField for an InlineEmbedField.
/// The nested type definition is created separately in `build_context`.
fn build_template_field_for_inline_embed(inline_embed: &InlineEmbedField) -> TemplateField {
    let nested_item_name = inline_embed.name.to_pascal_case();
    let field_type = match &inline_embed.cardinality {
        Some(Cardinality::Array) => format!("List<{}>", nested_item_name),
        Some(Cardinality::Optional) => nested_item_name.clone(), // Classes are nullable
        None => nested_item_name,
    };
    TemplateField {
        name: inline_embed.name.to_pascal_case(),
        field_type,
        attributes: vec![], // Inline embeds don't have constraints in the current grammar
    }
}

/// PolyGen 타입을 C# 타입으로 매핑합니다.
fn map_type(ty: &TypeWithCardinality) -> String {
    let base_type_str = match &ty.base_type {
        TypeName::Basic(b) => match b {
            BasicType::String => "string".to_string(),
            BasicType::I8 => "sbyte".to_string(),
            BasicType::I16 => "short".to_string(),
            BasicType::I32 => "int".to_string(),
            BasicType::I64 => "long".to_string(),
            BasicType::U8 => "byte".to_string(),
            BasicType::U16 => "ushort".to_string(),
            BasicType::U32 => "uint".to_string(),
            BasicType::U64 => "ulong".to_string(),
            BasicType::F32 => "float".to_string(),
            BasicType::F64 => "double".to_string(),
            BasicType::Bool => "bool".to_string(),
            BasicType::Bytes => "byte[]".to_string(),
        },
        TypeName::Path(p) => p.join("."),
    };

    match ty.cardinality {
        Some(Cardinality::Optional) if !is_csharp_reference_type(&base_type_str) => {
            format!("{}?", base_type_str)
        }
        Some(Cardinality::Optional) => base_type_str, // string, List<>, byte[] are already nullable
        Some(Cardinality::Array) => format!("List<{}>", base_type_str),
        None => base_type_str,
    }
}

fn is_csharp_reference_type(t: &str) -> bool {
    !(t == "sbyte"
        || t == "short"
        || t == "int"
        || t == "long"
        || t == "byte"
        || t == "ushort"
        || t == "uint"
        || t == "ulong"
        || t == "float"
        || t == "double"
        || t == "bool")
}

/// 제약조건을 C# 어트리뷰트로 매핑합니다.
fn map_constraint_to_attribute(constraint: &Constraint) -> Result<Option<String>, GenError> {
    match constraint {
        Constraint::PrimaryKey => Ok(Some("[Key]".to_string())),
        Constraint::MaxLength(len) => Ok(Some(format!("[MaxLength({})]", len))),
        _ => Ok(None),
    }
}

/// 코드 생성을 위한 진입점 함수입니다.
pub fn generate_code(ast: &[Definition], template_str: &str) -> Result<String, GenError> {
    let mut tera = Tera::default();
    tera.add_raw_template("csharp", template_str)?;

    let context_data = build_context(ast)?;
    let context = Context::from_serialize(&context_data)?;

    let rendered = tera.render("csharp", &context)?;
    Ok(rendered)
}
