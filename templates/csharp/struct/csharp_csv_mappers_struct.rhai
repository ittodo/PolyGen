${
    import "templates/csharp/rhai_utils/type_mapping" as type_utils;
    import "templates/csharp/rhai_utils/csv_helpers" as csv_utils;

    let code = "";

    let writer_tmpl = include("templates/csharp/struct/csharp_csv_mappers_struct_writer.rhai");
    let reader_tmpl = include("templates/csharp/struct/csharp_csv_mappers_struct_reader.rhai");

    // If this struct appears as an embedded struct anywhere in the schema,
    // skip generating a top-level CSV class (it will be rendered nested under its owner).
    let is_embedded_globally = false;
    for file in schema.files {
        for ns in file.namespaces {
            // depth-first over namespace items
            let ns_stack = [ns];
            while ns_stack.len > 0 {
                let cur = ns_stack.pop();
                for it in cur.items {
                    if it.is_namespace() {
                        ns_stack.push(it.as_namespace());
                    } else if it.is_struct() {
                        let ss = it.as_struct();
                        for sit in ss.items {
                            if sit.is_embedded_struct() {
                                let es = sit.as_embedded_struct();
                                if es.fqn == s.fqn { is_embedded_globally = true; break; }
                            }
                        }
                    }
                }
                if is_embedded_globally { break; }
            }
            if is_embedded_globally { break; }
        }
        if is_embedded_globally { break; }
    }

    if is_embedded_globally {
        // Do not emit a separate top-level class for embedded structs
        return "";
    }

    // Stack-based rendering to allow nested classes without defining Rhai functions
    let stack = [];
    stack.push([s, 0]); // 0=open+body, 1=close

    while stack.len > 0 {
        let frame = stack.pop();
        let def = frame[0];
        let stage = frame[1];

        let saved_s = s;
        s = def;

        if stage == 0 {
            // Open class and emit body
            code += "public static class " + s.name + "\n";
            code += "{\n";
            let full_t = "global::" + s.fqn;
            code += eval("`" + writer_tmpl + "`");
            code += eval("`" + reader_tmpl + "`");

            // Schedule close after children
            stack.push([def, 1]);

            // Collect child struct defs to render nested: embedded only
            let children = [];
            for it in s.items { if it.is_embedded_struct() { children.push(it.as_embedded_struct()); } }
            // Push children in reverse to preserve source order
            let i = children.len;
            while i > 0 { i -= 1; stack.push([children[i], 0]); }
        } else {
            // Close class
            code += "}\n";
        }

        s = saved_s;
    }

    code
}
