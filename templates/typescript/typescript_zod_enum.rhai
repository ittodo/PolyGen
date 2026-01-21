${
    // Zod enum schema generator template
    // Expects 'e' to be an EnumDef object
    // Expects 'current_ns' to be the current namespace FQN

    import "templates/typescript/rhai_utils/type_mapping" as ts_types;

    // Compute the Types namespace path
    let ns_path = if current_ns != () && current_ns != "" {
        ts_types::ns_to_ts_path(current_ns)
    } else {
        ""
    };

    let types_ref = if ns_path != "" {
        "Types." + ns_path + "." + e.name
    } else {
        "Types." + e.name
    };

    // Generate Zod enum schema using z.nativeEnum
    let result = "";
    result += "export const " + e.name + "Schema = z.nativeEnum(" + types_ref + ");\n";

    result
}
