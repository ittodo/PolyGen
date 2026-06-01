// Python type mapping and rendering helpers.

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

fn table_name_from_fqn(fqn) {
    let name = fqn;
    name.replace(".", "_");
    name
}

fn quote_py_string(value) {
    let escaped = value;
    escaped.replace("\\", "\\\\");
    escaped.replace("'", "\\'");
    "'" + escaped + "'"
}

fn py_default_literal(type_ref, value) {
    if value == () {
        return ();
    }

    switch type_ref.type_name {
        "string" | "timestamp" => quote_py_string(value),
        "bool" => {
            if value == "true" || value == "True" {
                "True"
            } else {
                "False"
            }
        },
        _ => value
    }
}

fn py_type_base(type_ref) {
    switch type_ref.type_name {
        "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" => "int",
        "f32" | "f64" => "float",
        "bool" => "bool",
        "string" => "str",
        "bytes" => "bytes",
        "timestamp" => "datetime",
        _ => class_name_for_type(type_ref)
    }
}

fn py_type(type_ref) {
    let result = py_type_base(type_ref);
    if type_ref.is_list {
        result = "list[" + result + "]";
    }
    if type_ref.is_option {
        result = "Optional[" + result + "]";
    }
    result
}

fn pydantic_default(field) {
    if field.has_default_value {
        return py_default_literal(field.field_type, field.default_value);
    }
    if field.field_type.is_list {
        return "Field(default_factory=list)";
    }
    if field.field_type.is_option {
        return "None";
    }
    ()
}

fn dataclass_default(field) {
    if field.has_default_value {
        return py_default_literal(field.field_type, field.default_value);
    }
    if field.field_type.is_list {
        return "field(default_factory=list)";
    }
    if field.field_type.is_option {
        return "None";
    }
    ()
}

fn sqlalchemy_type(type_ref, max_length) {
    if type_ref.is_list || type_ref.is_struct {
        return "JSON";
    }

    switch type_ref.type_name {
        "u8" | "i8" | "u16" | "i16" => "SmallInteger",
        "u32" | "i32" => "Integer",
        "u64" | "i64" => "BigInteger",
        "f32" | "f64" => "Float",
        "bool" => "Boolean",
        "bytes" => "LargeBinary",
        "timestamp" => "DateTime",
        "string" => {
            if max_length != () {
                "String(" + max_length + ")"
            } else {
                "String"
            }
        },
        _ => "Integer"
    }
}

fn sqlalchemy_mapped_type(type_ref) {
    let result = py_type_base(type_ref);
    if type_ref.is_list || type_ref.is_struct {
        result = "dict";
    }
    if type_ref.is_option {
        result = "Optional[" + result + "]";
    }
    result
}
