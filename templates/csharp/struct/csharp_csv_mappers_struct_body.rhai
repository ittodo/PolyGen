${
    let code = "";
    // Build simple headers: primitives/enums only; lists use [0]
    code += "        private static readonly string[] __Headers_" + s.name + " = new string[] { ";
    let first=true; 
    for it in s.items { if it.is_field() { let f = it.as_field();
        let t = f.field_type;
        if t.starts_with("Option<") { let ls = "Option<".len(); let ll = t.len()-ls-1; t = t.sub_string(ls, ll); }
        if t.starts_with("List<") {
            if !first { code += ", "; } code += "\""+f.name+"[0]\""; first=false;
        } else {
            if !first { code += ", "; } code += "\""+f.name+"\""; first=false;
        }
    } }
    code += " };\n";
    code += "        public static int ColumnCount_" + s.name + "() => __Headers_" + s.name + ".Length;\n";
    code += "        public static string[] GetHeader_" + s.name + "() => (string[])__Headers_" + s.name + ".Clone();\n";
    code += "        public static void AppendRow(" + s.name + " obj, List<string> cols)\n        {\n";
    for it in s.items { if it.is_field() { let f=it.as_field();
        let t = f.field_type; let expr = "obj."+f.name;
        // unwrap option
        if t.starts_with("Option<") { let ls = "Option<".len(); let ll = t.len()-ls-1; t = t.sub_string(ls, ll); }
        if t.starts_with("List<") {
            let ls = "List<".len(); let ll = t.len()-ls-1; let inner = t.sub_string(ls, ll);
            // primitive-like and enum handling; otherwise add empty placeholder(s)
            if inner == "u8" || inner == "i8" || inner == "u16" || inner == "i16" || inner == "u32" || inner == "i32" || inner == "u64" || inner == "i64" || inner == "f32" || inner == "f64" || inner == "bool" || inner == "string" {
                code += "if ("+expr+" != null && "+expr+".Count > 0) cols.Add(CsvUtils.ToStringInvariant("+expr+"[0])); else cols.Add(string.Empty);\n";
            } else if inner.ends_with("__Enum") { 
                code += "if ("+expr+" != null && "+expr+".Count > 0) cols.Add("+expr+"[0].ToString()); else cols.Add(string.Empty);\n";
            } else {
                code += "cols.Add(string.Empty);\n";
            }
        } else if t == "u8" || t == "i8" || t == "u16" || t == "i16" || t == "u32" || t == "i32" || t == "u64" || t == "i64" || t == "f32" || t == "f64" || t == "bool" || t == "string" {
            code += "cols.Add(CsvUtils.ToStringInvariant("+expr+"));\n";
        } else if t.ends_with("__Enum") || current_ns_enums.contains(t) {
            code += "cols.Add("+expr+".ToString());\n";
        } else {
            code += "cols.Add(string.Empty);\n";
        }
    } }
    code += "        }\n";
    code += "        public static string[] ToRow("+s.name+" obj) { var list = new List<string>(ColumnCount_"+s.name+"()); AppendRow(obj, list); return list.ToArray(); }\n";
    code += "        public static void WriteCsv(IEnumerable<"+s.name+"> items, string path, bool writeHeader = true, char sep = ',') { using var sw = new StreamWriter(path, false, new UTF8Encoding(false)); if (writeHeader) sw.WriteLine(CsvUtils.Join(GetHeader_"+s.name+"(), sep)); foreach (var it in items) { var row = ToRow(it); sw.WriteLine(CsvUtils.Join(row, sep)); } }\n";
    code
}
