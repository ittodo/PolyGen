// MessagePack schema descriptor helpers.

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

fn json_string(value) {
    let escaped = value;
    escaped.replace("\\", "\\\\");
    escaped.replace("\"", "\\\"");
    escaped.replace("\r", "\\r");
    escaped.replace("\n", "\\n");
    "\"" + escaped + "\""
}

fn json_bool(value) {
    if value { "true" } else { "false" }
}

fn type_name_from_fqn(fqn) {
    if fqn == () || fqn == "" {
        return "";
    }
    to_pascal_case(join_with(fqn.split("."), "_"))
}

fn msgpack_wire_type(type_ref) {
    if type_ref.is_list {
        return "array";
    }
    if type_ref.is_enum {
        return "int";
    }
    if type_ref.is_struct {
        return "object";
    }

    switch type_ref.type_name {
        "u8" | "u16" | "u32" | "u64" => "uint",
        "i8" | "i16" | "i32" | "i64" => "int",
        "f32" => "float32",
        "f64" => "float64",
        "bool" => "bool",
        "string" => "string",
        "bytes" => "binary",
        "timestamp" => "timestamp",
        _ => "object"
    }
}

fn msgpack_element_wire_type(type_ref) {
    if !type_ref.is_list {
        return ();
    }
    if type_ref.inner != () {
        return msgpack_wire_type(type_ref.inner);
    }
    msgpack_wire_type(type_ref)
}

fn type_target(type_ref) {
    if (type_ref.is_struct || type_ref.is_enum) && type_ref.fqn != () && type_ref.fqn != "" {
        return type_ref.fqn;
    }
    ()
}
