${

let csharp_namespace = include("templates/csharp/csharp_namespace.rhai");


let content = "";
for ns in file.namespaces
{
    content += eval("`" + csharp_namespace + "`");
}
content;
}
