${
    import "templates/csharp/rhai_utils/type_mapping" as type_utils;

    let body = "";
    let current_struct_inline_enums = [];
    for it in s.items { if it.is_inline_enum() { let e = it.as_inline_enum(); current_struct_inline_enums.push(e.name); } }

    body += "        w.WriteStartObject();\n";
    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            let t = f.field_type.original;
            let prop = f.name;

            if t.starts_with("Option<") {
                let __s = "Option<".len(); let __l = t.len() - __s - 1; let inner = t.sub_string(__s, __l);
                if inner == "string" {
                    body += "        JsonUtils.WriteOptionRef<string>(w, \"" + prop + "\", obj." + f.name + ", (w2,v)=> w2.WriteStringValue(v));\n";
                } else if inner == "u8" || inner == "i8" || inner == "u16" || inner == "i16" || inner == "u32" || inner == "i32" || inner == "u64" || inner == "i64" || inner == "f32" || inner == "f64" || inner == "bool" {
                    let mapped = type_utils::map_type(inner);
                    let write_expr = switch inner {
                        "u8"=>"(w2,v)=>w2.WriteNumberValue(v)", "i8"=>"(w2,v)=>w2.WriteNumberValue(v)",
                        "u16"=>"(w2,v)=>w2.WriteNumberValue(v)", "i16"=>"(w2,v)=>w2.WriteNumberValue(v)",
                        "u32"=>"(w2,v)=>w2.WriteNumberValue(v)", "i32"=>"(w2,v)=>w2.WriteNumberValue(v)",
                        "u64"=>"(w2,v)=>w2.WriteNumberValue(v)", "i64"=>"(w2,v)=>w2.WriteNumberValue(v)",
                        "f32"=>"(w2,v)=>w2.WriteNumberValue(v)", "f64"=>"(w2,v)=>w2.WriteNumberValue(v)",
                        "bool"=>"(w2,v)=>w2.WriteBooleanValue(v)", _=>"(w2,v)=>{}"
                    };
                    body += "        JsonUtils.WriteOptionStruct<" + mapped + ">(w, \"" + prop + "\", obj." + f.name + ", " + write_expr + ");\n";
                } else if inner.ends_with("__Enum") || current_ns_enums.contains(inner) || current_struct_inline_enums.contains(inner) {
                    let enum_type = if inner.ends_with("__Enum") { s.name + "." + inner } else { inner };
                    body += "        JsonUtils.WriteOptionStruct<" + enum_type + ">(w, \"" + prop + "\", obj." + f.name + ", JsonUtils.WriteEnum<" + enum_type + ">);\n";
                } else {
                    let parts = inner.split("."); let call_type = parts[parts.len-1];
                    let call_ns = if inner.contains(".") { let __ns = ""; let __i=0; for seg in parts { if __i < parts.len-1 { if __ns != "" { __ns += "."; } __ns += seg; } __i+=1; } __ns } else { current_ns_name };
                    body += "        JsonUtils.WriteOptionRef<" + inner + ">(w, \"" + prop + "\", obj." + f.name + ", " + call_ns + ".JsonMappers.Write" + call_type + ");\n";
                }
                continue;
            }

            if t.starts_with("List<") {
                let __s = "List<".len(); let __l = t.len() - __s - 1; let inner = t.sub_string(__s, __l);
                if inner == "string" {
                    body += "        JsonUtils.WriteList<string>(w, \"" + prop + "\", obj." + f.name + ", (w2,v)=>w2.WriteStringValue(v));\n";
                } else if inner == "u8" || inner == "i8" || inner == "u16" || inner == "i16" || inner == "u32" || inner == "i32" || inner == "u64" || inner == "i64" || inner == "f32" || inner == "f64" {
                    body += "        JsonUtils.WriteList<" + inner + ">(w, \"" + prop + "\", obj." + f.name + ", (w2,v)=>w2.WriteNumberValue(v));\n";
                } else if inner == "bool" {
                    body += "        JsonUtils.WriteList<bool>(w, \"" + prop + "\", obj." + f.name + ", (w2,v)=>w2.WriteBooleanValue(v));\n";
                } else if inner.ends_with("__Enum") || current_ns_enums.contains(inner) || current_struct_inline_enums.contains(inner) {
                    let enum_type = if inner.ends_with("__Enum") { s.name + "." + inner } else { inner };
                    body += "        JsonUtils.WriteList<" + enum_type + ">(w, \"" + prop + "\", obj." + f.name + ", JsonUtils.WriteEnum<" + enum_type + ">);\n";
                } else {
                    let parts = inner.split("."); let call_type = parts[parts.len-1];
                    let call_ns = if inner.contains(".") { let __ns = ""; let __i=0; for seg in parts { if __i < parts.len-1 { if __ns != "" { __ns += "."; } __ns += seg; } __i+=1; } __ns } else { current_ns_name };
                    body += "        JsonUtils.WriteList<" + inner + ">(w, \"" + prop + "\", obj." + f.name + ", " + call_ns + ".JsonMappers.Write" + call_type + ");\n";
                }
                continue;
            }

            if t == "string" { body += "        w.WriteString(\"" + prop + "\", obj." + f.name + ");\n"; }
            else if t == "u8" || t == "i8" || t == "u16" || t == "i16" || t == "u32" || t == "i32" || t == "u64" || t == "i64" || t == "f32" || t == "f64" { body += "        w.WriteNumber(\"" + prop + "\", obj." + f.name + ");\n"; }
            else if t == "bool" { body += "        w.WriteBoolean(\"" + prop + "\", obj." + f.name + ");\n"; }
            else if t.ends_with("__Enum") || current_ns_enums.contains(t) || current_struct_inline_enums.contains(t) {
                let enum_type = if t.ends_with("__Enum") { s.name + "." + t } else { t };
                body += "        w.WritePropertyName(\"" + prop + "\"); JsonUtils.WriteEnum<" + enum_type + ">(w, obj." + f.name + ");\n";
            } else {
                let parts = t.split("."); let call_type = parts[parts.len-1];
                let call_ns = if t.contains(".") { let __ns = ""; let __i=0; for seg in parts { if __i < parts.len-1 { if __ns != "" { __ns += "."; } __ns += seg; } __i+=1; } __ns } else { current_ns_name };
                body += "        w.WritePropertyName(\"" + prop + "\"); " + call_ns + ".JsonMappers.Write" + call_type + "(w, obj." + f.name + ");\n";
            }
        }
    }
    body += "        w.WriteEndObject();\n}";
    body
}
