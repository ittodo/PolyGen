// Unreal Loaders File Template for PolyGen
// Generates DataTable row structures and CSV/JSON loading utilities

import "templates/unreal/rhai_utils/type_mapping" as ue_types;

// Generate CSV parsing helper for a field type
fn csv_parse_expr(type_ref, column_expr) {
    let n = type_ref.type_name;
    if n == "string" { return column_expr; }
    if n == "bool" { return column_expr + ".ToBool()"; }
    if n == "u8" || n == "u16" || n == "u32" || n == "i8" || n == "i16" || n == "i32" {
        return "FCString::Atoi(*" + column_expr + ")";
    }
    if n == "u64" || n == "i64" {
        return "FCString::Atoi64(*" + column_expr + ")";
    }
    if n == "f32" || n == "f64" {
        return "FCString::Atof(*" + column_expr + ")";
    }

    // For enums, parse as int and cast
    if type_ref.is_enum {
        let enum_name = ue_types::get_enum_name(type_ref.type_name);
        return "static_cast<" + enum_name + ">(FCString::Atoi(*" + column_expr + "))";
    }

    return column_expr;
}

// Generate JSON parsing for a struct
fn generate_json_loader(s, namespace_fqn) {
    let struct_name = ue_types::get_struct_name(s.name);
    let result = "";

    result += "// JSON loader for " + struct_name + "\n";
    result += "inline bool LoadFromJson(const FString& JsonString, " + struct_name + "& OutStruct)\n";
    result += "{\n";
    result += "    TSharedPtr<FJsonObject> JsonObject;\n";
    result += "    TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(JsonString);\n";
    result += "    \n";
    result += "    if (!FJsonSerializer::Deserialize(Reader, JsonObject) || !JsonObject.IsValid())\n";
    result += "    {\n";
    result += "        return false;\n";
    result += "    }\n";
    result += "    \n";
    result += "    return FJsonObjectConverter::JsonObjectToUStruct(JsonObject.ToSharedRef(), &OutStruct);\n";
    result += "}\n\n";

    result += "// JSON array loader for " + struct_name + "\n";
    result += "inline bool LoadArrayFromJson(const FString& JsonString, TArray<" + struct_name + ">& OutArray)\n";
    result += "{\n";
    result += "    return FJsonObjectConverter::JsonArrayStringToUStruct(JsonString, &OutArray);\n";
    result += "}\n\n";

    return result;
}

// Generate CSV loader for a struct
fn generate_csv_loader(s, namespace_fqn) {
    let struct_name = ue_types::get_struct_name(s.name);
    let result = "";

    result += "// CSV loader for " + struct_name + "\n";
    result += "inline bool LoadFromCsv(const FString& CsvPath, TArray<" + struct_name + ">& OutArray)\n";
    result += "{\n";
    result += "    FString FileContent;\n";
    result += "    if (!FFileHelper::LoadFileToString(FileContent, *CsvPath))\n";
    result += "    {\n";
    result += "        UE_LOG(LogTemp, Error, TEXT(\"Failed to load CSV file: %s\"), *CsvPath);\n";
    result += "        return false;\n";
    result += "    }\n";
    result += "    \n";
    result += "    TArray<FString> Lines;\n";
    result += "    FileContent.ParseIntoArrayLines(Lines);\n";
    result += "    \n";
    result += "    if (Lines.Num() < 2) // Need at least header + 1 row\n";
    result += "    {\n";
    result += "        return true; // Empty but valid\n";
    result += "    }\n";
    result += "    \n";
    result += "    // Parse header to get column indices\n";
    result += "    TArray<FString> Headers;\n";
    result += "    Lines[0].ParseIntoArray(Headers, TEXT(\",\"));\n";
    result += "    \n";

    // Build column index map
    let field_count = 0;
    for item in s.items {
        if item.is_field() {
            field_count += 1;
        }
    }

    result += "    TMap<FString, int32> ColumnMap;\n";
    result += "    for (int32 i = 0; i < Headers.Num(); ++i)\n";
    result += "    {\n";
    result += "        ColumnMap.Add(Headers[i].TrimStartAndEnd(), i);\n";
    result += "    }\n";
    result += "    \n";

    result += "    // Parse data rows\n";
    result += "    for (int32 RowIdx = 1; RowIdx < Lines.Num(); ++RowIdx)\n";
    result += "    {\n";
    result += "        if (Lines[RowIdx].IsEmpty()) continue;\n";
    result += "        \n";
    result += "        TArray<FString> Columns;\n";
    result += "        Lines[RowIdx].ParseIntoArray(Columns, TEXT(\",\"));\n";
    result += "        \n";
    result += "        " + struct_name + " Row;\n";

    // Parse each field
    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            let field_name = f.name;

            // Skip arrays and complex types for CSV
            if !f.field_type.is_list && !f.field_type.is_option {
                result += "        if (const int32* ColIdx = ColumnMap.Find(TEXT(\"" + field_name + "\")))\n";
                result += "        {\n";
                result += "            if (*ColIdx < Columns.Num())\n";
                result += "            {\n";
                result += "                Row." + field_name + " = " + csv_parse_expr(f.field_type, "Columns[*ColIdx]") + ";\n";
                result += "            }\n";
                result += "        }\n";
            }
        }
    }

    result += "        OutArray.Add(Row);\n";
    result += "    }\n";
    result += "    \n";
    result += "    return true;\n";
    result += "}\n\n";

    return result;
}

