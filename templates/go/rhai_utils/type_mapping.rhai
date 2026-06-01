// Go type and container helpers

fn go_lang_type(type_ref) {
    if type_ref.is_list {
        let item_type = if type_ref.has_inner {
            go_lang_type(type_ref.inner)
        } else {
            go_base_type(type_ref)
        };
        let list_type = "[]" + item_type;
        if type_ref.is_option {
            return "*" + list_type;
        }
        return list_type;
    }

    if type_ref.is_option {
        if type_ref.has_inner {
            return "*" + go_lang_type(type_ref.inner);
        }
        return "*" + go_base_type(type_ref);
    }

    go_base_type(type_ref)
}

fn go_base_type(type_ref) {
    if type_ref.is_primitive || go_is_primitive_name(type_ref.type_name) {
        return go_primitive_type(type_ref.type_name);
    }
    type_ref.type_name
}

fn go_is_primitive_name(type_name) {
    switch type_name {
        "u8" | "u16" | "u32" | "u64" |
        "i8" | "i16" | "i32" | "i64" |
        "f32" | "f64" | "bool" | "string" | "bytes" => true,
        _ => false
    }
}

fn go_primitive_type(type_name) {
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
        _ => type_name
    }
}

fn go_row_key_expr(struct_def) {
    for item in struct_def.items {
        if item.is_field {
            let field = as_field(item);
            if field.is_primary_key {
                return "row." + to_pascal_case(field.name);
            }
        }
    }
    "\"\""
}
