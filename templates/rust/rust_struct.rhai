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
    // current_ns is expected to be set by the caller (namespace FQN)
    let ns_fqn = if current_ns != () { current_ns } else { "" };
    let fields = "";
    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            let field_name = to_snake_case(f.name);
            let rust_type = rust_types::map_type_with_ns(f.field_type, ns_fqn);

            // Build field attributes
            let serde_attrs = "";

            // Add serde rename if field name differs from original
            if field_name != f.name {
                serde_attrs = "rename = \"" + f.name + "\"";
            }

            // Add serde default if field has default value
            if f.default_value != () {
                if serde_attrs != "" {
                    serde_attrs += ", ";
                }
                serde_attrs += "default";
            }

            // Generate the attribute line if any attributes are present
            let attr_line = "";
            if serde_attrs != "" {
                attr_line = "#[serde(" + serde_attrs + ")]\n    ";
            }

            fields += "    " + attr_line + "pub " + field_name + ": " + rust_type + ",\n";
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
            let res = eval("let s = es; let current_ns = \"" + ns_fqn + "\";\n`" + struct_tmpl + "`");
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
    result += "#[derive(Debug, Clone, Default, Serialize, Deserialize)]\n";
    result += "pub struct " + s.name + " {\n";
    result += fields;
    result += "}\n";

    // Add Pack/Unpack methods for embeds with @pack annotation
    if s.is_embed && s.pack_separator != () {
        let sep = s.pack_separator;
        let field_list = [];

        // Collect field info
        for item in s.items {
            if item.is_field() {
                let f = item.as_field();
                field_list.push(f);
            }
        }

        if field_list.len() > 0 {
            result += "\nimpl " + s.name + " {\n";

            // Pack method
            result += "    /// Packs all fields into a single string using '" + sep + "' as separator.\n";
            result += "    pub fn pack(&self) -> String {\n";
            result += "        format!(\"";
            for i in 0..field_list.len() {
                if i > 0 {
                    result += sep;
                }
                result += "{}";
            }
            result += "\"";
            for i in 0..field_list.len() {
                let f = field_list[i];
                let field_name = to_snake_case(f.name);
                result += ", self." + field_name;
            }
            result += ")\n";
            result += "    }\n";

            // Unpack method
            result += "\n    /// Unpacks a string into fields using '" + sep + "' as separator.\n";
            result += "    pub fn unpack(value: &str) -> Result<Self, String> {\n";
            result += "        let parts: Vec<&str> = value.split('" + sep + "').collect();\n";
            result += "        if parts.len() != " + field_list.len() + " {\n";
            result += "            return Err(format!(\n";
            result += "                \"Expected " + field_list.len() + " parts but got {} when unpacking " + s.name + "\",\n";
            result += "                parts.len()\n";
            result += "            ));\n";
            result += "        }\n";
            result += "        Ok(Self {\n";

            for i in 0..field_list.len() {
                let f = field_list[i];
                let field_name = to_snake_case(f.name);
                let type_name = f.field_type.type_name;
                let parse_expr = "";

                // Generate parse expression based on type
                if type_name == "string" {
                    parse_expr = "parts[" + i + "].to_string()";
                } else if type_name == "bool" {
                    parse_expr = "parts[" + i + "].parse().map_err(|e| format!(\"Failed to parse " + field_name + ": {}\", e))?";
                } else {
                    parse_expr = "parts[" + i + "].parse().map_err(|e| format!(\"Failed to parse " + field_name + ": {}\", e))?";
                }

                result += "            " + field_name + ": " + parse_expr + ",\n";
            }

            result += "        })\n";
            result += "    }\n";
            result += "}\n";
        }
    }

    // Generate on_update_{field_name}() methods for auto_update fields
    let auto_update_fields = [];
    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            if f.auto_update != () {
                auto_update_fields.push(f);
            }
        }
    }

    if auto_update_fields.len() > 0 {
        // Check if we already have an impl block from @pack
        let need_impl = !(s.is_embed && s.pack_separator != ());

        if need_impl {
            result += "\nimpl " + s.name + " {\n";
        }

        for f in auto_update_fields {
            let field_name = to_snake_case(f.name);
            let datetime_expr = rust_types::get_datetime_expr_for_timezone(f.auto_update);

            result += "    /// Updates " + field_name + " to the current timestamp.\n";
            result += "    pub fn on_update_" + field_name + "(&mut self) {\n";
            result += "        self." + field_name + " = " + datetime_expr + ";\n";
            result += "    }\n\n";
        }

        // Generate on_update_all() if there are multiple auto_update fields
        if auto_update_fields.len() > 1 {
            result += "    /// Updates all auto-update timestamp fields.\n";
            result += "    pub fn on_update_all(&mut self) {\n";
            for f in auto_update_fields {
                let field_name = to_snake_case(f.name);
                let datetime_expr = rust_types::get_datetime_expr_for_timezone(f.auto_update);
                result += "        self." + field_name + " = " + datetime_expr + ";\n";
            }
            result += "    }\n";
        }

        if need_impl {
            result += "}\n";
        }
    }

    result += embedded;

    result
}
