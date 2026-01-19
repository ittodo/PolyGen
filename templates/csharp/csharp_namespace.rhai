
${
    import "templates/rhai_utils/indent" as indent_utils;
    let struct_tmpl = include("templates/csharp/class/csharp_logic_struct.rhai");
    let enum_tmpl = include("templates/csharp/enum/csharp_enum.rhai");
    let ns_tmpl = include("templates/csharp/csharp_namespace.rhai");

    let methods = "";
    let nested_blocks = "";

    // emit current namespace members
    for item in ns.items {
        if item.is_struct() {
            let s = item.as_struct();
            let res = eval("`" + struct_tmpl + "`");
            methods += indent_utils::indent_text(res, 1) + "\n";
        } else if item.is_enum() {
            let e = item.as_enum();
            let res = eval("`" + enum_tmpl + "`");
            methods += indent_utils::indent_text(res, 1) + "\n";
        } else if item.is_comment() {
            let c = item.as_comment();
            methods += indent_utils::indent_text("// " + c, 1) + "\n";
        } else if item.is_namespace() {
            let child = item.as_namespace();
            let res = eval("let ns = child;\n`" + ns_tmpl + "`");
            nested_blocks += res + "\n";
        }
    }

    let block = "";
    block += "namespace " + ns.name + "\n";
    block += "{\n";
    block += methods;
    block += "}\n\n";

    block + nested_blocks
}
