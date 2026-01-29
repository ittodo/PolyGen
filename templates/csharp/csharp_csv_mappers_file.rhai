let using_template = include("templates/csharp/csharp_using_csv.rhai");
let mappers_template = include("templates/csharp/csharp_csv_mappers.rhai");

for file in schema.files {
    if file.path == () || file.path == "" { continue; }

    let parts = file.path.split("/");
    let filename = parts[parts.len - 1];
    let output_filename = filename;
    output_filename.replace(".poly", ".CsvMappers.cs");
    let final_path = output_dir + "/csharp/" + output_filename;

    let file_content = "";
    let indent = 0;
    let using_result = eval("`" + using_template + "`");
    file_content += using_result;
    let ns_prefix = "";
    let current_ns_name = "";
    let mappers_result = eval("`" + mappers_template + "`");
    file_content += mappers_result;

    print("Generating file: " + final_path);
    write_file(final_path, file_content);
}

""

