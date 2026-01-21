${
    // Rust module generator template
    // Converts namespace to Rust mod

    import "templates/rhai_utils/indent" as indent_utils;
    import "templates/rust/rhai_utils/type_mapping" as rust_types;

    let struct_tmpl = include("templates/rust/rust_struct.rhai");
    let enum_tmpl = include("templates/rust/rust_enum.rhai");
    let mod_tmpl = include("templates/rust/rust_mod.rhai");

    // Convert namespace name to snake_case for Rust module
    let mod_name = to_snake_case(ns.name);

    let members = "";
    let nested_mods = "";

    // Process items in the namespace
    for item in ns.items {
        if item.is_struct() {
            let s = item.as_struct();
            let res = eval("`" + struct_tmpl + "`");
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
            let res = eval("let ns = child;\n`" + mod_tmpl + "`");
            nested_mods += indent_utils::indent_text(res, 1) + "\n";
        }
    }

    let block = "";
    block += "pub mod " + mod_name + " {\n";
    if members != "" || nested_mods != "" {
        block += "    use serde::{Deserialize, Serialize};\n\n";
    }
    block += members;
    block += nested_mods;
    block += "}\n";

    block
}
