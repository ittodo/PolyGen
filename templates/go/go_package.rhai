${
    // Go package generator template
    // Processes a namespace and generates Go types

    import "templates/rhai_utils/indent" as indent_utils;
    import "templates/go/rhai_utils/type_mapping" as go_types;

    let struct_tmpl = include("templates/go/go_struct.rhai");
    let enum_tmpl = include("templates/go/go_enum.rhai");
    let pkg_tmpl = include("templates/go/go_package.rhai");

    let members = "";

    // Current namespace FQN for type resolution
    let current_ns = ns.name;

    // Add namespace comment
    members += "// --- " + to_pascal_case(ns.name) + " types ---\n\n";

    // Process items in the namespace
    for item in ns.items {
        if item.is_struct() {
            let s = item.as_struct();
            let res = eval("let current_ns = \"" + current_ns + "\";\n`" + struct_tmpl + "`");
            members += res + "\n";
        } else if item.is_enum() {
            let e = item.as_enum();
            let res = eval("`" + enum_tmpl + "`");
            members += res + "\n";
        } else if item.is_comment() {
            let c = item.as_comment();
            members += "// " + c + "\n";
        } else if item.is_namespace() {
            let child = item.as_namespace();
            let res = eval("let ns = child;\n`" + pkg_tmpl + "`");
            members += res;
        }
    }

    members
}
