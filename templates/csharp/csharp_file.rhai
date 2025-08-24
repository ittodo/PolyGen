let namespace_template = include("templates/csharp/csharp_namespace.rhai");
let using_template = include("templates/csharp/csharp_using.rhai");
let csharp_logic_namespace = include("templates/csharp/csharp_logic_namespace.rhai");

for file in schema.files {
    // file.path 값을 바로 출력합니다.
    print("Processing file.path: " + file.path);
    // file.path가 비어있으면, 이 file 객체는 건너뜁니다.
    if file.path == () || file.path == "" {
        continue;
    }

    // NOTE: The 'file.path' needs to be exposed from the IR. Assuming it is.
    // Example: "schemas/player.poly" -> "player.cs"
    let parts = file.path.split("/");
    let filename = parts[parts.len - 1];
    print("  - Extracted filename: " + filename); // <== 추가

    let output_filename = filename;
    output_filename.replace(".poly", ".cs");
    print("  - Replaced filename: " + output_filename); // <== 추가
    let final_path = "output/csharp/" + output_filename;

    let file_content = "";
    let indent = 0;

    // Add using statements and the common logic namespace
    file_content += eval("`" + using_template + "`");
    file_content += eval("`" + csharp_logic_namespace + "`");

    

    print("Generating file: " + final_path);
    write_file(final_path, file_content);
}

""