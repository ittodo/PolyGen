${
    // Zod namespace schema generator template

    import "templates/rhai_utils/indent" as indent_utils;
    import "templates/typescript/rhai_utils/type_mapping" as ts_types;

    let schema_tmpl = include("templates/typescript/typescript_zod_interface.rhai");
    let enum_tmpl = include("templates/typescript/typescript_zod_enum.rhai");
    let ns_tmpl = include("templates/typescript/typescript_zod_namespace.rhai");

    // Convert namespace name to PascalCase for TypeScript
    let ns_name = to_pascal_case(ns.name);

    // Build the full TypeScript namespace path for Types reference
    // parent_ns_path is passed from parent namespace (empty for top-level)
    let parent_path = if parent_ns_path != () && parent_ns_path != "" { parent_ns_path } else { "" };
    let current_ts_path = if parent_path != "" {
        parent_path + "." + ns_name
    } else {
        ns_name
    };

    let members = "";
    let nested_ns = "";

    // Current namespace FQN for type resolution
    let current_ns = ns.name;

    // Process items in the namespace
    for item in ns.items {
        if item.is_struct() {
            let s = item.as_struct();
            let res = eval("let current_ns = \"" + current_ns + "\"; let current_ts_path = \"" + current_ts_path + "\";\n`" + schema_tmpl + "`");
            members += indent_utils::indent_text(res, 1) + "\n";
        } else if item.is_enum() {
            let e = item.as_enum();
            let res = eval("let current_ns = \"" + current_ns + "\"; let current_ts_path = \"" + current_ts_path + "\";\n`" + enum_tmpl + "`");
            members += indent_utils::indent_text(res, 1) + "\n";
        } else if item.is_namespace() {
            let child = item.as_namespace();
            let res = eval("let ns = child; let parent_ns_path = \"" + current_ts_path + "\";\n`" + ns_tmpl + "`");
            nested_ns += indent_utils::indent_text(res, 1) + "\n";
        }
    }

    let block = "";
    block += "export namespace " + ns_name + " {\n";
    block += members;
    block += nested_ns;
    block += "}\n";

    block
}
