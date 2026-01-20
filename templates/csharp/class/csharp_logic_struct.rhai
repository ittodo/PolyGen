${
    let csharp_logic_struct_header = include("templates/csharp/class/csharp_logic_struct_header.rhai");
    let result = eval("`" + csharp_logic_struct_header + "`");
    result
}
public class ${s.name} : IDataRow
{
${
    let csharp_logic_struct_body = include("templates/csharp/class/csharp_logic_struct_body.rhai");
    let result = eval("`" + csharp_logic_struct_body + "`");
    result
}
}

