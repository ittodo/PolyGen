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
        _ => type_name
    }
}

/// Check if a type name is a primitive
fn is_primitive_name(type_name) {
    switch type_name {
        "string" | "bool" | "bytes" |
        "u8" | "u16" | "u32" | "u64" |
        "i8" | "i16" | "i32" | "i64" |
        "f32" | "f64" => true,
        _ => false
    }
}

/// Map a TypeRef to its Rust type string
/// current_ns: the current namespace FQN (e.g., "game.item")
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
            // Different namespace - need to determine which file contains it
            // Convert namespace to module path
            let ns_path = ref_ns;
            ns_path.replace(".", "_");

            // Heuristic: check if it's likely from common_types file
            // Namespaces containing "common" are assumed to be in common_types.rs
            // This is a workaround - proper solution needs namespace-to-file mapping in IR
            if ref_ns.starts_with("game.common") {
                base_type = "crate::common_types::" + ns_path + "::" + type_ref.type_name;
            } else {
                // Assume it's in the same file but different namespace module
                base_type = "super::" + ns_path + "::" + type_ref.type_name;
            }
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
