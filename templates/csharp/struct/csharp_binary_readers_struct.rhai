public static global::${s.fqn} Read${s.name}(this BinaryReader br)
{
    var obj = new global::${s.fqn}();
${
    let body_tmpl = include("templates/csharp/struct/csharp_binary_readers_struct_body.rhai");
    // Capture current namespace name and inject it into inner eval scope
    let __ns_name = if current_ns_name == () { "" } else { current_ns_name };
    let injected = "let current_ns_name = \"" + __ns_name + "\";\n";
    let body = eval(injected + "`" + body_tmpl + "`");
    body
}    
    return obj;
}

${
    // Generate readers for embedded structs within this struct
    import "templates/rhai_utils/indent" as indent_utils;
    let out = "";
    for item in s.items {
        if item.is_embedded_struct() {
            let s = item.as_embedded_struct();
            let tmpl = include("templates/csharp/struct/csharp_binary_readers_struct.rhai");
            let __ns_name = if current_ns_name == () { "" } else { current_ns_name };
            let injected = "let current_ns_name = \"" + __ns_name + "\";\n";
            let result = eval(injected + "`" + tmpl + "`");
            out += "\n" + result + "\n";
        }
    }
    out
}
