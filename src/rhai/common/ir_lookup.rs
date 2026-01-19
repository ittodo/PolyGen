//! IR Lookup Utilities
//!
//! This module provides functions for searching and resolving types within
//! the IR (Intermediate Representation) structure. These functions are
//! used by various code generators to find structs and enums across
//! namespaces and files.
//!
//! ## Function Categories
//!
//! ### Type String Utilities
//! - [`unwrap_option`]: Extracts inner type from `Option<T>`
//!
//! ### Single Namespace Search (non-recursive)
//! - [`find_struct_in_ns`]: Find struct in one namespace
//! - [`find_enum_in_ns`]: Find enum in one namespace
//!
//! ### Tree Search (recursive within namespace hierarchy)
//! - [`find_struct_in_tree`]: Find struct by FQN in namespace tree
//! - [`find_enum_in_tree`]: Find enum by FQN in namespace tree
//!
//! ### Cross-File Search
//! - [`get_struct_at`]: Find struct by namespace and name across files
//! - [`get_enum_at`]: Find enum by namespace and name across files
//! - [`any_struct_named`]: Find struct by name only (any namespace)
//! - [`any_enum_named`]: Find enum by name only (any namespace)
//!
//! ### Type Resolution (handles Option<>, List<>, FQN)
//! - [`resolve_struct`]: Resolve struct from type string
//! - [`resolve_enum`]: Resolve enum from type string
//! - [`resolve_struct_with_ns`]: Resolve struct with namespace info
//!
//! ## Resolution Strategy
//!
//! When resolving a type string like `"List<Option<MyType>>"`:
//! 1. Unwrap `Option<>` wrapper if present
//! 2. Unwrap `List<>` wrapper if present
//! 3. If FQN (contains `.`), search in specific namespace
//! 4. Otherwise, search current namespace first, then global
//!
//! ---
//!
//! 이 모듈은 IR(중간 표현) 구조 내에서 타입을 검색하고 해석하는 함수들을 제공합니다.
//! 이 함수들은 여러 코드 생성기에서 네임스페이스와 파일 전체에 걸쳐
//! struct와 enum을 찾는 데 사용됩니다.

use crate::ir_model::{EnumDef, FileDef, NamespaceDef, NamespaceItem, StructDef};

// =============================================================================
// Type String Utilities
// =============================================================================

/// Unwraps an `Option<T>` type string to get the inner type.
///
/// Returns the inner type if wrapped in `Option<>`, otherwise returns as-is.
///
/// `Option<>`으로 감싸진 경우 내부 타입을 반환하고, 그렇지 않으면 그대로 반환합니다.
pub fn unwrap_option(t: &str) -> &str {
    const P: &str = "Option<";
    if t.starts_with(P) && t.ends_with('>') {
        &t[P.len()..t.len() - 1]
    } else {
        t
    }
}

/// Finds a struct by name within a single namespace (non-recursive).
///
/// English: Searches only the direct items of the namespace.
///
/// 한국어: 네임스페이스의 직접 항목만 검색합니다 (재귀 아님).
pub fn find_struct_in_ns<'a>(ns: &'a NamespaceDef, target_name: &str) -> Option<&'a StructDef> {
    ns.items.iter().find_map(|item| match item {
        NamespaceItem::Struct(s) if s.name == target_name => Some(s),
        _ => None,
    })
}

/// Finds an enum by name within a single namespace (non-recursive).
///
/// English: Searches only the direct items of the namespace.
///
/// 한국어: 네임스페이스의 직접 항목만 검색합니다 (재귀 아님).
pub fn find_enum_in_ns<'a>(ns: &'a NamespaceDef, target_name: &str) -> Option<&'a EnumDef> {
    ns.items.iter().find_map(|item| match item {
        NamespaceItem::Enum(e) if e.name == target_name => Some(e),
        _ => None,
    })
}

