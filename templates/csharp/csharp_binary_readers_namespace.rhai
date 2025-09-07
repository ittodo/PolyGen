${
    import "templates/rhai_utils/indent" as indent_utils;
    let struct_tmpl = include("templates/csharp/struct/csharp_binary_readers_struct.rhai");
    let nested_ns_tmpl = include("templates/csharp/csharp_binary_readers_namespace.rhai");

    // IR 의 ns.name 은 FQN 이므로 prefix를 사용하지 않는다
    let full_ns_name = ns.name;

    let methods = "";
    let nested_blocks = "";

    for item in ns.items {
        if item.is_struct() {
            let s = item.as_struct();
            let __prev_ns = current_ns_name;
            current_ns_name = full_ns_name;
            let __prev_enums = current_ns_enums;
            let __enums = [];
            for it in ns.items { if it.is_enum() { let e = it.as_enum(); __enums.push(e.name); } }
            current_ns_enums = __enums;
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
