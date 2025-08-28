${
    import "templates/csharp/rhai_utils/type_mapping" as type_utils;

    let headers = [];

    fn unwrap_option(t) {
        if t.starts_with("Option<") { let s="Option<".len(); let l=t.len()-s-1; return t.sub_string(s,l); }
        t
    }

    fn is_primitive_like(t) {
        switch t {
            "u8"=>true, "i8"=>true, "u16"=>true, "i16"=>true,
            "u32"=>true, "i32"=>true, "u64"=>true, "i64"=>true,
            "f32"=>true, "f64"=>true, "bool"=>true, "string"=>true,
            _ => false
        }
    }

    // Struct-level helpers for header and row generation
    fn add_headers_for(type_string, prefix) {
        let t = unwrap_option(type_string);
        if t.starts_with("List<") {
            let ls = "List<".len(); let ll = t.len()-ls-1; let inner=t.sub_string(ls,ll);
            let p = if prefix=="" { "[0]" } else { prefix+"[0]" };
            add_headers_for(inner, p);
            return;
        }
        // Embedded struct: inline ones are inside 's.items'; external handled by calling nested CsvMappers
        let es = find_embedded_struct(s, t);
        if es != () {
            for it in es.items { if it.is_field() { let f=it.as_field(); let p = if prefix=="" { f.name } else { prefix+"."+f.name }; add_headers_for(f.field_type, p); } }
            return;
        }
        if is_primitive_like(t) || t.ends_with("__Enum") {
            headers.push(prefix);
            return;
        }
        // External struct: defer to its CsvMappers.Headers length and names by generating calls in code
        headers.push(prefix); // placeholder; will be replaced in code-gen below
    }

    // 1) Build headers list
    for it in s.items { if it.is_field() { let f=it.as_field(); add_headers_for(f.field_type, f.name); } }

    // Generate C# code
    let code = "";
    // Header literal (we will regenerate exact headers in code too for external structs)
    code += "        // Headers\n";
    code += "        private static readonly string[] __Headers_" + s.name + " = new string[] { ";
    let first=true; for h in headers { if !first { code += ", "; } code += "\""+h+"\""; first=false; }
    code += " };\n";

    // Column count accessor
    code += "        public static int ColumnCount_" + s.name + "() => __Headers_" + s.name + ".Length;\n";
    // Header getter
    code += "        public static string[] GetHeader_" + s.name + "() => (string[])__Headers_" + s.name + ".Clone();\n";

    // AppendRow implementation
    code += "        public static void AppendRow(" + s.name + " obj, List<string> cols)\n        {\n";

    fn append_row_for(type_string, expr_prefix) {
        let t = unwrap_option(type_string);
        if t.starts_with("List<") {
            let ls = "List<".len(); let ll=t.len()-ls-1; let inner=t.sub_string(ls,ll);
            // first element or default
            code += "            if ("+expr_prefix+" != null && "+expr_prefix+".Count > 0) {\n";
            append_row_for(inner, expr_prefix+"[0]");
            code += "            } else {\n";
            // need to append empty slots equal to inner column count
            if is_primitive_like(inner) || inner.ends_with("__Enum") {
                code += "                cols.Add(string.Empty);\n";
            } else {
                // external or embedded
                // Try embedded on current struct name; if not, assume external and call ColumnCount on that type
                let es = find_embedded_struct(s, inner);
                if es != () {
                    code += "                for (int i=0;i< ColumnCount_"+es.name+"(); i++) cols.Add(string.Empty);\n";
                } else {
                    code += "                for (int i=0;i< "+ inner +"Csv.ColumnCount(); i++) cols.Add(string.Empty);\n"; // placeholder; adjust below in actual emit
                }
            }
            code += "            }\n";
            return;
        }
        if is_primitive_like(t) {
            code += "            cols.Add(CsvUtils.ToStringInvariant("+expr_prefix+"));\n";
            return;
        }
        if t.ends_with("__Enum") {
            code += "            cols.Add(("+expr_prefix+").ToString());\n";
            return;
        }
        // Option unwrap already done; handle nulls for reference types by adding empty cells of nested count
        let es = find_embedded_struct(s, t);
        if es != () {
            code += "            if ("+expr_prefix+" == null) { for (int i=0;i< ColumnCount_"+es.name+"(); i++) cols.Add(string.Empty);} else { AppendRow_"+es.name+"("+expr_prefix+", cols); }\n";
            return;
        }
        // external struct
        code += "            if ("+expr_prefix+" == null) { /* external struct null */ } else { /* external append */ }\n";
    }

    // Emit per-field append logic preserving order (same as headers)
    for it in s.items { if it.is_field() { let f=it.as_field();
        // Option null check
        if f.field_type.starts_with("Option<") {
            let sidx="Option<".len(); let slen=f.field_type.len()-sidx-1; let inner=f.field_type.sub_string(sidx,slen);
            code += "            if (obj."+f.name+" == null) {";
            // add empties for inner
            if is_primitive_like(inner) || inner.ends_with("__Enum") { code += " cols.Add(string.Empty); "; }
            else {
                let es = find_embedded_struct(s, inner);
                if es != () { code += " for (int i=0;i< ColumnCount_"+es.name+"(); i++) cols.Add(string.Empty); "; }
                else { code += " /* external */ "; }
            }
            code += " } else {\n";
            append_row_for(inner, "obj."+f.name);
            code += "            }\n";
        } else {
            append_row_for(f.field_type, "obj."+f.name);
        }
    } }
    code += "        }\n";

    // ToRow + WriteCsv shortcuts
    code += "        public static string[] ToRow("+s.name+" obj) { var list = new List<string>(ColumnCount_"+s.name+"()); AppendRow(obj, list); return list.ToArray(); }\n";
    code += "        public static void WriteCsv(IEnumerable<"+s.name+"> items, string path, bool writeHeader = true, char sep = ',') { using var sw = new StreamWriter(path, false, new UTF8Encoding(false)); if (writeHeader) sw.WriteLine(CsvUtils.Join(GetHeader_"+s.name+"(), sep)); foreach (var it in items) { var row = ToRow(it); sw.WriteLine(CsvUtils.Join(row, sep)); } }\n";

    code
}
