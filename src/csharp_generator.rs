use crate::ast::{
    AstRoot, BasicType, Cardinality, Definition, Embed, Enum, FieldDefinition, InlineEmbedField,
    Namespace, Table, TableMember, TypeName, TypeWithCardinality,
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
    for member in &table.members {
        if let TableMember::Field(field) = member {
            properties.push(convert_field_to_property(field)?);
        }
        // TODO: Handle nested definitions
    }

    Ok(TypeDef::Class(ClassDef {
        info: TypeInfo {
            name: table.name.clone(),
            comment: table.doc_comment.clone(),
        },
        properties,
        nested_classes: vec![], // TODO: 중첩 클래스 변환 로직 구현
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
        properties.push(convert_field_to_property(field)?);
    }

    Ok(TypeDef::Struct(StructDef {
        info: TypeInfo {
            name: embed.name.clone(),
            comment: embed.doc_comment.clone(),
        },
        properties,
    }))
}

/// Converts an `ast::FieldDefinition` to a `csharp_model::PropertyDef`.
fn convert_field_to_property(
    field: &FieldDefinition,
) -> Result<PropertyDef, Box<dyn std::error::Error>> {
    match field {
        FieldDefinition::Regular(rf) => Ok(PropertyDef {
            name: rf.name.to_pascal_case(),
            type_name: map_type_to_csharp(&rf.field_type)?,
            comment: rf.doc_comment.clone(),
            attributes: vec![], // TODO: 애노테이션을 속성으로 변환하는 로직 구현
        }),
        FieldDefinition::InlineEmbed(ief) => convert_inline_embed_to_property(ief),
    }
}

/// Converts an `ast::InlineEmbedField` to a `csharp_model::PropertyDef`.
/// 인라인 Embed 필드를 C# 속성으로 변환합니다. 이 때, 인라인 Embed는 새로운 클래스/구조체로 처리됩니다.
fn convert_inline_embed_to_property(
    ief: &InlineEmbedField,
) -> Result<PropertyDef, Box<dyn std::error::Error>> {
    // 새로운 클래스/구조체 이름을 생성합니다. (예: "부모클래스이름 + 속성이름")
    // 여기서는 단순히 속성 이름(파스칼 케이스)을 사용하며, 실제 클래스/구조체 정의는 별도로 처리됩니다.
    let type_name = ief.name.to_pascal_case();
    // TODO: 실제 클래스/구조체 정의를 생성하고 관리하는 로직을 추가해야 합니다.
    //       이 로직은 현재 함수의 호출자(예: convert_table_to_type)에서 처리될 수 있습니다.

    Ok(PropertyDef {
        name: ief.name.to_pascal_case(),
        type_name, // 생성된 클래스/구조체 이름을 사용합니다.
        comment: ief.doc_comment.clone(),
        attributes: vec![],
        // TODO: 인라인 Embed 필드의 속성에 대한 추가적인 처리 (예: JsonIgnore 등)가 필요하면 여기에 추가합니다.
        //       예를 들어, [JsonIgnore] 속성을 추가하여 C# 클래스에서 해당 속성을 직렬화/역직렬화에서 제외할 수 있습니다.
        //       이러한 처리는 ief.annotations를 참고하여 구현할 수 있습니다.
    })
}
