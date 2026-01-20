/// <summary>
/// Root data container for all entities in this schema file.
/// Provides unified access to all tables with their indexes.
/// </summary>
${
    // Generate root container class name from file path
    let file_name = file.path;
    // Remove .poly extension and directory path
    let parts = file_name.split("/");
    let base_name = "";
    for part in parts {
        base_name = part;
    }
    if base_name.ends_with(".poly") {
        base_name = base_name.sub_string(0, base_name.len() - 5);
    }
    // Convert to PascalCase
    let name_parts = base_name.split("_");
    let container_name = "";
    for part in name_parts {
        if part.len() > 0 {
            let first = part.sub_string(0, 1);
            first = first.to_upper();
            let rest = part.sub_string(1);
            container_name += first + rest;
        }
    }

    // Collect all structs from all namespaces (including nested)
    let all_structs = [];
    let ns_stack = [];
    for ns in file.namespaces {
        ns_stack.push(ns);
    }
    while ns_stack.len() > 0 {
        let ns = ns_stack.pop();
        for item in ns.items {
            if item.is_struct() {
                all_structs.push(item.as_struct());
            } else if item.is_namespace() {
                ns_stack.push(item.as_namespace());
            }
        }
    }

    // Build interface list
    let interfaces = "IDataContainer";
    for s in all_structs {
        interfaces += ", IHas" + s.name + "Table";
    }

    "public class " + container_name + "DataContainer : " + interfaces
}
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

    // Generate table properties
    for entry in all_structs {
        let s = entry.struct;
        body += "    public " + s.name + "Table " + s.name + "s { get; } = new();\n";
    }

    if all_structs.len() > 0 {
        body += "\n";
    }

    // Generate constructor - recalculate container name
    let file_name = file.path;
    let parts = file_name.split("/");
    let base_name = "";
    for part in parts {
        base_name = part;
    }
    if base_name.ends_with(".poly") {
        base_name = base_name.sub_string(0, base_name.len() - 5);
    }
    let name_parts = base_name.split("_");
    let container_name = "";
    for part in name_parts {
        if part.len() > 0 {
            let first = part.sub_string(0, 1);
            first = first.to_upper();
            let rest = part.sub_string(1);
            container_name += first + rest;
        }
    }

    body += "    public " + container_name + "DataContainer()\n";
    body += "    {\n";
    for entry in all_structs {
        let s = entry.struct;
        body += "        " + s.name + "s.SetContainer(this);\n";
    }
    body += "    }\n\n";

    // Generate Clear method
    body += "    /// <summary>\n";
    body += "    /// Clears all data from all tables.\n";
    body += "    /// </summary>\n";
    body += "    public void Clear()\n";
    body += "    {\n";
    for entry in all_structs {
        let s = entry.struct;
        body += "        " + s.name + "s.Clear();\n";
    }
    body += "    }\n";

    body
}
}
