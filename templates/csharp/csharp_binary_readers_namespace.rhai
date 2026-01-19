${
    import "templates/rhai_utils/indent" as indent_utils;
    import "templates/csharp/rhai_utils/type_utils" as type_utils;
    let struct_tmpl = include("templates/csharp/class/csharp_binary_readers_struct.rhai");
    let nested_ns_tmpl = include("templates/csharp/csharp_binary_readers_namespace.rhai");

    let full_ns_name = ns.name;
    let ns_enum_names = type_utils::collect_enum_names(ns);

    let methods = "";
    let nested_blocks = "";

    for item in ns.items {
        if item.is_struct() {
            let s = item.as_struct();
            let __prev_ns = current_ns_name;
            let __prev_enums = current_ns_enums;
            current_ns_name = full_ns_name;
            current_ns_enums = ns_enum_names;
            let result = eval("`" + struct_tmpl + "`");
            current_ns_enums = __prev_enums;
            current_ns_name = __prev_ns;
            methods += result + "\n";
        } else if item.is_namespace() {
            let ns = item.as_namespace();
            let result = eval("`" + nested_ns_tmpl + "`");
            nested_blocks += result + "\n";
        } else if item.is_comment() {
            let c = item.as_comment();
            methods += "// " + c + "\n";
        }
    }

    let block = "";
    block += "namespace " + full_ns_name + "\n";
    block += "{\n";
    block += "    public static class BinaryReaders\n";
    block += "    {\n";
    block += indent_utils::indent_text(methods, 2) + "\n";
    block += "    }\n";
    block += "}\n\n";

    block + nested_blocks
}
