let namespace_template = include("templates/csharp/csharp_namespace.rhai");
let csharp_logic_namespace = include("templates/csharp/csharp_logic_namespace.rhai");

let struct_template = include("templates/csharp/csharp_struct.rhai");
let enum_template = include("templates/csharp/csharp_enum.rhai");
let using_template = include("templates/csharp/csharp_using.rhai");

for file in schema.files {
    // file_path and final_path are no longer used for single file output
    // They might be useful for logging or base directory creation, but not for the final file path.
    // print(`filepath : ${file.path} ${file_path}`);
    // if(file_path == "")
    // {
    //     continue;
    // }
    // let final_path = "output/csharp/" + file_path;

    // No longer generating a single file_content for the entire namespace
    // Instead, generate individual files for each struct and enum

    let contents = eval("`" + using_template + "`");
    let indent = 0;
    contents += eval("`" + csharp_logic_namespace + "`");

    print(contents);


    for ns in file.namespaces {
        let namespace_dir = "output/csharp/" + ns.name; // Create a directory for the namespace
        // Ensure the directory exists (this might require a shell command or a Rhai function to create directories)
        // For now, let's assume the write_file function can handle creating directories if they don't exist, or we'll add a shell command later.

        for item in ns.items {
            if item.is_struct() {
                let s = item.as_struct();
                let struct_file_name = s.name + ".cs";
                let final_struct_path = namespace_dir + "/" + struct_file_name;
                let struct_content = eval( "`"+ struct_template +"`");
                print("Generating struct file: " + final_struct_path);
                write_file(final_struct_path, struct_content);
            } else if item.is_enum() {
                let e = item.as_enum();
                let enum_file_name = e.name + ".cs";
                let final_enum_path = namespace_dir + "/" + enum_file_name;
                let enum_content = eval("`"+enum_template+"`");
                print("Generating enum file: " + final_enum_path);
                write_file(final_enum_path, enum_content);
            }
        }
    }
    // print("Generating file: " + final_path);
    // write_file(final_path, file_content);

    
}

""