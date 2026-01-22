// C++ Container File Template for PolyGen
// Generates data containers with indexes
// NOTE: Rhai passes arrays by value, so we must collect structs inline

import "templates/cpp/rhai_utils/type_mapping" as cpp_types;

// Check if struct has indexes defined
fn has_indexes(s) {
    if s.indexes != () && s.indexes.len() > 0 {
        return true;
    }
    return false;
}

// Get the fully qualified C++ type name
fn get_full_cpp_type(namespace_fqn, type_name) {
    if namespace_fqn == "" {
        return type_name;
    }
    // Rhai's replace() mutates in-place and returns (), so we must copy first
    let ns_cpp = namespace_fqn + "";
    ns_cpp.replace(".", "::");
    return "::" + ns_cpp + "::" + type_name;
}

// Pluralize a name (simple version)
fn pluralize(name) {
    if name.ends_with("y") {
        return name.sub_string(0, name.len() - 1) + "ies";
    } else if name.ends_with("s") || name.ends_with("x") || name.ends_with("ch") || name.ends_with("sh") {
        return name + "es";
    } else {
        return name + "s";
    }
}

// Get C++ type for a single field type
fn get_field_cpp_type(t) {
    if t.is_primitive || cpp_types::is_primitive_name(t.type_name) {
        return cpp_types::map_primitive_type(t.type_name);
    } else {
        return t.type_name;
    }
}

// Get C++ type for index key (supports composite indexes)
fn get_index_key_type(index) {
    if index.is_composite {
        // Composite index: use std::tuple
        let types = [];
        for f in index.fields {
            types.push(get_field_cpp_type(f.field_type));
        }
        let result = "std::tuple<";
        for i in 0..types.len() {
            if i > 0 { result += ", "; }
            result += types[i];
        }
        result += ">";
        return result;
    } else {
        // Single field index
        let t = index.field_type;
        return get_field_cpp_type(t);
    }
}

// Get field accessor expression for index insert
fn get_index_key_expr(index, row_var) {
    if index.is_composite {
        // Composite: std::make_tuple(row.field1, row.field2, ...)
        let result = "std::make_tuple(";
        for i in 0..index.fields.len() {
            if i > 0 { result += ", "; }
            result += row_var + "." + index.fields[i].name;
        }
        result += ")";
        return result;
    } else {
        // Single field
        return row_var + "." + index.field_name;
    }
}

// Generate a table wrapper with indexes
fn generate_table_wrapper(s, namespace_fqn) {
    let result = "";
    let full_type = get_full_cpp_type(namespace_fqn, s.name);
    let table_name = s.name + "Table";
    let member_name = cpp_types::to_snake_case(pluralize(s.name));

    result += "// Table wrapper for " + s.name + " with indexes\n";
    result += "class " + table_name + " {\n";
    result += "public:\n";

    // Data storage
    result += "    polygen::DataTable<" + full_type + "> data;\n\n";

    // Index members
    if s.indexes != () {
        for idx in s.indexes {
            let key_type = get_index_key_type(idx);
            let index_member = idx.name;  // Use index name directly
            if idx.is_unique {
                result += "    polygen::UniqueIndex<" + key_type + ", " + full_type + "> " + index_member + ";\n";
            } else {
                result += "    polygen::GroupIndex<" + key_type + ", " + full_type + "> " + index_member + ";\n";
            }
        }
    }

    result += "\n";

    // add_row method
    result += "    void add_row(" + full_type + "&& row) {\n";
    result += "        data.add_row(std::move(row));\n";
    result += "        auto& added = data[data.count() - 1];\n";

    if s.indexes != () {
        for idx in s.indexes {
            let key_expr = get_index_key_expr(idx, "added");
            result += "        " + idx.name + ".insert(" + key_expr + ", &added);\n";
        }
    }

    result += "    }\n\n";

    result += "    void add_row(const " + full_type + "& row) {\n";
    result += "        add_row(" + full_type + "(row));\n";
    result += "    }\n\n";

    // Accessor methods
    result += "    size_t count() const { return data.count(); }\n";
    result += "    bool empty() const { return data.empty(); }\n\n";

    // get_by_X methods
    if s.indexes != () {
        for idx in s.indexes {
            let key_type = get_index_key_type(idx);
            // For composite indexes, use index name; for single use get_by_fieldname
            let getter_name = if idx.is_composite {
                "get_" + idx.name
            } else {
                "get_by_" + idx.field_name
            };

            if idx.is_unique {
                result += "    " + full_type + "* " + getter_name + "(const " + key_type + "& key) {\n";
                result += "        return " + idx.name + ".get(key);\n";
                result += "    }\n\n";
                result += "    const " + full_type + "* " + getter_name + "(const " + key_type + "& key) const {\n";
                result += "        return const_cast<" + table_name + "*>(this)->" + idx.name + ".get(key);\n";
                result += "    }\n\n";
            } else {
                result += "    const std::vector<" + full_type + "*>& " + getter_name + "(const " + key_type + "& key) const {\n";
                result += "        return " + idx.name + ".get(key);\n";
                result += "    }\n\n";
            }
        }
    }

    // Iterator support
    result += "    auto begin() { return data.begin(); }\n";
    result += "    auto end() { return data.end(); }\n";
    result += "    auto begin() const { return data.begin(); }\n";
    result += "    auto end() const { return data.end(); }\n\n";

    // Clear method
    result += "    void clear() {\n";
    result += "        data.clear();\n";
    if s.indexes != () {
        for idx in s.indexes {
            result += "        " + idx.name + ".clear();\n";
        }
    }
    result += "    }\n";

    result += "};\n\n";

    return result;
}

