${
    import "templates/rhai_utils/indent" as indent_utils;
    let container_struct = include("templates/csharp/container/csharp_container_datatable.rhai");
    let container_class = include("templates/csharp/container/csharp_container_class.rhai");

    let content = "";

    for ns in file.namespaces {
        // Collect all structs in this namespace for the container
        let structs = [];
        for item in ns.items {
            if item.is_struct() {
                structs.push(item.as_struct());
            }
        }

        if structs.len == 0 {
            continue;
        }

        let namespace_name = ns.name;
        if namespace_name == "" {
            namespace_name = "Global";
        }

        content += "namespace " + namespace_name + ".Container\n";
        content += "{\n";

        // Generate DataTable classes for each struct
        for s in structs {
            let table_content = eval("`" + container_struct + "`");
            content += indent_utils::indent_text(table_content, 1);
            content += "\n";
        }

        // Generate the root container class
        let container_content = eval("`" + container_class + "`");
        content += indent_utils::indent_text(container_content, 1);
        content += "\n";

        content += "}\n\n";
    }

    content
}
