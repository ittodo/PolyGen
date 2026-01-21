${
    // Rust struct generator template
    // Expects 's' to be a StructDef object

    import "templates/rust/rhai_utils/type_mapping" as rust_types;

    // Generate doc comments from header
    let doc_comments = "";
    for item in s.header {
        if item.is_comment() {
            doc_comments += "/// " + item.as_comment() + "\n";
        }
    }

    // Generate fields
    let fields = "";
    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            let field_name = to_snake_case(f.name);
            let rust_type = rust_types::map_type(f.field_type);

            // Add serde rename if field name differs from original
            let rename_attr = "";
            if field_name != f.name {
                rename_attr = "#[serde(rename = \"" + f.name + "\")]\n    ";
            }

            fields += "    " + rename_attr + "pub " + field_name + ": " + rust_type + ",\n";
        } else if item.is_comment() {
            fields += "    // " + item.as_comment() + "\n";
        }
    }

    // Generate embedded structs (inline)
    let embedded = "";
    for item in s.items {
        if item.is_embedded_struct() {
            let es = item.as_embedded_struct();
            let struct_tmpl = include("templates/rust/rust_struct.rhai");
            let res = eval("let s = es;\n`" + struct_tmpl + "`");
            embedded += "\n" + res;
        } else if item.is_inline_enum() {
            let ie = item.as_inline_enum();
            let enum_tmpl = include("templates/rust/rust_enum.rhai");
            let res = eval("let e = ie;\n`" + enum_tmpl + "`");
            embedded += "\n" + res;
        }
    }

    let result = "";
    result += doc_comments;
    result += "#[derive(Debug, Clone, Serialize, Deserialize)]\n";
    result += "pub struct " + s.name + " {\n";
    result += fields;
    result += "}\n";
    result += embedded;

    result
}
