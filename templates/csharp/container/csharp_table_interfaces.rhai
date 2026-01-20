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
        content += "/// <summary>\n";
        content += "/// Interface for containers that have a " + s.name + " table.\n";
        content += "/// </summary>\n";
        content += "public interface IHas" + s.name + "Table\n";
        content += "{\n";
        content += "    " + s.name + "Table " + s.name + "s { get; }\n";
        content += "}\n\n";
    }

    content
}