/// Recursively finds a struct in a namespace tree by fully-qualified namespace and name.
///
/// English: Traverses the namespace hierarchy to find a struct at a specific namespace path.
///
/// 한국어: 네임스페이스 계층을 순회하여 특정 네임스페이스 경로의 struct를 찾습니다.
pub fn find_struct_in_tree<'a>(
    ns: &'a NamespaceDef,
    prefix: &str,
    target_ns: &str,
    target_name: &str,
) -> Option<&'a StructDef> {
    let fqn_string = if prefix.is_empty() {
        ns.name.clone()
    } else {
        format!("{}.{}", prefix, ns.name)
    };
    let fqn = fqn_string.as_str();
    if fqn == target_ns {
        if let Some(s) = find_struct_in_ns(ns, target_name) {
            return Some(s);
        }
    }
    for item in &ns.items {
        if let NamespaceItem::Namespace(child) = item {
            if let Some(s) = find_struct_in_tree(child, fqn, target_ns, target_name) {
                return Some(s);
            }
        }
    }
    None
}

/// Recursively finds an enum in a namespace tree by fully-qualified namespace and name.
///
/// English: Traverses the namespace hierarchy to find an enum at a specific namespace path.
///
/// 한국어: 네임스페이스 계층을 순회하여 특정 네임스페이스 경로의 enum을 찾습니다.
pub fn find_enum_in_tree<'a>(
    ns: &'a NamespaceDef,
    prefix: &str,
    target_ns: &str,
    target_name: &str,
) -> Option<&'a EnumDef> {
    let fqn_string = if prefix.is_empty() {
        ns.name.clone()
    } else {
        format!("{}.{}", prefix, ns.name)
    };
    let fqn = fqn_string.as_str();
    if fqn == target_ns {
        if let Some(e) = find_enum_in_ns(ns, target_name) {
            return Some(e);
        }
    }
    for item in &ns.items {
        if let NamespaceItem::Namespace(child) = item {
            if let Some(e) = find_enum_in_tree(child, fqn, target_ns, target_name) {
                return Some(e);
            }
        }
    }
    None
}

/// Finds a struct by namespace and name across all files.
///
/// English: Searches all files and their namespace hierarchies.
///
/// 한국어: 모든 파일과 네임스페이스 계층을 검색합니다.
pub fn get_struct_at<'a>(
    files: &'a [FileDef],
    target_ns: &str,
    target_name: &str,
) -> Option<&'a StructDef> {
    for file in files {
        for root_ns in &file.namespaces {
            if let Some(s) = find_struct_in_tree(root_ns, "", target_ns, target_name) {
                return Some(s);
            }
        }
    }
    None
}

/// Finds an enum by namespace and name across all files.
///
/// English: Searches all files and their namespace hierarchies.
///
/// 한국어: 모든 파일과 네임스페이스 계층을 검색합니다.
pub fn get_enum_at<'a>(
    files: &'a [FileDef],
    target_ns: &str,
    target_name: &str,
) -> Option<&'a EnumDef> {
    for file in files {
        for root_ns in &file.namespaces {
            if let Some(e) = find_enum_in_tree(root_ns, "", target_ns, target_name) {
                return Some(e);
            }
        }
    }
    None
}

/// Recursively searches for a struct by name only (ignoring namespace).
fn any_struct_named_in<'a>(ns: &'a NamespaceDef, target_name: &str) -> Option<&'a StructDef> {
    if let Some(s) = find_struct_in_ns(ns, target_name) {
        return Some(s);
    }
    for item in &ns.items {
        if let NamespaceItem::Namespace(child) = item {
            if let Some(s) = any_struct_named_in(child, target_name) {
                return Some(s);
            }
        }
    }
    None
}

