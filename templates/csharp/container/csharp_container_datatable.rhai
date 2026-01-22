/// <summary>
/// Data table for ${s.name} entities.
/// </summary>
public class ${s.name}Table : DataTableBase<${s.name}>
{
${
    let body = "";

    // Generate indexes (using type_utils functions)
    for idx in s.indexes {
        let key_type = type_utils::get_index_key_type_cs(idx);
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
        let key_expr = type_utils::get_index_key_expr_cs(idx, "row");
        body += "        " + idx.name + ".Add(" + key_expr + ", row);\n";
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
