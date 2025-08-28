${
    import "templates/csharp/rhai_utils/type_mapping" as type_utils;

    let body = "";
    // inline enums in this struct
    let current_struct_inline_enums = [];
    for it in s.items { if it.is_inline_enum() { let e = it.as_inline_enum(); current_struct_inline_enums.push(e.name); } }

    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            let t = f.field_type;

            // Option<T>
            if t.starts_with("Option<") {
                let __s = "Option<".len();
                let __l = t.len() - __s - 1;
                let inner = t.sub_string(__s, __l);
                if inner == "string" {
                    body += "        BinaryUtils.WriteOptionRef<string>(bw, obj." + f.name + ", BinaryUtils.WriteUtf8String);\n";
                } else if inner.starts_with("List<") {
                    // Option<List<T>>
                    let lst_start = "List<".len();
                    let il = inner.len() - lst_start - 1;
                    let inner2 = inner.sub_string(lst_start, il);
                    let parts = inner2.split(".");
                    let elem_short = parts[parts.len-1];
                    let elem_ns = if inner2.contains(".") { let __ns = ""; let __i=0; for seg in parts { if __i < parts.len-1 { if __ns != "" { __ns += "."; } __ns += seg; } __i+=1; } __ns } else { current_ns_name };
                    if inner2 == "string" {
                        body += "        BinaryUtils.WriteOptionRef<IList<string>>(bw, obj." + f.name + ", (bw2, list) => BinaryUtils.WriteList<string>(bw2, list, BinaryUtils.WriteUtf8String));\n";
                    } else if inner2.ends_with("__Enum") || current_struct_inline_enums.contains(inner2) || current_ns_enums.contains(inner2) {
                        let enum_type = if inner2.ends_with("__Enum") { s.name + "." + inner2 } else { inner2 };
                        body += "        BinaryUtils.WriteOptionRef<IList<" + enum_type + ">>(bw, obj." + f.name + ", (bw2, list) => BinaryUtils.WriteList<" + enum_type + ">(bw2, list, BinaryUtils.WriteEnumInt32<" + enum_type + ">));\n";
                    } else if inner2 == "u8" || inner2 == "i8" || inner2 == "u16" || inner2 == "i16" || inner2 == "u32" || inner2 == "i32" || inner2 == "u64" || inner2 == "i64" || inner2 == "f32" || inner2 == "f64" || inner2 == "bool" {
                        let mapped = type_utils::map_type(inner2);
                        let write_expr = switch inner2 {
                            "u8" => "(bw2,v)=>bw2.Write(v)",
                            "i8" => "(bw2,v)=>bw2.Write(v)",
                            "u16" => "(bw2,v)=>bw2.Write(v)",
                            "i16" => "(bw2,v)=>bw2.Write(v)",
                            "u32" => "(bw2,v)=>bw2.Write(v)",
                            "i32" => "(bw2,v)=>bw2.Write(v)",
                            "u64" => "(bw2,v)=>bw2.Write(v)",
                            "i64" => "(bw2,v)=>bw2.Write(v)",
                            "f32" => "(bw2,v)=>bw2.Write(v)",
                            "f64" => "(bw2,v)=>bw2.Write(v)",
                            "bool" => "(bw2,v)=>bw2.Write(v)",
                            _ => "(bw2,v)=>{ }"
                        };
                        body += "        BinaryUtils.WriteOptionRef<IList<" + mapped + ">>(bw, obj." + f.name + ", (bw2, list) => BinaryUtils.WriteList<" + mapped + ">(bw2, list, " + write_expr + "));\n";
                    } else {
                        body += "        BinaryUtils.WriteOptionRef<IList<" + inner2 + ">>(bw, obj." + f.name + ", (bw2, list) => BinaryUtils.WriteList<" + inner2 + ">(bw2, list, " + elem_ns + ".BinaryWriters.Write" + elem_short + "));\n";
                    }
                } else if inner.ends_with("__Enum") || current_struct_inline_enums.contains(inner) || current_ns_enums.contains(inner) {
                    let enum_type = if inner.ends_with("__Enum") { s.name + "." + inner } else { inner };
                    body += "        BinaryUtils.WriteOptionStruct<" + enum_type + ">(bw, obj." + f.name + ", BinaryUtils.WriteEnumInt32<" + enum_type + ">);\n";
                } else if inner == "u8" || inner == "i8" || inner == "u16" || inner == "i16" || inner == "u32" || inner == "i32" || inner == "u64" || inner == "i64" || inner == "f32" || inner == "f64" || inner == "bool" {
                    let mapped = type_utils::map_type(inner);
                    body += "        BinaryUtils.WriteOptionStruct<" + mapped + ">(bw, obj." + f.name + ", (bw2,v)=>bw2.Write(v));\n";
                } else {
                    let parts = inner.split(".");
                    let call_type = parts[parts.len-1];
                    let call_ns = if inner.contains(".") { let __ns = ""; let __i=0; for seg in parts { if __i < parts.len-1 { if __ns != "" { __ns += "."; } __ns += seg; } __i+=1; } __ns } else { current_ns_name };
                    body += "        BinaryUtils.WriteOptionRef<" + inner + ">(bw, obj." + f.name + ", " + call_ns + ".BinaryWriters.Write" + call_type + ");\n";
                }
                continue;
            }

            // List<T>
            if t.starts_with("List<") {
                let __s = "List<".len();
                let __l = t.len() - __s - 1;
                let inner = t.sub_string(__s, __l);
                if inner == "string" {
                    body += "        BinaryUtils.WriteList<string>(bw, obj." + f.name + ", BinaryUtils.WriteUtf8String);\n";
                } else if inner.ends_with("__Enum") || current_struct_inline_enums.contains(inner) || current_ns_enums.contains(inner) {
                    let enum_type = if inner.ends_with("__Enum") { s.name + "." + inner } else { inner };
                    body += "        BinaryUtils.WriteList<" + enum_type + ">(bw, obj." + f.name + ", BinaryUtils.WriteEnumInt32<" + enum_type + ">);\n";
                } else if inner == "u8" || inner == "i8" || inner == "u16" || inner == "i16" || inner == "u32" || inner == "i32" || inner == "u64" || inner == "i64" || inner == "f32" || inner == "f64" || inner == "bool" {
                    let mapped = type_utils::map_type(inner);
                    body += "        BinaryUtils.WriteList<" + mapped + ">(bw, obj." + f.name + ", (bw2,v)=>bw2.Write(v));\n";
                } else {
                    let parts = inner.split(".");
                    let call_type = parts[parts.len-1];
                    let call_ns = if inner.contains(".") { let __ns = ""; let __i=0; for seg in parts { if __i < parts.len-1 { if __ns != "" { __ns += "."; } __ns += seg; } __i+=1; } __ns } else { current_ns_name };
                    body += "        BinaryUtils.WriteList<" + inner + ">(bw, obj." + f.name + ", " + call_ns + ".BinaryWriters.Write" + call_type + ");\n";
                }
                continue;
            }

            // string
            if t == "string" {
                body += "        BinaryUtils.WriteUtf8String(bw, obj." + f.name + ");\n";
            }
            // enum (inline or same-namespace)
            else if t.ends_with("__Enum") || current_struct_inline_enums.contains(t) || current_ns_enums.contains(t) {
                let enum_type = if t.ends_with("__Enum") { s.name + "." + t } else { t };
                body += "        BinaryUtils.WriteEnumInt32<" + enum_type + ">(bw, obj." + f.name + ");\n";
            }
            // primitives
            else if t == "u8" || t == "i8" || t == "u16" || t == "i16" || t == "u32" || t == "i32" || t == "u64" || t == "i64" || t == "f32" || t == "f64" || t == "bool" {
                body += "        bw.Write(obj." + f.name + ");\n";
            }
            // custom class
            else {
                let parts = t.split(".");
                let call_type = parts[parts.len-1];
                let call_ns = if t.contains(".") { let __ns = ""; let __i=0; for seg in parts { if __i < parts.len-1 { if __ns != "" { __ns += "."; } __ns += seg; } __i+=1; } __ns } else { current_ns_name };
                body += "        " + call_ns + ".BinaryWriters.Write" + call_type + "(bw, obj." + f.name + ");\n";
            }
        }
    }
    body
}
