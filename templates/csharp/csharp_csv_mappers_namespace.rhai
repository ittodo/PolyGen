${
    import "templates/rhai_utils/indent" as indent_utils;
    let struct_tmpl = include("templates/csharp/struct/csharp_csv_mappers_struct.rhai");
    let nested_ns_tmpl = include("templates/csharp/csharp_csv_mappers_namespace.rhai");

    let full_ns_name = if ns_prefix == () || ns_prefix == "" { ns.name } else { ns_prefix + "." + ns.name };

    let blocks = "";
    for item in ns.items {
        if item.is_struct() {
            let s = item.as_struct();
            let __prev_ns = current_ns_name; current_ns_name = full_ns_name;
            let result = eval("`" + struct_tmpl + "`");
            current_ns_name = __prev_ns;
            blocks += indent_utils::indent_text(result, 1) + "\n";
        } else if item.is_namespace() {
            let ns = item.as_namespace();
            let __prev_prefix = ns_prefix; ns_prefix = full_ns_name;
            let result = eval("`" + nested_ns_tmpl + "`");
            ns_prefix = __prev_prefix;
            blocks += indent_utils::indent_text(result, 0) + "\n";
        }
    }

    let content = "";
    content += "namespace " + full_ns_name + "\n{\n";
    content += blocks;
    content += "}\n";
    content
}

