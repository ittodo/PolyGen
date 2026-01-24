/// <summary>
/// Data loader for populating the ${container_name}DataContainer from CSV/JSON files.
/// Supports pattern-based loading with wildcards (e.g., "data/*.csv", "data/players/").
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
                // Extract @load annotation patterns
                let csv_pattern = "";
                let json_pattern = "";
                let s = item.as_struct();
                for header_item in s.header {
                    if header_item.is_annotation() {
                        let ann = header_item.as_annotation();
                        if ann.name == "load" {
                            for param in ann.params {
                                if param.key == "csv" {
                                    csv_pattern = param.value;
                                } else if param.key == "json" {
                                    json_pattern = param.value;
                                }
                            }
                        }
                    }
                }

                // Find primary key field for duplicate detection
                let pk_field = "";
                for item in s.items {
                    if item.is_field() {
                        let field = item.as_field();
                        if field.is_primary_key {
                            pk_field = field.name;
                        }
                    }
                }

                all_structs.push(#{
                    struct: s,
                    namespace: ns_name,
                    csv_pattern: csv_pattern,
                    json_pattern: json_pattern,
                    pk_field: pk_field
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
        let csv_pattern = entry.csv_pattern;
        let json_pattern = entry.json_pattern;
        let pk_field = entry.pk_field;

        // CSV loader using generated mappers (single file)
        body += "    /// <summary>\n";
        body += "    /// Loads " + s.name + " data from a CSV file.\n";
        body += "    /// </summary>\n";
        body += "    public static void Load" + s.name + "sFromCsv(" + container_name + "DataContainer container, string filePath, char separator = ',', CsvUtils.GapMode gapMode = CsvUtils.GapMode.Break)\n";
        body += "    {\n";
        body += "        foreach (var item in Csv." + ns + "." + s.name + ".ReadRowsWithHeader(filePath, separator, gapMode))\n";
        body += "        {\n";
        body += "            container." + s.name + "s.Add(item);\n";
        body += "        }\n";
        body += "    }\n\n";

        // CSV pattern loader (supports wildcards and directories)
        body += "    /// <summary>\n";
        body += "    /// Loads " + s.name + " data from CSV files matching a pattern.\n";
        body += "    /// Supports: single file, wildcards (*.csv), directories (data/).\n";
        body += "    /// Files are sorted alphabetically and merged sequentially.\n";
        body += "    /// </summary>\n";
        body += "    /// <exception cref=\"DuplicateKeyException\">Thrown when duplicate primary keys are found across files.</exception>\n";
        body += "    public static void Load" + s.name + "sFromCsvPattern(" + container_name + "DataContainer container, string pattern, char separator = ',', CsvUtils.GapMode gapMode = CsvUtils.GapMode.Break)\n";
        body += "    {\n";
        body += "        var files = PatternLoader.ResolvePattern(container.RootDirectory, pattern, \".csv\");\n";

        if pk_field != "" {
            body += "        var seenKeys = new HashSet<object>();\n";
            body += "        string? firstFile = null;\n";
        }

        body += "        foreach (var file in files)\n";
        body += "        {\n";
        body += "            foreach (var item in Csv." + ns + "." + s.name + ".ReadRowsWithHeader(file, separator, gapMode))\n";
        body += "            {\n";

        if pk_field != "" {
            body += "                var key = item." + pk_field + ";\n";
            body += "                if (seenKeys.Contains(key))\n";
            body += "                {\n";
            body += "                    throw new DuplicateKeyException(key?.ToString() ?? \"(null)\", firstFile ?? file, file);\n";
            body += "                }\n";
            body += "                seenKeys.Add(key);\n";
            body += "                if (firstFile == null) firstFile = file;\n";
        }

        body += "                container." + s.name + "s.Add(item);\n";
        body += "            }\n";
        body += "        }\n";
        body += "    }\n\n";

        // JSON loader using System.Text.Json (single file)
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

        // JSON pattern loader (supports wildcards and directories)
        body += "    /// <summary>\n";
        body += "    /// Loads " + s.name + " data from JSON files matching a pattern.\n";
        body += "    /// </summary>\n";
        body += "    /// <exception cref=\"DuplicateKeyException\">Thrown when duplicate primary keys are found across files.</exception>\n";
        body += "    public static void Load" + s.name + "sFromJsonPattern(" + container_name + "DataContainer container, string pattern)\n";
        body += "    {\n";
        body += "        var files = PatternLoader.ResolvePattern(container.RootDirectory, pattern, \".json\");\n";

        if pk_field != "" {
            body += "        var seenKeys = new HashSet<object>();\n";
            body += "        string? firstFile = null;\n";
        }

        body += "        foreach (var file in files)\n";
        body += "        {\n";
        body += "            var json = File.ReadAllText(file);\n";
        body += "            var items = System.Text.Json.JsonSerializer.Deserialize<List<global::" + ns + "." + s.name + ">>(json);\n";
        body += "            if (items != null)\n";
        body += "            {\n";
        body += "                foreach (var item in items)\n";
        body += "                {\n";

        if pk_field != "" {
            body += "                    var key = item." + pk_field + ";\n";
            body += "                    if (seenKeys.Contains(key))\n";
            body += "                    {\n";
            body += "                        throw new DuplicateKeyException(key?.ToString() ?? \"(null)\", firstFile ?? file, file);\n";
            body += "                    }\n";
            body += "                    seenKeys.Add(key);\n";
            body += "                    if (firstFile == null) firstFile = file;\n";
        }

        body += "                    container." + s.name + "s.Add(item);\n";
        body += "                }\n";
        body += "            }\n";
        body += "        }\n";
        body += "    }\n\n";
    }

    // Generate LoadAll method using @load annotation patterns
    body += "    /// <summary>\n";
    body += "    /// Loads all tables using patterns from @load annotations.\n";
    body += "    /// Files are resolved relative to container.RootDirectory.\n";
    body += "    /// </summary>\n";
    body += "    public static void LoadAll(" + container_name + "DataContainer container, char separator = ',', CsvUtils.GapMode gapMode = CsvUtils.GapMode.Break)\n";
    body += "    {\n";

    for entry in all_structs {
        let s = entry.struct;
        let csv_pattern = entry.csv_pattern;
        let json_pattern = entry.json_pattern;

        if csv_pattern != "" {
            body += "        // " + s.name + ": @load(csv: \"" + csv_pattern + "\")\n";
            body += "        Load" + s.name + "sFromCsvPattern(container, \"" + csv_pattern + "\", separator, gapMode);\n\n";
        }
        if json_pattern != "" {
            body += "        // " + s.name + ": @load(json: \"" + json_pattern + "\")\n";
            body += "        Load" + s.name + "sFromJsonPattern(container, \"" + json_pattern + "\");\n\n";
        }
    }

    body += "    }\n\n";

    // Generate LoadAll with validation option
    body += "    /// <summary>\n";
    body += "    /// Loads all tables using patterns from @load annotations and optionally validates.\n";
    body += "    /// </summary>\n";
    body += "    /// <param name=\"container\">The container to load data into.</param>\n";
    body += "    /// <param name=\"validate\">If true, validates data after loading.</param>\n";
    body += "    /// <param name=\"separator\">CSV field separator character.</param>\n";
    body += "    /// <param name=\"gapMode\">How to handle gaps in CSV data.</param>\n";
    body += "    /// <returns>A ValidationResult containing any validation errors (empty if validate=false).</returns>\n";
    body += "    public static ValidationResult LoadAll(" + container_name + "DataContainer container, bool validate, char separator = ',', CsvUtils.GapMode gapMode = CsvUtils.GapMode.Break)\n";
    body += "    {\n";
    body += "        LoadAll(container, separator, gapMode);\n";
    body += "        return validate ? container.ValidateAll() : new ValidationResult();\n";
    body += "    }\n\n";

    // Generate LoadAllAndValidate method
    body += "    /// <summary>\n";
    body += "    /// Loads all tables and validates data. Throws exception if validation fails.\n";
    body += "    /// </summary>\n";
    body += "    /// <exception cref=\"ValidationException\">Thrown when validation fails after loading.</exception>\n";
    body += "    public static void LoadAllAndValidate(" + container_name + "DataContainer container, char separator = ',', CsvUtils.GapMode gapMode = CsvUtils.GapMode.Break)\n";
    body += "    {\n";
    body += "        LoadAll(container, separator, gapMode);\n";
    body += "        container.ValidateOrThrow();\n";
    body += "    }\n\n";

    // Generate a convenience method to load all tables from a directory (legacy support)
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
