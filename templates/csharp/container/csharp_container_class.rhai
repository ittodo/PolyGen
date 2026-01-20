/// <summary>
/// Root data container for ${namespace_name} entities.
/// </summary>
${
    // Generate container name by removing dots from namespace
    let parts = namespace_name.split(".");
    let container_name = "";
    for part in parts {
        container_name += part;
    }
    "public class " + container_name + "DataContainer : IDataContainer"
}
{
${
    let body = "";

    // Generate table properties
    for s in structs {
        body += "    public " + s.name + "Table " + s.name + "s { get; } = new();\n";
    }

    if structs.len > 0 {
        body += "\n";
    }

    // Generate constructor - recalculate container_name
    let parts = namespace_name.split(".");
    let container_name = "";
    for part in parts {
        container_name += part;
    }
    body += "    public " + container_name + "DataContainer()\n";
    body += "    {\n";
    for s in structs {
        body += "        " + s.name + "s.SetContainer(this);\n";
    }
    body += "    }\n\n";

    // Generate Clear method
    body += "    /// <summary>\n";
    body += "    /// Clears all data from all tables.\n";
    body += "    /// </summary>\n";
    body += "    public void Clear()\n";
    body += "    {\n";
    for s in structs {
        body += "        " + s.name + "s.Clear();\n";
    }
    body += "    }\n";

    body
}
}
