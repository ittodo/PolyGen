//! C# Code Generation Module
//!
//! This module provides Rhai functions for generating C# code. It is organized
//! into submodules for different aspects of code generation.
//!
//! ## Submodules
//!
//! - [`type_mapping`]: IR to C# type conversion utilities
//! - [`loaders`]: Data loader code generation (CSV, JSON, Binary)
//!
//! ## Registered Rhai Functions
//!
//! The following functions are registered when [`register_csharp`] is called:
//!
//! ### Field Helpers
//! - `cs_csv_header_name(field)` - Get CSV header name for a field
//! - `cs_write_csv_expr(field, obj_var)` - Generate CSV write expression
//!
//! ### CSV Loader Functions (from loaders module)
//! - `csv_headers_for_struct(struct, ns, files)` - Collect CSV headers
//! - `csv_append_code_for(...)` - Generate append code
//! - `csv_read_fields_for_struct(...)` - Generate read code
//! - `csv_read_fields_for_struct_indexed(...)` - Generate indexed read code
//! - `csv_dynamic_methods_for_struct(...)` - Generate dynamic methods
//!
//! ## Usage in Templates
//!
//! ```rhai
//! // In a Rhai template:
//! let headers = csv_headers_for_struct(my_struct, current_ns, all_files);
//! let read_code = csv_read_fields_for_struct(my_struct, "obj", "\"\"", ns, files);
//! ```
//!
//! ---
//!
//! 이 모듈은 C# 코드 생성을 위한 Rhai 함수들을 제공합니다.
//! 코드 생성의 여러 측면을 위한 하위 모듈로 구성되어 있습니다.

pub mod loaders;
pub mod type_mapping;

use crate::ir_model::{FieldDef, TypeRef};
use rhai::Engine;

/// Registers all C# code generation helpers.
///
/// English: Call after `register_core`. Includes type mapping and loader helpers.
///
/// 한국어: `register_core` 이후에 호출하세요. 타입 매핑과 로더 헬퍼를 포함합니다.
pub fn register_csharp(engine: &mut Engine) {
    register_csharp_helpers(engine);
    loaders::register_csv_loaders(engine);
}

pub(crate) fn register_csharp_helpers(engine: &mut Engine) {
    engine.register_fn("cs_csv_header_name", cs_csv_header_name);
    engine.register_fn("cs_write_csv_expr", cs_write_csv_expr);
    engine.register_fn("cs_map_type", type_mapping::cs_map_type);
    engine.register_fn("cs_field_default_literal", cs_field_default_literal);
}

/// Generates the CSV header name for a field.
/// e.g., "tags" -> "tags[0]" if it's a list.
fn cs_csv_header_name(field: &mut FieldDef) -> String {
    let base_name = &field.name;
    if field.field_type.is_list {
        format!("{}[0]", base_name)
    } else {
        base_name.clone()
    }
}

/// Generates the C# expression to write a field to the CSV row list.
/// e.g., `cols.Add(obj.name);` or `cols.Add(CsvUtils.ToStringInvariant(obj.score));`
fn cs_write_csv_expr(field: &mut FieldDef, obj_var: &str) -> String {
    let field_name = &field.name;
    let access = format!("{}.{}", obj_var, field_name);
    let type_ref = &field.field_type;

    generate_write_logic(type_ref, &access)
}

fn generate_write_logic(t: &TypeRef, access: &str) -> String {
    if t.is_list {
        if let Some(inner) = t.inner_type.as_ref() {
            let inner_access = format!("{}[0]", access);
            let inner_write = generate_value_write(inner, &inner_access);

            format!(
                "if ({access} != null && {access}.Count > 0) {{ {inner_write} }} else {{ cols.Add(string.Empty); }}"
            )
        } else {
            "cols.Add(string.Empty);".to_string()
        }
    } else {
        generate_value_write(t, access)
    }
}

fn generate_value_write(t: &TypeRef, access: &str) -> String {
    if t.is_primitive {
        if t.lang_type == "string" {
            format!("cols.Add(CsvUtils.Escape({} ?? string.Empty));", access)
        } else {
            format!("cols.Add(CsvUtils.ToStringInvariant({}));", access)
        }
    } else if t.is_enum {
        format!("cols.Add({}.ToString());", access)
    } else if t.is_option {
        format!("cols.Add(CsvUtils.ToStringInvariant({}));", access)
    } else {
        "cols.Add(string.Empty);".to_string()
    }
}

pub fn cs_field_default_literal(field: FieldDef) -> String {
    let Some(default_value) = field.default_value.as_deref() else {
        return String::new();
    };

    let base_type = field
        .field_type
        .inner_type
        .as_deref()
        .filter(|_| field.field_type.is_option)
        .unwrap_or(&field.field_type);

    if base_type.is_enum || base_type.lang_type.ends_with("Enum") {
        let enum_type = csharp_default_type_name(base_type);
        if default_value.parse::<i64>().is_ok() {
            return format!("({enum_type}){default_value}");
        }
        return format!("{enum_type}.{default_value}");
    }

    match base_type.lang_type.as_str() {
        "f32" => format!("{default_value}f"),
        "f64" => default_value.to_string(),
        "u64" | "i64" => format!("{default_value}L"),
        "string" => format!("\"{default_value}\""),
        "bool" => default_value.to_lowercase(),
        _ => default_value.to_string(),
    }
}

fn csharp_default_type_name(type_ref: &TypeRef) -> String {
    if type_ref.lang_type.contains('.') && !type_ref.lang_type.starts_with("global::") {
        format!("global::{}", type_ref.lang_type)
    } else {
        type_ref.lang_type.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir_model::TypeRef;

    fn enum_field(default_value: &str) -> FieldDef {
        FieldDef {
            name: "state".to_string(),
            field_type: TypeRef {
                original: "StateEnum".to_string(),
                fqn: "Task.StateEnum".to_string(),
                namespace_fqn: "Task".to_string(),
                type_name: "StateEnum".to_string(),
                parent_type_path: "Task".to_string(),
                lang_type: "StateEnum".to_string(),
                is_primitive: false,
                is_struct: false,
                is_enum: true,
                is_option: false,
                is_list: false,
                inner_type: None,
            },
            attributes: vec![],
            is_primary_key: false,
            is_unique: false,
            is_index: false,
            foreign_key: None,
            max_length: None,
            default_value: Some(default_value.to_string()),
            range: None,
            regex_pattern: None,
            auto_create: None,
            auto_update: None,
            search_index: None,
        }
    }

    #[test]
    fn csharp_enum_default_literal_is_qualified() {
        assert_eq!(
            cs_field_default_literal(enum_field("Todo")),
            "StateEnum.Todo"
        );
    }

    #[test]
    fn csharp_enum_integer_default_literal_is_cast() {
        assert_eq!(cs_field_default_literal(enum_field("1")), "(StateEnum)1");
    }
}
