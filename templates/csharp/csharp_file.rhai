let namespace_template = include("templates/csharp/csharp_namespace.rhai");

for ns in schema.namespaces {
    let name = "";
    for ch in ns.name.chars() {
        name += if ch == "." { "/" } else { ch };
    }
    
    let file_content = eval(namespace_template);
    let file_path;

    if ns.name == "" {
        file_path = "output/csharp/GlobalTypes.cs";
    } else {
        file_path = "output/csharp/" + name + ".cs";
    }
    
    print("Generating file: " + file_path);
    write_file(file_path, file_content);
}

"" // Return an empty string as the main output is now file generation