${
    let code = "";
    // Build simple headers: primitives/enums only; lists use [0]
    code += "        private static readonly string[] __Headers_" + s.name + " = new string[] { ";
    let first=true; 
    for it in s.items { if it.is_field() { let f = it.as_field();
        let tf=f.field_type; let base = if tf.is_option { tf.inner } else { tf };
        if base.is_list {
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
        let tf=f.field_type; let base = if tf.is_option { tf.inner } else { tf }; let expr = "obj."+f.name;
        if base.is_list { let inner = base.inner;
            // primitive-like and enum handling; otherwise add empty placeholder(s)
            if inner.is_primitive || inner.lang_type == "string" {
                code += "if ("+expr+" != null && "+expr+".Count > 0) cols.Add(CsvUtils.ToStringInvariant("+expr+"[0])); else cols.Add(string.Empty);\n";
            } else if inner.is_enum { 
                code += "if ("+expr+" != null && "+expr+".Count > 0) cols.Add("+expr+"[0].ToString()); else cols.Add(string.Empty);\n";
            } else {
                code += "cols.Add(string.Empty);\n";
            }
        } else if base.is_primitive || base.lang_type == "string" {
            code += "cols.Add(CsvUtils.ToStringInvariant("+expr+"));\n";
        } else if base.is_enum {
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
