// Rust type mapping utilities
// Converts PolyGen types to Rust types

/// Map a primitive type name to its Rust equivalent
fn map_primitive(type_name) {
    switch type_name {
        "string" => "String",
        "bool" => "bool",
        "bytes" => "Vec<u8>",
        "u8" => "u8",
        "u16" => "u16",
        "u32" => "u32",
        "u64" => "u64",
        "i8" => "i8",
        "i16" => "i16",
        "i32" => "i32",
        "i64" => "i64",
        "f32" => "f32",
        "f64" => "f64",
        "timestamp" => "chrono::DateTime<chrono::Utc>",
        _ => type_name
    }
}

/// Check if a type name is a primitive
fn is_primitive_name(type_name) {
    switch type_name {
        "string" | "bool" | "bytes" | "timestamp" |
        "u8" | "u16" | "u32" | "u64" |
        "i8" | "i16" | "i32" | "i64" |
        "f32" | "f64" => true,
        _ => false
    }
}

/// Convert namespace FQN to Rust module name
/// "game.common" -> "game_common"
fn ns_to_module_name(ns_fqn) {
    let result = ns_fqn;
    result.replace(".", "_");
    to_snake_case(result)
}

/// Get parent namespace from FQN
/// "game.character" -> "game"
/// "game" -> ""
fn get_parent_ns(ns_fqn) {
    let last_dot = -1;
    for i in 0..ns_fqn.len() {
        if ns_fqn[i] == '.' {
            last_dot = i;
        }
    }
    if last_dot < 0 {
        return "";
    }
    ns_fqn.sub_string(0, last_dot)
}

/// Build namespace FQN from parts array up to index (exclusive)
/// ["app", "data", "enums"], index=2 -> "app.data"
fn build_ns_fqn(parts, end_index) {
    let result = "";
    for i in 0..end_index {
        if i > 0 {
            result += ".";
        }
        result += parts[i];
    }
    result
}

/// Compute relative module path from current_ns to ref_ns
/// Both are namespace FQNs like "game.character" and "game.common"
fn compute_relative_path(current_ns, ref_ns) {
    // Get parents
    let current_parent = get_parent_ns(current_ns);
    let ref_parent = get_parent_ns(ref_ns);

    // If both have the same parent, they're siblings
    // Use super::<ref_mod>::
    if current_parent == ref_parent && current_parent != "" {
        return "super::" + ns_to_module_name(ref_ns) + "::";
    }

    // If ref_ns is the parent of current_ns
    // e.g., current = "game.character.skills", ref = "game.character"
    if current_ns.starts_with(ref_ns + ".") {
        let depth = current_ns.split(".").len - ref_ns.split(".").len;
        let path = "";
        for i in 0..depth {
            path += "super::";
        }
        return path;
    }

    // If current_ns is parent of ref_ns (ref is a child namespace)
    // e.g., current = "game", ref = "game.common"
    if ref_ns.starts_with(current_ns + ".") {
        // Navigate down to the child module - build path one level at a time
        let ref_parts = ref_ns.split(".");
        let current_depth = current_ns.split(".").len;
        let path = "";
        for i in current_depth..ref_parts.len {
            let level_ns = build_ns_fqn(ref_parts, i + 1);
            path += ns_to_module_name(level_ns) + "::";
        }
        return path;
    }

    // General case: find common ancestor and compute path
    let current_parts = current_ns.split(".");
    let ref_parts = ref_ns.split(".");

    // Find common prefix length
    let common_len = 0;
    let min_len = if current_parts.len < ref_parts.len { current_parts.len } else { ref_parts.len };
    for i in 0..min_len {
        if current_parts[i] == ref_parts[i] {
            common_len = i + 1;
        } else {
            break;
        }
    }

    // Calculate number of super:: needed to reach common ancestor
    let supers_needed = current_parts.len - common_len;

    // Build the path with super::
    let path = "";
    for i in 0..supers_needed {
        path += "super::";
    }

    // Navigate down from common ancestor to target, one level at a time
    for i in common_len..ref_parts.len {
        let level_ns = build_ns_fqn(ref_parts, i + 1);
        path += ns_to_module_name(level_ns) + "::";
    }

    return path;
}

/// Map a TypeRef to its Rust type string
/// current_ns: the current namespace FQN (e.g., "game.character")
fn map_type_with_ns(type_ref, current_ns) {
    let base_type = "";

    // Check if it's a primitive by name (workaround for IR classification issues)
    if type_ref.is_primitive || is_primitive_name(type_ref.type_name) {
        base_type = map_primitive(type_ref.type_name);
    } else {
        // For non-primitives, check if it's from the same namespace
        let ref_ns = type_ref.namespace_fqn;
        if ref_ns == "" || ref_ns == current_ns {
            // Same namespace, use type name directly
            base_type = type_ref.type_name;
        } else if ref_ns.starts_with(current_ns + ".") {
            // Inline type - namespace_fqn includes parent struct name (e.g., "game.character.Player")
            // but in generated Rust, inline enums are siblings in the same module
            // So just use the type name directly
            base_type = type_ref.type_name;
        } else {
            // Different namespace - compute relative module path
            base_type = compute_relative_path(current_ns, ref_ns) + type_ref.type_name;
        }
    }

    // Handle container types
    if type_ref.is_list {
        if type_ref.is_option {
            return "Option<Vec<" + base_type + ">>";
        } else {
            return "Vec<" + base_type + ">";
        }
    } else if type_ref.is_option {
        return "Option<" + base_type + ">";
    }

    base_type
}

/// Simple map_type without namespace context (backward compatible)
fn map_type(type_ref) {
    let base_type = "";

    if type_ref.is_primitive || is_primitive_name(type_ref.type_name) {
        base_type = map_primitive(type_ref.type_name);
    } else {
        base_type = type_ref.type_name;
    }

    if type_ref.is_list {
        if type_ref.is_option {
            return "Option<Vec<" + base_type + ">>";
        } else {
            return "Vec<" + base_type + ">";
        }
    } else if type_ref.is_option {
        return "Option<" + base_type + ">";
    }

    base_type
}

// =============================================================================
// Timestamp Utilities
// =============================================================================

/// Gets the Rust expression for current time based on timezone specification.
/// Used for auto_create and auto_update timestamp fields.
fn get_datetime_expr_for_timezone(tz) {
    if tz == () {
        return "chrono::Utc::now()";
    }

    let kind = tz.kind;

    if kind == "utc" {
        return "chrono::Utc::now()";
    } else if kind == "local" {
        return "chrono::Local::now().with_timezone(&chrono::Utc)";
    } else if kind == "offset" {
        let hours = tz.offset_hours;
        let minutes = tz.offset_minutes;
        if hours == () { hours = 0; }
        if minutes == () { minutes = 0; }
        let total_secs = hours * 3600 + minutes * 60;
        return "chrono::Utc::now()";
    } else if kind == "named" {
        // Named timezones would require chrono-tz crate
        return "chrono::Utc::now()";
    }

    return "chrono::Utc::now()";
}
