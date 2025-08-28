import "templates/rhai_utils/indent" as indent_utils;
import "templates/csharp/rhai_utils/type_mapping" as type_utils;
// Local header collectors (mirrors csharp_csv_columns_file.rhai) to avoid recursion/import issues

fn unwrap_option(t) { if t.starts_with("Option<") { let s="Option<".len(); let l=t.len()-s-1; return t.sub_string(s,l); } t }
fn find_embedded_struct(s, name) { for it in s.items { if it.is_embedded_struct() { let es=it.as_embedded_struct(); if es.name==name { return es; } } } () }
fn find_struct_in_ns(ns, target_name) { for item in ns.items { if item.is_struct() { let s=item.as_struct(); if s.name==target_name { return s; } } } () }
fn find_struct_in_tree(ns, prefix, target_ns, target_name) {
    let fqn = if prefix == "" { ns.name } else { prefix + "." + ns.name };
    if fqn == target_ns { let s = find_struct_in_ns(ns, target_name); if s != () { return s; } }
    for item in ns.items { if item.is_namespace() { let child=item.as_namespace(); let s=find_struct_in_tree(child, fqn, target_ns, target_name); if s!=() { return s; } } }
    ()
}
fn get_struct_at(all_files, target_ns, target_name) { for file in all_files { for root_ns in file.namespaces { let s=find_struct_in_tree(root_ns, "", target_ns, target_name); if s!=() { return s; } } } () }
fn any_struct_named_in(ns, target_name, prefix) {
    let s=find_struct_in_ns(ns, target_name); if s!=() { return s; }
    for item in ns.items { if item.is_namespace() { let child=item.as_namespace(); let t=any_struct_named_in(child, target_name, if prefix=="" { ns.name } else { prefix+"."+ns.name }); if t!=() { return t; } } }
    ()
}
fn any_struct_named(all_files, target_name) { for file in all_files { for root_ns in file.namespaces { let s=find_struct_in_ns(root_ns, target_name); if s!=() { return s; } for item in root_ns.items { if item.is_namespace() { let child=item.as_namespace(); let t=any_struct_named_in(child, target_name, root_ns.name); if t!=() { return t; } } } } } () }
fn resolve_struct(all_files, type_string, current_ns_name) {
    let core = unwrap_option(type_string);
    if core.starts_with("List<") { let ls="List<".len(); let ll=core.len()-ls-1; let inner=core.sub_string(ls,ll); return resolve_struct(all_files, inner, current_ns_name); }
    if core.contains(".") {
        let parts=core.split("."); let name=parts[parts.len-1]; let ns=""; let i=0; for seg in parts { if i<parts.len-1 { if ns!="" { ns += "."; } ns += seg; } i+=1; }
        return get_struct_at(all_files, ns, name);
    } else {
        if current_ns_name != () && current_ns_name != "" { let s=get_struct_at(all_files, current_ns_name, core); if s!=() { return s; } }
        return any_struct_named(all_files, core);
    }
}
fn collect_columns_with(ctx_struct, prefix, type_string, visited, depth, current_ns_name, all_files) {
    let cols = [];
    let t = unwrap_option(type_string);
    if depth >= 10 { cols.push(prefix); return cols; }
    if t.starts_with("List<") { let ls="List<".len(); let ll=t.len()-ls-1; let inner=t.sub_string(ls,ll); let np = if prefix=="" { "[0]" } else { prefix+"[0]" }; let sub=collect_columns_with(ctx_struct, np, inner, visited, depth+1, current_ns_name, all_files); for c in sub { cols.push(c); } return cols; }
    let es = find_embedded_struct(ctx_struct, t);
    if es != () { if visited.contains(es.name) { return cols; } let next=visited+[es.name]; for it in es.items { if it.is_field() { let f=it.as_field(); let np = if prefix=="" { f.name } else { prefix+"."+f.name }; let sub=collect_columns_with(es, np, f.field_type, next, depth+1, current_ns_name, all_files); for c in sub { cols.push(c); } } } return cols; }
    let ext = resolve_struct(all_files, t, current_ns_name);
    if ext != () { if visited.contains(ext.name) { return cols; } let next=visited+[ext.name]; for it in ext.items { if it.is_field() { let f=it.as_field(); let np= if prefix=="" { f.name } else { prefix+"."+f.name }; let sub=collect_columns_with(ext, np, f.field_type, next, depth+1, current_ns_name, all_files); for c in sub { cols.push(c); } } } return cols; }
    cols.push(prefix); cols
}
fn headers_for_struct(cur_struct, current_ns_name, all_files) {
    let headers=[]; for it in cur_struct.items { if it.is_field() { let f=it.as_field(); let sub=collect_columns_with(cur_struct, f.name, f.field_type, [cur_struct.name], 0, current_ns_name, all_files); for c in sub { headers.push(c); } } } headers
}

