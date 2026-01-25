${
    import "templates/rhai_utils/indent" as indent_utils;
    import "templates/csharp/rhai_utils/type_utils" as type_utils;
    let body = "";

    // Container reference for navigation properties
    body += "    private IDataContainer? _container;\n\n";
    body += "    public void SetContainer(IDataContainer container)\n";
    body += "    {\n";
    body += "        _container = container;\n";
    body += "    }\n\n";

    // Fields
    for item in s.items
    {
        if item.is_comment()
        {
            let c = item.as_comment();
            body += "    // " + c + "\n";
        }
        else if item.is_field()
        {
            let f = item.as_field();

            for attributes in f.attributes
            {
                body += "    [" + attributes + "]\n";
            }

            // Generate field with optional default value
            let field_decl = "    public " + cs_map_type(f.field_type) + " " + f.name;
            if f.default_value != () {
                // Format default value for C#
                let default_val = f.default_value;
                let type_name = f.field_type.type_name;

                // Add type suffix for numeric literals
                if type_name == "f32" {
                    // Ensure float has 'f' suffix
                    if !default_val.ends_with("f") && !default_val.ends_with("F") {
                        default_val += "f";
                    }
                } else if type_name == "f64" {
                    // Ensure double has 'd' suffix or no suffix
                    if default_val.ends_with("f") || default_val.ends_with("F") {
                        default_val = default_val.sub_string(0, default_val.len() - 1);
                    }
                } else if type_name == "u64" || type_name == "i64" {
                    // Long types need 'L' suffix
                    if !default_val.ends_with("L") && !default_val.ends_with("l") {
                        default_val += "L";
                    }
                } else if type_name == "string" {
                    // Strings need quotes
                    if !default_val.starts_with("\"") {
                        default_val = "\"" + default_val + "\"";
                    }
                } else if type_name == "bool" {
                    // Ensure boolean is lowercase
                    default_val = default_val.to_lower();
                }

                field_decl += " = " + default_val;
            }
            field_decl += ";\n";
            body += field_decl;
        }
        else if item.is_embedded_struct()
        {
            let structString = render_items([item.as_embedded_struct()], "templates/csharp/class/csharp_logic_struct.rhai", "s");

            body += indent_utils::indent_text(structString , 1) + "\n";
        }
        else if item.is_inline_enum()
        {
            let enumString = render_items([item.as_inline_enum()], "templates/csharp/enum/csharp_enum.rhai", "e");
            body += indent_utils::indent_text(enumString , 1) + "\n";
        }
    }

    // Check if we have any navigation properties to generate
    let has_fk = false;
    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            if f.foreign_key != () {
                has_fk = true;
            }
        }
    }
    let has_relations = s.relations.len() > 0;

    // Forward navigation properties (from foreign_key)
    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            if f.foreign_key != () {
                let fk = f.foreign_key;
                // Get target table FQN for type reference
                let target_fqn = fk.target_table_fqn;

                // If target_fqn doesn't contain a dot, it's a relative reference
                // Use the current struct's parent namespace
                if !target_fqn.contains(".") {
                    // Get the parent namespace from s.fqn
                    let struct_fqn = s.fqn;
                    if struct_fqn.contains(".") {
                        let last_dot = struct_fqn.index_of(".");
                        let ns = "";
                        let parts = struct_fqn.split(".");
                        for i in 0..parts.len()-1 {
                            if ns != "" { ns += "."; }
                            ns += parts[i];
                        }
                        target_fqn = ns + "." + target_fqn;
                    }
                }

                // Get target table name (last part of FQN)
                let fqn_parts = target_fqn.split(".");
                let target_table_name = "";
                for part in fqn_parts {
                    target_table_name = part;
                }

                // Navigation property name is field name without "Id" suffix, or target table name
                let nav_name = target_table_name;
                if f.name.ends_with("Id") {
                    nav_name = f.name.sub_string(0, f.name.len() - 2);
                } else if f.name.ends_with("_id") {
                    nav_name = f.name.sub_string(0, f.name.len() - 3);
                }

                body += "\n    /// <summary>\n";
                body += "    /// Navigation property to " + target_table_name + ".\n";
                body += "    /// </summary>\n";
                body += "    public global::" + target_fqn + "? " + nav_name + "\n";
                body += "    {\n";
                body += "        get\n";
                body += "        {\n";
                body += "            if (_container is Schema.Container.IHas" + target_table_name + "Table c)\n";
                body += "                return c." + target_table_name + "s.ById[" + f.name + "];\n";
                body += "            return null;\n";
                body += "        }\n";
                body += "    }\n";
            }
        }
    }

    // Reverse navigation properties (from relations)
    for rel in s.relations {
        // Convert source_field to PascalCase for index name
        let field_parts = rel.source_field.split("_");
        let pascal_field = "";
        for part in field_parts {
            if part.len() > 0 {
                let first = part.sub_string(0, 1);
                first = first.to_upper();
                let rest = part.sub_string(1);
                pascal_field += first + rest;
            }
        }

        body += "\n    /// <summary>\n";
        body += "    /// Reverse navigation: " + rel.source_table_name + " rows referencing this " + s.name + ".\n";
        body += "    /// </summary>\n";
        body += "    public IReadOnlyList<" + rel.source_table_name + "> " + rel.name + "\n";
        body += "    {\n";
        body += "        get\n";
        body += "        {\n";
        body += "            if (_container is Schema.Container.IHas" + rel.source_table_name + "Table c)\n";
        body += "                return c." + rel.source_table_name + "s.By" + pascal_field + "[Id];\n";
        body += "            return Array.Empty<" + rel.source_table_name + ">();\n";
        body += "        }\n";
        body += "    }\n";
    }

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
            // Pack method - serialize fields to string
            body += "\n    /// <summary>\n";
            body += "    /// Packs all fields into a single string using '" + sep + "' as separator.\n";
            body += "    /// </summary>\n";
            body += "    public string Pack()\n";
            body += "    {\n";
            body += "        return ";
            for i in 0..fields.len() {
                let f = fields[i];
                if i > 0 {
                    body += " + \"" + sep + "\" + ";
                }
                body += f.name + ".ToString()";
            }
            body += ";\n";
            body += "    }\n";

            // Unpack method - deserialize from string
            body += "\n    /// <summary>\n";
            body += "    /// Unpacks a string into fields using '" + sep + "' as separator.\n";
            body += "    /// </summary>\n";
            body += "    /// <param name=\"value\">The packed string value.</param>\n";
            body += "    /// <returns>A new instance with parsed values.</returns>\n";
            body += "    /// <exception cref=\"FormatException\">Thrown when parsing fails.</exception>\n";
            body += "    public static " + s.name + " Unpack(string value)\n";
            body += "    {\n";
            body += "        if (string.IsNullOrEmpty(value))\n";
            body += "            throw new FormatException(\"Cannot unpack null or empty string to " + s.name + "\");\n";
            body += "\n";
            body += "        var parts = value.Split('" + sep + "');\n";
            body += "        if (parts.Length != " + fields.len() + ")\n";
            body += "            throw new FormatException($\"Expected " + fields.len() + " parts but got {parts.Length} when unpacking " + s.name + "\");\n";
            body += "\n";
            body += "        try\n";
            body += "        {\n";
            body += "            return new " + s.name + "\n";
            body += "            {\n";

            for i in 0..fields.len() {
                let f = fields[i];
                let type_name = f.field_type.type_name;
                let cs_type = cs_map_type(f.field_type);
                let parse_expr = "";

                // Generate parse expression based on type
                if type_name == "string" {
                    parse_expr = "parts[" + i + "]";
                } else if type_name == "bool" {
                    parse_expr = "bool.Parse(parts[" + i + "])";
                } else if type_name == "u8" || type_name == "i8" || type_name == "u16" || type_name == "i16" || type_name == "u32" || type_name == "i32" || type_name == "u64" || type_name == "i64" {
                    parse_expr = cs_type + ".Parse(parts[" + i + "])";
                } else if type_name == "f32" {
                    parse_expr = "float.Parse(parts[" + i + "], System.Globalization.CultureInfo.InvariantCulture)";
                } else if type_name == "f64" {
                    parse_expr = "double.Parse(parts[" + i + "], System.Globalization.CultureInfo.InvariantCulture)";
                } else {
                    // For other types, try generic Parse
                    parse_expr = cs_type + ".Parse(parts[" + i + "])";
                }

                body += "                " + f.name + " = " + parse_expr;
                if i < fields.len() - 1 {
                    body += ",";
                }
                body += "\n";
            }

            body += "            };\n";
            body += "        }\n";
            body += "        catch (Exception ex)\n";
            body += "        {\n";
            body += "            throw new FormatException($\"Failed to unpack '" + s.name + "' from '{value}': {ex.Message}\", ex);\n";
            body += "        }\n";
            body += "    }\n";

            // TryUnpack method - safe parsing
            body += "\n    /// <summary>\n";
            body += "    /// Tries to unpack a string into fields. Returns false on failure.\n";
            body += "    /// </summary>\n";
            body += "    public static bool TryUnpack(string value, out " + s.name + " result)\n";
            body += "    {\n";
            body += "        result = new " + s.name + "();\n";
            body += "        try\n";
            body += "        {\n";
            body += "            result = Unpack(value);\n";
            body += "            return true;\n";
            body += "        }\n";
            body += "        catch\n";
            body += "        {\n";
            body += "            return false;\n";
            body += "        }\n";
            body += "    }\n";
        }
    }

    // Generate OnUpdate{FieldName}() methods for auto_update fields
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
        body += "\n    // Auto-update timestamp methods\n";

        for f in auto_update_fields {
            let tz = f.auto_update;
            let datetime_expr = type_utils::get_datetime_expr_for_timezone(tz);

            // Convert field name to PascalCase for method name
            let field_parts = f.name.split("_");
            let pascal_name = "";
            for part in field_parts {
                if part.len() > 0 {
                    let first = part.sub_string(0, 1);
                    first = first.to_upper();
                    let rest = part.sub_string(1);
                    pascal_name += first + rest;
                }
            }

            body += "\n    /// <summary>\n";
            body += "    /// Updates " + f.name + " to the current timestamp.\n";
            body += "    /// </summary>\n";
            body += "    public void OnUpdate" + pascal_name + "()\n";
            body += "    {\n";
            body += "        " + f.name + " = " + datetime_expr + ";\n";
            body += "    }\n";
        }

        // Generate OnUpdateAll() if there are multiple auto_update fields
        if auto_update_fields.len() > 1 {
            body += "\n    /// <summary>\n";
            body += "    /// Updates all auto-update timestamp fields.\n";
            body += "    /// </summary>\n";
            body += "    public void OnUpdateAll()\n";
            body += "    {\n";
            for f in auto_update_fields {
                let tz = f.auto_update;
                let datetime_expr = type_utils::get_datetime_expr_for_timezone(tz);
                body += "        " + f.name + " = " + datetime_expr + ";\n";
            }
            body += "    }\n";
        }
    }

    body
}
