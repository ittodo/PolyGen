${
    // Go struct generator template
    // Expects 's' to be a StructDef object

    import "templates/go/rhai_utils/type_mapping" as go_types;

    // Generate doc comments from header
    let doc_comments = "";
    for item in s.header {
        if item.is_comment() {
            doc_comments += "// " + item.as_comment() + "\n";
        }
    }

    // Current namespace FQN for type resolution
    let ns_fqn = if current_ns != () { current_ns } else { "" };

    // Generate fields
    let fields = "";
    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            let field_name = to_pascal_case(f.name);  // Go uses PascalCase for exported fields
            let go_type = go_types::map_type_with_ns(f.field_type, ns_fqn);
            let json_tag = to_snake_case(f.name);

            // Add omitempty for optional fields
            let tag_suffix = "";
            if f.field_type.is_option {
                tag_suffix = ",omitempty";
            }

            fields += "\t" + field_name + " " + go_type + " `json:\"" + json_tag + tag_suffix + "\"`\n";
        } else if item.is_comment() {
            fields += "\t// " + item.as_comment() + "\n";
        }
    }

    // Generate embedded structs (inline)
    let embedded = "";
    for item in s.items {
        if item.is_embedded_struct() {
            let es = item.as_embedded_struct();
            let struct_tmpl = include("templates/go/go_struct.rhai");
            let res = eval("let s = es; let current_ns = \"" + ns_fqn + "\";\n`" + struct_tmpl + "`");
            embedded += "\n" + res;
        } else if item.is_inline_enum() {
            let ie = item.as_inline_enum();
            let enum_tmpl = include("templates/go/go_enum.rhai");
            let res = eval("let e = ie;\n`" + enum_tmpl + "`");
            embedded += "\n" + res;
        }
    }

    // Check if any field has a default value
    let has_defaults = false;
    let default_assignments = "";
    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            if f.default_value != () {
                has_defaults = true;
                let field_name = to_pascal_case(f.name);
                let go_type = go_types::map_type_with_ns(f.field_type, ns_fqn);
                let default_val = f.default_value;

                // Format default value for Go
                if f.field_type.type_name == "string" {
                    default_val = "\"" + default_val + "\"";
                } else if f.field_type.type_name == "bool" {
                    default_val = if default_val == "true" { "true" } else { "false" };
                }
                // For numeric types, Go handles them directly

                if f.field_type.is_option {
                    // For optional fields, we need to create a pointer
                    let tmp_var = to_snake_case(f.name) + "Default";
                    default_assignments += "\t" + tmp_var + " := " + default_val + "\n";
                    default_assignments += "\tr." + field_name + " = &" + tmp_var + "\n";
                } else {
                    default_assignments += "\tr." + field_name + " = " + default_val + "\n";
                }
            }
        }
    }

    let result = "";
    result += doc_comments;
    result += "type " + s.name + " struct {\n";
    result += fields;
    result += "}\n";

    // Generate constructor with defaults if needed
    if has_defaults {
        result += "\n// New" + s.name + " creates a new " + s.name + " with default values.\n";
        result += "func New" + s.name + "() *" + s.name + " {\n";
        result += "\tr := &" + s.name + "{}\n";
        result += default_assignments;
        result += "\treturn r\n";
        result += "}\n";
    }

    result += embedded;

    result
}
