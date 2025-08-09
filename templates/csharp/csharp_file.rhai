let namespace_template = include("templates/csharp/csharp_namespace.rhai");

for file in schema.files {
    let file_path = file.path;
    file_path.replace(".poly", ".cs");
    print(`filepath : ${file.path} ${file_path}`);
    if(file_path == "")
    {
        continue;
    }
    let final_path = "output/csharp/" + file_path;

    let file_content = "";
    for ns in file.namespaces {
        file_content += eval(namespace_template);
    }

    print("Generating file: " + final_path);
    write_file(final_path, file_content);
}

""