${
    import "templates/csharp/rhai_utils/type_mapping" as type_utils;
    import "templates/csharp/rhai_utils/read_mapping" as read_utils;

    let body = "";
    // collect inline enums in this struct
    let current_struct_inline_enums = [];
    for it in s.items { if it.is_inline_enum() { let e = it.as_inline_enum(); current_struct_inline_enums.push(e.name); } }

    // helper builders (inline to avoid Rhai nested fn defs)

    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            let t = f.field_type;

            // Option<T>
            if t.starts_with("Option<") {
                let __s = "Option<".len();
                let __l = t.len() - __s - 1;
                let inner = t.sub_string(__s, __l);
                body += "        {\n";
                if inner == "string" {
                    body += "            obj." + f.name + " = BinaryUtils.ReadOption<string>(br, BinaryUtils.ReadUtf8String);\n";
                } else if read_utils::is_primitive(inner) {
                    body += "            obj." + f.name + " = BinaryUtils.ReadOption<" + inner + ">(br, _ => " + read_utils::read_expr(inner, "br") + ");\n";
                } else if inner.ends_with("__Enum") || current_ns_enums.contains(inner) || current_struct_inline_enums.contains(inner) {
                    let enum_type = if inner.ends_with("__Enum") { s.name + "." + inner } else { inner };
                    body += "            obj." + f.name + " = BinaryUtils.ReadOption<" + enum_type + ">(br, BinaryUtils.ReadEnumInt32<" + enum_type + ">);\n";
                } else {
                    let parts = inner.split(".");
                    let call_type = parts[parts.len-1];
                    let call_ns = if inner.contains(".") {
                        let __ns = ""; let __i = 0;
                        for seg in parts { if __i < parts.len-1 { if __ns != "" { __ns += "."; } __ns += seg; } __i += 1; }
                        __ns
                    } else { current_ns_name };
                    body += "            obj." + f.name + " = BinaryUtils.ReadOption<" + inner + ">(br, " + call_ns + ".BinaryReaders.Read" + call_type + ");\n";
                }
                body += "        }\n";
                continue;
            }

            // List<T>
            if t.starts_with("List<") {
                let __s = "List<".len();
                let __l = t.len() - __s - 1;
                let inner = t.sub_string(__s, __l);
                body += "        {\n";
                if inner == "string" {
                    body += "            obj." + f.name + " = BinaryUtils.ReadList<string>(br, BinaryUtils.ReadUtf8String);\n";
                } else if read_utils::is_primitive(inner) {
                    body += "            obj." + f.name + " = BinaryUtils.ReadList<" + inner + ">(br, _ => " + read_utils::read_expr(inner, "br") + ");\n";
                } else if inner.ends_with("__Enum") || current_ns_enums.contains(inner) || current_struct_inline_enums.contains(inner) {
                    let enum_type = if inner.ends_with("__Enum") { s.name + "." + inner } else { inner };
                    body += "            obj." + f.name + " = BinaryUtils.ReadList<" + enum_type + ">(br, BinaryUtils.ReadEnumInt32<" + enum_type + ">);\n";
                } else {
                    let parts = inner.split(".");
                    let call_type = parts[parts.len-1];
                    let call_ns = if inner.contains(".") {
                        let __ns = ""; let __i = 0;
                        for seg in parts { if __i < parts.len-1 { if __ns != "" { __ns += "."; } __ns += seg; } __i += 1; }
                        __ns
                    } else { current_ns_name };
                    body += "            obj." + f.name + " = BinaryUtils.ReadList<" + inner + ">(br, " + call_ns + ".BinaryReaders.Read" + call_type + ");\n";
                }
                body += "        }\n";
                continue;
            }

            // string
            if t == "string" {
                body += "        { obj." + f.name + " = BinaryUtils.ReadUtf8String(br); }\n";
            }
            // enum (inline or same-namespace)
            else if t.ends_with("__Enum") || current_ns_enums.contains(t) || current_struct_inline_enums.contains(t) {
                let enum_type = if t.ends_with("__Enum") { s.name + "." + t } else { t };
                body += "        obj." + f.name + " = BinaryUtils.ReadEnumInt32<" + enum_type + ">(br);\n";
            }
            // primitives
            else if read_utils::is_primitive(t) {
                body += "        obj." + f.name + " = " + read_utils::read_expr(t, "br") + ";\n";
            }
            // custom struct
            else {
                let parts = t.split(".");
                let call_type = parts[parts.len-1];
                let call_ns = if t.contains(".") {
                    let __ns = ""; let __i = 0;
                    for seg in parts { if __i < parts.len-1 { if __ns != "" { __ns += "."; } __ns += seg; } __i += 1; }
                    __ns
                } else { current_ns_name };
                body += "        obj." + f.name + " = " + call_ns + ".BinaryReaders.Read" + call_type + "(br);\n";
            }
        } else if item.is_comment() {
            let c = item.as_comment();
            body += "        // " + c + "\n";
        } else if item.is_embedded_struct() {
            let es = item.as_embedded_struct();
            body += "        // TODO: Embedded struct '" + es.name + "' binary read not implemented\n";
        } else if item.is_inline_enum() {
            let ie = item.as_inline_enum();
            body += "        // TODO: Inline enum '" + ie.name + "' binary read not implemented\n";
        }
    }
    body
}
