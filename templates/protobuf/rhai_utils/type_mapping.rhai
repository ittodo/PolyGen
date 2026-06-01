// Protocol Buffers type mapping helpers.

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

fn proto_package(file) {
    for ns in file.namespaces {
        if ns.name != () && ns.name != "" {
            return ns.name;
        }
    }
    "polygen"
}

fn type_name_from_fqn(fqn) {
    if fqn == () || fqn == "" {
        return "";
    }
    to_pascal_case(join_with(fqn.split("."), "_"))
}

fn type_name_for_ref(type_ref) {
    if (type_ref.is_struct || type_ref.is_enum) && type_ref.fqn != () && type_ref.fqn != "" {
        return type_name_from_fqn(type_ref.fqn);
    }
    type_ref.type_name
}

fn scalar_type(type_ref) {
    switch type_ref.type_name {
        "u8" | "u16" | "u32" => "uint32",
        "u64" => "uint64",
        "i8" | "i16" | "i32" => "int32",
        "i64" => "int64",
        "f32" => "float",
        "f64" => "double",
        "bool" => "bool",
        "string" => "string",
        "bytes" => "bytes",
        "timestamp" => "google.protobuf.Timestamp",
        _ => type_name_for_ref(type_ref)
    }
}

fn field_type(type_ref) {
    scalar_type(type_ref)
}

fn field_label(type_ref) {
    if type_ref.is_list {
        return "repeated ";
    }
    if type_ref.is_option {
        return "optional ";
    }
    ""
}

fn enum_member_name(enum_name, member_name) {
    let result = to_snake_case(enum_name + "_" + member_name);
    result.to_upper()
}
