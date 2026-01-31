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
    self, AnnotationDef, AnnotationParam, EnumDef, EnumItem, EnumMember, FieldDef, FileDef,
    ForeignKeyDef, IndexDef, NamespaceDef, NamespaceItem, RangeDef, RelationDef, RenameInfo,
    RenameKind, SchemaContext, StructDef, StructItem, TimezoneRef, TypeRef,
};
use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
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
pub fn register_core(engine: &mut Engine, preview_mode: bool) {
    register_types_and_getters(engine);
    register_common_helpers(engine, preview_mode, None);
}

/// Registers core IR types and helpers with an optional entry-point template name.
/// When `entry_template` is provided in preview mode, `write_file()` will wrap
/// content that has no source markers with the entry-point template name.
pub fn register_core_with_entry(engine: &mut Engine, preview_mode: bool, entry_template: &str) {
    register_types_and_getters(engine);
    register_common_helpers(engine, preview_mode, Some(entry_template.to_string()));
}

pub(crate) fn register_types_and_getters(engine: &mut Engine) {
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
    engine.register_get("renames", |f: &mut FileDef| {
        f.renames
            .iter()
            .map(|r| Dynamic::from(r.clone()))
            .collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<NamespaceDef>("NamespaceDef");
    engine.register_get("name", |ns: &mut NamespaceDef| ns.name.clone());
    engine.register_get("datasource", |ns: &mut NamespaceDef| {
        ns.datasource
            .clone()
            .map(Dynamic::from)
            .unwrap_or(Dynamic::UNIT)
    });
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
    engine.register_get("indexes", |s: &mut StructDef| {
        s.indexes
            .iter()
            .map(|idx| Dynamic::from(idx.clone()))
            .collect::<Vec<Dynamic>>()
    });
    engine.register_get("relations", |s: &mut StructDef| {
        s.relations
            .iter()
            .map(|rel| Dynamic::from(rel.clone()))
            .collect::<Vec<Dynamic>>()
    });
    engine.register_get("datasource", |s: &mut StructDef| {
        s.datasource
            .clone()
            .map(Dynamic::from)
            .unwrap_or(Dynamic::UNIT)
    });
    engine.register_get("cache_strategy", |s: &mut StructDef| {
        s.cache_strategy
            .clone()
            .map(Dynamic::from)
            .unwrap_or(Dynamic::UNIT)
    });
    engine.register_get("is_readonly", |s: &mut StructDef| s.is_readonly);
    engine.register_get("soft_delete_field", |s: &mut StructDef| {
        s.soft_delete_field
            .clone()
            .map(Dynamic::from)
            .unwrap_or(Dynamic::UNIT)
    });
    engine.register_get("is_embed", |s: &mut StructDef| s.is_embed);
    engine.register_get("pack_separator", |s: &mut StructDef| {
        s.pack_separator
            .clone()
            .map(Dynamic::from)
            .unwrap_or(Dynamic::UNIT)
    });

    // Register IndexDef type and getters
    engine.register_type_with_name::<IndexDef>("IndexDef");
    engine.register_get("name", |idx: &mut IndexDef| idx.name.clone());
    // Backward compatible: first field name/type for single-field indexes
    engine.register_get("field_name", |idx: &mut IndexDef| {
        idx.field_name().to_string()
    });
    engine.register_get("field_type", |idx: &mut IndexDef| {
        idx.field_type().cloned().unwrap_or_else(|| TypeRef {
            original: String::new(),
            fqn: String::new(),
            namespace_fqn: String::new(),
            type_name: String::new(),
            parent_type_path: String::new(),
            lang_type: String::new(),
            is_primitive: false,
            is_struct: false,
            is_enum: false,
            is_option: false,
            is_list: false,
            inner_type: None,
        })
    });
    engine.register_get("is_unique", |idx: &mut IndexDef| idx.is_unique);
    // New composite index support
    engine.register_get("fields", |idx: &mut IndexDef| {
        idx.fields
            .iter()
            .map(|f| Dynamic::from(f.clone()))
            .collect::<Vec<Dynamic>>()
    });
    engine.register_get("is_composite", |idx: &mut IndexDef| idx.is_composite());
    engine.register_get("field_count", |idx: &mut IndexDef| idx.field_count() as i64);
    engine.register_get("source", |idx: &mut IndexDef| idx.source.clone());

    // Register IndexFieldDef type and getters
    engine.register_type_with_name::<ir_model::IndexFieldDef>("IndexFieldDef");
    engine.register_get("name", |f: &mut ir_model::IndexFieldDef| f.name.clone());
    engine.register_get("field_type", |f: &mut ir_model::IndexFieldDef| {
        f.field_type.clone()
    });

    // Register RelationDef type and getters
    engine.register_type_with_name::<RelationDef>("RelationDef");
    engine.register_get("name", |rel: &mut RelationDef| rel.name.clone());
    engine.register_get("source_table_fqn", |rel: &mut RelationDef| {
        rel.source_table_fqn.clone()
    });
    engine.register_get("source_table_name", |rel: &mut RelationDef| {
        rel.source_table_name.clone()
    });
    engine.register_get("source_field", |rel: &mut RelationDef| {
        rel.source_field.clone()
    });

    // Register ForeignKeyDef type and getters
    engine.register_type_with_name::<ForeignKeyDef>("ForeignKeyDef");
    engine.register_get("target_table_fqn", |fk: &mut ForeignKeyDef| {
        fk.target_table_fqn.clone()
    });
    engine.register_get("target_field", |fk: &mut ForeignKeyDef| {
        fk.target_field.clone()
    });
    engine.register_get("alias", |fk: &mut ForeignKeyDef| {
        fk.alias.clone().map(Dynamic::from).unwrap_or(Dynamic::UNIT)
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
                StructItem::Field(f) => Ok((**f).clone()),
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
    engine.register_get("is_primary_key", |f: &mut FieldDef| f.is_primary_key);
    engine.register_get("is_unique", |f: &mut FieldDef| f.is_unique);
    engine.register_get("is_index", |f: &mut FieldDef| f.is_index);
    engine.register_get("foreign_key", |f: &mut FieldDef| {
        f.foreign_key
            .clone()
            .map(Dynamic::from)
            .unwrap_or(Dynamic::UNIT)
    });
    engine.register_get("max_length", |f: &mut FieldDef| {
        f.max_length
            .map(|v| Dynamic::from(v as i64))
            .unwrap_or(Dynamic::UNIT)
    });
    engine.register_get("default_value", |f: &mut FieldDef| {
        f.default_value
            .clone()
            .map(Dynamic::from)
            .unwrap_or(Dynamic::UNIT)
    });
    engine.register_get("range", |f: &mut FieldDef| {
        f.range.clone().map(Dynamic::from).unwrap_or(Dynamic::UNIT)
    });
    engine.register_get("regex_pattern", |f: &mut FieldDef| {
        f.regex_pattern
            .clone()
            .map(Dynamic::from)
            .unwrap_or(Dynamic::UNIT)
    });
    // Convenience methods for checking if constraints are set
    engine.register_get("has_max_length", |f: &mut FieldDef| f.max_length.is_some());
    engine.register_get("has_default_value", |f: &mut FieldDef| {
        f.default_value.is_some()
    });
    engine.register_get("has_range", |f: &mut FieldDef| f.range.is_some());
    engine.register_get("has_regex_pattern", |f: &mut FieldDef| {
        f.regex_pattern.is_some()
    });
    engine.register_get("has_foreign_key", |f: &mut FieldDef| {
        f.foreign_key.is_some()
    });
    // Auto-timestamp getters
    engine.register_get("auto_create", |f: &mut FieldDef| {
        f.auto_create
            .clone()
            .map(Dynamic::from)
            .unwrap_or(Dynamic::UNIT)
    });
    engine.register_get("auto_update", |f: &mut FieldDef| {
        f.auto_update
            .clone()
            .map(Dynamic::from)
            .unwrap_or(Dynamic::UNIT)
    });
    engine.register_get("has_auto_create", |f: &mut FieldDef| {
        f.auto_create.is_some()
    });
    engine.register_get("has_auto_update", |f: &mut FieldDef| {
        f.auto_update.is_some()
    });

    // Register TimezoneRef type and getters
    engine.register_type_with_name::<TimezoneRef>("TimezoneRef");
    engine.register_get("kind", |tz: &mut TimezoneRef| tz.kind.clone());
    engine.register_get("offset_hours", |tz: &mut TimezoneRef| {
        tz.offset_hours
            .map(|v| Dynamic::from(v as i64))
            .unwrap_or(Dynamic::UNIT)
    });
    engine.register_get("offset_minutes", |tz: &mut TimezoneRef| {
        tz.offset_minutes
            .map(|v| Dynamic::from(v as i64))
            .unwrap_or(Dynamic::UNIT)
    });
    engine.register_get("name", |tz: &mut TimezoneRef| {
        tz.name.clone().map(Dynamic::from).unwrap_or(Dynamic::UNIT)
    });

    // Register RangeDef type and getters
    engine.register_type_with_name::<RangeDef>("RangeDef");
    engine.register_get("min", |r: &mut RangeDef| r.min.clone());
    engine.register_get("max", |r: &mut RangeDef| r.max.clone());
    engine.register_get("literal_type", |r: &mut RangeDef| r.literal_type.clone());

    // TypeRef: expose type info to templates
    engine.register_type_with_name::<TypeRef>("TypeRef");
    engine.register_get("original", |t: &mut TypeRef| t.original.clone());
    engine.register_get("fqn", |t: &mut TypeRef| t.fqn.clone());
    engine.register_get("lang_type", |t: &mut TypeRef| t.lang_type.clone());
    engine.register_get("namespace_fqn", |t: &mut TypeRef| t.namespace_fqn.clone());
    engine.register_get("type_name", |t: &mut TypeRef| t.type_name.clone());
    engine.register_get("parent_type_path", |t: &mut TypeRef| {
        t.parent_type_path.clone()
    });
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
    engine.register_get("value", |m: &mut EnumMember| {
        m.value.map(Dynamic::from).unwrap_or(Dynamic::UNIT)
    });

    engine.register_type_with_name::<AnnotationDef>("AnnotationDef");
    engine.register_get("name", |a: &mut AnnotationDef| a.name.clone());
    engine.register_get("positional_args", |a: &mut AnnotationDef| {
        a.positional_args
            .iter()
            .map(|s| Dynamic::from(s.clone()))
            .collect::<Vec<Dynamic>>()
    });
    engine.register_get("params", |a: &mut AnnotationDef| {
        a.params
            .iter()
            .map(|p| Dynamic::from(p.clone()))
            .collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<AnnotationParam>("AnnotationParam");
    engine.register_get("key", |p: &mut AnnotationParam| p.key.clone());
    engine.register_get("value", |p: &mut AnnotationParam| p.value.clone());

    // Register RenameInfo type and getters
    engine.register_type_with_name::<RenameInfo>("RenameInfo");
    engine.register_get("kind", |r: &mut RenameInfo| r.kind.clone());
    engine.register_get("from_path", |r: &mut RenameInfo| {
        r.from_path
            .iter()
            .map(|s| Dynamic::from(s.clone()))
            .collect::<Vec<Dynamic>>()
    });
    engine.register_get("to_name", |r: &mut RenameInfo| r.to_name.clone());

    // Register RenameKind type and helpers
    engine.register_type_with_name::<RenameKind>("RenameKind");
    engine.register_fn("is_table", |k: &mut RenameKind| {
        matches!(k, RenameKind::Table)
    });
    engine.register_fn("is_field", |k: &mut RenameKind| {
        matches!(k, RenameKind::Field)
    });
    engine.register_fn("to_string", |k: &mut RenameKind| match k {
        RenameKind::Table => "table".to_string(),
        RenameKind::Field => "field".to_string(),
    });
}

fn register_common_helpers(
    engine: &mut Engine,
    preview_mode: bool,
    entry_template: Option<String>,
) {
    // Case conversion functions
    engine.register_fn("to_snake_case", |s: &str| s.to_snake_case());
    engine.register_fn("to_pascal_case", |s: &str| s.to_pascal_case());
    engine.register_fn("to_camel_case", |s: &str| s.to_lower_camel_case());

    // source_mark(template_name, content) -> content wrapped with source markers
    // Used in preview mode to track which template generated each code block.
    // Markers: /*@source:template_name*/ ... /*@/source*/
    engine.register_fn(
        "source_mark",
        |template_name: &str, content: &str| -> String {
            format!("/*@source:{}*/\n{}/*@/source*/\n", template_name, content)
        },
    );

    // render_items(items[], template_path, var_name) -> string
    // In preview mode, wraps each rendered item with source markers.
    let preview_render = preview_mode;
    engine.register_fn(
        "render_items",
        move |context: NativeCallContext,
              items: Array,
              template_path: &str,
              var_name: &str|
              -> Result<String, Box<EvalAltResult>> {
            let template = match std::fs::read_to_string(template_path) {
                Ok(s) => s,
                Err(e) => {
                    return Err(
                        format!("Failed to read template file '{}': {}", template_path, e).into(),
                    )
                }
            };

            let engine = context.engine();
            let mut result = String::new();
            let template_literal = format!("`{}`", template);

            for item in items {
                let mut scope = Scope::new();
                scope.push(var_name, item);
                let rendered = engine.eval_with_scope::<String>(&mut scope, &template_literal)?;
                if preview_render {
                    let filename = Path::new(template_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(template_path);
                    result.push_str(&format!(
                        "/*@source:{}*/\n{}/*@/source*/\n",
                        filename, rendered
                    ));
                } else {
                    result.push_str(&rendered);
                }
            }
            Ok(result)
        },
    );

    // include(path) -> string
    // In preview mode, wraps the template content with source markers so that
    // after eval(), the output is automatically annotated with its source template.
    let preview_include = preview_mode;
    engine.register_fn(
        "include",
        move |path: &str| -> Result<String, Box<EvalAltResult>> {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    if preview_include {
                        let filename = Path::new(path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(path);
                        Ok(format!(
                            "/*@source:{}*/\n{}\n/*@/source*/",
                            filename, content
                        ))
                    } else {
                        Ok(content)
                    }
                }
                Err(e) => Err(Box::new(EvalAltResult::ErrorSystem(
                    format!("File Read Error at path: {}", path),
                    e.to_string().into(),
                ))),
            }
        },
    );

    // write_file(path, content)
    // In preview mode with entry_template set, wraps content with a source marker
    // if no markers exist yet (for inline templates that don't use include()).
    let preview_write = preview_mode;
    let entry_tmpl = entry_template;
    engine.register_fn(
        "write_file",
        move |path: &str, content: &str| -> Result<(), Box<EvalAltResult>> {
            // In preview mode, wrap unmarked content with the entry-point template name
            let final_content = if preview_write {
                if let Some(ref tmpl_name) = entry_tmpl {
                    if !content.contains("/*@source:") {
                        format!("/*@source:{}*/\n{}/*@/source*/\n", tmpl_name, content)
                    } else {
                        content.to_string()
                    }
                } else {
                    content.to_string()
                }
            } else {
                content.to_string()
            };

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

            match std::fs::write(path, &final_content) {
                Ok(_) => Ok(()),
                Err(e) => Err(Box::new(EvalAltResult::ErrorSystem(
                    "File Write Error".to_string(),
                    e.to_string().into(),
                ))),
            }
        },
    );
}
