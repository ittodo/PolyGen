${
    // TypeScript interface generator template
    // Expects 's' to be a StructDef object

    import "templates/typescript/rhai_utils/type_mapping" as ts_types;

    // Generate doc comments from header
    let doc_comments = "";
    for item in s.header {
        if item.is_comment() {
            doc_comments += "/** " + item.as_comment() + " */\n";
        }
    }

    // Current namespace FQN for type resolution
    let ns_fqn = if current_ns != () { current_ns } else { "" };

    // Generate fields
    let fields = "";
    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            let field_name = to_camel_case(f.name);
            let ts_type = ts_types::map_type_with_ns(f.field_type, ns_fqn);

            // Use optional property syntax for nullable types
            let optional_marker = "";
            if f.field_type.is_option {
                optional_marker = "?";
                // Remove " | null" suffix if using optional marker
                ts_type = ts_types::map_type(f.field_type);
                if ts_type.ends_with(" | null") {
                    ts_type = ts_type.sub_string(0, ts_type.len() - 7);
                }
            }

            fields += "    " + field_name + optional_marker + ": " + ts_type + ";\n";
        } else if item.is_comment() {
            fields += "    // " + item.as_comment() + "\n";
        }
    }

    // Generate embedded interfaces (inline)
    let embedded = "";
    for item in s.items {
        if item.is_embedded_struct() {
            let es = item.as_embedded_struct();
            let interface_tmpl = include("templates/typescript/typescript_interface.rhai");
            let res = eval("let s = es; let current_ns = \"" + ns_fqn + "\";\n`" + interface_tmpl + "`");
            embedded += "\n" + res;
        } else if item.is_inline_enum() {
            let ie = item.as_inline_enum();
            let enum_tmpl = include("templates/typescript/typescript_enum.rhai");
            let res = eval("let e = ie;\n`" + enum_tmpl + "`");
            embedded += "\n" + res;
        }
    }

    let result = "";
    result += doc_comments;
    result += "export interface " + s.name + " {\n";
    result += fields;
    result += "}\n";
    result += embedded;

    result
}
