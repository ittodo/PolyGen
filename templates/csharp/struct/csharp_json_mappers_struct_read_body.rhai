${
    import "templates/csharp/rhai_utils/type_mapping" as type_utils;

    let body = "";
    let current_struct_inline_enums = [];
    for it in s.items { if it.is_inline_enum() { let e = it.as_inline_enum(); current_struct_inline_enums.push(e.name); } }

    body += "        var obj = new " + s.name + "();\n";
    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            let t = f.field_type.original;
            let prop = f.name;

            if t.starts_with("Option<") {
                let __s = "Option<".len(); let __l = t.len() - __s - 1; let inner = t.sub_string(__s, __l);
                body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p)) {\n";
                body += "            if (__p.ValueKind != JsonValueKind.Null) {\n";
                if inner == "string" { body += "                obj." + f.name + " = __p.GetString();\n"; }
                else if inner == "u8" { body += "                obj." + f.name + " = __p.GetByte();\n"; }
                else if inner == "i8" { body += "                obj." + f.name + " = __p.GetSByte();\n"; }
                else if inner == "u16" { body += "                obj." + f.name + " = __p.GetUInt16();\n"; }
                else if inner == "i16" { body += "                obj." + f.name + " = __p.GetInt16();\n"; }
                else if inner == "u32" { body += "                obj." + f.name + " = __p.GetUInt32();\n"; }
                else if inner == "i32" { body += "                obj." + f.name + " = __p.GetInt32();\n"; }
                else if inner == "u64" { body += "                obj." + f.name + " = __p.GetUInt64();\n"; }
                else if inner == "i64" { body += "                obj." + f.name + " = __p.GetInt64();\n"; }
                else if inner == "f32" { body += "                obj." + f.name + " = __p.GetSingle();\n"; }
                else if inner == "f64" { body += "                obj." + f.name + " = __p.GetDouble();\n"; }
                else if inner == "bool" { body += "                obj." + f.name + " = __p.GetBoolean();\n"; }
                else if inner.ends_with("__Enum") || current_ns_enums.contains(inner) || current_struct_inline_enums.contains(inner) {
                    let enum_type = if inner.ends_with("__Enum") { s.name + "." + inner } else { inner };
                    body += "                obj." + f.name + " = JsonUtils.ReadEnum<" + enum_type + ">(__p);\n";
                } else {
                    let parts = inner.split("."); let call_type = parts[parts.len-1];
                    let call_ns = if inner.contains(".") { let __ns = ""; let __i=0; for seg in parts { if __i < parts.len-1 { if __ns != "" { __ns += "."; } __ns += seg; } __i+=1; } __ns } else { current_ns_name };
                    body += "                obj." + f.name + " = " + call_ns + ".JsonMappers.Read" + call_type + "(__p);\n";
                }
                body += "            }\n";
                body += "        }\n";
                continue;
            }

            if t.starts_with("List<") {
                let __s = "List<".len(); let __l = t.len() - __s - 1; let inner = t.sub_string(__s, __l);
                body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p) && __p.ValueKind == JsonValueKind.Array) {\n";
                if inner == "string" { body += "            obj." + f.name + " = JsonUtils.ReadList<string>(__p, el => el.GetString()!);\n"; }
                else if inner == "u8" { body += "            obj." + f.name + " = JsonUtils.ReadList<byte>(__p, el => el.GetByte());\n"; }
                else if inner == "i8" { body += "            obj." + f.name + " = JsonUtils.ReadList<sbyte>(__p, el => el.GetSByte());\n"; }
                else if inner == "u16" { body += "            obj." + f.name + " = JsonUtils.ReadList<ushort>(__p, el => el.GetUInt16());\n"; }
                else if inner == "i16" { body += "            obj." + f.name + " = JsonUtils.ReadList<short>(__p, el => el.GetInt16());\n"; }
                else if inner == "u32" { body += "            obj." + f.name + " = JsonUtils.ReadList<uint>(__p, el => el.GetUInt32());\n"; }
                else if inner == "i32" { body += "            obj." + f.name + " = JsonUtils.ReadList<int>(__p, el => el.GetInt32());\n"; }
                else if inner == "u64" { body += "            obj." + f.name + " = JsonUtils.ReadList<ulong>(__p, el => el.GetUInt64());\n"; }
                else if inner == "i64" { body += "            obj." + f.name + " = JsonUtils.ReadList<long>(__p, el => el.GetInt64());\n"; }
                else if inner == "f32" { body += "            obj." + f.name + " = JsonUtils.ReadList<float>(__p, el => el.GetSingle());\n"; }
                else if inner == "f64" { body += "            obj." + f.name + " = JsonUtils.ReadList<double>(__p, el => el.GetDouble());\n"; }
                else if inner == "bool" { body += "            obj." + f.name + " = JsonUtils.ReadList<bool>(__p, el => el.GetBoolean());\n"; }
                else if inner.ends_with("__Enum") || current_ns_enums.contains(inner) || current_struct_inline_enums.contains(inner) {
                    let enum_type = if inner.ends_with("__Enum") { s.name + "." + inner } else { inner };
                    body += "            obj." + f.name + " = JsonUtils.ReadList<" + enum_type + ">(__p, el => JsonUtils.ReadEnum<" + enum_type + ">(el));\n";
                } else {
                    let parts = inner.split("."); let call_type = parts[parts.len-1];
                    let call_ns = if inner.contains(".") { let __ns = ""; let __i=0; for seg in parts { if __i < parts.len-1 { if __ns != "" { __ns += "."; } __ns += seg; } __i+=1; } __ns } else { current_ns_name };
                    body += "            obj." + f.name + " = JsonUtils.ReadList<" + inner + ">(__p, el => " + call_ns + ".JsonMappers.Read" + call_type + "(el));\n";
                }
                body += "        }\n";
                continue;
            }

            if t == "string" { body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p) && __p.ValueKind != JsonValueKind.Null) obj." + f.name + " = __p.GetString();\n"; }
            else if t == "u8" { body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p)) obj." + f.name + " = __p.GetByte();\n"; }
            else if t == "i8" { body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p)) obj." + f.name + " = __p.GetSByte();\n"; }
            else if t == "u16" { body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p)) obj." + f.name + " = __p.GetUInt16();\n"; }
            else if t == "i16" { body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p)) obj." + f.name + " = __p.GetInt16();\n"; }
            else if t == "u32" { body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p)) obj." + f.name + " = __p.GetUInt32();\n"; }
            else if t == "i32" { body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p)) obj." + f.name + " = __p.GetInt32();\n"; }
            else if t == "u64" { body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p)) obj." + f.name + " = __p.GetUInt64();\n"; }
            else if t == "i64" { body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p)) obj." + f.name + " = __p.GetInt64();\n"; }
            else if t == "f32" { body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p)) obj." + f.name + " = __p.GetSingle();\n"; }
            else if t == "f64" { body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p)) obj." + f.name + " = __p.GetDouble();\n"; }
            else if t == "bool" { body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p)) obj." + f.name + " = __p.GetBoolean();\n"; }
            else if t.ends_with("__Enum") || current_ns_enums.contains(t) || current_struct_inline_enums.contains(t) {
                let enum_type = if t.ends_with("__Enum") { s.name + "." + t } else { t };
                body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p)) obj." + f.name + " = JsonUtils.ReadEnum<" + enum_type + ">(__p);\n";
            } else {
                let parts = t.split("."); let call_type = parts[parts.len-1];
                let call_ns = if t.contains(".") { let __ns = ""; let __i=0; for seg in parts { if __i < parts.len-1 { if __ns != "" { __ns += "."; } __ns += seg; } __i+=1; } __ns } else { current_ns_name };
                body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p) && __p.ValueKind == JsonValueKind.Object) obj." + f.name + " = " + call_ns + ".JsonMappers.Read" + call_type + "(__p);\n";
            }
        }
    }
    body
}
