${
    // Zod schema generator template for interfaces
    // Expects 's' to be a StructDef object

    import "templates/typescript/rhai_utils/type_mapping" as ts_types;

    // Current namespace FQN for type resolution
    let ns_fqn = if current_ns != () { current_ns } else { "" };

    // Generate fields
    let fields = "";
    let field_count = 0;
    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            let field_name = to_camel_case(f.name);
            let zod_type = ts_types::map_type_to_zod_with_ns(f.field_type, ns_fqn);

            if field_count > 0 {
                fields += ",\n";
            }
            fields += "    " + field_name + ": " + zod_type;
            field_count += 1;
        }
    }

    // Generate embedded schemas
    let embedded = "";
    for item in s.items {
        if item.is_embedded_struct() {
            let es = item.as_embedded_struct();
            let schema_tmpl = include("templates/typescript/typescript_zod_interface.rhai");
            let res = eval("let s = es; let current_ns = \"" + ns_fqn + "\";\n`" + schema_tmpl + "`");
            embedded += res + "\n";
        } else if item.is_inline_enum() {
            let ie = item.as_inline_enum();
            let enum_tmpl = include("templates/typescript/typescript_zod_enum.rhai");
            let res = eval("let e = ie;\n`" + enum_tmpl + "`");
            embedded += res + "\n";
        }
    }

    let result = "";
    result += embedded;
    result += "export const " + s.name + "Schema = z.object({\n";
    result += fields;
    if field_count > 0 {
        result += "\n";
    }
    result += "});\n";

    result
}
