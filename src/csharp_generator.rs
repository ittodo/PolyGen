use crate::ast::{
    AstRoot, BasicType, Cardinality, Definition, Embed, Enum, FieldDefinition, InlineEmbedField,
    Namespace, RegularField, Table, TableMember, TypeName, TypeWithCardinality,
};
use crate::csharp_model::{
    CSharpFile, ClassDef, EnumDef, NamespaceDef, PropertyDef, StructDef, TypeDef, TypeInfo,
};
use askama::Template;
use std::fs;
use std::path::Path;

use heck::{ToPascalCase, ToUpperCamelCase};

/// Askama 템플릿을 위한 구조체입니다.
/// `CSharpFile` 모델을 참조하여 C# 코드를 렌더링합니다.
#[derive(Template)]
#[template(path = "csharp/main.cs.txt")]
struct CSharpTemplate<'a> {
    file: &'a CSharpFile,
}

/// Generates C# code for each `AstRoot` and saves it to the output directory.
pub fn generate_csharp_code(
    all_asts: &[AstRoot],
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output_dir)?;

    for ast in all_asts {
        let file_name = ast
            .path
            .file_name()
            .ok_or("파일 이름을 가져올 수 없습니다.")?;

        let mut output_cs_path = output_dir.join(file_name);
        output_cs_path.set_extension("cs");

        let csharp_file_model = build_csharp_model(ast)?;

        let template = CSharpTemplate {
            file: &csharp_file_model,
        };
        let csharp_code = template.render()?;
        fs::write(&output_cs_path, &csharp_code)?;
        println!("C# 파일이 생성되었습니다: {}", output_cs_path.display());
    }
    Ok(())
}

/// Builds a `CSharpFile` model from a single `AstRoot`.
fn build_csharp_model(ast: &AstRoot) -> Result<CSharpFile, Box<dyn std::error::Error>> {
    println!("Building C# model for {}...", ast.path.display());

    let mut csharp_file = CSharpFile::default();
    let mut global_types: Vec<TypeDef> = Vec::new();

    for def in &ast.definitions {
        match def {
            Definition::Namespace(ns) => {
                let namespace_def = process_namespace(ns)?;
                csharp_file.namespaces.push(namespace_def);
            }
            Definition::Table(table) => global_types.push(convert_table_to_type(table)?),
            Definition::Enum(e) => global_types.push(convert_enum_to_type(e)?),
            Definition::Embed(embed) => global_types.push(convert_embed_to_type(embed)?),
        }
    }

    // 파일 최상단에 정의된 타입들을 파일 이름 기반의 네임스페이스로 묶습니다.
    if !global_types.is_empty() {
        let default_namespace = ast
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Global")
            .to_upper_camel_case();

        // 이미 같은 이름의 네임스페이스가 있다면 타입을 추가하고, 없다면 새로 생성합니다.
        if let Some(existing_ns) = csharp_file
            .namespaces
            .iter_mut()
            .find(|ns| ns.name == default_namespace)
        {
            existing_ns.types.extend(global_types);
        } else {
            csharp_file.namespaces.push(NamespaceDef {
                name: default_namespace,
                types: global_types,
                nested_namespaces: Vec::new(),
            });
        }
    }

    Ok(csharp_file)
}

/// Processes an `ast::Namespace` into a `csharp_model::NamespaceDef`.
fn process_namespace(ns: &Namespace) -> Result<NamespaceDef, Box<dyn std::error::Error>> {
    let mut types = Vec::new();
    let mut nested_namespaces = Vec::new();

    for def in &ns.definitions {
        match def {
            Definition::Table(table) => types.push(convert_table_to_type(table)?),
            Definition::Enum(e) => types.push(convert_enum_to_type(e)?),
            Definition::Embed(embed) => types.push(convert_embed_to_type(embed)?),
            Definition::Namespace(nested_ns) => {
                nested_namespaces.push(process_namespace(nested_ns)?)
            }
        }
    }
    Ok(NamespaceDef {
        name: ns.path.join("."),
        types,
        nested_namespaces,
    })
}

/// Polygen 타입을 C# 타입 문자열로 변환합니다.
fn map_type_to_csharp(
    poly_type: &TypeWithCardinality,
) -> Result<String, Box<dyn std::error::Error>> {
    let base_type_str = match &poly_type.base_type {
        TypeName::Basic(basic) => match basic {
            BasicType::Bool => "bool".to_string(),
            BasicType::I8 => "sbyte".to_string(),
            BasicType::U8 => "byte".to_string(),
            BasicType::I16 => "short".to_string(),
            BasicType::U16 => "ushort".to_string(),
            BasicType::I32 => "int".to_string(),
            BasicType::U32 => "uint".to_string(),
            BasicType::I64 => "long".to_string(),
            BasicType::U64 => "ulong".to_string(),
            BasicType::F32 => "float".to_string(),
            BasicType::F64 => "double".to_string(),
            BasicType::String => "string".to_string(),
            BasicType::Bytes => "byte[]".to_string(),
        },
        TypeName::Path(parts) => parts.join("."),
    };

    match &poly_type.cardinality {
        Some(Cardinality::Array) => Ok(format!(
            "System.Collections.Generic.List<{}>",
            base_type_str
        )),
        Some(Cardinality::Optional) => {
            if let TypeName::Basic(basic) = &poly_type.base_type {
                match basic {
                    BasicType::Bool
                    | BasicType::I8
                    | BasicType::U8
                    | BasicType::I16
                    | BasicType::U16
                    | BasicType::I32
                    | BasicType::U32
                    | BasicType::I64
                    | BasicType::U64
                    | BasicType::F32
                    | BasicType::F64 => return Ok(format!("{}?", base_type_str)),
                    _ => {} // Non-nullable value types will be handled below.
                }
            }
            Ok(base_type_str) // Other types (string, byte[], custom types) are nullable by default in C#.
        }
        None => Ok(base_type_str), // No cardinality means it's a single, non-optional value.
    }
}

