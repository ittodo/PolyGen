let using_template = include("templates/csharp/csharp_using_csv.rhai");
let mappers_template = include("templates/csharp/csharp_csv_mappers.rhai");

for file in schema.files {
    if file.path == () || file.path == "" { continue; }

    let parts = file.path.split("/");
    let filename = parts[parts.len - 1];
    let output_filename = filename;
    output_filename.replace(".poly", ".CsvMappers.cs");
    let final_path = "output/csharp/" + output_filename;

    let file_content = "";
    let indent = 0;
    file_content += eval("`" + using_template + "`");
    let ns_prefix = "";
    let current_ns_name = "";
    let current_ns_enums = [];
    file_content += eval("`" + mappers_template + "`");

    print("Generating file: " + final_path);
    write_file(final_path, file_content);
}

""

