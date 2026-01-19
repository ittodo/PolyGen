${
    let body = "";
    let current_struct_inline_enums = [];
    for it in s.items { if it.is_inline_enum() { let e = it.as_inline_enum(); current_struct_inline_enums.push(e.name); } }

    body += "        var obj = new " + s.name + "();\n";
    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            let tf = f.field_type; let base = if tf.is_option { tf.inner } else { tf };
            let prop = f.name;

            if tf.is_option {
                let inner = base;
                body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p)) {\n";
                body += "            if (__p.ValueKind != JsonValueKind.Null) {\n";
                if inner.is_primitive {
                    let k = inner.lang_type; // primitive tag
                    let g = switch k { "string"=>"GetString()", "u8"=>"GetByte()", "i8"=>"GetSByte()", "u16"=>"GetUInt16()", "i16"=>"GetInt16()", "u32"=>"GetUInt32()", "i32"=>"GetInt32()", "u64"=>"GetUInt64()", "i64"=>"GetInt64()", "f32"=>"GetSingle()", "f64"=>"GetDouble()", "bool"=>"GetBoolean()", _=>"GetString()" };
                    body += "                obj." + f.name + " = __p." + g + ";\n";
                } else if inner.is_enum || current_struct_inline_enums.contains(inner.type_name) {
                    let enum_type = if inner.is_enum { s.name + "." + inner.type_name } else { inner.fqn };
                    body += "                obj." + f.name + " = JsonUtils.ReadEnum<" + enum_type + ">(__p);\n";
                } else {
                    let call_ns = if inner.namespace_fqn == "" { current_ns_name } else { inner.namespace_fqn };
                    body += "                obj." + f.name + " = " + call_ns + ".JsonMappers.Read" + inner.type_name + "(__p);\n";
                }
                body += "            }\n";
                body += "        }\n";
                continue;
            }

            if base.is_list {
                let inner = base.inner;
                body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p) && __p.ValueKind == JsonValueKind.Array) {\n";
                if inner.is_primitive {
                    let k = inner.lang_type;
                    let cs = switch k { "string"=>"string", "u8"=>"byte", "i8"=>"sbyte", "u16"=>"ushort", "i16"=>"short", "u32"=>"uint", "i32"=>"int", "u64"=>"ulong", "i64"=>"long", "f32"=>"float", "f64"=>"double", "bool"=>"bool", _=>"string" };
                    let g = switch k { "string"=>"el.GetString()!", "u8"=>"el.GetByte()", "i8"=>"el.GetSByte()", "u16"=>"el.GetUInt16()", "i16"=>"el.GetInt16()", "u32"=>"el.GetUInt32()", "i32"=>"el.GetInt32()", "u64"=>"el.GetUInt64()", "i64"=>"el.GetInt64()", "f32"=>"el.GetSingle()", "f64"=>"el.GetDouble()", "bool"=>"el.GetBoolean()", _=>"el.GetString()!" };
                    body += "            obj." + f.name + " = JsonUtils.ReadList<" + cs + ">(__p, el => " + g + ");\n";
                } else if inner.is_enum || current_struct_inline_enums.contains(inner.type_name) {
                    let enum_type = if inner.is_enum { s.name + "." + inner.type_name } else { inner.fqn };
                    body += "            obj." + f.name + " = JsonUtils.ReadList<" + enum_type + ">(__p, el => JsonUtils.ReadEnum<" + enum_type + ">(el));\n";
                } else {
                    let call_ns = if inner.namespace_fqn == "" { current_ns_name } else { inner.namespace_fqn };
                    body += "            obj." + f.name + " = JsonUtils.ReadList<" + inner.type_name + ">(__p, el => " + call_ns + ".JsonMappers.Read" + inner.type_name + "(el));\n";
                }
                body += "        }\n";
                continue;
            }

            if base.is_primitive {
                let k = base.lang_type; let g = switch k { "string"=>"GetString()", "u8"=>"GetByte()", "i8"=>"GetSByte()", "u16"=>"GetUInt16()", "i16"=>"GetInt16()", "u32"=>"GetUInt32()", "i32"=>"GetInt32()", "u64"=>"GetUInt64()", "i64"=>"GetInt64()", "f32"=>"GetSingle()", "f64"=>"GetDouble()", "bool"=>"GetBoolean()", _=>"GetString()" };
                body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p)) obj." + f.name + " = __p." + g + ";\n";
            }
            else if base.is_enum || current_struct_inline_enums.contains(base.type_name) {
                let enum_type = if base.is_enum { s.name + "." + base.type_name } else { base.fqn };
                body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p)) obj." + f.name + " = JsonUtils.ReadEnum<" + enum_type + ">(__p);\n";
            } else {
                let call_ns = if base.namespace_fqn == "" { current_ns_name } else { base.namespace_fqn };
                body += "        if (JsonUtils.TryGetPropertyCaseInsensitive(e, \"" + prop + "\", out var __p) && __p.ValueKind == JsonValueKind.Object) obj." + f.name + " = " + call_ns + ".JsonMappers.Read" + base.type_name + "(__p);\n";
            }
        }
    }
    body
}
