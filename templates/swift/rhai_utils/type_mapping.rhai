// Swift type mapping helpers.

fn join_with(arr, sep) {
    let result = "";
    let i = 0;
    for item in arr {
        if i > 0 { result += sep; }
        result += item;
        i += 1;
    }
    result
}

fn class_name_from_fqn(fqn) {
    if fqn == () || fqn == "" {
        return "";
    }
    let name = fqn;
    name.replace(".", "_");
    to_pascal_case(name)
}

fn class_name_for_type(type_ref) {
    if (type_ref.is_struct || type_ref.is_enum) && type_ref.fqn != () && type_ref.fqn != "" {
        return class_name_from_fqn(type_ref.fqn);
    }
    type_ref.type_name
}

fn string_literal(value) {
    let escaped = value;
    escaped.replace("\\", "\\\\");
    escaped.replace("\"", "\\\"");
    "\"" + escaped + "\""
}

fn scalar_type(type_ref) {
    switch type_ref.type_name {
        "u8" => "UInt8",
        "u16" => "UInt16",
        "u32" => "UInt32",
        "u64" => "UInt64",
        "i8" => "Int8",
        "i16" => "Int16",
        "i32" => "Int32",
        "i64" => "Int64",
        "f32" => "Float",
        "f64" => "Double",
        "bool" => "Bool",
        "string" => "String",
        "bytes" => "Data",
        "timestamp" => "Date",
        _ => class_name_for_type(type_ref)
    }
}

fn swift_type(type_ref) {
    let result = scalar_type(type_ref);
    if type_ref.is_list {
        result = "[" + result + "]";
    }
    if type_ref.is_option {
        result += "?";
    }
    result
}

fn default_literal(type_ref, value) {
    if value == () {
        return ();
    }

    switch type_ref.type_name {
        "string" | "timestamp" => string_literal(value),
        "bool" => {
            if value == "true" || value == "True" {
                "true"
            } else {
                "false"
            }
        },
        _ => value
    }
}

fn property_default(field) {
    if field.has_default_value {
        return default_literal(field.field_type, field.default_value);
    }
    if field.field_type.is_list {
        return "[]";
    }
    if field.field_type.is_option {
        return "nil";
    }
    ()
}

fn is_transformable(type_ref) {
    type_ref.is_list || type_ref.is_struct || type_ref.type_name == "bytes"
}
