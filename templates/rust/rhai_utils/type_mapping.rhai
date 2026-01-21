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
fn map_type(type_ref) {
    let base_type = "";

    // Check if it's a primitive by name (workaround for IR classification issues)
    if type_ref.is_primitive || is_primitive_name(type_ref.type_name) {
        base_type = map_primitive(type_ref.type_name);
    } else {
        // For non-primitives, use the type name directly
        base_type = type_ref.type_name;
    }

    // Handle container types
    if type_ref.is_list {
        if type_ref.is_option {
            // Option<Vec<T>>
            return "Option<Vec<" + base_type + ">>";
        } else {
            // Vec<T>
            return "Vec<" + base_type + ">";
        }
    } else if type_ref.is_option {
        // Option<T>
        return "Option<" + base_type + ">";
    }

    base_type
}
