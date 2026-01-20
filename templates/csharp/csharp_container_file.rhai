// Container generation template for C#
// Generates DataTable classes and a root Container class for each schema file

let container_namespace = include("templates/csharp/csharp_container_namespace.rhai");
let root_container = include("templates/csharp/container/csharp_root_container.rhai");
let table_interfaces = include("templates/csharp/container/csharp_table_interfaces.rhai");
let data_loader = include("templates/csharp/container/csharp_data_loader.rhai");
let using_template = include("templates/csharp/csharp_using_container.rhai");
import "templates/rhai_utils/indent" as indent_utils;

for file in schema.files {
    if file.path == () || file.path == "" {
        continue;
    }

    let parts = file.path.split("/");
    let filename = parts[parts.len - 1];
    let output_filename = filename;
    output_filename.replace(".poly", ".Container.cs");
    let final_path = output_dir + "/csharp/" + output_filename;

    let file_content = "";
    let indent = 0;

    // Add using statements
    file_content += eval("`" + using_template + "`");

    // Generate namespace container content (DataTable classes per namespace)
    file_content += eval("`" + container_namespace + "`");

    // Generate root container that aggregates all tables
    // Get the container namespace name from file path
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
    let container_ns_name = "";
    for part in name_parts {
        if part.len() > 0 {
            let first = part.sub_string(0, 1);
            first = first.to_upper();
            let rest = part.sub_string(1);
            container_ns_name += first + rest;
        }
    }

    file_content += "namespace " + container_ns_name + ".Container\n";
    file_content += "{\n";

    // Generate table interfaces
    let interfaces_content = eval("`" + table_interfaces + "`");
    file_content += indent_utils::indent_text(interfaces_content, 1);

    // Generate root container
    let container_name = container_ns_name;
    let root_content = eval("`" + root_container + "`");
    file_content += indent_utils::indent_text(root_content, 1);
    file_content += "\n\n";

    // Generate data loader
    let loader_content = eval("`" + data_loader + "`");
    file_content += indent_utils::indent_text(loader_content, 1);
    file_content += "\n}\n";

    print("Generating file: " + final_path);
    write_file(final_path, file_content);
}

""