// Convert snake_case or kebab-case to PascalCase
fn to_pascal_case(name) {
    let result = "";
    let capitalize_next = true;

    for i in 0..name.len() {
        let c = name.sub_string(i, 1);
        if c == "_" || c == "-" {
            capitalize_next = true;
        } else if capitalize_next {
            result += c.to_upper();
            capitalize_next = false;
        } else {
            result += c;
        }
    }

    return result;
}

// Generate the main container class
fn generate_container(base_name, tables) {
    let result = "";
    let container_name = to_pascal_case(base_name) + "Container";

    result += "// Main data container with all tables\n";
    result += "class " + container_name + " {\n";
    result += "public:\n";

    // Table members
    for entry in tables {
        let s = entry.struct_def;
        let member_name = cpp_types::to_snake_case(pluralize(s.name));
        result += "    " + s.name + "Table " + member_name + ";\n";
    }

    result += "\n";

    // Clear all method
    result += "    void clear() {\n";
    for entry in tables {
        let s = entry.struct_def;
        let member_name = cpp_types::to_snake_case(pluralize(s.name));
        result += "        " + member_name + ".clear();\n";
    }
    result += "    }\n";

    result += "};\n\n";

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
    let base_name = filename;
    base_name.replace(".poly", "");

    // Collect all tables (structs with indexes) from all namespaces in this file
    // Rhai passes arrays by value, so helper functions won't work - must inline
    let tables = [];
    for ns in file.namespaces {
        for item in ns.items {
            if item.is_struct() {
                let s = item.as_struct();
                if has_indexes(s) {
                    tables.push(#{ struct_def: s, namespace_name: ns.name });
                }
            } else if item.is_namespace() {
                // Handle nested namespaces
                let child_ns = item.as_namespace();
                for child_item in child_ns.items {
                    if child_item.is_struct() {
                        let cs = child_item.as_struct();
                        if has_indexes(cs) {
                            tables.push(#{ struct_def: cs, namespace_name: child_ns.name });
                        }
                    }
                }
            }
        }
    }

    if tables.is_empty() {
        continue;
    }

    let output_filename = base_name + "_container.hpp";
    let final_path = output_dir + "/cpp/" + output_filename;

    let result = "";

    // File header
    result += "// Auto-generated by PolyGen - DO NOT EDIT\n";
    result += "// Data container with indexed tables\n";
    result += "#pragma once\n\n";

    // Includes
    result += "#include \"" + base_name + ".hpp\"\n";
    result += "#include \"" + base_name + "_loaders.hpp\"\n";
    result += "#include \"polygen_support.hpp\"\n";
    result += "#include <tuple>\n\n";

    // Open container namespace
    // Rhai's replace() returns (), so copy first
    let container_ns = base_name + "";
    container_ns.replace("-", "_");
    container_ns += "_container";
    result += "namespace " + container_ns + " {\n\n";

    // Generate table wrappers
    for entry in tables {
        let s = entry.struct_def;
        let ns = entry.namespace_name;
        result += generate_table_wrapper(s, ns);
    }

    // Generate main container
    result += generate_container(base_name, tables);

    // Close container namespace
    result += "} // namespace " + container_ns + "\n";

    print("Generating container file: " + final_path);
    write_file(final_path, result);
}

""
