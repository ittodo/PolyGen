// Unreal Engine Type Mapping Utilities for PolyGen
// Maps IR types to Unreal Engine types

// Map primitive types to Unreal types
fn map_primitive_type(type_name) {
    switch type_name {
        "string" => "FString",
        "bool" => "bool",
        "u8" => "uint8",
        "u16" => "int32",
        "u32" => "int32",
        "u64" => "int64",
        "i8" => "int32",
        "i16" => "int32",
        "i32" => "int32",
        "i64" => "int64",
        "f32" => "float",
        "f64" => "double",
        "bytes" => "TArray<uint8>",
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

// Check if a type name looks like an enum based on common naming patterns
fn is_likely_enum_name(type_name) {
    // Common enum naming patterns
    if type_name == "Element" { return true; }
    if type_name == "Status" { return true; }
    if type_name == "ItemType" { return true; }

    // Common suffixes for enums
    let name_len = type_name.len();
    if name_len > 4 {
        let suffix4 = type_name.sub_string(name_len - 4, 4);
        if suffix4 == "Type" { return true; }
        if suffix4 == "Kind" { return true; }
        if suffix4 == "Mode" { return true; }
    }
    if name_len > 5 {
        let suffix5 = type_name.sub_string(name_len - 5, 5);
        if suffix5 == "State" { return true; }
    }
    if name_len > 6 {
        let suffix6 = type_name.sub_string(name_len - 6, 6);
        if suffix6 == "Status" { return true; }
    }

    return false;
}

// Map a TypeRef to its Unreal type string with namespace context
fn map_type_with_ns(type_ref, current_ns) {
    let base_type = "";

    if type_ref.is_primitive || is_primitive_name(type_ref.type_name) {
        base_type = map_primitive_type(type_ref.type_name);
    } else {
        // Custom type (struct or enum)
        let type_name = type_ref.type_name;

        // Determine if this is an enum type
        // Use is_enum flag if available, otherwise use heuristics based on name
        let is_enum_type = type_ref.is_enum;

        // Check common enum names if not explicitly marked as enum
        if !is_enum_type {
            if type_name == "Element" { is_enum_type = true; }
            if type_name == "Status" { is_enum_type = true; }
            if type_name == "ItemType" { is_enum_type = true; }

            // Check common suffixes
            let name_len = type_name.len();
            if name_len > 4 {
                let suffix = type_name.sub_string(name_len - 4, 4);
                if suffix == "Type" || suffix == "Kind" || suffix == "Mode" {
                    is_enum_type = true;
                }
            }
            if name_len > 6 {
                let suffix = type_name.sub_string(name_len - 6, 6);
                if suffix == "Status" {
                    is_enum_type = true;
                }
            }
        }

        if is_enum_type {
            base_type = get_enum_name(type_name);
        } else {
            base_type = get_struct_name(type_name);
        }
    }

    // Apply cardinality
    if type_ref.is_list {
        return "TArray<" + base_type + ">";
    } else if type_ref.is_option {
        // For Blueprint compatibility, use pointer or wrapper
        // TOptional is not Blueprint-exposed, so we use regular type with meta
        return base_type;
    }

    base_type
}

// Get default value for a type
fn get_default_value(type_ref) {
    if type_ref.is_list {
        return "";  // TArray default initializes to empty
    } else if type_ref.is_option {
        return "";  // Will be handled separately
    }

    if type_ref.is_primitive || is_primitive_name(type_ref.type_name) {
        switch type_ref.type_name {
            "string" => "TEXT(\"\")",
            "bool" => "false",
            "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" => "0",
            "f32" | "f64" => "0.0f",
            "bytes" => "",
            _ => ""
        }
    } else if type_ref.is_enum || is_likely_enum_name(type_ref.type_name) {
        "static_cast<" + get_enum_name(type_ref.type_name) + ">(0)"
    } else {
        ""
    }
}

// Convert PascalCase to snake_case (for CSV column names)
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

// Get Unreal struct name with a PolyGen-specific reflected name.
// UHT strips the leading F/U/A prefix when checking engine names, so plain
// FPlayer or FColor can collide with engine UPlayer/FColor.
fn get_struct_name(name) {
    if name.starts_with("FPolygen") {
        return name;
    }
    if name.starts_with("Polygen") {
        return "F" + name;
    }
    if name.starts_with("F") {
        return "FPolygen" + name.sub_string(1, name.len() - 1);
    }
    return "FPolygen" + name;
}

// Get Unreal enum name with a PolyGen-specific reflected name.
fn get_enum_name(name) {
    if name.starts_with("EPolygen") {
        return name;
    }
    if name.starts_with("Polygen") {
        return "E" + name;
    }
    if name.starts_with("E") {
        return "EPolygen" + name.sub_string(1, name.len() - 1);
    }
    return "EPolygen" + name;
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

// Check if type needs UPROPERTY with specific specifiers
fn get_uproperty_specifiers(type_ref) {
    let specifiers = "EditAnywhere, BlueprintReadWrite";

    // Add Category for organization
    specifiers += ", Category = \"Data\"";

    return specifiers;
}
