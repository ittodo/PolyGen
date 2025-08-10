

namespace ${ns.name}
{

${
    import "templates/rhai_utils/indent" as indent_utils;
    let content = "";
    let csharp_logic_struct = include("templates/csharp/struct/csharp_logic_struct.rhai");
    let enum_template = include("templates/csharp/enum/csharp_enum.rhai");
    indent+=1;
    for item in ns.items {
        if item.is_struct() {
            let s = item.as_struct();
            let result = eval("`" + csharp_logic_struct + "`");
            content += indent_utils::indent_text(result, indent) + "\n";
        }
        if item.is_enum() {
            let e = item.as_enum();
        }
    }
    indent+=-1;
    content;
}
}
