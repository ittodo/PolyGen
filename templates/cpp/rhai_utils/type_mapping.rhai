// C++ Type Mapping Utilities for PolyGen
// Maps IR types to C++ types

// Map primitive types to C++ types
fn map_primitive_type(type_name) {
    switch type_name {
        "string" => "std::string",
        "bool" => "bool",
        "u8" => "uint8_t",
        "u16" => "uint16_t",
        "u32" => "uint32_t",
        "u64" => "uint64_t",
        "i8" => "int8_t",
        "i16" => "int16_t",
        "i32" => "int32_t",
        "i64" => "int64_t",
        "f32" => "float",
        "f64" => "double",
        "bytes" => "std::vector<uint8_t>",
        _ => type_name
    }
}

// Check if a type name is a primitive
fn is_primitive_name(type_name) {
    switch type_name {
        "string" | "bool" | "bytes" |
        "u8" | "u16" | "u32" | "u64" |
        "i8" | "i16" | "i32" | "i64" |
        "f32" | "f64" => true,
        _ => false
    }
}

// Map a TypeRef to its C++ type string with namespace context
fn map_type_with_ns(type_ref, current_ns) {
    let base_type = "";

    if type_ref.is_primitive || is_primitive_name(type_ref.type_name) {
        base_type = map_primitive_type(type_ref.type_name);
    } else {
        // Custom type (struct or enum)
        let type_namespace = type_ref.namespace_fqn;
        let type_name = type_ref.type_name;

        if type_namespace == "" || type_namespace == current_ns {
            // Same namespace or no namespace - use type name directly
            base_type = type_name;
        } else if current_ns != "" && type_namespace.starts_with(current_ns + ".") {
            // Nested or inline types - use type name directly
            base_type = type_name;
        } else {
            // Different namespace - use fully qualified with global scope prefix
            // Note: Rhai's replace() mutates in place, so we need to copy first
            let ns_copy = type_namespace + "";
            ns_copy.replace(".", "::");
            // Always use fully qualified name for cross-namespace references
            base_type = "::" + ns_copy + "::" + type_name;
        }
    }

    // Apply cardinality
    if type_ref.is_list {
        if type_ref.is_option {
            return "std::optional<std::vector<" + base_type + ">>";
        } else {
            return "std::vector<" + base_type + ">";
        }
    } else if type_ref.is_option {
        return "std::optional<" + base_type + ">";
    }

    base_type
}

// Get default value for a type
fn get_default_value(type_ref) {
    if type_ref.is_list {
        return "{}";
    } else if type_ref.is_option {
        return "std::nullopt";
    }

    if type_ref.is_primitive || is_primitive_name(type_ref.type_name) {
        switch type_ref.type_name {
            "string" => "\"\"",
            "bool" => "false",
            "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" => "0",
            "f32" | "f64" => "0.0",
            "bytes" => "{}",
            _ => "{}"
        }
    } else {
        // struct or enum - use default constructor
        return "{}";
    }
}

// Convert PascalCase to snake_case
fn to_snake_case(name) {
    let result = "";
    let prev_lower = false;

    for i in 0..name.len() {
        let c = name.sub_string(i, 1);
        let is_upper = c >= "A" && c <= "Z";

        if is_upper && prev_lower && result != "" {
            result += "_";
        }

        if is_upper {
            result += c.to_lower();
        } else {
            result += c;
        }

        prev_lower = !is_upper;
    }

    return result;
}

// Get first enum member name for default value
fn get_first_member(e) {
    for item in e.items {
        if item.is_member() {
            return item.as_member().name;
        }
    }
    return "Unknown";
}