/// Recursively searches for an enum by name only (ignoring namespace).
fn any_enum_named_in<'a>(ns: &'a NamespaceDef, target_name: &str) -> Option<&'a EnumDef> {
    if let Some(e) = find_enum_in_ns(ns, target_name) {
        return Some(e);
    }
    for item in &ns.items {
        if let NamespaceItem::Namespace(child) = item {
            if let Some(e) = any_enum_named_in(child, target_name) {
                return Some(e);
            }
        }
    }
    None
}

/// Finds any struct with the given name across all files.
///
/// English: Searches by name only, without requiring a specific namespace.
///
/// 한국어: 특정 네임스페이스 없이 이름만으로 검색합니다.
pub fn any_struct_named<'a>(files: &'a [FileDef], target_name: &str) -> Option<&'a StructDef> {
    for file in files {
        for root_ns in &file.namespaces {
            if let Some(s) = find_struct_in_ns(root_ns, target_name) {
                return Some(s);
            }
            if let Some(s) = any_struct_named_in(root_ns, target_name) {
                return Some(s);
            }
        }
    }
    None
}

/// Finds any enum with the given name across all files.
///
/// English: Searches by name only, without requiring a specific namespace.
///
/// 한국어: 특정 네임스페이스 없이 이름만으로 검색합니다.
pub fn any_enum_named<'a>(files: &'a [FileDef], target_name: &str) -> Option<&'a EnumDef> {
    for file in files {
        for root_ns in &file.namespaces {
            if let Some(e) = find_enum_in_ns(root_ns, target_name) {
                return Some(e);
            }
            if let Some(e) = any_enum_named_in(root_ns, target_name) {
                return Some(e);
            }
        }
    }
    None
}

/// Resolves a struct from a type string (handles Option<>, List<>, and FQN).
///
/// English: Parses the type string and finds the corresponding struct definition.
/// Handles optional types, list types, and both simple and fully-qualified names.
///
/// 한국어: 타입 문자열을 파싱하여 해당 struct 정의를 찾습니다.
/// Option 타입, List 타입, 단순 이름 및 정규화된 이름을 처리합니다.
pub fn resolve_struct<'a>(
    files: &'a [FileDef],
    type_string: &str,
    current_ns_name: &str,
) -> Option<&'a StructDef> {
    let mut core = unwrap_option(type_string);
    if let Some(inner) = core.strip_prefix("List<").and_then(|s| s.strip_suffix('>')) {
        core = inner;
    }
    if core.contains('.') {
        let mut parts = core.split('.').collect::<Vec<_>>();
        let name = parts.pop().unwrap();
        let ns = parts.join(".");
        get_struct_at(files, &ns, name)
    } else {
        if !current_ns_name.is_empty() {
            if let Some(s) = get_struct_at(files, current_ns_name, core) {
                return Some(s);
            }
        }
        any_struct_named(files, core)
    }
}

/// Resolves an enum from a type string (handles Option<>, List<>, and FQN).
///
/// English: Parses the type string and finds the corresponding enum definition.
///
/// 한국어: 타입 문자열을 파싱하여 해당 enum 정의를 찾습니다.
pub fn resolve_enum<'a>(
    files: &'a [FileDef],
    type_string: &str,
    current_ns_name: &str,
) -> Option<&'a EnumDef> {
    let mut core = unwrap_option(type_string);
    if let Some(inner) = core.strip_prefix("List<").and_then(|s| s.strip_suffix('>')) {
        core = inner;
    }
    if core.contains('.') {
        let mut parts = core.split('.').collect::<Vec<_>>();
        let name = parts.pop().unwrap();
        let ns = parts.join(".");
        get_enum_at(files, &ns, name)
    } else {
        if !current_ns_name.is_empty() {
            if let Some(e) = get_enum_at(files, current_ns_name, core) {
                return Some(e);
            }
        }
        any_enum_named(files, core)
    }
}

// --- Functions that return struct with namespace info ---