let using_template = include("templates/csharp/csharp_using_csv.rhai");

fn emit_struct(ns_name, s, all_files) {
    let headers = csv_headers_for_struct(s, ns_name, all_files);
    let code = "";
    code += "namespace " + ns_name + "\n";
    code += "{\n";
    code += "public static class " + s.name + "Csv\n";
    code += "{\n";
    code += "        private static readonly string[] __Headers_" + s.name + " = new string[] { ";
    let first=true; for h in headers { if !first { code += ", "; } code += "\""+h+"\""; first=false; }
    code += " };\n";
    code += "        public static int ColumnCount_" + s.name + "() => __Headers_" + s.name + ".Length;\n";
    code += "        public static string[] GetHeader_" + s.name + "() => (string[])__Headers_" + s.name + ".Clone();\n";
    code += "        public static void AppendRow(" + s.name + " obj, List<string> cols)\n        {\n";
    for it in s.items { if it.is_field() { let f = it.as_field(); code += csv_append_code_for(s, f.field_type, "obj."+f.name, ns_name, all_files, [s.name], 0); } }
    code += "        }\n";
    code += "        public static string[] ToRow("+s.name+" obj) { var list = new List<string>(ColumnCount_"+s.name+"()); AppendRow(obj, list); return list.ToArray(); }\n";
    code += "        public static void WriteCsv(IEnumerable<"+s.name+"> items, string path, bool writeHeader = true, char sep = ',') { using var sw = new StreamWriter(path, false, new UTF8Encoding(false)); if (writeHeader) sw.WriteLine(CsvUtils.Join(GetHeader_"+s.name+"(), sep)); foreach (var it in items) { var row = ToRow(it); sw.WriteLine(CsvUtils.Join(row, sep)); } }\n";

    // Dynamic (2-pass) writer: emit via Rust helper to avoid template recursion
    code += csv_dynamic_methods_for_struct(s, ns_name, all_files);
    // Read API (dictionary-based, legacy)
    code += "        public static " + s.name + " FromRow(IDictionary<string,string> row) => FromRowWithPrefix(row, string.Empty);\n";
    code += "        public static " + s.name + " FromRowWithPrefix(IDictionary<string,string> row, string prefix)\n        {\n";
    code += "            var obj = new " + s.name + "();\n";
    // generate field reads via Rust helper
    code += csv_read_fields_for_struct(s, "obj", "prefix", ns_name, all_files);
    code += "            return obj;\n";
    code += "        }\n";

    // Convenience: read entire CSV file using header + row[] based reader
    code += "        public static IEnumerable<"+s.name+"> ReadCsv(string path, Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)\n        {\n";
    code += "            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;\n";
    code += "            var header = lines[0].Split(',');\n";
    code += "            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(','); yield return FromRow(header, row, gap); }\n";
    code += "        }\n";

    // Read API (header + row[] based)
    code += "        public static " + s.name + " FromRow(string[] header, string[] row, Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) => FromRowWithPrefixAndHeader(header, row, string.Empty, gap);\n";
    code += "        public static " + s.name + " FromRowWithPrefixAndHeader(string[] header, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)\n        {\n";
    code += "            var obj = new " + s.name + "();\n";
    code += "            var map = Polygen.Common.CsvUtils.IndexHeader(header);\n";
    code += "            int gap = (int)gap; // 0=Break,1=Sparse\n";
    code += csv_read_fields_for_struct_indexed(s, "obj", "prefix", ns_name, all_files);
    code += "            return obj;\n";
    code += "        }\n";
    code += "}\n";
    code += "}\n\n";
    code
}

fn walk_namespace(ns_obj, prefix, all_files) {
    let full_ns_name = if prefix == () || prefix == "" { ns_obj.name } else { prefix + "." + ns_obj.name };
    let blocks = "";
    for item in ns_obj.items { if item.is_struct() { let s = item.as_struct(); blocks += emit_struct(full_ns_name, s, all_files); } }
    for item in ns_obj.items { if item.is_namespace() { let child = item.as_namespace(); blocks += walk_namespace(child, full_ns_name, all_files); } }
    blocks
}

for file in schema.files {
    if file.path == () || file.path == "" { continue; }
    let parts = file.path.split("/");
    let filename = parts[parts.len - 1];
    let output_filename = filename; output_filename.replace(".poly", ".CsvMappers.cs");
    let final_path = "output/csharp/" + output_filename;

    let content = "";
    content += using_template;
    for ns in file.namespaces { content += walk_namespace(ns, "", schema.files); }

    print("Generating file: " + final_path);
    write_file(final_path, content);
}

""
