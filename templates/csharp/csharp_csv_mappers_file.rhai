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

    // Fast Index-based reader (phase 2: scalars + lists + nested)
    code += "        public sealed class CsvIndex\n        {\n";
    for it in s.items { if it.is_field() { let f=it.as_field(); let t=unwrap_option(f.field_type);
        if t.starts_with("List<") {
            let ls="List<".len(); let ll=t.len()-ls-1; let inner=t.sub_string(ls,ll);
            // primitive/enum list → store column indices per element
            if type_utils::map_type(inner) != inner || inner == "string" || inner == "bool" || inner == "u8" || inner == "i8" || inner == "u16" || inner == "i16" || inner == "u32" || inner == "i32" || inner == "u64" || inner == "i64" || inner == "f32" || inner == "f64" || inner.contains("_") /* enum heuristic */ {
                code += "            public System.Collections.Generic.List<int> idx_" + f.name + " = new System.Collections.Generic.List<int>();\n";
            } else {
                // struct list → store list of child indices
                let es = find_embedded_struct(s, inner);
                if es != () {
                    code += "            public System.Collections.Generic.List<" + ns_name + "." + es.name + "Csv.CsvIndex> idx_" + f.name + " = new System.Collections.Generic.List<" + ns_name + "." + es.name + "Csv.CsvIndex>();\n";
                } else {
                    let ext = resolve_struct(all_files, inner, ns_name);
                    if ext != () { let child = ext; code += "            public System.Collections.Generic.List<" + child.name + "Csv.CsvIndex> idx_" + f.name + " = new System.Collections.Generic.List<" + child.name + "Csv.CsvIndex>();\n"; }
                }
            }
        } else {
            let es = find_embedded_struct(s, t);
            let ext = resolve_struct(all_files, t, ns_name);
            if es != () {
                code += "            public " + ns_name + "." + es.name + "Csv.CsvIndex idx_" + f.name + ";\n";
            } else if ext != () { let child = ext; code += "            public " + child.name + "Csv.CsvIndex idx_" + f.name + ";\n"; }
            else { code += "            public int idx_" + f.name + " = -1;\n"; }
        }
    } }
    code += "        }\n";

    code += "        public static bool CsvIndexHasAny(Index idx)\n        {\n";
    for it in s.items { if it.is_field() { let f=it.as_field(); let t=unwrap_option(f.field_type);
        if t.starts_with("List<") {
            code += "            if (idx.idx_" + f.name + " != null && idx.idx_" + f.name + ".Count > 0) return true;\n";
        } else {
            let es = find_embedded_struct(s, t);
            let ext = resolve_struct(all_files, t, ns_name);
            if es != () {
                code += "            if (idx.idx_" + f.name + " != null && " + ns_name + "." + es.name + "Csv.CsvIndexHasAny(idx.idx_" + f.name + ")) return true;\n";
            } else if ext != () { let child = ext; code += "            if (idx.idx_" + f.name + " != null && " + child.name + "Csv.CsvIndexHasAny(idx.idx_" + f.name + ")) return true;\n"; }
            else { code += "            if (idx.idx_" + f.name + " >= 0) return true;\n"; }
        }
    } }
    code += "            return false;\n        }\n";

    code += "        public static bool CsvIndexHasValues(Index idx, string[] row)\n        {\n";
    for it in s.items { if it.is_field() { let f=it.as_field(); let t=unwrap_option(f.field_type);
        if t.starts_with("List<") {
            code += "            if (idx.idx_" + f.name + " != null) { for (int i=0;i<idx.idx_" + f.name + ".Count;i++) { var __ix = idx.idx_" + f.name + "[i]; if (__ix>=0 && __ix<row.Length && !string.IsNullOrEmpty(row[__ix])) return true; } }\n";
        } else {
            let es = find_embedded_struct(s, t);
            let ext = resolve_struct(all_files, t, ns_name);
            if es != () {
                code += "            if (idx.idx_" + f.name + " != null && " + ns_name + "." + es.name + "Csv.CsvIndexHasValues(idx.idx_" + f.name + ", row)) return true;\n";
            } else if ext != () { let child = ext; code += "            if (idx.idx_" + f.name + " != null && " + child.name + "Csv.CsvIndexHasValues(idx.idx_" + f.name + ", row)) return true;\n"; }
            else { code += "            if (idx.idx_" + f.name + " >= 0 && idx.idx_" + f.name + " < row.Length && !string.IsNullOrEmpty(row[idx.idx_" + f.name + "])) return true;\n"; }
        }
    } }
    code += "            return false;\n        }\n";

    code += "        public static CsvIndex BuildCsvIndex(string[] header, string prefix)\n        {\n";
    code += "            var idx = new CsvIndex();\n";
    code += "            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);\n";
    for it in s.items { if it.is_field() { let f=it.as_field(); let t=unwrap_option(f.field_type);
        if t.starts_with("List<") {
            let ls="List<".len(); let ll=t.len()-ls-1; let inner=t.sub_string(ls,ll);
            // primitive/enum list
            if type_utils::map_type(inner) != inner || inner == "string" || inner == "bool" || inner == "u8" || inner == "i8" || inner == "u16" || inner == "i16" || inner == "u32" || inner == "i32" || inner == "u64" || inner == "i64" || inner == "f32" || inner == "f64" || inner.contains("_") {
                code += "            for (int i=0;;i++) { int __ix; if (!map.TryGetValue(prefix + \"" + f.name + "[\"+i+\"]\", out __ix)) break; idx.idx_" + f.name + ".Add(__ix); }\n";
            } else {
                // struct list
                let es = find_embedded_struct(s, inner);
                if es != () {
                    code += "            for (int i=0;;i++) { var sub = " + ns_name + "." + es.name + "Csv.BuildIndex(header, prefix + \"" + f.name + "[\"+i+\"].\"); if (!" + ns_name + "." + es.name + "Csv.CsvIndexHasAny(sub)) break; idx.idx_" + f.name + ".Add(sub); }\n";
                } else {
                    let ext = resolve_struct(all_files, inner, ns_name);
                    if ext != () { let child = ext; code += "            for (int i=0;;i++) { var sub = " + child.name + "Csv.BuildIndex(header, prefix + \"" + f.name + "[\"+i+\"].\"); if (!" + child.name + "Csv.CsvIndexHasAny(sub)) break; idx.idx_" + f.name + ".Add(sub); }\n"; }
                }
            }
        } else {
            let es = find_embedded_struct(s, t);
            let ext = resolve_struct(all_files, t, ns_name);
            if es != () {
                code += "            idx.idx_" + f.name + " = " + ns_name + "." + es.name + "Csv.BuildIndex(header, prefix + \"" + f.name + ".\");\n";
            } else if ext != () { let child = ext; code += "            idx.idx_" + f.name + " = " + child.name + "Csv.BuildIndex(header, prefix + \"" + f.name + ".\");\n"; }
            else { code += "            if (!map.TryGetValue(prefix + \"" + f.name + "\", out idx.idx_" + f.name + ")) idx.idx_" + f.name + " = -1;\n"; }
        }
    } }
    code += "            return idx;\n        }\n";

    code += "        public static " + s.name + " FromRowWithCsvIndex(Index idx, string[] row, Polygen.Common.CsvUtils.GapMode gap)\n        {\n";
    code += "            var obj = new " + s.name + "();\n";
    for it in s.items { if it.is_field() { let f=it.as_field(); let t=unwrap_option(f.field_type);
        if t.starts_with("List<") {
            let ls="List<".len(); let ll=t.len()-ls-1; let inner=t.sub_string(ls,ll); let cs_t = type_utils::map_type(inner);
            // primitive/enum list
            if type_utils::map_type(inner) != inner || inner == "string" || inner == "bool" || inner == "u8" || inner == "i8" || inner == "u16" || inner == "i16" || inner == "u32" || inner == "i32" || inner == "u64" || inner == "i64" || inner == "f32" || inner == "f64" || inner.contains("_") {
                code += "            { var list = new System.Collections.Generic.List<" + cs_t + ">(); for (int i=0;i<idx.idx_" + f.name + ".Count;i++) { int __ix = idx.idx_" + f.name + "[i]; if (__ix < 0 || __ix >= row.Length) { if (i==0 || gap==Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } var __cell = row[__ix]; if (string.IsNullOrEmpty(__cell)) { if (i==0 || gap==Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } list.Add(DataSourceFactory.ConvertValue<" + cs_t + ">( __cell )); } obj." + f.name + " = list; }\n";
            } else {
                // struct list
                let es = find_embedded_struct(s, inner);
                if es != () {
                    code += "            { var list = new System.Collections.Generic.List<" + ns_name + "." + es.name + ">(); for (int i=0;i<idx.idx_" + f.name + ".Count;i++) { var subIdx = idx.idx_" + f.name + "[i]; if (!" + ns_name + "." + es.name + "Csv.CsvIndexHasValues(subIdx, row)) { if (i==0 || gap==Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } var sub = " + ns_name + "." + es.name + "Csv.FromRowWithIndex(subIdx, row, gap); list.Add(sub); } obj." + f.name + " = list; }\n";
                } else {
                    let ext = resolve_struct(all_files, inner, ns_name);
                    if ext != () { let child = ext; code += "            { var list = new System.Collections.Generic.List<" + child.name + ">(); for (int i=0;i<idx.idx_" + f.name + ".Count;i++) { var subIdx = idx.idx_" + f.name + "[i]; if (!" + child.name + "Csv.CsvIndexHasValues(subIdx, row)) { if (i==0 || gap==Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } var sub = " + child.name + "Csv.FromRowWithIndex(subIdx, row, gap); list.Add(sub); } obj." + f.name + " = list; }\n"; }
                }
            }
        } else {
            let es = find_embedded_struct(s, t);
            let ext = resolve_struct(all_files, t, ns_name);
            if es != () {
                code += "            if (!" + ns_name + "." + es.name + "Csv.CsvIndexHasValues(idx.idx_" + f.name + ", row)) { obj." + f.name + " = null; } else { obj." + f.name + " = " + ns_name + "." + es.name + "Csv.FromRowWithIndex(idx.idx_" + f.name + ", row, gap); }\n";
            } else if ext != () { let child = ext; code += "            if (!" + child.name + "Csv.CsvIndexHasValues(idx.idx_" + f.name + ", row)) { obj." + f.name + " = null; } else { obj." + f.name + " = " + child.name + "Csv.FromRowWithIndex(idx.idx_" + f.name + ", row, gap); }\n"; }
            else { let cs_t = type_utils::map_type(t); code += "            if (idx.idx_" + f.name + " >= 0 && idx.idx_" + f.name + " < row.Length) { var __cell = row[idx.idx_" + f.name + "]; obj." + f.name + " = DataSourceFactory.ConvertValue<" + cs_t + ">( __cell ); }\n"; }
        }
    } }
    code += "            return obj;\n        }\n";

    // Reusable reader that carries a prebuilt Index
    code += "        public sealed class Reader\n        {\n";
    code += "            public readonly CsvIndex Index;\n            public readonly char Sep;\n            public readonly Polygen.Common.CsvUtils.GapMode Gap;\n";
    code += "            public Reader(Index index, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { CsvIndex = index; Sep = sep; Gap = gap; }\n";
    code += "            public " + s.name + " Parse(string[] row) => FromRowWithCsvIndex(Index, row, Gap);\n";
    code += "            public static Reader FromHeader(string[] header, char sep = ',', string prefix = \"\", Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) { var idx = BuildCsvIndex(header, prefix); return new Reader(idx, sep, gap); }\n";
    code += "        }\n";

    // Read with provided CsvIndex (no header processing)
    code += "        public static System.Collections.Generic.IEnumerable<"+s.name+"> ReadCsvWithIndex(string path, CsvIndex idx, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)\n        {\n";
    code += "            var lines = File.ReadAllLines(path); if (lines.Length <= 1) yield break;\n";
    code += "            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!IndexHasValues(idx, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithCsvIndex(idx, row, gap); }\n";
    code += "        }\n";

    // Convenience: fast read using CsvIndex (no per-row header map)
    code += "        public static System.Collections.Generic.IEnumerable<"+s.name+"> ReadCsvFast(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)\n        {\n";
    code += "            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;\n";
    code += "            var header = lines[0].Split(sep);\n";
    code += "            var idx = BuildCsvIndex(header, string.Empty);\n";
    code += "            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!IndexHasValues(idx, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithCsvIndex(idx, row, gap); }\n";
    code += "        }\n";
    // Read API (dictionary-based, legacy)
    code += "        public static " + s.name + " FromRow(IDictionary<string,string> row) => FromRowWithPrefix(row, string.Empty);\n";
    code += "        public static " + s.name + " FromRowWithPrefix(IDictionary<string,string> row, string prefix)\n        {\n";
    code += "            var obj = new " + s.name + "();\n";
    // generate field reads via Rust helper
    code += csv_read_fields_for_struct(s, "obj", "prefix", ns_name, all_files);
    code += "            return obj;\n";
    code += "        }\n";

    // Convenience: read entire CSV file using header + row[] based reader
    code += "        public static IEnumerable<"+s.name+"> ReadCsv(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)\n        {\n";
    code += "            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;\n";
    code += "            var header = lines[0].Split(sep);\n";
    code += "            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);\n";
    code += "            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); yield return FromRowWithPrefixAndHeaderMap(map, row, string.Empty, gap); }\n";
    code += "        }\n";

    // Read API (header + row[] based)
    code += "        public static " + s.name + " FromRow(string[] header, string[] row, Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break) => FromRowWithPrefixAndHeader(header, row, string.Empty, gap);\n";
    code += "        public static " + s.name + " FromRowWithPrefixAndHeader(string[] header, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)\n        {\n";
    code += "            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);\n";
    code += "            return FromRowWithPrefixAndHeaderMap(map, row, prefix, gap);\n";
    code += "        }\n";
    code += "        public static " + s.name + " FromRowWithPrefixAndHeaderMap(System.Collections.Generic.Dictionary<string,int> map, string[] row, string prefix, Polygen.Common.CsvUtils.GapMode gap)\n        {\n";
    code += "            var obj = new " + s.name + "();\n";
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


