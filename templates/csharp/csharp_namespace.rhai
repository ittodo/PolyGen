

namespace ${ns.name}
{

${
    import "templates/rhai_utils/indent" as indent_utils;
    let content = "";
    let csharp_logic_struct = include("templates/csharp/struct/csharp_logic_struct.rhai");
    let enum_template = include("templates/csharp/enum/csharp_enum.rhai");
    let csharp_logic_namespace = include("templates/csharp/csharp_namespace.rhai");
    indent+=1;
    for item in ns.items {
        if item.is_struct() {
            let s = item.as_struct();
            let result = eval("`" + csharp_logic_struct + "`");
            content += indent_utils::indent_text(result, indent) + "\n";
        } else if item.is_enum() {
            let e = item.as_enum();
            let result = eval("`" + enum_template + "`");
            content += indent_utils::indent_text(result, indent) + "\n";
        } else if item.is_comment() {
            let c = item.as_comment();
            content += indent_utils::indent_text("// " + c, indent) + "\n";
        } else if item.is_namespace() {
            let ns = item.as_namespace();
            let result = eval("`" + csharp_logic_namespace + "`");
            content += indent_utils::indent_text(result, indent) + "\n";
        }
        else {

            // 이제 진짜 알 수 없는 타입만 경고합니다.
            print("Warning: Namespace '" + ns.name + "'에 알 수 없는 아이템 타입이 있습니다. 타입: " + item);

        }
    }
    indent+=-1;
    content;
}
}
