${
    import "templates/rhai_utils/indent" as indent_utils;
    let struct_tmpl = include("templates/csharp/struct/csharp_csv_mappers_struct.rhai");
    let nested_ns_tmpl = include("templates/csharp/csharp_csv_mappers_namespace.rhai");

    // IR의 네임스페이스 이름은 이미 FQN이므로 접두어를 합치지 않는다
    let full_ns_name = ns.name;

    let members = "";
    let nested_blocks = "";
    for item in ns.items {
        if item.is_struct() {
            let s = item.as_struct();
            let __prev_ns = current_ns_name; current_ns_name = full_ns_name;
            let result = eval("`" + struct_tmpl + "`");
            current_ns_name = __prev_ns;
            members += result + "\n";
        } else if item.is_namespace() {
            let ns = item.as_namespace();
            let result = eval("`" + nested_ns_tmpl + "`");
            nested_blocks += result + "\n";
        } else if item.is_comment() {
            let c = item.as_comment();
            members += "// " + c + "\n";
        }
    }

    let block = "";
    block += "namespace Csv." + full_ns_name + "\n";
    block += "{\n";
    block += indent_utils::indent_text(members, 1) + "\n";
    block += "}\n\n";
    block + nested_blocks
}
