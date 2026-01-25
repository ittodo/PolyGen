${
    import "templates/rhai_utils/indent" as indent_utils;

    // Collect all foreign keys that cross datasource boundaries
    // We need to find FKs where source datasource != target datasource
    let cross_container_fks = [];

    // Build a map of FQN -> datasource for all structs
    let fqn_to_datasource = #{};
    for ds_name in ds_keys {
        let entries = datasource_map[ds_name];
        for entry in entries {
            for s in entry.structs {
                fqn_to_datasource[s.fqn] = ds_name;
            }
        }
    }

    // Find cross-container foreign keys
    for ds_name in ds_keys {
        let entries = datasource_map[ds_name];
        for entry in entries {
            for s in entry.structs {
                for item in s.items {
                    if item.is_field() {
                        let f = item.as_field();
                        if f.foreign_key != () {
                            let fk = f.foreign_key;
                            // Extract target table FQN
                            let target_fqn = fk.target_table_fqn;
                            let target_field = fk.target_field;

                            // Check if target is in a different datasource
                            let target_ds = fqn_to_datasource[target_fqn];
                            if target_ds != () && target_ds != ds_name {
                                // This is a cross-container FK
                                cross_container_fks.push(#{
                                    source_ds: ds_name,
                                    source_struct: s,
                                    source_field: f,
                                    target_ds: target_ds,
                                    target_fqn: target_fqn,
                                    target_field: target_field
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    let body = "";

    // DataContext class
    body += "/// <summary>\n";
    body += "/// Root data context that holds all datasource containers.\n";
    body += "/// Provides cross-container foreign key validation.\n";
    body += "/// </summary>\n";
    body += "public class DataContext\n";
    body += "{\n";

    // Container properties for each datasource
    for ds_name in ds_keys {
        // Convert datasource name to PascalCase
        let ds_parts = ds_name.split("_");
        let prop_name = "";
        for part in ds_parts {
            if part.len() > 0 {
                let first = part.sub_string(0, 1);
                first = first.to_upper();
                let rest = part.sub_string(1);
                prop_name += first + rest;
            }
        }
        let container_type = prop_name + "Container";

        body += "    /// <summary>Container for '" + ds_name + "' datasource.</summary>\n";
        body += "    public " + container_type + " " + prop_name + " { get; } = new();\n";
    }

    body += "\n";

    // Constructor
    body += "    public DataContext()\n";
    body += "    {\n";
    body += "    }\n\n";

    // Constructor with root directory
    body += "    public DataContext(string rootDirectory)\n";
    body += "    {\n";
    for ds_name in ds_keys {
        let ds_parts = ds_name.split("_");
        let prop_name = "";
        for part in ds_parts {
            if part.len() > 0 {
                let first = part.sub_string(0, 1);
                first = first.to_upper();
                let rest = part.sub_string(1);
                prop_name += first + rest;
            }
        }
        body += "        " + prop_name + ".RootDirectory = rootDirectory;\n";
    }
    body += "    }\n\n";

    // SetRootDirectory method
    body += "    /// <summary>\n";
    body += "    /// Sets the root directory for all containers.\n";
    body += "    /// </summary>\n";
    body += "    public void SetRootDirectory(string rootDirectory)\n";
    body += "    {\n";
    for ds_name in ds_keys {
        let ds_parts = ds_name.split("_");
        let prop_name = "";
        for part in ds_parts {
            if part.len() > 0 {
                let first = part.sub_string(0, 1);
                first = first.to_upper();
                let rest = part.sub_string(1);
                prop_name += first + rest;
            }
        }
        body += "        " + prop_name + ".RootDirectory = rootDirectory;\n";
    }
    body += "    }\n\n";

    // Clear method
    body += "    /// <summary>\n";
    body += "    /// Clears all data from all containers.\n";
    body += "    /// </summary>\n";
    body += "    public void Clear()\n";
    body += "    {\n";
    for ds_name in ds_keys {
        let ds_parts = ds_name.split("_");
        let prop_name = "";
        for part in ds_parts {
            if part.len() > 0 {
                let first = part.sub_string(0, 1);
                first = first.to_upper();
                let rest = part.sub_string(1);
                prop_name += first + rest;
            }
        }
        body += "        " + prop_name + ".Clear();\n";
    }
    body += "    }\n\n";

    // ValidateAll method
    body += "    /// <summary>\n";
    body += "    /// Validates all tables in all containers, including cross-container foreign keys.\n";
    body += "    /// </summary>\n";
    body += "    /// <returns>A ValidationResult containing any validation errors.</returns>\n";
    body += "    public ValidationResult ValidateAll()\n";
    body += "    {\n";
    body += "        var result = new ValidationResult();\n\n";

    body += "        // Validate each container internally\n";
    for ds_name in ds_keys {
        let ds_parts = ds_name.split("_");
        let prop_name = "";
        for part in ds_parts {
            if part.len() > 0 {
                let first = part.sub_string(0, 1);
                first = first.to_upper();
                let rest = part.sub_string(1);
                prop_name += first + rest;
            }
        }
        body += "        result.Merge(" + prop_name + ".ValidateAll());\n";
    }

    body += "\n        // Validate cross-container foreign keys\n";
    body += "        result.Merge(ValidateCrossContainerReferences());\n\n";
    body += "        return result;\n";
    body += "    }\n\n";

    // ValidateOrThrow method
    body += "    /// <summary>\n";
    body += "    /// Validates all tables and throws an exception if any errors are found.\n";
    body += "    /// </summary>\n";
    body += "    /// <exception cref=\"ValidationException\">Thrown when validation fails.</exception>\n";
    body += "    public void ValidateOrThrow()\n";
    body += "    {\n";
    body += "        var result = ValidateAll();\n";
    body += "        if (!result.IsValid)\n";
    body += "            throw new ValidationException(result);\n";
    body += "    }\n\n";

    // ValidateCrossContainerReferences method
    body += "    /// <summary>\n";
    body += "    /// Validates foreign key references that cross container boundaries.\n";
    body += "    /// </summary>\n";
    body += "    private ValidationResult ValidateCrossContainerReferences()\n";
    body += "    {\n";
    body += "        var result = new ValidationResult();\n\n";

    if cross_container_fks.len() == 0 {
        body += "        // No cross-container foreign keys defined\n";
    } else {
        for fk_info in cross_container_fks {
            let source_ds = fk_info.source_ds;
            let source_struct = fk_info.source_struct;
            let source_field = fk_info.source_field;
            let target_ds = fk_info.target_ds;
            let target_fqn = fk_info.target_fqn;
            let target_field = fk_info.target_field;

            // Get source container property name
            let ds_parts = source_ds.split("_");
            let source_prop = "";
            for part in ds_parts {
                if part.len() > 0 {
                    let first = part.sub_string(0, 1);
                    first = first.to_upper();
                    let rest = part.sub_string(1);
                    source_prop += first + rest;
                }
            }

            // Get target container property name
            let ds_parts = target_ds.split("_");
            let target_prop = "";
            for part in ds_parts {
                if part.len() > 0 {
                    let first = part.sub_string(0, 1);
                    first = first.to_upper();
                    let rest = part.sub_string(1);
                    target_prop += first + rest;
                }
            }

            // Extract target struct name from FQN
            let target_struct_name = target_fqn;
            if target_fqn.contains(".") {
                let parts = target_fqn.split(".");
                target_struct_name = parts[parts.len() - 1];
            }

            // Convert target_field to PascalCase for method name
            let target_field_pascal = "";
            let tf_parts = target_field.split("_");
            for part in tf_parts {
                if part.len() > 0 {
                    let first = part.sub_string(0, 1);
                    first = first.to_upper();
                    let rest = part.sub_string(1);
                    target_field_pascal += first + rest;
                }
            }

            // Find primary key field of source struct for error message
            let pk_field = "Id";
            for item in source_struct.items {
                if item.is_field() {
                    let f = item.as_field();
                    if f.is_primary_key {
                        pk_field = f.name;
                    }
                }
            }

            // Check if field is optional
            let is_optional = source_field.field_type.is_option;

            body += "        // " + source_struct.name + "." + source_field.name + " -> " + target_fqn + "." + target_field + "\n";
            body += "        foreach (var item in " + source_prop + "." + source_struct.name + "s)\n";
            body += "        {\n";

            if is_optional {
                body += "            if (item." + source_field.name + ".HasValue)\n";
                body += "            {\n";
                body += "                if (!" + target_prop + "." + target_struct_name + "s.TryGetBy" + target_field_pascal + "(item." + source_field.name + ".Value, out _))\n";
                body += "                {\n";
                body += "                    result.AddError(\n";
                body += "                        \"" + source_struct.name + "\",\n";
                body += "                        item." + pk_field + "?.ToString() ?? \"(null)\",\n";
                body += "                        \"" + source_field.name + "\",\n";
                body += "                        $\"Cross-container FK reference not found in " + target_ds + "." + target_struct_name + ": {item." + source_field.name + ".Value}\");\n";
                body += "                }\n";
                body += "            }\n";
            } else {
                body += "            if (!" + target_prop + "." + target_struct_name + "s.TryGetBy" + target_field_pascal + "(item." + source_field.name + ", out _))\n";
                body += "            {\n";
                body += "                result.AddError(\n";
                body += "                    \"" + source_struct.name + "\",\n";
                body += "                    item." + pk_field + "?.ToString() ?? \"(null)\",\n";
                body += "                    \"" + source_field.name + "\",\n";
                body += "                    $\"Cross-container FK reference not found in " + target_ds + "." + target_struct_name + ": {item." + source_field.name + "}\");\n";
                body += "            }\n";
            }
            body += "        }\n\n";
        }
    }

    body += "        return result;\n";
    body += "    }\n";

    body += "}\n";

    body
}
