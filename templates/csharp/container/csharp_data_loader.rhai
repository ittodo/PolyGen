/// <summary>
/// Data loader for populating the ${container_name}DataContainer from CSV/JSON files.
/// </summary>
public static class ${container_name}DataLoader
{
${
    let body = "";

    // Collect all structs from all namespaces (including nested)
    let all_structs = [];
    let ns_stack = [];
    for ns in file.namespaces {
        ns_stack.push(#{ns: ns, ns_name: ns.name});
    }
    while ns_stack.len() > 0 {
        let entry = ns_stack.pop();
        let ns = entry.ns;
        let ns_name = entry.ns_name;
        for item in ns.items {
            if item.is_struct() {
                all_structs.push(#{
                    struct: item.as_struct(),
                    namespace: ns_name
                });
            } else if item.is_namespace() {
                let nested = item.as_namespace();
                ns_stack.push(#{ns: nested, ns_name: nested.name});
            }
        }
    }

    // Generate Load methods for each struct
    for entry in all_structs {
        let s = entry.struct;
        let ns = entry.namespace;

        // CSV loader using generated mappers
        body += "    /// <summary>\n";
        body += "    /// Loads " + s.name + " data from a CSV file using the generated CSV mapper.\n";
        body += "    /// </summary>\n";
        body += "    public static void Load" + s.name + "sFromCsv(" + container_name + "DataContainer container, string filePath, char separator = ',', CsvUtils.GapMode gapMode = CsvUtils.GapMode.Break)\n";
        body += "    {\n";
        body += "        foreach (var item in Csv." + ns + "." + s.name + ".ReadRowsWithHeader(filePath, separator, gapMode))\n";
        body += "        {\n";
        body += "            container." + s.name + "s.Add(item);\n";
        body += "        }\n";
        body += "    }\n\n";

        // JSON loader using System.Text.Json
        body += "    /// <summary>\n";
        body += "    /// Loads " + s.name + " data from a JSON file.\n";
        body += "    /// </summary>\n";
        body += "    public static void Load" + s.name + "sFromJson(" + container_name + "DataContainer container, string filePath)\n";
        body += "    {\n";
        body += "        var json = File.ReadAllText(filePath);\n";
        body += "        var items = System.Text.Json.JsonSerializer.Deserialize<List<global::" + ns + "." + s.name + ">>(json);\n";
        body += "        if (items != null)\n";
        body += "        {\n";
        body += "            foreach (var item in items)\n";
        body += "            {\n";
        body += "                container." + s.name + "s.Add(item);\n";
        body += "            }\n";
        body += "        }\n";
        body += "    }\n\n";
    }

    // Generate a convenience method to load all tables from a directory
    body += "    /// <summary>\n";
    body += "    /// Loads all tables from CSV files in a directory.\n";
    body += "    /// Files should be named: {TableName}.csv\n";
    body += "    /// </summary>\n";
    body += "    public static void LoadAllFromCsvDirectory(" + container_name + "DataContainer container, string directoryPath, char separator = ',', CsvUtils.GapMode gapMode = CsvUtils.GapMode.Break)\n";
    body += "    {\n";

    for entry in all_structs {
        let s = entry.struct;
        body += "        var " + s.name.to_lower() + "Path = Path.Combine(directoryPath, \"" + s.name + ".csv\");\n";
        body += "        if (File.Exists(" + s.name.to_lower() + "Path))\n";
        body += "            Load" + s.name + "sFromCsv(container, " + s.name.to_lower() + "Path, separator, gapMode);\n\n";
    }

    body += "    }\n\n";

    // Generate a convenience method to load all tables from JSON files in a directory
    body += "    /// <summary>\n";
    body += "    /// Loads all tables from JSON files in a directory.\n";
    body += "    /// Files should be named: {TableName}.json\n";
    body += "    /// </summary>\n";
    body += "    public static void LoadAllFromJsonDirectory(" + container_name + "DataContainer container, string directoryPath)\n";
    body += "    {\n";

    for entry in all_structs {
        let s = entry.struct;
        body += "        var " + s.name.to_lower() + "Path = Path.Combine(directoryPath, \"" + s.name + ".json\");\n";
        body += "        if (File.Exists(" + s.name.to_lower() + "Path))\n";
        body += "            Load" + s.name + "sFromJson(container, " + s.name.to_lower() + "Path);\n\n";
    }

    body += "    }\n";

    body
}
}
