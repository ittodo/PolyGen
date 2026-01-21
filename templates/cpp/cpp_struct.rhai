${
    // C++ struct generator template
    // Expects 's' to be a StructDef object and 'current_ns' to be set

    import "templates/cpp/rhai_utils/type_mapping" as cpp_types;

    // Main generation
    let ns_fqn = if current_ns != () { current_ns } else { "" };
    let result = "";

    // Generate embedded structs FIRST (they must be declared before parent struct uses them)
    // Recursion handles nested embedded structs (e.g., Enchantment before DropItems)
    for item in s.items {
        if item.is_embedded_struct() {
            let es = item.as_embedded_struct();
            let struct_tmpl = include("templates/cpp/cpp_struct.rhai");
            let res = eval("let s = es; let current_ns = \"" + ns_fqn + "\";\n`" + struct_tmpl + "`");
            result += res + "\n";
        }
    }

    // Generate doc comments from header
    for item in s.header {
        if item.is_comment() {
            result += "/// " + item.as_comment() + "\n";
        }
    }

    // Struct declaration
    result += "struct " + s.name + " {\n";

    // First pass: Generate inline enums (must be defined before use)
    for item in s.items {
        if item.is_inline_enum() {
            let ie = item.as_inline_enum();
            result += "    enum class " + ie.name + " : int32_t {\n";
            for enum_item in ie.items {
                if enum_item.is_member() {
                    let m = enum_item.as_member();
                    if m.value != () {
                        result += "        " + m.name + " = " + m.value + ",\n";
                    } else {
                        result += "        " + m.name + ",\n";
                    }
                } else if enum_item.is_comment() {
                    result += "        // " + enum_item.as_comment() + "\n";
                }
            }
            result += "    };\n\n";
        }
    }

    // Second pass: Generate fields and comments
    for item in s.items {
        if item.is_field() {
            let f = item.as_field();

            // Field type and name
            let cpp_type = cpp_types::map_type_with_ns(f.field_type, ns_fqn);
            result += "    " + cpp_type + " " + f.name;

            // Default value
            let default_val = cpp_types::get_default_value(f.field_type);
            if default_val != "{}" || f.field_type.is_list || f.field_type.is_option {
                result += " = " + default_val;
            }

            result += ";\n";
        } else if item.is_comment() {
            result += "    // " + item.as_comment() + "\n";
        }
    }

    // Default constructor
    result += "\n";
    result += "    " + s.name + "() = default;\n";

    result += "};\n";

    result
}
