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

            // Default value - prefer user-specified value over type default
            if f.default_value != () {
                // Use user-specified default value
                let default_val = f.default_value;
                let type_name = f.field_type.type_name;

                // Format for C++
                if type_name == "f32" {
                    // Ensure float has 'f' suffix
                    if !default_val.ends_with("f") && !default_val.ends_with("F") {
                        default_val += "f";
                    }
                } else if type_name == "bool" {
                    default_val = default_val.to_lower();
                } else if type_name == "string" {
                    if !default_val.starts_with("\"") {
                        default_val = "\"" + default_val + "\"";
                    }
                }

                result += " = " + default_val;
            } else {
                // Use type-based default
                let default_val = cpp_types::get_default_value(f.field_type);
                if default_val != "{}" || f.field_type.is_list || f.field_type.is_option {
                    result += " = " + default_val;
                }
            }

            result += ";\n";
        } else if item.is_comment() {
            result += "    // " + item.as_comment() + "\n";
        }
    }

    // Default constructor
    result += "\n";
    result += "    " + s.name + "() = default;\n";

    // Add Pack/Unpack methods for embeds with @pack annotation
    if s.is_embed && s.pack_separator != () {
        let sep = s.pack_separator;
        let fields = [];

        // Collect field info
        for item in s.items {
            if item.is_field() {
                let f = item.as_field();
                fields.push(f);
            }
        }

        if fields.len() > 0 {
            result += "\n";
            result += "    /// Packs all fields into a single string using '" + sep + "' as separator.\n";
            result += "    [[nodiscard]] std::string pack() const {\n";
            result += "        std::ostringstream oss;\n";
            for i in 0..fields.len() {
                let f = fields[i];
                if i > 0 {
                    result += "        oss << \"" + sep + "\";\n";
                }
                result += "        oss << " + f.name + ";\n";
            }
            result += "        return oss.str();\n";
            result += "    }\n";

            result += "\n";
            result += "    /// Unpacks a string into fields using '" + sep + "' as separator.\n";
            result += "    static " + s.name + " unpack(const std::string& value) {\n";
            result += "        " + s.name + " result;\n";
            result += "        std::istringstream iss(value);\n";
            result += "        std::string part;\n";

            for i in 0..fields.len() {
                let f = fields[i];
                let type_name = f.field_type.type_name;

                if i < fields.len() - 1 {
                    result += "        if (!std::getline(iss, part, '" + sep + "')) {\n";
                    result += "            throw std::runtime_error(\"Failed to unpack " + s.name + ": missing field '" + f.name + "'\");\n";
                    result += "        }\n";
                } else {
                    result += "        if (!std::getline(iss, part)) {\n";
                    result += "            throw std::runtime_error(\"Failed to unpack " + s.name + ": missing field '" + f.name + "'\");\n";
                    result += "        }\n";
                }

                // Generate parse expression based on type
                if type_name == "string" {
                    result += "        result." + f.name + " = part;\n";
                } else if type_name == "bool" {
                    result += "        result." + f.name + " = (part == \"true\" || part == \"1\");\n";
                } else if type_name == "f32" {
                    result += "        result." + f.name + " = std::stof(part);\n";
                } else if type_name == "f64" {
                    result += "        result." + f.name + " = std::stod(part);\n";
                } else if type_name == "i8" || type_name == "i16" || type_name == "i32" {
                    result += "        result." + f.name + " = static_cast<" + cpp_types::map_type_with_ns(f.field_type, ns_fqn) + ">(std::stoi(part));\n";
                } else if type_name == "i64" {
                    result += "        result." + f.name + " = std::stoll(part);\n";
                } else if type_name == "u8" || type_name == "u16" || type_name == "u32" {
                    result += "        result." + f.name + " = static_cast<" + cpp_types::map_type_with_ns(f.field_type, ns_fqn) + ">(std::stoul(part));\n";
                } else if type_name == "u64" {
                    result += "        result." + f.name + " = std::stoull(part);\n";
                } else {
                    // For other types, try string assignment
                    result += "        result." + f.name + " = part;\n";
                }
            }

            result += "        return result;\n";
            result += "    }\n";

            result += "\n";
            result += "    /// Tries to unpack a string. Returns true on success.\n";
            result += "    static bool try_unpack(const std::string& value, " + s.name + "& out) noexcept {\n";
            result += "        try {\n";
            result += "            out = unpack(value);\n";
            result += "            return true;\n";
            result += "        } catch (...) {\n";
            result += "            return false;\n";
            result += "        }\n";
            result += "    }\n";
        }
    }

    result += "};\n";

    result
}
