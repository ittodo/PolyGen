${
    // C++ namespace generator template
    // Converts namespace to C++ namespace

    import "templates/rhai_utils/indent" as indent_utils;

    let struct_tmpl = include("templates/cpp/cpp_struct.rhai");
    let enum_tmpl = include("templates/cpp/cpp_enum.rhai");
    let mod_tmpl = include("templates/cpp/cpp_mod.rhai");

    let members = "";
    let nested_mods = "";

    // Current namespace FQN for type resolution
    let current_ns = ns.name;

    // Initialize parent_ns if not set (for top-level calls from cpp_file.rhai)
    // Rhai doesn't have a way to check if a variable exists, so we use try-catch
    let is_nested = false;
    try {
        if parent_ns != () && parent_ns != "" {
            is_nested = true;
        }
    } catch {
        // parent_ns not defined, this is a top-level call
        is_nested = false;
    }

    // Process items in the namespace
    for item in ns.items {
        if item.is_struct() {
            let s = item.as_struct();
            let res = eval("let current_ns = \"" + current_ns + "\";\n`" + struct_tmpl + "`");
            members += indent_utils::indent_text(res, 1) + "\n";
        } else if item.is_enum() {
            let e = item.as_enum();
            let res = eval("`" + enum_tmpl + "`");
            members += indent_utils::indent_text(res, 1) + "\n";
        } else if item.is_comment() {
            let c = item.as_comment();
            members += indent_utils::indent_text("// " + c, 1) + "\n";
        } else if item.is_namespace() {
            let child = item.as_namespace();
            // Pass current namespace as parent_ns for nested namespace detection
            let res = eval("let ns = child; let parent_ns = \"" + current_ns + "\";\n`" + mod_tmpl + "`");
            nested_mods += indent_utils::indent_text(res, 1) + "\n";
        }
    }

    // Convert namespace name to C++ namespace
    // For top-level namespaces: use full path (game.item -> game::item)
    // For nested namespaces (when parent_ns is set): only use the last segment
    let cpp_ns_name = "";
    if is_nested {
        // Nested call - use only the last segment
        let ns_parts = ns.name.split(".");
        cpp_ns_name = ns_parts[ns_parts.len() - 1];
    } else {
        // Top-level call - use full namespace path with :: separator
        cpp_ns_name = ns.name + "";
        cpp_ns_name.replace(".", "::");
    }

    let block = "";
    block += "namespace " + cpp_ns_name + " {\n\n";
    block += members;
    block += nested_mods;
    block += "} // namespace " + cpp_ns_name + "\n\n";

    block
}
