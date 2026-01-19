${
    import "templates/csharp/rhai_utils/binary_mapping" as binary_utils;
    import "templates/csharp/rhai_utils/type_utils" as type_utils;

    let body = "";
    // collect inline enums in this struct
    let current_struct_inline_enums = [];
    for it in s.items { if it.is_inline_enum() { let e = it.as_inline_enum(); current_struct_inline_enums.push(e.name); } }

    // helper builders (inline to avoid Rhai nested fn defs)

    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            let tf = f.field_type; let base = if tf.is_option { tf.inner } else { tf };

            // Option<T>
            if tf.is_option {
                let inner = base;
                body += "        {\n";
                if inner.lang_type == "string" {
                    body += "            obj." + f.name + " = BinaryUtils.ReadOption<string>(br, BinaryUtils.ReadUtf8String);\n";
                } else if inner.is_primitive {
                    let k = inner.lang_type; let cs = cs_map_type(inner);
                    body += "            obj." + f.name + " = BinaryUtils.ReadOption<" + cs + ">(br, _ => " + binary_utils::read_expr(k, "br") + ");\n";
                } else if inner.is_enum || current_struct_inline_enums.contains(inner.type_name) {
                    let enum_type = inner.fqn;
                    body += "            obj." + f.name + " = BinaryUtils.ReadOption<" + enum_type + ">(br, BinaryUtils.ReadEnumInt32<" + enum_type + ">);\n";
                } else {
                    let es_nl = type_utils::find_embedded_struct(s, inner.type_name);
                    let call_ns = if es_nl != () { current_ns_name } else { if inner.namespace_fqn == "" { current_ns_name } else { inner.namespace_fqn } };
                    body += "            obj." + f.name + " = BinaryUtils.ReadOption<global::" + inner.fqn + ">(br, " + call_ns + ".BinaryReaders.Read" + inner.type_name + ");\n";
                }
                body += "        }\n";
                continue;
            }

            // List<T>
            if base.is_list {
                let inner = base.inner;
                body += "        {\n";
                if inner.lang_type == "string" {
                    body += "            obj." + f.name + " = BinaryUtils.ReadList<string>(br, BinaryUtils.ReadUtf8String);\n";
                } else if inner.is_primitive {
                    let k = inner.lang_type; let cs = cs_map_type(inner);
                    body += "            obj." + f.name + " = BinaryUtils.ReadList<" + cs + ">(br, _ => " + binary_utils::read_expr(k, "br") + ");\n";
                } else if inner.is_enum || current_struct_inline_enums.contains(inner.type_name) {
                    let enum_type = inner.fqn;
                    body += "            obj." + f.name + " = BinaryUtils.ReadList<" + enum_type + ">(br, BinaryUtils.ReadEnumInt32<" + enum_type + ">);\n";
                } else {
                    let es_nl = type_utils::find_embedded_struct(s, inner.type_name);
                    let call_ns = if es_nl != () { current_ns_name } else { if inner.namespace_fqn == "" { current_ns_name } else { inner.namespace_fqn } };
                    body += "            obj." + f.name + " = BinaryUtils.ReadList<global::" + inner.fqn + ">(br, " + call_ns + ".BinaryReaders.Read" + inner.type_name + ");\n";
                }
                body += "        }\n";
                continue;
            }

            // string
            if base.lang_type == "string" {
                body += "        obj." + f.name + " = BinaryUtils.ReadUtf8String(br);\n";
            }
            // enum (inline or same-namespace)
            else if base.is_enum || current_struct_inline_enums.contains(base.type_name) {
                let enum_type = base.fqn;
                body += "        obj." + f.name + " = BinaryUtils.ReadEnumInt32<" + enum_type + ">(br);\n";
            }
            // primitives
            else if base.is_primitive {
                body += "        obj." + f.name + " = " + binary_utils::read_expr(base.lang_type, "br") + ";\n";
            }
            // custom struct
            else {
                let es_nl = type_utils::find_embedded_struct(s, base.type_name);
                let call_ns = if es_nl != () { current_ns_name } else { if base.namespace_fqn == "" { current_ns_name } else { base.namespace_fqn } };
                body += "        obj." + f.name + " = " + call_ns + ".BinaryReaders.Read" + base.type_name + "(br);\n";
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
