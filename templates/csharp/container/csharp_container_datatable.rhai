/// <summary>
/// Data table for ${s.name} entities.
/// </summary>
public class ${s.name}Table : DataTableBase<${s.name}>
{
${
    let body = "";

    // Generate indexes
    for idx in s.indexes {
        let key_type = cs_map_type(idx.field_type);
        if idx.is_unique {
            body += "    public UniqueIndex<" + key_type + ", " + s.name + "> " + idx.name + " { get; } = new();\n";
        } else {
            body += "    public GroupIndex<" + key_type + ", " + s.name + "> " + idx.name + " { get; } = new();\n";
        }
    }

    if s.indexes.len > 0 {
        body += "\n";
    }

    // Generate Add method
    body += "    /// <summary>\n";
    body += "    /// Adds a row to the table.\n";
    body += "    /// </summary>\n";
    body += "    public void Add(" + s.name + " row)\n";
    body += "    {\n";
    body += "        AddRowInternal(row);\n";
    body += "    }\n\n";

    // Generate OnRowAdded override
    body += "    protected override void OnRowAdded(" + s.name + " row)\n";
    body += "    {\n";

    for idx in s.indexes {
        let field_name = idx.field_name;
        // Convert snake_case to PascalCase for C# property name
        let parts = field_name.split("_");
        let pascal_name = "";
        for part in parts {
            if part.len > 0 {
                let first = part.sub_string(0, 1);
                first = first.to_upper();
                let rest = part.sub_string(1);
                pascal_name += first + rest;
            }
        }

        body += "        " + idx.name + ".Add(row." + pascal_name + ", row);\n";
    }

    body += "    }\n\n";

    // Generate OnCleared override
    body += "    protected override void OnCleared()\n";
    body += "    {\n";

    for idx in s.indexes {
        body += "        " + idx.name + ".Clear();\n";
    }

    body += "    }\n";

    body
}
}