/// Converts an `ast::Table` to a `csharp_model::TypeDef`.
fn convert_table_to_type(table: &Table) -> Result<TypeDef, Box<dyn std::error::Error>> {
    let mut properties = Vec::new();
    let mut nested_classes = Vec::new();

    for member in &table.members {
        if let TableMember::Field(field) = member {
            match field {
                FieldDefinition::Regular(rf) => {
                    properties.push(convert_regular_field_to_property(rf)?);
                }
                FieldDefinition::InlineEmbed(ief) => {
                    // InlineEmbed를 처리하는 로직입니다.
                    // 1. 부모 클래스에 해당 중첩 클래스를 타입으로 사용하는 속성을 생성합니다.
                    let base_type_name = ief.name.to_upper_camel_case();

                    // Cardinality(배열 등)를 처리합니다.
                    let type_name = match &ief.cardinality {
                        Some(Cardinality::Array) => {
                            format!("System.Collections.Generic.List<{}>", base_type_name)
                        }
                        Some(Cardinality::Optional) => format!("{}?", base_type_name),
                        None => base_type_name,
                    };

                    properties.push(PropertyDef {
                        name: ief.name.to_pascal_case(),
                        type_name,
                        comment: ief.doc_comment.clone(),
                        attributes: vec![],
                    });

                    // 2. 중첩 클래스 자체의 정의를 생성합니다.
                    let nested_class = convert_inline_embed_to_nested_class(ief)?;
                    nested_classes.push(nested_class);
                }
            }
        }
        // TODO: Handle TableMember::Embed (named embeds inside tables)
    }

    Ok(TypeDef::Class(ClassDef {
        info: TypeInfo {
            name: table.name.clone(),
            comment: table.doc_comment.clone(),
        },
        properties,
        nested_classes,
    }))
}

/// Converts an `ast::Enum` to a `csharp_model::TypeDef`.
fn convert_enum_to_type(e: &Enum) -> Result<TypeDef, Box<dyn std::error::Error>> {
    Ok(TypeDef::Enum(EnumDef {
        info: TypeInfo {
            name: e.name.clone(),
            comment: e.doc_comment.clone(),
        },
        variants: e.variants.clone(),
    }))
}

/// Converts an `ast::Embed` to a `csharp_model::TypeDef`.
fn convert_embed_to_type(embed: &Embed) -> Result<TypeDef, Box<dyn std::error::Error>> {
    let mut properties = Vec::new();
    for field in &embed.fields {
        // Named embeds are converted to structs, which cannot contain nested classes in this model.
        if let FieldDefinition::Regular(rf) = field {
            properties.push(convert_regular_field_to_property(rf)?);
        } else {
            return Err("Named embeds cannot contain inline embeds in the C# generator.".into());
        }
    }

    Ok(TypeDef::Struct(StructDef {
        info: TypeInfo {
            name: embed.name.clone(),
            comment: embed.doc_comment.clone(),
        },
        properties,
    }))
}

/// Converts an `ast::RegularField` to a `csharp_model::PropertyDef`.
fn convert_regular_field_to_property(
    rf: &RegularField,
) -> Result<PropertyDef, Box<dyn std::error::Error>> {
    Ok(PropertyDef {
        name: rf.name.to_pascal_case(),
        type_name: map_type_to_csharp(&rf.field_type)?,
        comment: rf.doc_comment.clone(),
        attributes: vec![], // TODO: 애노테이션을 속성으로 변환하는 로직 구현
    })
}

/// Converts an `ast::InlineEmbedField` to a `csharp_model::ClassDef` for a nested class.
fn convert_inline_embed_to_nested_class(
    ief: &InlineEmbedField,
) -> Result<ClassDef, Box<dyn std::error::Error>> {
    let mut properties = Vec::new();

    for field in &ief.fields {
        match field {
            FieldDefinition::Regular(rf) => {
                properties.push(convert_regular_field_to_property(rf)?);
            }
            FieldDefinition::InlineEmbed(_) => {
                // For simplicity, we are not supporting recursively nested inline embeds for now.
                return Err("Recursively nested inline embeds are not supported.".into());
            }
        }
    }

    Ok(ClassDef {
        info: TypeInfo {
            name: ief.name.to_upper_camel_case(),
            comment: ief.doc_comment.clone(),
        },
        properties,
        nested_classes: vec![], // No deeper nesting.
    })
}
