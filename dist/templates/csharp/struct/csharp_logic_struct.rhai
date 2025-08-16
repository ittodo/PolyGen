${
    let csharp_logic_struct_header = include("templates/csharp/struct/csharp_logic_struct_header.rhai");
    let result = eval("`" + csharp_logic_struct_header + "`");
    result
}
class ${s.name}
{
${
        let csharp_logic_struct_body = include("templates/csharp/struct/csharp_logic_struct_body.rhai");
        let result = eval("`" + csharp_logic_struct_body + "`");
        result
    }}
