${
    import "templates/rhai_utils/indent" as indent_utils;
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

            body += "    public " + cs_map_type(f.field_type) + " " + f.name + ";\n";
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

    body
}
