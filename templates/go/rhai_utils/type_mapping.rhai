// Go type mapping utilities

/// Map primitive types to Go types
fn map_primitive(type_name) {
    switch type_name {
        "u8" => "uint8",
        "u16" => "uint16",
        "u32" => "uint32",
        "u64" => "uint64",
        "i8" => "int8",
        "i16" => "int16",
        "i32" => "int32",
        "i64" => "int64",
        "f32" => "float32",
        "f64" => "float64",
        "bool" => "bool",
        "string" => "string",
        "bytes" => "[]byte",
        _ => ()
    }
}

/// Convert namespace FQN to Go package name
/// "game.character" -> "gamecharacter"
fn ns_to_package_name(ns_fqn) {
    let result = ns_fqn;
    result.replace(".", "");
    to_lower(result)
}

/// Convert namespace FQN to Go type prefix
/// "game.character" -> "GameCharacter"
fn ns_to_type_prefix(ns_fqn) {
    let result = ns_fqn;
    result.replace(".", "_");
    to_pascal_case(result)
}

/// Extract the actual namespace from an FQN that might include struct name
fn get_actual_namespace(fqn) {
    let parts = fqn.split(".");
    if parts.len <= 1 {
        return fqn;
    }

    // Check if the last part starts with uppercase (likely a struct/table name)
    let last_part = parts[parts.len - 1];
    if last_part.len > 0 {
        let first_char = last_part.sub_string(0, 1);
        if first_char == first_char.to_upper() {
            let result = "";
            for i in 0..(parts.len - 1) {
                if i > 0 {
                    result += ".";
                }
                result += parts[i];
            }
            return result;
        }
    }
    fqn
}

/// Map a TypeRef to its Go type string
fn map_type(type_ref) {
    let base_type = "";

    // Check if it's a primitive type
    let primitive = map_primitive(type_ref.type_name);
    if primitive != () {
        base_type = primitive;
    } else {
        // Custom type - use the type name directly
        base_type = type_ref.type_name;
    }

    // Handle cardinality
    if type_ref.is_list {
        return "[]" + base_type;
    } else if type_ref.is_option {
        return "*" + base_type;
    }

    base_type
}

/// Map a TypeRef with namespace context
/// In Go, all types in a single .poly file are in the same package,
/// so we don't need namespace prefixes - just use the type name directly
fn map_type_with_ns(type_ref, current_ns) {
    let base_type = "";

    // Check if it's a primitive type
    let primitive = map_primitive(type_ref.type_name);
    if primitive != () {
        base_type = primitive;
    } else {
        // Custom type - use the type name directly (no namespace prefix needed)
        // All types in the same .poly file are in the same Go package
        base_type = type_ref.type_name;
    }

    // Handle cardinality
    if type_ref.is_list {
        return "[]" + base_type;
    } else if type_ref.is_option {
        return "*" + base_type;
    }

    base_type
}
