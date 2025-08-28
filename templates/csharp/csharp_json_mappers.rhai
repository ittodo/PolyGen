${
    let ns_tmpl = include("templates/csharp/csharp_json_mappers_namespace.rhai");
    let content = "";
    for ns in file.namespaces {
        content += eval("`" + ns_tmpl + "`");
    }
    content;
}
