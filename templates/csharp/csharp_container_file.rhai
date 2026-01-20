// Container generation template for C#
// Generates DataTable classes and a root Container class for each schema file

let container_namespace = include("templates/csharp/csharp_container_namespace.rhai");
let using_template = include("templates/csharp/csharp_using_container.rhai");

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

    // Generate container namespace content
    file_content += eval("`" + container_namespace + "`");

    print("Generating file: " + final_path);
    write_file(final_path, file_content);
}

""
