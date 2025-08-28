public static class ${s.name}Csv
{
${
    let body_tmpl = include("templates/csharp/struct/csharp_csv_mappers_struct_body.rhai");
    let __ns_name = if current_ns_name == () { "" } else { current_ns_name };
    let injected = "let current_ns_name = \"" + __ns_name + "\";\n";
    let body = eval(injected + "`" + body_tmpl + "`");
    body
}
}

