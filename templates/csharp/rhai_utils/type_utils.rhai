// Unified type utilities for C# code generation
// Requires global `schema` (SchemaContext) and a provided `current_ns_name` string in scope

// =============================================================================
// String Manipulation Utilities
// =============================================================================

/// Check if a type is an Option wrapper
fn is_option(t) {
    t.starts_with("Option<")
}

/// Check if a type is a List wrapper
fn is_list(t) {
    t.starts_with("List<")
}

/// Unwrap an Option<T> type to get T
fn unwrap_option(t) {
    let s = "Option<".len();
    let l = t.len() - s - 1;
    t.sub_string(s, l)
}

/// Unwrap a List<T> type to get T
fn unwrap_list(t) {
    let s = "List<".len();
    let l = t.len() - s - 1;
    t.sub_string(s, l)
}

/// Strip both Option<> and List<> wrappers from a type string
fn strip_wrappers(type_string) {
    let t = if type_of(type_string) == "TypeRef" { type_string.original } else { type_string };
    if t.starts_with("Option<") {
        let start = "Option<".len();
        let len = t.len() - start - 1;
        return t.sub_string(start, len);
    }
    if t.starts_with("List<") {
        let start = "List<".len();
        let len = t.len() - start - 1;
        return t.sub_string(start, len);
    }
    t
}

/// Get the short name (last component) of a fully qualified type name
fn short_name(type_string) {
    let type_string = if type_of(type_string) == "TypeRef" { type_string.original } else { type_string };
    let parts = type_string.split(".");
    parts[parts.len - 1]
}

/// Get the namespace prefix of a fully qualified type name
fn ns_prefix(type_string) {
    let type_string = if type_of(type_string) == "TypeRef" { type_string.original } else { type_string };
    if !type_string.contains(".") { return ""; }
    let idx = type_string.last_index_of(".");
    type_string.sub_string(0, idx)
}

// =============================================================================
// Enum Detection Utilities
// =============================================================================

/// Check if a name follows the inline enum naming convention
fn is_inline_enum_name(name) {
    name.ends_with("__Enum")
}

/// Find an enum by name in a specific namespace
fn find_enum_in_ns(ns, target_name) {
    for item in ns.items {
        if item.is_enum() {
            let e = item.as_enum();
            if e.name == target_name { return true; }
        }
    }
    false
}

/// Recursively find an enum in a namespace tree
fn find_enum_in_tree(ns, prefix, target_ns, target_name) {
    // compute full namespace
    let fqn = if prefix == "" { ns.name } else { prefix + "." + ns.name };
    if fqn == target_ns {
        if find_enum_in_ns(ns, target_name) { return true; }
    }
    // recurse into nested namespaces
    for item in ns.items {
        if item.is_namespace() {
            let child = item.as_namespace();
            if find_enum_in_tree(child, fqn, target_ns, target_name) { return true; }
        }
    }
    false
}

/// Check if an enum exists at a specific namespace path
fn enum_exists_at(target_ns, target_name) {
    for file in schema.files {
        for root_ns in file.namespaces {
            if find_enum_in_tree(root_ns, "", target_ns, target_name) { return true; }
        }
    }
    false
}

/// Check if an enum with the given name exists anywhere in the schema
fn any_enum_named(target_name) {
    for file in schema.files {
        for root_ns in file.namespaces {
            if find_enum_in_ns(root_ns, target_name) { return true; }
            // traverse children
            for item in root_ns.items {
                if item.is_namespace() {
                    let child = item.as_namespace();
                    if any_enum_named_in(child, target_name, root_ns.name) { return true; }
                }
            }
        }
    }
    false
}

/// Helper for any_enum_named - recursive search
fn any_enum_named_in(ns, target_name, prefix) {
    if find_enum_in_ns(ns, target_name) { return true; }
    for item in ns.items {
        if item.is_namespace() {
            let child = item.as_namespace();
            if any_enum_named_in(child, target_name, if prefix == "" { ns.name } else { prefix + "." + ns.name }) { return true; }
        }
    }
    false
}

