import "templates/csharp/rhai_utils/binary_mapping" as binary_utils;
import "templates/csharp/rhai_utils/type_utils" as type_utils;

let using_template = include("templates/csharp/csharp_using_writers.rhai");
let writers_template = include("templates/csharp/csharp_binary_writers.rhai");

for file in schema.files {
    if file.path == () || file.path == "" { continue; }

    let parts = file.path.split("/");
    let filename = parts[parts.len - 1];
    let output_filename = filename;
    output_filename.replace(".poly", ".BinaryWriters.cs");
    let final_path = output_dir + "/csharp/" + output_filename;

    let file_content = "";
    let indent = 0;
    let using_result = eval("`" + using_template + "`");
    file_content += using_result;
    let ns_prefix = "";
    let current_ns_name = "";
    let current_ns_enums = [];
    let writers_result = eval("`" + writers_template + "`");
    file_content += writers_result;

    print("Generating file: " + final_path);
    write_file(final_path, file_content);
}

""

