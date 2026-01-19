//! Core IR Type Registration for Rhai
//!
//! This module registers all IR (Intermediate Representation) types and their
//! accessors with the Rhai scripting engine. This enables Rhai templates to
//! inspect and traverse the schema structure.
//!
//! ## Registered Types
//!
//! | Type | Description |
//! |------|-------------|
//! | `SchemaContext` | Root context containing all files |
//! | `FileDef` | A single schema file with namespaces |
//! | `NamespaceDef` | A namespace containing items |
//! | `NamespaceItem` | Enum: Struct, Enum, Comment, or Namespace |
//! | `StructDef` | A struct/table definition |
//! | `StructItem` | Enum: Field, Comment, or EmbeddedStruct |
//! | `FieldDef` | A field within a struct |
//! | `TypeRef` | Type information for a field |
//! | `EnumDef` | An enum definition |
//! | `EnumItem` | Enum: Member or Comment |
//! | `EnumMember` | A single enum variant |
//! | `AnnotationDef` | An annotation on a type or field |
//! | `AnnotationParam` | A parameter within an annotation |
//!
//! ## Helper Functions
//!
//! - `write_file(path, content)` - Write content to a file
//! - `run_template(path, scope)` - Execute another template
//! - `run_template_str(script, scope)` - Execute template from string
//! - `join(arr, sep)` - Join array elements with separator
//! - `indent(text, spaces)` - Indent text by N spaces
//! - `to_pascal_case(s)` / `to_camel_case(s)` / `to_snake_case(s)` - Case conversion
//!
//! ---
//!
//! 이 모듈은 모든 IR(중간 표현) 타입과 접근자를 Rhai 스크립팅 엔진에 등록합니다.
//! 이를 통해 Rhai 템플릿에서 스키마 구조를 검사하고 순회할 수 있습니다.

use crate::ir_model::{
    AnnotationDef, AnnotationParam, EnumDef, EnumItem, EnumMember, FieldDef, FileDef, NamespaceDef,
    NamespaceItem, SchemaContext, StructDef, StructItem, TypeRef,
};
use rhai::{Array, Dynamic, Engine, EvalAltResult, NativeCallContext, Scope};
use std::path::Path;

/// Registers core IR types and common helper functions into a Rhai Engine.
///
/// This function must be called before any template execution. It sets up:
/// - All IR type registrations with getters
/// - Type checking functions (`is_struct`, `is_enum`, etc.)
/// - Type conversion functions (`as_struct`, `as_enum`, etc.)
/// - Utility functions (`write_file`, `join`, `indent`, etc.)
///
/// # Example
///
/// ```ignore
/// let mut engine = Engine::new();
/// register_core(&mut engine);
/// // Engine is now ready for template execution
/// ```
pub fn register_core(engine: &mut Engine) {
    register_types_and_getters(engine);
    register_common_helpers(engine);
}

