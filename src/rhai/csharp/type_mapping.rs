//! C# Type Mapping Utilities
//!
//! This module provides functions for mapping IR types to C# types.
//! These utilities handle primitive type conversion, list types,
//! optional types, and inline enum names.
//!
//! ## Type Mapping Table
//!
//! | IR Type  | C# Type  |
//! |----------|----------|
//! | `u8`     | `byte`   |
//! | `i8`     | `sbyte`  |
//! | `u16`    | `ushort` |
//! | `i16`    | `short`  |
//! | `u32`    | `uint`   |
//! | `i32`    | `int`    |
//! | `u64`    | `ulong`  |
//! | `i64`    | `long`   |
//! | `f32`    | `float`  |
//! | `f64`    | `double` |
//! | `bool`   | `bool`   |
//! | `string` | `string` |
//!
//! ## Special Cases
//!
//! - `Option<T>` → unwrapped to inner type `T`
//! - `List<T>` → `List<T>` with inner type converted
//! - Inline enums (nested in the current struct) → `StructName.EnumName`
//! - Custom types → passed through as-is
//!
//! ---
//!
//! 이 모듈은 IR 타입을 C# 타입으로 매핑하는 함수들을 제공합니다.
//! 기본 타입 변환, 리스트 타입, 옵션 타입, 인라인 열거형 이름을 처리합니다.

use crate::ir_model::{FileDef, StructDef, StructItem, TypeRef};
use crate::rhai::common::unwrap_option;

/// Checks if a type string represents a primitive-like type.
///
/// English: Returns true for numeric types, bool, string, and timestamp.
///
/// 한국어: 숫자 타입, bool, string, timestamp에 대해 true를 반환합니다.
pub fn is_primitive_like(t: &str) -> bool {
    matches!(
        t,
        "u8" | "i8"
            | "u16"
            | "i16"
            | "u32"
            | "i32"
            | "u64"
            | "i64"
            | "f32"
            | "f64"
            | "bool"
            | "string"
            | "bytes"
            | "timestamp"
    )
}

/// Checks if a name uses the legacy inline enum suffix.
///
/// English: Kept for older generated IR/tests that used `Field__Enum`.
///
/// 한국어: 예전 `Field__Enum` 형식과의 호환을 위해 유지합니다.
pub fn is_inline_enum_name(name: &str) -> bool {
    name.ends_with("__Enum")
}

/// Checks whether a name is an inline enum nested in the current struct.
pub fn is_inline_enum_name_in_struct(ctx_struct: &StructDef, name: &str) -> bool {
    is_inline_enum_name(name)
        || ctx_struct
            .items
            .iter()
            .any(|item| matches!(item, StructItem::InlineEnum(e) if e.name == name))
}

pub fn cs_enum_type_for(
    files: &[FileDef],
    ctx_struct: &StructDef,
    current_ns_name: &str,
    name: &str,
) -> Option<String> {
    if is_inline_enum_name_in_struct(ctx_struct, name) {
        Some(format!("{}.{}", ctx_struct.name, name))
    } else if crate::rhai::common::resolve_enum(files, name, current_ns_name).is_some() {
        Some(name.to_string())
    } else {
        None
    }
}

/// Maps an IR primitive type to a C# primitive type.
///
/// English: Returns the corresponding C# type name for IR primitives.
///
/// 한국어: IR 기본 타입에 해당하는 C# 타입 이름을 반환합니다.
pub fn map_cs_primitive(t: &str) -> Option<&'static str> {
    match t {
        "u8" => Some("byte"),
        "i8" => Some("sbyte"),
        "u16" => Some("ushort"),
        "i16" => Some("short"),
        "u32" => Some("uint"),
        "i32" => Some("int"),
        "u64" => Some("ulong"),
        "i64" => Some("long"),
        "f32" => Some("float"),
        "f64" => Some("double"),
        "bool" => Some("bool"),
        "string" => Some("string"),
        "bytes" => Some("byte[]"),
        "timestamp" => Some("DateTime"),
        _ => None,
    }
}

