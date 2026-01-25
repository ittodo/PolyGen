// DataContext generation template for C#
// Generates DataContext class that groups containers by @datasource annotation
// and provides cross-container foreign key validation.

let datacontext_class = include("templates/csharp/container/csharp_datacontext_class.rhai");
let datasource_container = include("templates/csharp/container/csharp_datasource_container.rhai");
let using_template = include("templates/csharp/csharp_using_container.rhai");
import "templates/rhai_utils/indent" as indent_utils;

for file in schema.files {
    if file.path == () || file.path == "" {
        continue;
    }

    // Collect all namespaces with datasource info (including nested)
    let datasource_map = #{};  // datasource_name -> [{ ns, structs }]
    let ns_stack = [];
    for ns in file.namespaces {
        ns_stack.push(ns);
    }

    while ns_stack.len() > 0 {
        let ns = ns_stack.pop();

        // Check for nested namespaces
        for item in ns.items {
            if item.is_namespace() {
                ns_stack.push(item.as_namespace());
            }
        }

        // Get datasource for this namespace
        let ds = ns.datasource;
        if ds == () || ds == "" {
            continue;  // Skip namespaces without @datasource
        }

        // Collect structs in this namespace
        let structs = [];
        for item in ns.items {
            if item.is_struct() {
                structs.push(item.as_struct());
            }
        }

        if structs.len() == 0 {
            continue;
        }

        // Add to datasource map
        if !datasource_map.contains(ds) {
            datasource_map[ds] = [];
        }
        datasource_map[ds].push(#{
            namespace: ns,
            structs: structs
        });
    }

    // Skip if no datasources defined
    let ds_keys = datasource_map.keys();
    if ds_keys.len() == 0 {
        continue;
    }

    // Determine output filename
    let parts = file.path.split("/");
    let filename = parts[parts.len - 1];
    let output_filename = filename;
    output_filename.replace(".poly", ".DataContext.cs");
    let final_path = output_dir + "/csharp/" + output_filename;

    // Get container namespace name from file path
    let file_name = file.path;
    let path_parts = file_name.split("/");
    let base_name = "";
    for part in path_parts {
        base_name = part;
    }
    if base_name.ends_with(".poly") {
        base_name = base_name.sub_string(0, base_name.len() - 5);
    }
    // Convert to PascalCase
    let name_parts = base_name.split("_");
    let context_ns_name = "";
    for part in name_parts {
        if part.len() > 0 {
            let first = part.sub_string(0, 1);
            first = first.to_upper();
            let rest = part.sub_string(1);
            context_ns_name += first + rest;
        }
    }

    let file_content = "";

    // Add using statements
    file_content += eval("`" + using_template + "`");

    // Generate namespace
    file_content += "namespace " + context_ns_name + ".DataContext\n";
    file_content += "{\n";

    // Generate container class for each datasource
    for ds_name in ds_keys {
        let ds_entries = datasource_map[ds_name];
        let container_content = eval("`" + datasource_container + "`");
        file_content += indent_utils::indent_text(container_content, 1);
        file_content += "\n";
    }

    // Generate DataContext class
    let context_content = eval("`" + datacontext_class + "`");
    file_content += indent_utils::indent_text(context_content, 1);

    file_content += "}\n";

    print("Generating file: " + final_path);
    write_file(final_path, file_content);
}

""