/// Public API: detect if a type_string represents an enum
fn is_enum_type(type_string, current_ns_name) {
    let type_string = if type_of(type_string) == "TypeRef" { type_string.original } else { type_string };
    if is_inline_enum_name(type_string) { return true; }
    let core = strip_wrappers(type_string);
    if core.contains(".") {
        let ns = ns_prefix(core);
        let name = short_name(core);
        return enum_exists_at(ns, name);
    } else {
        if current_ns_name != () && current_ns_name != "" {
            if enum_exists_at(current_ns_name, core) { return true; }
        }
        return any_enum_named(core);
    }
}

// =============================================================================
// Struct Utilities
// =============================================================================

/// Find an embedded struct by name in the current struct
fn find_embedded_struct(cur_s, name) {
    for it in cur_s.items {
        if it.is_embedded_struct() {
            let es = it.as_embedded_struct();
            if es.name == name { return es; }
        }
    }
    ()
}

// =============================================================================
// Namespace Utilities
// =============================================================================

/// Collect all enum names from a namespace
fn collect_enum_names(ns) {
    let names = [];
    for item in ns.items {
        if item.is_enum() {
            let e = item.as_enum();
            names.push(e.name);
        }
    }
    names
}

/// Collect all struct items from a namespace
fn collect_structs(ns) {
    let structs = [];
    for item in ns.items {
        if item.is_struct() {
            structs.push(item.as_struct());
        }
    }
    structs
}

/// Collect all nested namespace items from a namespace
fn collect_namespaces(ns) {
    let namespaces = [];
    for item in ns.items {
        if item.is_namespace() {
            namespaces.push(item.as_namespace());
        }
    }
    namespaces
}

/// Collect all comment items from a namespace
fn collect_comments(ns) {
    let comments = [];
    for item in ns.items {
        if item.is_comment() {
            comments.push(item.as_comment());
        }
    }
    comments
}

// =============================================================================
// Index Utilities
// =============================================================================

/// Convert snake_case to PascalCase for C# property names
fn to_pascal_case_cs(name) {
    let parts = name.split("_");
    let result = "";
    for part in parts {
        if part.len > 0 {
            let first = part.sub_string(0, 1);
            first = first.to_upper();
            let rest = part.sub_string(1);
            result += first + rest;
        }
    }
    result
}

/// Get key type for index (supports composite indexes)
/// Returns C# tuple syntax for composite: (Type1, Type2, ...)
fn get_index_key_type_cs(idx) {
    if idx.is_composite {
        // C# tuple syntax: (Type1, Type2, ...)
        let result = "(";
        for i in 0..idx.fields.len() {
            if i > 0 { result += ", "; }
            result += cs_map_type(idx.fields[i].field_type);
        }
        result += ")";
        return result;
    } else {
        return cs_map_type(idx.field_type);
    }
}

/// Get key expression for index Add call (supports composite indexes)
/// Returns C# tuple syntax: (row.field1, row.field2, ...)
fn get_index_key_expr_cs(idx, row_var) {
    if idx.is_composite {
        let result = "(";
        for i in 0..idx.fields.len() {
            if i > 0 { result += ", "; }
            result += row_var + "." + idx.fields[i].name;
        }
        result += ")";
        return result;
    } else {
        return row_var + "." + idx.field_name;
    }
}

// =============================================================================
// Timestamp Utilities
// =============================================================================

/// Gets the DateTime expression for a given timezone specification.
/// Used for auto_create and auto_update timestamp fields.
fn get_datetime_expr_for_timezone(tz) {
    if tz == () {
        return "DateTime.UtcNow";
    }

    let kind = tz.kind;

    if kind == "utc" {
        return "DateTime.UtcNow";
    } else if kind == "local" {
        return "DateTime.Now";
    } else if kind == "offset" {
        let hours = tz.offset_hours;
        let minutes = tz.offset_minutes;
        if hours == () { hours = 0; }
        if minutes == () { minutes = 0; }
        // Use DateTimeOffset to handle timezone offset
        return "DateTimeOffset.UtcNow.ToOffset(new TimeSpan(" + hours + ", " + minutes + ", 0)).DateTime";
    } else if kind == "named" {
        let tz_name = tz.name;
        if tz_name == () { tz_name = "UTC"; }
        return "TimeZoneInfo.ConvertTimeFromUtc(DateTime.UtcNow, TimeZoneInfo.FindSystemTimeZoneById(\"" + tz_name + "\"))";
    }

    return "DateTime.UtcNow";
}
