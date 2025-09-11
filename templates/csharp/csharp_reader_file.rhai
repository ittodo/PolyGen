let using_all = include("templates/csharp/csharp_using_reader_combined.rhai");
let csv_readers_ns = include("templates/csharp/csharp_csv_readers_namespace.rhai");

for file in schema.files {
    if file.path == () || file.path == "" { continue; }

    let parts = file.path.split("/");
    let filename = parts[parts.len - 1];
    let output_filename = filename;
    output_filename.replace(".poly", ".Reader.cs");
    let final_path = "output/csharp/" + output_filename;

    let file_content = "";
    file_content += eval("`" + using_all + "`");

    // Generate CSV reader mappers (includes ReadBinary) under namespace Reader.<ns>
    let current_ns_name = ""; let current_ns_enums = [];
    for ns in file.namespaces { file_content += eval("`" + csv_readers_ns + "`"); }

    // compact excessive blank lines
    while file_content.contains("\n\n\n") { file_content.replace("\n\n\n", "\n\n"); }
    print("Generating file: " + final_path);
    write_file(final_path, file_content);
}

""
