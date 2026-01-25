${
    import "templates/csharp/rhai_utils/binary_mapping" as binary_utils;
    import "templates/csharp/rhai_utils/type_utils" as type_utils;

    let body = "";
    // inline enums in this struct
    let current_struct_inline_enums = [];
    for it in s.items { if it.is_inline_enum() { let e = it.as_inline_enum(); current_struct_inline_enums.push(e.name); } }

    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            let tf = f.field_type; let base = if tf.is_option { tf.inner } else { tf };

            // Option<T>
            if tf.is_option {
                let inner = base;
                if inner.lang_type == "string" {
                    body += "        BinaryUtils.WriteOptionRef<string>(bw, obj." + f.name + ", BinaryUtils.WriteUtf8String);\n";
                } else if inner.is_list {
                    let elem = inner.inner;
                    if elem.lang_type == "string" {
                        body += "        BinaryUtils.WriteOptionRef<IList<string>>(bw, obj." + f.name + ", (bw2, list) => BinaryUtils.WriteList<string>(bw2, list, BinaryUtils.WriteUtf8String));\n";
                    } else if elem.is_enum || current_struct_inline_enums.contains(elem.type_name) {
                        let enum_type = if elem.is_enum { s.name + "." + elem.type_name } else { elem.fqn };
                        body += "        BinaryUtils.WriteOptionRef<IList<" + enum_type + ">>(bw, obj." + f.name + ", (bw2, list) => BinaryUtils.WriteList<" + enum_type + ">(bw2, list, BinaryUtils.WriteEnumInt32<" + enum_type + ">));\n";
                    } else if elem.is_primitive {
                        let mapped = cs_map_type(elem);
                        let write_expr = "(bw2,v)=>bw2.Write(v)";
                        body += "        BinaryUtils.WriteOptionRef<IList<" + mapped + ">>(bw, obj." + f.name + ", (bw2, list) => BinaryUtils.WriteList<" + mapped + ">(bw2, list, " + write_expr + "));\n";
                } else {
                    let es_nl = type_utils::find_embedded_struct(s, elem.type_name);
                    let call_ns = if es_nl != () { current_ns_name } else { if elem.namespace_fqn == "" { current_ns_name } else { elem.namespace_fqn } };
                    body += "        BinaryUtils.WriteOptionRef<IList<global::" + elem.fqn + ">>( bw, obj." + f.name + ", (bw2, list) => BinaryUtils.WriteList<global::" + elem.fqn + ">(bw2, list, " + call_ns + ".BinaryWriters.Write" + elem.type_name + "));\n";
                }
                } else if inner.is_enum || current_struct_inline_enums.contains(inner.type_name) {
                    let enum_type = inner.fqn;
                    body += "        BinaryUtils.WriteOptionStruct<" + enum_type + ">(bw, obj." + f.name + ", BinaryUtils.WriteEnumInt32<" + enum_type + ">);\n";
                } else if inner.lang_type == "timestamp" {
                    body += "        BinaryUtils.WriteOptionStruct<DateTime>(bw, obj." + f.name + ", (bw2,v)=>bw2.Write(v.Ticks));\n";
                } else if inner.is_primitive {
                    let mapped = cs_map_type(inner);
                    body += "        BinaryUtils.WriteOptionStruct<" + mapped + ">(bw, obj." + f.name + ", (bw2,v)=>bw2.Write(v));\n";
                } else {
                    let es_nl = type_utils::find_embedded_struct(s, inner.type_name);
                    let call_ns = if es_nl != () { current_ns_name } else { if inner.namespace_fqn == "" { current_ns_name } else { inner.namespace_fqn } };
                    body += "        BinaryUtils.WriteOptionRef<global::" + inner.fqn + ">(bw, obj." + f.name + ", " + call_ns + ".BinaryWriters.Write" + inner.type_name + ");\n";
                }
                continue;
            }

            // List<T>
            if base.is_list {
                let inner = base.inner;
                if inner.lang_type == "string" {
                    body += "        BinaryUtils.WriteList<string>(bw, obj." + f.name + ", BinaryUtils.WriteUtf8String);\n";
                } else if inner.is_enum || current_struct_inline_enums.contains(inner.type_name) {
                    let enum_type = inner.fqn;
                    body += "        BinaryUtils.WriteList<" + enum_type + ">(bw, obj." + f.name + ", BinaryUtils.WriteEnumInt32<" + enum_type + ">);\n";
                } else if inner.lang_type == "timestamp" {
                    body += "        BinaryUtils.WriteList<DateTime>(bw, obj." + f.name + ", (bw2,v)=>bw2.Write(v.Ticks));\n";
                } else if inner.is_primitive {
                    let mapped = cs_map_type(inner);
                    body += "        BinaryUtils.WriteList<" + mapped + ">(bw, obj." + f.name + ", (bw2,v)=>bw2.Write(v));\n";
                } else {
                    let es_nl = type_utils::find_embedded_struct(s, inner.type_name);
                    let call_ns = if es_nl != () { current_ns_name } else { if inner.namespace_fqn == "" { current_ns_name } else { inner.namespace_fqn } };
                    body += "        BinaryUtils.WriteList<global::" + inner.fqn + ">(bw, obj." + f.name + ", " + call_ns + ".BinaryWriters.Write" + inner.type_name + ");\n";
                }
                continue;
            }

            // string
            if base.lang_type == "string" {
                body += "        BinaryUtils.WriteUtf8String(bw, obj." + f.name + ");\n";
            }
            // bytes
            else if base.lang_type == "bytes" {
                body += "        BinaryUtils.WriteBytes(bw, obj." + f.name + ");\n";
            }
            // timestamp
            else if base.lang_type == "timestamp" {
                body += "        bw.Write(obj." + f.name + ".Ticks);\n";
            }
            // enum (inline or same-namespace)
            else if base.is_enum || current_struct_inline_enums.contains(base.type_name) {
                let enum_type = base.fqn;
                body += "        BinaryUtils.WriteEnumInt32<" + enum_type + ">(bw, obj." + f.name + ");\n";
            }
            // primitives
            else if base.is_primitive {
                body += "        bw.Write(obj." + f.name + ");\n";
            }
            // custom class (non-optional)
            else {
                let es_nl = type_utils::find_embedded_struct(s, base.type_name);
                let call_ns = if es_nl != () { current_ns_name } else { if base.namespace_fqn == "" { current_ns_name } else { base.namespace_fqn } };
                body += "        " + call_ns + ".BinaryWriters.Write" + base.type_name + "(bw, obj." + f.name + ");\n";
            }
        }
    }
    body
}
