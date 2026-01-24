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

    // RootDirectory property for pattern-based loading
    body += "    /// <summary>\n";
    body += "    /// Gets or sets the root directory for resolving relative file paths.\n";
    body += "    /// </summary>\n";
    body += "    public string RootDirectory { get; set; } = \".\";\n\n";

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
    body += "    public " + container_name + "DataContainer(string rootDirectory = \".\")\n";
    body += "    {\n";
    body += "        RootDirectory = rootDirectory;\n";
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
    body += "    }\n\n";

    // Generate ValidateAll method
    body += "    /// <summary>\n";
    body += "    /// Validates all tables against field and foreign key constraints.\n";
    body += "    /// </summary>\n";
    body += "    /// <returns>A ValidationResult containing any validation errors.</returns>\n";
    body += "    public ValidationResult ValidateAll()\n";
    body += "    {\n";
    body += "        var result = new ValidationResult();\n\n";
    body += "        // Validate field constraints (MaxLength, Range, Regex)\n";
    for s in structs {
        body += "        result.Merge(" + s.name + "s.Validate());\n";
    }
    body += "\n";
    body += "        // Validate foreign key references\n";
    for s in structs {
        body += "        result.Merge(" + s.name + "s.ValidateForeignKeys(this));\n";
    }
    body += "\n";
    body += "        return result;\n";
    body += "    }\n\n";

    // Generate ValidateOrThrow method
    body += "    /// <summary>\n";
    body += "    /// Validates all tables and throws an exception if any errors are found.\n";
    body += "    /// </summary>\n";
    body += "    /// <exception cref=\"ValidationException\">Thrown when validation fails.</exception>\n";
    body += "    public void ValidateOrThrow()\n";
    body += "    {\n";
    body += "        var result = ValidateAll();\n";
    body += "        if (!result.IsValid)\n";
    body += "            throw new ValidationException(result);\n";
    body += "    }\n";

    body
}
}
