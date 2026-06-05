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

fn go_search_base_type(type_ref) {
    if type_ref.is_option {
        return type_ref.inner;
    }
    return type_ref;
}

fn go_search_base_name(type_ref) {
    let base = go_search_base_type(type_ref);
    if base.lang_type != "" {
        return base.lang_type;
    }
    return base.type_name;
}

fn go_search_target_enabled(search) {
    return search.target == "csharp" || search.target == "go" || search.target == "go_container";
}

fn go_search_supported(field) {
    if !field.has_search_index {
        return false;
    }

    let search = field.search_index;
    if !go_search_target_enabled(search) {
        return false;
    }

    let type_ref = field.field_type;
    if type_ref.is_list {
        return false;
    }

    let base = go_search_base_type(type_ref);
    let base_name = go_search_base_name(type_ref);
    let mode = go_search_mode(field);
    if mode == "ngram" || mode == "word" {
        return base_name == "string";
    }

    if mode != "exact" {
        return false;
    }

    if base_name == "string" || base_name == "bool" {
        return true;
    }
    if base.is_enum {
        return true;
    }
    if base.is_primitive && base_name != "bytes" && base_name != "f32" && base_name != "f64" && base_name != "timestamp" {
        return true;
    }
    return false;
}

fn go_search_key_type(field) {
    let mode = go_search_mode(field);
    if mode == "ngram" || mode == "word" {
        return "string";
    }
    return go_lang_type(go_search_base_type(field.field_type));
}

fn go_search_var_name(field) {
    "search" + to_pascal_case(field.search_index.name)
}

fn go_search_method_name(field) {
    "SearchBy" + to_pascal_case(field.search_index.name)
}

fn go_search_mode(field) {
    let mode = field.search_index.mode;
    if mode != "" {
        return mode;
    }
    let base_name = go_search_base_name(field.field_type);
    if base_name == "string" {
        return "ngram";
    }
    return "exact";
}

fn go_search_normalize(field) {
    let normalize = field.search_index.normalize;
    if normalize != "" {
        return normalize;
    }
    let base_name = go_search_base_name(field.field_type);
    if base_name == "string" {
        return "lower_trim";
    }
    return "none";
}

fn go_search_n(field) {
    let n = field.search_index.n;
    if n > 0 {
        return n;
    }
    return 2;
}

fn go_search_min(field) {
    let min = field.search_index.min;
    if min > 0 {
        return min;
    }
    return go_search_n(field);
}

fn go_index_name(idx) {
    if idx.is_composite {
        let result = "";
        for field in idx.fields {
            result += to_pascal_case(field.name);
        }
        return result;
    }
    return idx.field_name;
}

fn go_index_member_name(idx) {
    "by" + to_pascal_case(go_index_name(idx))
}

fn go_index_method_name(idx) {
    "GetBy" + to_pascal_case(go_index_name(idx))
}

fn go_index_key_type(struct_def, idx) {
    if idx.is_composite {
        return struct_def.name + to_pascal_case(idx.name) + "Key";
    }
    return go_lang_type(idx.field_type);
}

fn go_index_key_expr(struct_def, idx, row_var) {
    if idx.is_composite {
        let result = go_index_key_type(struct_def, idx) + "{";
        for i in 0..idx.fields.len() {
            if i > 0 {
                result += ", ";
            }
            let field = idx.fields[i];
            result += to_pascal_case(field.name) + ": " + row_var + "." + to_pascal_case(field.name);
        }
        result += "}";
        return result;
    }
    return row_var + "." + to_pascal_case(idx.field_name);
}