/// Converts an IR type string to a C# type string.
///
/// English: Handles List<>, Option<>, primitives, inline enums, and custom types.
///
/// 한국어: List<>, Option<>, 기본 타입, 인라인 열거형, 사용자 정의 타입을 처리합니다.
pub fn cs_type_for(
    _files: &[FileDef],
    ctx_struct: &StructDef,
    _current_ns_name: &str,
    type_string: &str,
) -> String {
    let core = unwrap_option(type_string);
    if let Some(inner) = core.strip_prefix("List<").and_then(|s| s.strip_suffix('>')) {
        let inner_cs = cs_type_for(_files, ctx_struct, _current_ns_name, inner);
        return format!("List<{}>", inner_cs);
    }
    if let Some(p) = map_cs_primitive(core) {
        return p.to_string();
    }
    // enum type
    if is_inline_enum_name_in_struct(ctx_struct, core) {
        return format!("{}.{}", ctx_struct.name, core);
    }
    // external or same-namespace struct/enum: assume type_string already usable as C# type
    core.to_string()
}

/// Maps a TypeRef to a C# type string.
///
/// English: Handles Option<T>, List<T>, primitives, and custom types.
/// Adds `global::` prefix for custom types containing ".".
///
/// 한국어: Option<T>, List<T>, 기본 타입, 사용자 정의 타입을 처리합니다.
/// "."를 포함하는 사용자 정의 타입에는 `global::` 접두사를 추가합니다.
pub fn cs_map_type(type_ref: &mut TypeRef) -> String {
    cs_map_type_string(&type_ref.original)
}

/// Maps a type string to a C# type string.
///
/// Recursively handles nested types like `Option<List<T>>`.
fn cs_map_type_string(type_string: &str) -> String {
    // Handle Option<T> -> T?
    if let Some(inner) = type_string
        .strip_prefix("Option<")
        .and_then(|s| s.strip_suffix('>'))
    {
        let mapped_inner = cs_map_type_string(inner);
        return format!("{}?", mapped_inner);
    }

    // Handle List<T> -> List<mapped_inner>
    if let Some(inner) = type_string
        .strip_prefix("List<")
        .and_then(|s| s.strip_suffix('>'))
    {
        let mapped_inner = cs_map_type_string(inner);
        return format!("List<{}>", mapped_inner);
    }

    // Handle primitives
    if let Some(cs_type) = map_cs_primitive(type_string) {
        return cs_type.to_string();
    }

    // Custom types: add global:: prefix if contains "." and not already prefixed
    if type_string.contains('.') && !type_string.starts_with("global::") {
        format!("global::{}", type_string)
    } else {
        type_string.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir_model::{EnumDef, NamespaceDef};

    fn struct_with_inline_enum(enum_name: &str) -> StructDef {
        StructDef {
            name: "Task".to_string(),
            fqn: "Task".to_string(),
            is_embed: false,
            datasource: None,
            cache_strategy: None,
            load: None,
            is_readonly: false,
            soft_delete_field: None,
            pack_separator: None,
            items: vec![StructItem::InlineEnum(EnumDef {
                name: enum_name.to_string(),
                fqn: format!("Task.{enum_name}"),
                items: vec![],
            })],
            header: vec![],
            indexes: vec![],
            refs: vec![],
            relations: vec![],
        }
    }

    #[test]
    fn csharp_inline_enum_type_is_nested_in_owner_struct() {
        let task = struct_with_inline_enum("StateEnum");
        assert_eq!(cs_type_for(&[], &task, "", "StateEnum"), "Task.StateEnum");
        assert_eq!(
            cs_enum_type_for(&[], &task, "", "StateEnum"),
            Some("Task.StateEnum".to_string())
        );
    }

    #[test]
    fn csharp_named_enum_type_stays_unqualified() {
        let file = FileDef {
            path: "test.poly".to_string(),
            namespaces: vec![NamespaceDef {
                name: "".to_string(),
                datasource: None,
                items: vec![crate::ir_model::NamespaceItem::Enum(EnumDef {
                    name: "Status".to_string(),
                    fqn: "Status".to_string(),
                    items: vec![],
                })],
            }],
            renames: vec![],
        };
        let task = struct_with_inline_enum("StateEnum");
        assert_eq!(
            cs_enum_type_for(&[file], &task, "", "Status"),
            Some("Status".to_string())
        );
    }
}
