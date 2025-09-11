${
    import "templates/rhai_utils/indent" as indent_utils;
    let struct_tmpl = include("templates/csharp/struct/csharp_csv_writers_struct.rhai");
    let nested_ns_tmpl = include("templates/csharp/csharp_csv_writers_namespace.rhai");

    let full_ns_name = ns.name;
    let ns_decl = if full_ns_name == "" { "Writer" } else { "Writer." + full_ns_name };

    let members = "";
    let nested_blocks = "";
    for item in ns.items {
        if item.is_struct() {
            let s = item.as_struct();
            let __prev_ns = current_ns_name; current_ns_name = full_ns_name;
            let result = eval("`" + struct_tmpl + "`");
            current_ns_name = __prev_ns;
            members += result;
        } else if item.is_namespace() {
            let ns = item.as_namespace();
            let result = eval("`" + nested_ns_tmpl + "`");
            nested_blocks += result;
        } else if item.is_comment() {
            // Skip schema comments in Writer outputs
        }
    }

    let block = "";
    block += "namespace " + ns_decl + "\n";
    block += "{\n";
    block += members;
    block += "}\n";
    block + nested_blocks
}
