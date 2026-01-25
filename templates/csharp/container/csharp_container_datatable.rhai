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

    // Check for auto_create fields
    let auto_create_field = ();
    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            if f.auto_create != () {
                auto_create_field = f;
            }
        }
    }

    // Generate Add method
    body += "    /// <summary>\n";
    body += "    /// Adds a row to the table.\n";
    if auto_create_field != () {
        body += "    /// Sets the auto_create timestamp field automatically.\n";
    }
    body += "    /// </summary>\n";
    body += "    public void Add(" + s.name + " row)\n";
    body += "    {\n";

    // Set auto_create field if present
    if auto_create_field != () {
        let datetime_expr = type_utils::get_datetime_expr_for_timezone(auto_create_field.auto_create);
        body += "        row." + auto_create_field.name + " = " + datetime_expr + ";\n";
    }

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

    body += "    }\n\n";

    // Generate Validate() method for field constraints
    body += "    /// <summary>\n";
    body += "    /// Validates all rows against field constraints (MaxLength, Range, Regex).\n";
    body += "    /// </summary>\n";
    body += "    public ValidationResult Validate()\n";
    body += "    {\n";
    body += "        var result = new ValidationResult();\n";
    body += "        foreach (var row in _rows)\n";
    body += "        {\n";

    // Find primary key field for error reporting
    let pk_field = "";
    for item in s.items {
        if item.is_field() {
            let field = item.as_field();
            if field.is_primary_key {
                pk_field = field.name;
            }
        }
    }

    let key_expr = if pk_field != "" { "row." + pk_field } else { "null" };
    let has_validations = false;

    for item in s.items {
        if item.is_field() {
            let field = item.as_field();
            let ft = field.field_type;

            // MaxLength validation
            if field.has_max_length {
                has_validations = true;
                let max_len = field.max_length;
                if ft.is_option {
                    body += "            if (row." + field.name + " != null && row." + field.name + ".Length > " + max_len + ")\n";
                } else {
                    body += "            if (row." + field.name + " != null && row." + field.name + ".Length > " + max_len + ")\n";
                }
                body += "                result.AddError(ValidationHelpers.MaxLengthError(\"" + s.name + "\", \"" + field.name + "\", " + key_expr + ", " + max_len + ", row." + field.name + ".Length));\n";
            }

            // Range validation
            if field.has_range {
                has_validations = true;
                let range = field.range;
                if ft.is_option {
                    body += "            if (row." + field.name + ".HasValue && !ValidationHelpers.ValidateRange(row." + field.name + ".Value, " + range.min + ", " + range.max + "))\n";
                } else {
                    body += "            if (!ValidationHelpers.ValidateRange(row." + field.name + ", " + range.min + ", " + range.max + "))\n";
                }
                body += "                result.AddError(ValidationHelpers.RangeError(\"" + s.name + "\", \"" + field.name + "\", " + key_expr + ", " + range.min + ", " + range.max + ", row." + field.name + "));\n";
            }

            // Regex validation
            if field.has_regex_pattern {
                has_validations = true;
                let pattern = field.regex_pattern;
                // Escape quotes in pattern for C# string
                let escaped_pattern = pattern;
                if ft.is_option {
                    body += "            if (row." + field.name + " != null && !ValidationHelpers.ValidateRegex(row." + field.name + ", @\"" + escaped_pattern + "\"))\n";
                } else {
                    body += "            if (!ValidationHelpers.ValidateRegex(row." + field.name + ", @\"" + escaped_pattern + "\"))\n";
                }
                body += "                result.AddError(ValidationHelpers.RegexError(\"" + s.name + "\", \"" + field.name + "\", " + key_expr + ", @\"" + escaped_pattern + "\", row." + field.name + "));\n";
            }
        }
    }

    if !has_validations {
        body += "            // No field constraints to validate\n";
        body += "            _ = row; // Suppress unused variable warning\n";
    }

    body += "        }\n";
    body += "        return result;\n";
    body += "    }\n\n";

    // Generate ValidateForeignKeys() method
    body += "    /// <summary>\n";
    body += "    /// Validates foreign key references exist in related tables.\n";
    body += "    /// </summary>\n";
    body += "    public ValidationResult ValidateForeignKeys(IDataContainer container)\n";
    body += "    {\n";
    body += "        var result = new ValidationResult();\n";

    // Check if any FK fields exist
    let has_fk = false;
    for item in s.items {
        if item.is_field() {
            let field = item.as_field();
            if field.has_foreign_key {
                has_fk = true;
            }
        }
    }

    if !has_fk {
        body += "        // No foreign key constraints to validate\n";
        body += "        _ = container; // Suppress unused variable warning\n";
    } else {
        body += "        foreach (var row in _rows)\n";
        body += "        {\n";

        for item in s.items {
            if item.is_field() {
                let field = item.as_field();
                if field.has_foreign_key {
                    let fk = field.foreign_key;
                    // Get target table name from FQN
                    let target_parts = fk.target_table_fqn.split(".");
                    let target_table = "";
                    for part in target_parts {
                        target_table = part;
                    }

                    if field.field_type.is_option {
                        body += "            if (row." + field.name + ".HasValue)\n";
                        body += "            {\n";
                        body += "                // Check FK reference to " + target_table + "\n";
                        body += "                // Note: Container must implement IHas" + target_table + "Table for FK validation\n";
                        body += "                if (container is IHas" + target_table + "Table fkContainer" + field.name + ")\n";
                        body += "                {\n";
                        body += "                    if (fkContainer" + field.name + "." + target_table + "s.ById[row." + field.name + ".Value] == null)\n";
                        body += "                        result.AddError(ValidationHelpers.ForeignKeyError(\"" + s.name + "\", \"" + field.name + "\", " + key_expr + ", \"" + target_table + "\", row." + field.name + ".Value));\n";
                        body += "                }\n";
                        body += "            }\n";
                    } else {
                        body += "            // Check FK reference to " + target_table + "\n";
                        body += "            // Note: Container must implement IHas" + target_table + "Table for FK validation\n";
                        body += "            if (container is IHas" + target_table + "Table fkContainer" + field.name + ")\n";
                        body += "            {\n";
                        body += "                if (fkContainer" + field.name + "." + target_table + "s.ById[row." + field.name + "] == null)\n";
                        body += "                    result.AddError(ValidationHelpers.ForeignKeyError(\"" + s.name + "\", \"" + field.name + "\", " + key_expr + ", \"" + target_table + "\", row." + field.name + "));\n";
                        body += "            }\n";
                    }
                }
            }
        }

        body += "        }\n";
    }

    body += "        return result;\n";
    body += "    }\n";

    body
}
}
