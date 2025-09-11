${
    import "templates/csharp/rhai_utils/type_mapping" as type_utils;
    import "templates/csharp/rhai_utils/csv_helpers" as csv_utils;
    import "templates/rhai_utils/indent" as indent_utils;

    let code = "";
    let writer_tmpl = include("templates/csharp/struct/csharp_csv_mappers_struct_writer.rhai");
    let bin_body_tmpl = include("templates/csharp/struct/csharp_binary_writers_struct_body_writer.rhai");

    let is_embedded_globally = false;
    for file in schema.files {
        for ns in file.namespaces {
            let ns_stack = [ns];
            while ns_stack.len > 0 {
                let cur = ns_stack.pop();
                for it in cur.items {
                    if it.is_namespace() { ns_stack.push(it.as_namespace()); }
                    else if it.is_struct() {
                        let ss = it.as_struct();
                        for sit in ss.items { if sit.is_embedded_struct() { let es = sit.as_embedded_struct(); if es.fqn == s.fqn { is_embedded_globally = true; break; } } }
                    }
                }
                if is_embedded_globally { break; }
            }
            if is_embedded_globally { break; }
        }
        if is_embedded_globally { break; }
    }
    if is_embedded_globally { return ""; }

    let stack = [];
    stack.push([s, 0, 0]);
    while stack.len > 0 {
        let frame = stack.pop();
        let def = frame[0]; let stage = frame[1]; let depth = frame[2];
        let saved_s = s; s = def;
        if stage == 0 {
            let header = "public static class " + s.name + "\n" + "{\n";
            code += indent_utils::indent_text(header, depth + 1);
            // Provide full_t for nested templates
            let full_t = "global::" + s.fqn;
            let body = eval("`" + writer_tmpl + "`");
            code += indent_utils::indent_text(body, depth);

            // Binary writer method merged into this class
            let wb_open = "    public static void WriteBinary(this BinaryWriter bw, global::" + s.fqn + " obj)\n" +
                          "    {\n";
            let __ns_name = current_ns_name;
            let injected = "let current_ns_name = \"" + __ns_name + "\";\n";
            let bin_body = eval(injected + "`" + bin_body_tmpl + "`");
            let wb_close = "    }\n";
            code += indent_utils::indent_text(wb_open + bin_body + wb_close, depth + 1);
            stack.push([def, 1, depth]);
            let children = [];
            for it in s.items { if it.is_embedded_struct() { children.push(it.as_embedded_struct()); } }
            let i = children.len; while i > 0 { i -= 1; stack.push([children[i], 0, depth + 1]); }
        } else {
            code += indent_utils::indent_text("}\n", depth + 1);
        }
        s = saved_s;
    }
    code
}
