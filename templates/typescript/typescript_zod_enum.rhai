${
    // Zod enum schema generator template
    // Expects 'e' to be an EnumDef object
    // Expects 'current_ts_path' to be the current TypeScript namespace path (e.g., "TestEnums" or "App.AppData.AppDataEnums")

    // Build the Types reference using the actual TypeScript namespace path
    let ts_path = if current_ts_path != () && current_ts_path != "" {
        current_ts_path
    } else {
        ""
    };

    let types_ref = if ts_path != "" {
        "Types." + ts_path + "." + e.name
    } else {
        "Types." + e.name
    };

    // Generate Zod enum schema using z.nativeEnum
    let result = "";
    result += "export const " + e.name + "Schema = z.nativeEnum(" + types_ref + ");\n";

    result
}
