${
    import "templates/csharp/rhai_utils/type_mapping" as type_utils;
    import "templates/csharp/rhai_utils/read_mapping" as read_utils;
    import "templates/csharp/rhai_utils/csv_helpers" as csv_utils;

    let body = "";
    let current_struct_inline_enums = [];
    for it in s.items { if it.is_inline_enum() { let e = it.as_inline_enum(); current_struct_inline_enums.push(e.name); } }

    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            let tf = f.field_type; let base = if tf.is_option { tf.inner } else { tf };

            if tf.is_option {
                let inner = base;
                body += "        {\n";
                if inner.lang_type == "string" {
                    body += "            obj." + f.name + " = BinaryUtils.ReadOption<string>(br, BinaryUtils.ReadUtf8String);\n";
                } else if inner.is_primitive {
                    let k = inner.lang_type; let cs = type_utils::map_type(inner);
                    body += "            obj." + f.name + " = BinaryUtils.ReadOption<" + cs + ">(br, _ => " + read_utils::read_expr(k, "br") + ");\n";
                } else if inner.is_enum || current_struct_inline_enums.contains(inner.type_name) {
                    let enum_type = inner.fqn;
                    body += "            obj." + f.name + " = BinaryUtils.ReadOption<" + enum_type + ">(br, BinaryUtils.ReadEnumInt32<" + enum_type + ">);\n";
                } else {
                    let es_nl = csv_utils::find_embedded_struct(s, inner.type_name);
                    let call_ns = if es_nl != () { current_ns_name } else { if inner.namespace_fqn == "" { current_ns_name } else { inner.namespace_fqn } };
                    let call_ns_path = if call_ns == "" { "Reader" } else { "Reader." + call_ns };
                    body += "            obj." + f.name + " = BinaryUtils.ReadOption<global::" + inner.fqn + ">(br, " + call_ns_path + "." + inner.type_name + ".ReadBinary);\n";
                }
                body += "        }\n";
                continue;
            }

            if base.is_list {
                let inner = base.inner;
                body += "        {\n";
                if inner.lang_type == "string" {
                    body += "            obj." + f.name + " = BinaryUtils.ReadList<string>(br, BinaryUtils.ReadUtf8String);\n";
                } else if inner.is_primitive {
                    let k = inner.lang_type; let cs = type_utils::map_type(inner);
                    body += "            obj." + f.name + " = BinaryUtils.ReadList<" + cs + ">(br, _ => " + read_utils::read_expr(k, "br") + ");\n";
                } else if inner.is_enum || current_struct_inline_enums.contains(inner.type_name) {
                    let enum_type = inner.fqn;
                    body += "            obj." + f.name + " = BinaryUtils.ReadList<" + enum_type + ">(br, BinaryUtils.ReadEnumInt32<" + enum_type + ">);\n";
                } else {
                    let es_nl = csv_utils::find_embedded_struct(s, inner.type_name);
                    let call_ns = if es_nl != () { current_ns_name } else { if inner.namespace_fqn == "" { current_ns_name } else { inner.namespace_fqn } };
                    let call_ns_path = if call_ns == "" { "Reader" } else { "Reader." + call_ns };
                    body += "            obj." + f.name + " = BinaryUtils.ReadList<global::" + inner.fqn + ">(br, " + call_ns_path + "." + inner.type_name + ".ReadBinary);\n";
                }
                body += "        }\n";
                continue;
            }

            if base.lang_type == "string" {
                body += "        obj." + f.name + " = BinaryUtils.ReadUtf8String(br);\n";
            } else if base.is_enum || current_struct_inline_enums.contains(base.type_name) {
                let enum_type = base.fqn;
                body += "        obj." + f.name + " = BinaryUtils.ReadEnumInt32<" + enum_type + ">(br);\n";
            } else if base.is_primitive {
                body += "        obj." + f.name + " = " + read_utils::read_expr(base.lang_type, "br") + ";\n";
            } else {
                let es_nl = csv_utils::find_embedded_struct(s, base.type_name);
                let call_ns = if es_nl != () { current_ns_name } else { if base.namespace_fqn == "" { current_ns_name } else { base.namespace_fqn } };
                let call_ns_path = if call_ns == "" { "Reader" } else { "Reader." + call_ns };
                body += "        obj." + f.name + " = " + call_ns_path + "." + base.type_name + ".ReadBinary(br);\n";
            }
        } else if item.is_comment() {
            // Skip schema comments in Reader outputs
        } else if item.is_embedded_struct() {
            // no-op for codegen body
        } else if item.is_inline_enum() {
            // no-op for codegen body
        }
    }
    body
}
