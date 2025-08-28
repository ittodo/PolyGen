${
    import "templates/rhai_utils/indent" as indent_utils;
    let struct_tmpl = include("templates/csharp/struct/csharp_binary_writers_struct.rhai");
    let nested_ns_tmpl = include("templates/csharp/csharp_binary_writers_namespace.rhai");

    let full_ns_name = if ns_prefix == () || ns_prefix == "" { ns.name } else { ns_prefix + "." + ns.name };

    let methods = "";
    let nested_blocks = "";

    for item in ns.items {
        if item.is_struct() {
            let s = item.as_struct();
            let __prev_ns = current_ns_name;
            current_ns_name = full_ns_name;
            let result = eval("`" + struct_tmpl + "`");
            current_ns_name = __prev_ns;
            methods += result + "\n";
        } else if item.is_namespace() {
            let ns = item.as_namespace();
            let __prev_prefix = ns_prefix; ns_prefix = full_ns_name;
            let result = eval("`" + nested_ns_tmpl + "`");
            nested_blocks += result + "\n";
            ns_prefix = __prev_prefix;
        } else if item.is_comment() {
            let c = item.as_comment();
            methods += "// " + c + "\n";
        }
    }

    let block = "";
    block += "namespace " + full_ns_name + "\n";
    block += "{\n";
    block += "    public static class BinaryWriters\n";
    block += "    {\n";
    block += indent_utils::indent_text(methods, 2) + "\n";
    block += "    }\n";
    block += "}\n\n";

    block + nested_blocks
}

