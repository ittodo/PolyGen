public static ${s.name} Read${s.name}(in JsonElement e)
{
${
    let body_tmpl = include("templates/csharp/struct/csharp_json_mappers_struct_read_body.rhai");
    let __ns_name = if current_ns_name == () { "" } else { current_ns_name };
    let injected = "let current_ns_name = \"" + __ns_name + "\";\n";
    let body = eval(injected + "`" + body_tmpl + "`");
    body
}
    return obj;
}

public static void Write${s.name}(Utf8JsonWriter w, in ${s.name} obj)
{
${
    let body_tmpl = include("templates/csharp/struct/csharp_json_mappers_struct_write_body.rhai");
    let __ns_name = if current_ns_name == () { "" } else { current_ns_name };
    let injected = "let current_ns_name = \"" + __ns_name + "\";\n";
    let body = eval(injected + "`" + body_tmpl + "`");
    body
}

${
    // Emit embedded struct mappers
    import "templates/rhai_utils/indent" as indent_utils;
    let out = "";
    for item in s.items {
        if item.is_embedded_struct() {
            let s = item.as_embedded_struct();
            let tmpl = include("templates/csharp/struct/csharp_json_mappers_struct.rhai");
            let __ns_name = if current_ns_name == () { "" } else { current_ns_name };
            let injected = "let current_ns_name = \"" + __ns_name + "\";\n";
            let result = eval(injected + "`" + tmpl + "`");
            out += "\n" + result + "\n";
        }
    }
    out
}
