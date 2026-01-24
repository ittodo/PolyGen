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
            // Use the new function that includes field constraints
            let zod_type = ts_types::map_field_to_zod_with_constraints(f, ns_fqn);

            if field_count > 0 {
                fields += ",\n";
            }
            fields += "    " + field_name + ": " + zod_type;
            field_count += 1;
        }
    }

    // Get current TypeScript path for enum references
    let ts_path = if current_ts_path != () && current_ts_path != "" { current_ts_path } else { "" };

    // Generate embedded schemas
    let embedded = "";
    for item in s.items {
        if item.is_embedded_struct() {
            let es = item.as_embedded_struct();
            let schema_tmpl = include("templates/typescript/typescript_zod_interface.rhai");
            let res = eval("let s = es; let current_ns = \"" + ns_fqn + "\"; let current_ts_path = \"" + ts_path + "\";\n`" + schema_tmpl + "`");
            embedded += res + "\n";
        } else if item.is_inline_enum() {
            let ie = item.as_inline_enum();
            let enum_tmpl = include("templates/typescript/typescript_zod_enum.rhai");
            let res = eval("let e = ie; let current_ts_path = \"" + ts_path + "\";\n`" + enum_tmpl + "`");
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
    result += "export type " + s.name + " = z.infer<typeof " + s.name + "Schema>;\n";

    // Add Pack/Unpack functions for embeds with @pack annotation
    if s.is_embed && s.pack_separator != () {
        let sep = s.pack_separator;
        let field_list = [];

        // Collect field info
        for item in s.items {
            if item.is_field() {
                let f = item.as_field();
                field_list.push(f);
            }
        }

        if field_list.len() > 0 {
            // Pack function
            result += "\n/** Packs all fields into a single string using '" + sep + "' as separator. */\n";
            result += "export function pack" + s.name + "(value: " + s.name + "): string {\n";
            result += "    return [";
            for i in 0..field_list.len() {
                let f = field_list[i];
                let field_name = to_camel_case(f.name);
                if i > 0 {
                    result += ", ";
                }
                result += "value." + field_name;
            }
            result += "].join('" + sep + "');\n";
            result += "}\n";

            // Unpack function
            result += "\n/** Unpacks a string into fields using '" + sep + "' as separator. */\n";
            result += "export function unpack" + s.name + "(value: string): " + s.name + " {\n";
            result += "    const parts = value.split('" + sep + "');\n";
            result += "    if (parts.length !== " + field_list.len() + ") {\n";
            result += "        throw new Error(`Expected " + field_list.len() + " parts but got ${parts.length} when unpacking " + s.name + "`);\n";
            result += "    }\n";
            result += "    return {\n";

            for i in 0..field_list.len() {
                let f = field_list[i];
                let field_name = to_camel_case(f.name);
                let type_name = f.field_type.type_name;
                let parse_expr = "";

                // Generate parse expression based on type
                if type_name == "string" {
                    parse_expr = "parts[" + i + "]";
                } else if type_name == "bool" {
                    parse_expr = "parts[" + i + "] === 'true'";
                } else if type_name == "f32" || type_name == "f64" {
                    parse_expr = "parseFloat(parts[" + i + "])";
                } else if type_name == "u8" || type_name == "u16" || type_name == "u32" || type_name == "u64" || type_name == "i8" || type_name == "i16" || type_name == "i32" || type_name == "i64" {
                    parse_expr = "parseInt(parts[" + i + "], 10)";
                } else {
                    parse_expr = "parts[" + i + "]";
                }

                result += "        " + field_name + ": " + parse_expr;
                if i < field_list.len() - 1 {
                    result += ",";
                }
                result += "\n";
            }

            result += "    };\n";
            result += "}\n";

            // TryUnpack function
            result += "\n/** Tries to unpack a string. Returns null on failure. */\n";
            result += "export function tryUnpack" + s.name + "(value: string): " + s.name + " | null {\n";
            result += "    try {\n";
            result += "        return unpack" + s.name + "(value);\n";
            result += "    } catch {\n";
            result += "        return null;\n";
            result += "    }\n";
            result += "}\n";
        }
    }

    result
}
