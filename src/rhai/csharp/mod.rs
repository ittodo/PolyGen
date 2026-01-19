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

fn register_csharp_helpers(engine: &mut Engine) {
    engine.register_fn("cs_csv_header_name", cs_csv_header_name);
    engine.register_fn("cs_write_csv_expr", cs_write_csv_expr);
    engine.register_fn("cs_map_type", type_mapping::cs_map_type);
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
        let inner = t.inner_type.as_ref().unwrap();
        let inner_access = format!("{}[0]", access);
        let inner_write = generate_value_write(inner, &inner_access);

        format!(
            "if ({access} != null && {access}.Count > 0) {{ {inner_write} }} else {{ cols.Add(string.Empty); }}"
        )
    } else if t.is_option {
        let _inner = t.inner_type.as_ref().unwrap();
        generate_value_write(t, access)
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
