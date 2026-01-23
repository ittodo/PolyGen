${
    let content = "";

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

    // Generate interface for each table
    for s in all_structs {
        // Get the namespace part from fqn (fqn = namespace.StructName)
        let ns_fqn = s.fqn;
        let last_dot = ns_fqn.index_of(".");
        let ns_path = "";
        if last_dot != () {
            // Find the last dot to separate namespace from struct name
            let parts = ns_fqn.split(".");
            for i in 0..(parts.len() - 1) {
                if i > 0 { ns_path += "."; }
                ns_path += parts[i];
            }
        }
        let table_fqn = if ns_path != "" { "global::" + ns_path + ".Container." + s.name + "Table" } else { s.name + "Table" };

        content += "/// <summary>\n";
        content += "/// Interface for containers that have a " + s.name + " table.\n";
        content += "/// </summary>\n";
        content += "public interface IHas" + s.name + "Table\n";
        content += "{\n";
        content += "    " + table_fqn + " " + s.name + "s { get; }\n";
        content += "}\n\n";
    }

    content
}