/// Recursively finds a struct with its namespace path.
fn find_struct_with_ns_in_tree<'a>(
    ns: &'a NamespaceDef,
    prefix: &str,
    target_ns: &str,
    target_name: &str,
) -> Option<(&'a StructDef, String)> {
    let fqn_string = if prefix.is_empty() {
        ns.name.clone()
    } else {
        format!("{}.{}", prefix, ns.name)
    };
    let fqn = fqn_string.as_str();
    if fqn == target_ns {
        if let Some(s) = find_struct_in_ns(ns, target_name) {
            return Some((s, fqn_string));
        }
    }
    for item in &ns.items {
        if let NamespaceItem::Namespace(child) = item {
            if let Some(res) = find_struct_with_ns_in_tree(child, fqn, target_ns, target_name) {
                return Some(res);
            }
        }
    }
    None
}

/// Finds a struct with its namespace path across all files.
fn get_struct_with_ns_at<'a>(
    files: &'a [FileDef],
    target_ns: &str,
    target_name: &str,
) -> Option<(&'a StructDef, String)> {
    for file in files {
        for root_ns in &file.namespaces {
            if let Some(res) = find_struct_with_ns_in_tree(root_ns, "", target_ns, target_name) {
                return Some(res);
            }
        }
    }
    None
}

/// Recursively searches for a struct by name and returns it with its namespace.
fn any_struct_named_with_ns_in<'a>(
    ns: &'a NamespaceDef,
    target_name: &str,
    prefix: &str,
) -> Option<(&'a StructDef, String)> {
    if let Some(s) = find_struct_in_ns(ns, target_name) {
        let fqn = if prefix.is_empty() {
            ns.name.clone()
        } else {
            format!("{}.{}", prefix, ns.name)
        };
        return Some((s, fqn));
    }
    for item in &ns.items {
        if let NamespaceItem::Namespace(child) = item {
            let next_prefix = if prefix.is_empty() {
                ns.name.clone()
            } else {
                format!("{}.{}", prefix, ns.name)
            };
            if let Some(res) = any_struct_named_with_ns_in(child, target_name, &next_prefix) {
                return Some(res);
            }
        }
    }
    None
}

/// Finds any struct by name and returns it with its namespace.
fn any_struct_named_with_ns<'a>(
    files: &'a [FileDef],
    target_name: &str,
) -> Option<(&'a StructDef, String)> {
    for file in files {
        for root_ns in &file.namespaces {
            if let Some(s) = find_struct_in_ns(root_ns, target_name) {
                return Some((s, root_ns.name.clone()));
            }
            if let Some(res) = any_struct_named_with_ns_in(root_ns, target_name, &root_ns.name) {
                return Some(res);
            }
        }
    }
    None
}

/// Resolves a struct from a type string and returns it with its namespace.
///
/// English: Like resolve_struct, but also returns the fully-qualified namespace path.
/// This is useful when generating code that needs to reference types by their full path.
///
/// 한국어: resolve_struct와 같지만 정규화된 네임스페이스 경로도 함께 반환합니다.
/// 타입을 전체 경로로 참조해야 하는 코드를 생성할 때 유용합니다.
pub fn resolve_struct_with_ns<'a>(
    files: &'a [FileDef],
    type_string: &str,
    current_ns_name: &str,
) -> Option<(&'a StructDef, String)> {
    let mut core = unwrap_option(type_string);
    if let Some(inner) = core.strip_prefix("List<").and_then(|s| s.strip_suffix('>')) {
        core = inner;
    }
    if core.contains('.') {
        let mut parts = core.split('.').collect::<Vec<_>>();
        let name = parts.pop().unwrap();
        let ns = parts.join(".");
        get_struct_with_ns_at(files, &ns, name)
    } else {
        if !current_ns_name.is_empty() {
            if let Some(res) = get_struct_with_ns_at(files, current_ns_name, core) {
                return Some(res);
            }
        }
        any_struct_named_with_ns(files, core)
    }
}
