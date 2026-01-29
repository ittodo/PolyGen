import "templates/csharp/rhai_utils/binary_mapping" as binary_utils;
import "templates/csharp/rhai_utils/type_utils" as type_utils;

let using_template = include("templates/csharp/csharp_using_readers.rhai");
let readers_template = include("templates/csharp/csharp_binary_readers.rhai");

for file in schema.files {
    // Skip invalid file entries
    if file.path == () || file.path == "" { continue; }

    let parts = file.path.split("/");
    let filename = parts[parts.len - 1];
    let output_filename = filename;
    output_filename.replace(".poly", ".BinaryReaders.cs");
    let final_path = output_dir + "/csharp/" + output_filename;

    let file_content = "";
    let indent = 0;
    let using_result = eval("`" + using_template + "`");
    file_content += using_result;
    // Initialize namespace variables for readers templates
    let ns_prefix = "";
    let current_ns_name = "";
    let current_ns_enums = [];
    let readers_result = eval("`" + readers_template + "`");
    file_content += readers_result;

    print("Generating file: " + final_path);
    write_file(final_path, file_content);
}

""
