${
    let body = "";
    let current_struct_inline_enums = [];
    for it in s.items { if it.is_inline_enum() { let e = it.as_inline_enum(); current_struct_inline_enums.push(e.name); } }

    body += "        w.WriteStartObject();\n";
    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            let tf = f.field_type; let base = if tf.is_option { tf.inner } else { tf };
            let prop = f.name;

            if tf.is_option {
                let inner = base;
                if inner.lang_type == "string" {
                    body += "        JsonUtils.WriteOptionRef<string>(w, \"" + prop + "\", obj." + f.name + ", (w2,v)=> w2.WriteStringValue(v));\n";
                } else if inner.is_primitive {
                    let mapped = cs_map_type(inner);
                    let write_expr = switch inner.lang_type {
                        "u8"=>"(w2,v)=>w2.WriteNumberValue(v)", "i8"=>"(w2,v)=>w2.WriteNumberValue(v)",
                        "u16"=>"(w2,v)=>w2.WriteNumberValue(v)", "i16"=>"(w2,v)=>w2.WriteNumberValue(v)",
                        "u32"=>"(w2,v)=>w2.WriteNumberValue(v)", "i32"=>"(w2,v)=>w2.WriteNumberValue(v)",
                        "u64"=>"(w2,v)=>w2.WriteNumberValue(v)", "i64"=>"(w2,v)=>w2.WriteNumberValue(v)",
                        "f32"=>"(w2,v)=>w2.WriteNumberValue(v)", "f64"=>"(w2,v)=>w2.WriteNumberValue(v)",
                        "bool"=>"(w2,v)=>w2.WriteBooleanValue(v)", _=>"(w2,v)=>{}"
                    };
                    body += "        JsonUtils.WriteOptionStruct<" + mapped + ">(w, \"" + prop + "\", obj." + f.name + ", " + write_expr + ");\n";
                } else if inner.is_enum || current_struct_inline_enums.contains(inner.type_name) {
                    let enum_type = if inner.is_enum { s.name + "." + inner.type_name } else { inner.fqn };
                    body += "        JsonUtils.WriteOptionStruct<" + enum_type + ">(w, \"" + prop + "\", obj." + f.name + ", JsonUtils.WriteEnum<" + enum_type + ">);\n";
                } else {
                    let call_ns = if inner.namespace_fqn == "" { current_ns_name } else { inner.namespace_fqn };
                    body += "        JsonUtils.WriteOptionRef<" + inner.type_name + ">(w, \"" + prop + "\", obj." + f.name + ", " + call_ns + ".JsonMappers.Write" + inner.type_name + ");\n";
                }
                continue;
            }

            if base.is_list {
                let inner = base.inner;
                if inner.lang_type == "string" {
                    body += "        JsonUtils.WriteList<string>(w, \"" + prop + "\", obj." + f.name + ", (w2,v)=>w2.WriteStringValue(v));\n";
                } else if inner.is_primitive && inner.lang_type == "bool" {
                    body += "        JsonUtils.WriteList<bool>(w, \"" + prop + "\", obj." + f.name + ", (w2,v)=>w2.WriteBooleanValue(v));\n";
                } else if inner.is_primitive {
                    let mapped = cs_map_type(inner);
                    body += "        JsonUtils.WriteList<" + mapped + ">(w, \"" + prop + "\", obj." + f.name + ", (w2,v)=>w2.WriteNumberValue(v));\n";
                } else if inner.is_enum || current_struct_inline_enums.contains(inner.type_name) {
                    let enum_type = if inner.is_enum { s.name + "." + inner.type_name } else { inner.fqn };
                    body += "        JsonUtils.WriteList<" + enum_type + ">(w, \"" + prop + "\", obj." + f.name + ", JsonUtils.WriteEnum<" + enum_type + ">);\n";
                } else {
                    let call_ns = if inner.namespace_fqn == "" { current_ns_name } else { inner.namespace_fqn };
                    body += "        JsonUtils.WriteList<" + inner.type_name + ">(w, \"" + prop + "\", obj." + f.name + ", " + call_ns + ".JsonMappers.Write" + inner.type_name + ");\n";
                }
                continue;
            }

            if base.lang_type == "string" { body += "        w.WriteString(\"" + prop + "\", obj." + f.name + ");\n"; }
            else if base.is_primitive && base.lang_type == "bool" { body += "        w.WriteBoolean(\"" + prop + "\", obj." + f.name + ");\n"; }
            else if base.is_primitive { body += "        w.WriteNumber(\"" + prop + "\", obj." + f.name + ");\n"; }
            else if base.is_enum || current_struct_inline_enums.contains(base.type_name) {
                let enum_type = if base.is_enum { s.name + "." + base.type_name } else { base.fqn };
                body += "        w.WritePropertyName(\"" + prop + "\"); JsonUtils.WriteEnum<" + enum_type + ">(w, obj." + f.name + ");\n";
            } else {
                let call_ns = if base.namespace_fqn == "" { current_ns_name } else { base.namespace_fqn };
                body += "        w.WritePropertyName(\"" + prop + "\"); " + call_ns + ".JsonMappers.Write" + base.type_name + "(w, obj." + f.name + ");\n";
            }
        }
    }
    body += "        w.WriteEndObject();\n}";
    body
}