fn register_types_and_getters(engine: &mut Engine) {
    engine.register_type_with_name::<SchemaContext>("SchemaContext");
    engine.register_get("files", |ctx: &mut SchemaContext| {
        ctx.files
            .iter()
            .map(|f| Dynamic::from(f.clone()))
            .collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<FileDef>("FileDef");
    engine.register_get("path", |f: &mut FileDef| f.path.clone());
    engine.register_get("namespaces", |f: &mut FileDef| {
        f.namespaces
            .iter()
            .map(|ns| Dynamic::from(ns.clone()))
            .collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<NamespaceDef>("NamespaceDef");
    engine.register_get("name", |ns: &mut NamespaceDef| ns.name.clone());
    engine.register_get("items", |ns: &mut NamespaceDef| {
        ns.items
            .iter()
            .map(|item| Dynamic::from(item.clone()))
            .collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<NamespaceItem>("NamespaceItem");
    engine.register_fn("is_struct", |item: &mut NamespaceItem| {
        matches!(item, NamespaceItem::Struct(_))
    });
    engine.register_fn(
        "as_struct",
        |item: &mut NamespaceItem| -> Result<StructDef, Box<EvalAltResult>> {
            match item {
                NamespaceItem::Struct(s) => Ok(s.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to StructDef".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );
    engine.register_fn("is_enum", |item: &mut NamespaceItem| {
        matches!(item, NamespaceItem::Enum(_))
    });
    engine.register_fn(
        "as_enum",
        |item: &mut NamespaceItem| -> Result<EnumDef, Box<EvalAltResult>> {
            match item {
                NamespaceItem::Enum(e) => Ok(e.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to EnumDef".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_fn("is_comment", |item: &mut NamespaceItem| {
        matches!(item, NamespaceItem::Comment(_))
    });
    engine.register_fn(
        "as_comment",
        |item: &mut NamespaceItem| -> Result<String, Box<EvalAltResult>> {
            match item {
                NamespaceItem::Comment(c) => Ok(c.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to Comment".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_fn("is_namespace", |item: &mut NamespaceItem| {
        matches!(item, NamespaceItem::Namespace(_))
    });
    engine.register_fn(
        "as_namespace",
        |item: &mut NamespaceItem| -> Result<NamespaceDef, Box<EvalAltResult>> {
            match item {
                NamespaceItem::Namespace(ns) => Ok((**ns).clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to NamespaceDef".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_type_with_name::<StructDef>("StructDef");
    engine.register_get("name", |s: &mut StructDef| s.name.clone());
    engine.register_get("fqn", |s: &mut StructDef| s.fqn.clone());
    engine.register_get("header", |s: &mut StructDef| {
        s.header
            .iter()
            .map(|header| Dynamic::from(header.clone()))
            .collect::<Vec<Dynamic>>()
    });
    engine.register_get("items", |s: &mut StructDef| {
        s.items
            .iter()
            .map(|item| Dynamic::from(item.clone()))
            .collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<StructItem>("StructItem");
    engine.register_fn("is_field", |item: &mut StructItem| {
        matches!(item, StructItem::Field(_))
    });

    engine.register_fn("is_comment", |item: &mut StructItem| {
        matches!(item, StructItem::Comment(_))
    });

    engine.register_fn("is_annotation", |item: &mut StructItem| {
        matches!(item, StructItem::Annotation(_))
    });

    engine.register_fn("is_embedded_struct", |item: &mut StructItem| {
        matches!(item, StructItem::EmbeddedStruct(_))
    });

    engine.register_fn("is_inline_enum", |item: &mut StructItem| {
        matches!(item, StructItem::InlineEnum(_))
    });

    engine.register_fn(
        "as_field",
        |item: &mut StructItem| -> Result<FieldDef, Box<EvalAltResult>> {
            match item {
                StructItem::Field(f) => Ok(f.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to FieldDef".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_fn(
        "as_comment",
        |item: &mut StructItem| -> Result<String, Box<EvalAltResult>> {
            match item {
                StructItem::Comment(f) => Ok(f.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to FieldDef".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_fn(
        "as_annotation",
        |item: &mut StructItem| -> Result<AnnotationDef, Box<EvalAltResult>> {
            match item {
                StructItem::Annotation(f) => Ok(f.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to FieldDef".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_fn(
        "as_embedded_struct",
        |item: &mut StructItem| -> Result<StructDef, Box<EvalAltResult>> {
            match item {
                StructItem::EmbeddedStruct(s) => Ok(s.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to EmbeddedStruct".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_fn(
        "as_inline_enum",
        |item: &mut StructItem| -> Result<EnumDef, Box<EvalAltResult>> {
            match item {
                StructItem::InlineEnum(e) => Ok(e.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to InlineEnum".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_type_with_name::<FieldDef>("FieldDef");
    engine.register_get("name", |f: &mut FieldDef| f.name.clone());
    engine.register_get("field_type", |f: &mut FieldDef| f.field_type.clone());
    engine.register_get("attributes", |f: &mut FieldDef| {
        f.attributes
            .iter()
            .map(|attr| Dynamic::from(attr.clone()))
            .collect::<Vec<Dynamic>>()
    });

    // TypeRef: expose type info to templates
    engine.register_type_with_name::<TypeRef>("TypeRef");
    engine.register_get("original", |t: &mut TypeRef| t.original.clone());
    engine.register_get("fqn", |t: &mut TypeRef| t.fqn.clone());
    engine.register_get("lang_type", |t: &mut TypeRef| t.lang_type.clone());
    engine.register_get("namespace_fqn", |t: &mut TypeRef| t.namespace_fqn.clone());
    engine.register_get("type_name", |t: &mut TypeRef| t.type_name.clone());
    engine.register_get("parent_type_path", |t: &mut TypeRef| t.parent_type_path.clone());
    engine.register_get("is_primitive", |t: &mut TypeRef| t.is_primitive);
    engine.register_get("is_option", |t: &mut TypeRef| t.is_option);
    engine.register_get("is_list", |t: &mut TypeRef| t.is_list);
    engine.register_get("is_struct", |t: &mut TypeRef| t.is_struct);
    engine.register_get("is_enum", |t: &mut TypeRef| t.is_enum);
    // For inner_type, return value or unit; and expose a convenience flag
    engine.register_get("has_inner", |t: &mut TypeRef| t.inner_type.is_some());
    engine.register_get("inner", |t: &mut TypeRef| {
        if let Some(inner) = &t.inner_type {
            Dynamic::from((**inner).clone())
        } else {
            Dynamic::UNIT
        }
    });

    engine.register_type_with_name::<EnumDef>("EnumDef");
    engine.register_get("name", |e: &mut EnumDef| e.name.clone());
    engine.register_get("items", |e: &mut EnumDef| {
        e.items
            .iter()
            .map(|item| Dynamic::from(item.clone()))
            .collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<EnumItem>("EnumItem");
    engine.register_fn("is_member", |item: &mut EnumItem| {
        matches!(item, EnumItem::Member(_))
    });
    engine.register_fn("is_comment", |item: &mut EnumItem| {
        matches!(item, EnumItem::Comment(_))
    });
    engine.register_fn(
        "as_member",
        |item: &mut EnumItem| -> Result<EnumMember, Box<EvalAltResult>> {
            match item {
                EnumItem::Member(m) => Ok(m.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to EnumMember".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );
    engine.register_fn(
        "as_comment",
        |item: &mut EnumItem| -> Result<String, Box<EvalAltResult>> {
            match item {
                EnumItem::Comment(m) => Ok(m.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to EnumItem.Comment".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_type_with_name::<EnumMember>("EnumMember");
    engine.register_get("name", |m: &mut EnumMember| m.name.clone());

    engine.register_type_with_name::<AnnotationDef>("AnnotationDef");
    engine.register_get("name", |a: &mut AnnotationDef| a.name.clone());
    engine.register_get("params", |a: &mut AnnotationDef| {
        a.params
            .iter()
            .map(|p| Dynamic::from(p.clone()))
            .collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<AnnotationParam>("AnnotationParam");
    engine.register_get("key", |p: &mut AnnotationParam| p.key.clone());
    engine.register_get("value", |p: &mut AnnotationParam| p.value.clone());
}

fn register_common_helpers(engine: &mut Engine) {
    // render_items(items[], template_path, var_name) -> string
    engine.register_fn(
        "render_items",
        |context: NativeCallContext,
         items: Array,
         template_path: &str,
         var_name: &str|
         -> Result<String, Box<EvalAltResult>> {
            let template = match std::fs::read_to_string(template_path) {
                Ok(s) => s,
                Err(e) => return Err(format!("Failed to read template file '{}': {}", template_path, e).into()),
            };

            let engine = context.engine();
            let mut result = String::new();
            let template_literal = format!("`{}`", template);

            for item in items {
                let mut scope = Scope::new();
                scope.push(var_name, item);
                let rendered = engine.eval_with_scope::<String>(&mut scope, &template_literal)?;
                result.push_str(&rendered);
            }
            Ok(result)
        },
    );

    // include(path) -> string
    engine.register_fn(
        "include",
        |path: &str| -> Result<String, Box<EvalAltResult>> {
            match std::fs::read_to_string(path) {
                Ok(s) => Ok(s),
                Err(e) => Err(Box::new(EvalAltResult::ErrorSystem(
                    format!("File Read Error at path: {}", path),
                    e.to_string().into(),
                ))),
            }
        },
    );

    // write_file(path, content)
    engine.register_fn(
        "write_file",
        |path: &str, content: &str| -> Result<(), Box<EvalAltResult>> {
            if let Some(p) = Path::new(path).parent() {
                if !p.exists() {
                    if let Err(e) = std::fs::create_dir_all(p) {
                        return Err(Box::new(EvalAltResult::ErrorSystem(
                            "Directory Creation Error".to_string(),
                            e.to_string().into(),
                        )));
                    }
                }
            }

            match std::fs::write(path, content) {
                Ok(_) => Ok(()),
                Err(e) => Err(Box::new(EvalAltResult::ErrorSystem(
                    "File Write Error".to_string(),
                    e.to_string().into(),
                ))),
            }
        },
    );
}