// Main entry point - runs for each file
for file in schema.files {
    if file.path == () || file.path == "" {
        continue;
    }

    // Extract filename from path
    let parts = file.path.split("/");
    let filename = parts[parts.len - 1];
    let base_name = filename + "";
    base_name.replace(".poly", "");

    // Capitalize for Unreal naming
    let first_char = base_name.sub_string(0, 1);
    let rest = base_name.sub_string(1, base_name.len() - 1);
    let pascal_name = first_char.to_upper() + rest;

    let output_filename = "Polygen" + pascal_name + "Loaders.h";
    let final_path = output_dir + "/unreal/" + output_filename;

    // Collect all structs from all namespaces in this file
    let all_structs = [];
    for ns in file.namespaces {
        for item in ns.items {
            if item.is_struct() {
                let s = item.as_struct();
                all_structs.push(#{ struct_def: s, namespace_name: ns.name });

                // Also collect embedded structs
                for si in s.items {
                    if si.is_embedded_struct() {
                        let es = si.as_embedded_struct();
                        all_structs.push(#{ struct_def: es, namespace_name: ns.name });
                    }
                }
            } else if item.is_namespace() {
                let child_ns = item.as_namespace();
                for child_item in child_ns.items {
                    if child_item.is_struct() {
                        let cs = child_item.as_struct();
                        all_structs.push(#{ struct_def: cs, namespace_name: child_ns.name });
                    }
                }
            }
        }
    }

    if all_structs.is_empty() {
        continue;
    }

    let result = "";

    // File header
    result += "// Generated by PolyGen - DO NOT EDIT\n";
    result += "// Loaders for " + file.path + "\n";
    result += "#pragma once\n\n";

    // Includes
    result += "#include \"Polygen" + pascal_name + ".h\"\n";
    result += "#include \"Misc/FileHelper.h\"\n";
    result += "#include \"JsonObjectConverter.h\"\n";
    result += "#include \"Serialization/JsonReader.h\"\n";
    result += "#include \"Serialization/JsonSerializer.h\"\n";
    result += "\n";

    // Namespace for loader functions
    result += "namespace PolygenLoaders\n";
    result += "{\n\n";

    // Generate JSON loaders
    result += "// ============================================================================\n";
    result += "// JSON Loaders\n";
    result += "// ============================================================================\n\n";

    for entry in all_structs {
        let s = entry.struct_def;
        let ns = entry.namespace_name;
        result += generate_json_loader(s, ns);
    }

    // Generate CSV loaders
    result += "// ============================================================================\n";
    result += "// CSV Loaders\n";
    result += "// ============================================================================\n\n";

    for entry in all_structs {
        let s = entry.struct_def;
        let ns = entry.namespace_name;
        result += generate_csv_loader(s, ns);
    }

    result += "} // namespace PolygenLoaders\n";

    print("Generating loader file: " + final_path);
    write_file(final_path, result);
}

""
