${
    import "templates/rhai_utils/indent" as indent_utils;

    // Convert datasource name to PascalCase for class name
    let ds_parts = ds_name.split("_");
    let container_class_name = "";
    for part in ds_parts {
        if part.len() > 0 {
            let first = part.sub_string(0, 1);
            first = first.to_upper();
            let rest = part.sub_string(1);
            container_class_name += first + rest;
        }
    }
    container_class_name += "Container";

    // Collect all structs from all namespaces in this datasource
    let all_structs = [];
    for entry in ds_entries {
        let ns = entry.namespace;
        for s in entry.structs {
            all_structs.push(#{
                struct: s,
                namespace: ns.name
            });
        }
    }

    let body = "";

    // Class declaration
    body += "/// <summary>\n";
    body += "/// Container for '" + ds_name + "' datasource tables.\n";
    body += "/// Groups all namespaces marked with @datasource(\"" + ds_name + "\").\n";
    body += "/// </summary>\n";
    body += "public class " + container_class_name + "\n";
    body += "{\n";

    // RootDirectory property
    body += "    /// <summary>\n";
    body += "    /// Gets or sets the root directory for resolving relative file paths.\n";
    body += "    /// </summary>\n";
    body += "    public string RootDirectory { get; set; } = \".\";\n\n";

    // Table properties
    for entry in all_structs {
        let s = entry.struct;
        let ns = entry.namespace;
        let table_type = if ns != "" { "global::" + ns + ".Container." + s.name + "Table" } else { s.name + "Table" };
        body += "    /// <summary>" + s.name + " table from " + ns + " namespace.</summary>\n";
        body += "    public " + table_type + " " + s.name + "s { get; } = new();\n";
    }

    if all_structs.len() > 0 {
        body += "\n";
    }

    // Constructor
    body += "    public " + container_class_name + "(string rootDirectory = \".\")\n";
    body += "    {\n";
    body += "        RootDirectory = rootDirectory;\n";
    body += "    }\n\n";

    // Clear method
    body += "    /// <summary>\n";
    body += "    /// Clears all data from all tables in this container.\n";
    body += "    /// </summary>\n";
    body += "    public void Clear()\n";
    body += "    {\n";
    for entry in all_structs {
        let s = entry.struct;
        body += "        " + s.name + "s.Clear();\n";
    }
    body += "    }\n\n";

    // LoadFromCsv method
    body += "    /// <summary>\n";
    body += "    /// Loads all tables from CSV files using @load annotation patterns.\n";
    body += "    /// </summary>\n";
    body += "    public void LoadFromCsv(char separator = ',', CsvUtils.GapMode gapMode = CsvUtils.GapMode.Break)\n";
    body += "    {\n";
    for entry in all_structs {
        let s = entry.struct;
        let ns = entry.namespace;
        // Extract @load csv pattern
        let csv_pattern = "";
        for header_item in s.header {
            if header_item.is_annotation() {
                let ann = header_item.as_annotation();
                if ann.name == "load" {
                    for param in ann.params {
                        if param.key == "csv" {
                            csv_pattern = param.value;
                        }
                    }
                }
            }
        }
        if csv_pattern != "" {
            body += "        // " + s.name + ": @load(csv: \"" + csv_pattern + "\")\n";
            body += "        foreach (var item in Csv." + ns + "." + s.name + ".ReadRowsWithHeader(\n";
            body += "            Path.Combine(RootDirectory, \"" + csv_pattern + "\"), separator, gapMode))\n";
            body += "        {\n";
            body += "            " + s.name + "s.Add(item);\n";
            body += "        }\n";
        }
    }
    body += "    }\n\n";

    // LoadFromCsvDirectory method
    body += "    /// <summary>\n";
    body += "    /// Loads all tables from CSV files in a directory.\n";
    body += "    /// Files should be named: {TableName}.csv\n";
    body += "    /// </summary>\n";
    body += "    public void LoadFromCsvDirectory(string directoryPath, char separator = ',', CsvUtils.GapMode gapMode = CsvUtils.GapMode.Break)\n";
    body += "    {\n";
    for entry in all_structs {
        let s = entry.struct;
        let ns = entry.namespace;
        let var_name = s.name.to_lower() + "Path";
        body += "        var " + var_name + " = Path.Combine(directoryPath, \"" + s.name + ".csv\");\n";
        body += "        if (File.Exists(" + var_name + "))\n";
        body += "        {\n";
        body += "            foreach (var item in Csv." + ns + "." + s.name + ".ReadRowsWithHeader(" + var_name + ", separator, gapMode))\n";
        body += "                " + s.name + "s.Add(item);\n";
        body += "        }\n";
    }
    body += "    }\n\n";

    // LoadFromJson method
    body += "    /// <summary>\n";
    body += "    /// Loads all tables from JSON files using @load annotation patterns.\n";
    body += "    /// </summary>\n";
    body += "    public void LoadFromJson()\n";
    body += "    {\n";
    for entry in all_structs {
        let s = entry.struct;
        let ns = entry.namespace;
        // Extract @load json pattern
        let json_pattern = "";
        for header_item in s.header {
            if header_item.is_annotation() {
                let ann = header_item.as_annotation();
                if ann.name == "load" {
                    for param in ann.params {
                        if param.key == "json" {
                            json_pattern = param.value;
                        }
                    }
                }
            }
        }
        if json_pattern != "" {
            body += "        // " + s.name + ": @load(json: \"" + json_pattern + "\")\n";
            body += "        {\n";
            body += "            var json = File.ReadAllText(Path.Combine(RootDirectory, \"" + json_pattern + "\"));\n";
            body += "            var items = System.Text.Json.JsonSerializer.Deserialize<List<global::" + ns + "." + s.name + ">>(json);\n";
            body += "            if (items != null)\n";
            body += "                foreach (var item in items)\n";
            body += "                    " + s.name + "s.Add(item);\n";
            body += "        }\n";
        }
    }
    body += "    }\n\n";

    // LoadFromJsonDirectory method
    body += "    /// <summary>\n";
    body += "    /// Loads all tables from JSON files in a directory.\n";
    body += "    /// Files should be named: {TableName}.json\n";
    body += "    /// </summary>\n";
    body += "    public void LoadFromJsonDirectory(string directoryPath)\n";
    body += "    {\n";
    for entry in all_structs {
        let s = entry.struct;
        let ns = entry.namespace;
        let var_name = s.name.to_lower() + "Path";
        body += "        var " + var_name + " = Path.Combine(directoryPath, \"" + s.name + ".json\");\n";
        body += "        if (File.Exists(" + var_name + "))\n";
        body += "        {\n";
        body += "            var json = File.ReadAllText(" + var_name + ");\n";
        body += "            var items = System.Text.Json.JsonSerializer.Deserialize<List<global::" + ns + "." + s.name + ">>(json);\n";
        body += "            if (items != null)\n";
        body += "                foreach (var item in items)\n";
        body += "                    " + s.name + "s.Add(item);\n";
        body += "        }\n";
    }
    body += "    }\n\n";

    // LoadFromBinary method
    body += "    /// <summary>\n";
    body += "    /// Loads all tables from a binary file.\n";
    body += "    /// </summary>\n";
    body += "    public void LoadFromBinary(string filePath)\n";
    body += "    {\n";
    body += "        using var fs = File.OpenRead(filePath);\n";
    body += "        using var br = new BinaryReader(fs);\n";
    for entry in all_structs {
        let s = entry.struct;
        let ns = entry.namespace;
        let reader_ns = if ns != "" { ns } else { "Global" };
        body += "        {\n";
        body += "            var count = br.ReadInt32();\n";
        body += "            for (var i = 0; i < count; i++)\n";
        body += "                " + s.name + "s.Add(" + reader_ns + ".BinaryReaders.Read" + s.name + "(br));\n";
        body += "        }\n";
    }
    body += "    }\n\n";

    // SaveToBinary method
    body += "    /// <summary>\n";
    body += "    /// Saves all tables to a binary file.\n";
    body += "    /// </summary>\n";
    body += "    public void SaveToBinary(string filePath)\n";
    body += "    {\n";
    body += "        using var fs = File.Create(filePath);\n";
    body += "        using var bw = new BinaryWriter(fs);\n";
    for entry in all_structs {
        let s = entry.struct;
        let ns = entry.namespace;
        let writer_ns = if ns != "" { ns } else { "Global" };
        body += "        bw.Write(" + s.name + "s.Count);\n";
        body += "        foreach (var item in " + s.name + "s)\n";
        body += "            " + writer_ns + ".BinaryWriters.Write" + s.name + "(bw, item);\n";
    }
    body += "    }\n\n";

    // ValidateAll method (internal validation only)
    body += "    /// <summary>\n";
    body += "    /// Validates all tables within this container.\n";
    body += "    /// Note: Does not validate cross-container foreign keys. Use DataContext.ValidateAll() for that.\n";
    body += "    /// </summary>\n";
    body += "    public ValidationResult ValidateAll()\n";
    body += "    {\n";
    body += "        var result = new ValidationResult();\n";
    for entry in all_structs {
        let s = entry.struct;
        body += "        result.Merge(" + s.name + "s.Validate());\n";
    }
    body += "        return result;\n";
    body += "    }\n";

    body += "}\n";

    body
}
