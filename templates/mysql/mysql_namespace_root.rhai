// Renders DDL for all namespaces in a file
${
    let tpl = include("templates/mysql/mysql_namespace.rhai");
    let content = "";
    for ns in file.namespaces {
        content += eval("`" + tpl + "`");
        content += "\n";
    }
    content
}
