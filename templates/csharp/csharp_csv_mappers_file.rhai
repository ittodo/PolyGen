import "templates/rhai_utils/indent" as indent_utils;
import "templates/csharp/rhai_utils/type_mapping" as type_utils;
// type_info not required; detect inline enums by current struct scan
// Local header collectors (mirrors csharp_csv_columns_file.rhai) to avoid recursion/import issues

fn unwrap_option(t) {
    let t = if type_of(t) == "TypeRef" { t.original } else { t };
    if t.starts_with("Option<") { let s="Option<".len(); let l=t.len()-s-1; return t.sub_string(s,l); }
    t
}
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
    let current_struct_inline_enums = [];
    for it in s.items { if it.is_inline_enum() { let e = it.as_inline_enum(); current_struct_inline_enums.push(e.name); } }
    let code = "";
    code += "namespace " + ns_name + "\n";
    code += "{\n";
    code += "public static class " + s.name + "Csv\n";
    code += "{\n";
    let full_t = "global::" + s.fqn;
    // per-struct CSV API: AppendRow/ToRow writers and header-tree based readers only
    let headers = csv_headers_for_struct(s, ns_name, all_files);
    code += "        private static readonly string[] __Headers_" + s.name + " = new string[] { ";
    let first=true; for h in headers { if !first { code += ", "; } code += "\""+h+"\""; first=false; }
    code += " };\n";
    code += "        public static int ColumnCount_" + s.name + "() => __Headers_" + s.name + ".Length;\n";
    code += "        public static string[] GetHeader_" + s.name + "() => (string[])__Headers_" + s.name + ".Clone();\n";
    code += "        public static void AppendRow(" + full_t + " obj, List<string> cols)\n        {\n";
    for it in s.items { if it.is_field() { let f = it.as_field(); code += csv_append_code_for(s, f.field_type.original, "obj."+f.name, ns_name, all_files, [s.name], 0); } }
    code += "        }\n";
    code += "        public static string[] ToRow("+full_t+" obj) { var list = new List<string>(ColumnCount_"+s.name+"()); AppendRow(obj, list); return list.ToArray(); }\n";
    code += "        public static void WriteCsv(IEnumerable<"+full_t+"> items, string path, bool writeHeader = true, char sep = ',') { using var sw = new StreamWriter(path, false, new UTF8Encoding(false)); if (writeHeader) sw.WriteLine(CsvUtils.Join(GetHeader_"+s.name+"(), sep)); foreach (var it in items) { var row = ToRow(it); sw.WriteLine(CsvUtils.Join(row, sep)); } }\n";

    // Dynamic writer omitted for demo simplicity

    // Reader class removed to keep only ReadCsvFast

    // BuildHeader: construct Polygen.Common.CsvIndexHeader tree for this struct
    code += "        public static Polygen.Common.CsvIndexHeader BuildHeader(string[] header, string prefix)\n        {\n";
    code += "            var map = Polygen.Common.CsvUtils.CsvIndexHeader(header);\n";
    code += "            var root = new Polygen.Common.CsvIndexHeader();\n";
    code += "            root.IndexList = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>();\n";
    for it in s.items { if it.is_field() {
        let f = it.as_field();
        let t = unwrap_option(f.field_type);
        if t.starts_with("List<") {
            let ls="List<".len(); let ll=t.len()-ls-1; let inner=t.sub_string(ls,ll);
            code += "            { var ch = new Polygen.Common.CsvIndexHeader(); ch.Index = -1; ch.IndexList = new System.Collections.Generic.List<Polygen.Common.CsvIndexHeader>();\n";
            // primitive/enum list
            if type_utils::map_type(inner) != inner || inner == "string" || inner == "bool" || inner == "u8" || inner == "i8" || inner == "u16" || inner == "i16" || inner == "u32" || inner == "i32" || inner == "u64" || inner == "i64" || inner == "f32" || inner == "f64" || inner.contains("_") {
                code += "                for (int i=0;;i++) { int __ix; if (!map.TryGetValue(prefix + \"" + f.name + "[\"+i+\"]\", out __ix)) break; var leaf = new Polygen.Common.CsvIndexHeader(); leaf.Index = __ix; ch.IndexList.Add(leaf); }\n";
            } else {
                // struct list
                let es = find_embedded_struct(s, inner);
                if es != () {
                    code += "                for (int i=0;;i++) { var sub = global::" + ns_name + "." + es.name + "Csv.BuildHeader(header, prefix + \"" + f.name + "[\"+i+\"].\"); if (!sub.HasAny()) break; ch.IndexList.Add(sub); }\n";
                } else {
                    let ext = resolve_struct(all_files, inner, ns_name);
                    if ext != () {
                        let child = ext;
                        let call_ns = if inner.contains(".") { let __ns = ""; let __i=0; let parts = inner.split("."); for seg in parts { if __i < parts.len-1 { if __ns != "" { __ns += "."; } __ns += seg; } __i+=1; } __ns } else { ns_name };
                        code += "                for (int i=0;;i++) { var sub = global::" + call_ns + "." + child.name + "Csv.BuildHeader(header, prefix + \"" + f.name + "[\"+i+\"].\"); if (!sub.HasAny()) break; ch.IndexList.Add(sub); }\n";
                    }
                }
            }
            code += "                root.IndexList.Add(ch); }\n";
        } else {
            let es = find_embedded_struct(s, t);
            let ext = resolve_struct(all_files, t, ns_name);
            if es != () {
                code += "            root.IndexList.Add(global::" + ns_name + "." + es.name + "Csv.BuildHeader(header, prefix + \"" + f.name + ".\"));\n";
            } else if ext != () {
                let child = ext;
                let call_ns = if t.contains(".") { let __ns = ""; let __i=0; let parts=t.split("."); for seg in parts { if __i<parts.len-1 { if __ns != "" { __ns += "."; } __ns += seg; } __i+=1; } __ns } else { ns_name };
                code += "            root.IndexList.Add(global::" + call_ns + "." + child.name + "Csv.BuildHeader(header, prefix + \"" + f.name + ".\"));\n";
            }
            else { code += "            { var ch = new Polygen.Common.CsvIndexHeader(); if (!map.TryGetValue(prefix + \"" + f.name + "\", out ch.Index)) ch.Index = -1; root.IndexList.Add(ch); }\n"; }
        }
    } }
    code += "            return root;\n        }\n";

    // HeaderHasValues is provided by Polygen.Common.CsvUtils

    // FromRowWithHeader: construct object using header tree
    code += "        public static " + full_t + " FromRowWithHeader(Polygen.Common.CsvIndexHeader h, string[] row, Polygen.Common.CsvUtils.GapMode gap)\n        {\n";
    code += "            var obj = new " + full_t + "();\n";
    code += "            int __pos = 0;\n";
    code += "            Polygen.Common.CsvIndexHeader __h = null;\n";
    for it in s.items { if it.is_field() {
        let f = it.as_field();
        let t = unwrap_option(f.field_type);
        // fetch child header for this field
        code += "            __h = (h != null && h.IndexList != null && __pos < h.IndexList.Count) ? h.IndexList[__pos] : null; __pos++;\n";
        if t.starts_with("List<") {
            let ls="List<".len(); let ll=t.len()-ls-1; let inner=t.sub_string(ls,ll);
            if type_utils::map_type(inner) != inner || inner == "string" || inner == "bool" || inner == "u8" || inner == "i8" || inner == "u16" || inner == "i16" || inner == "u32" || inner == "i32" || inner == "u64" || inner == "i64" || inner == "f32" || inner == "f64" || inner.contains("_") {
                let cs_t = type_utils::map_type(inner);
                code += "            { var list = new System.Collections.Generic.List<" + cs_t + ">(); if (__h != null && __h.IndexList != null) { for (int i=0;i<__h.IndexList.Count;i++) { var leaf = __h.IndexList[i]; if (leaf == null || leaf.Index < 0 || leaf.Index >= row.Length) { if (i==0 || gap==Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } var __cell = row[leaf.Index]; if (string.IsNullOrEmpty(__cell)) { if (i==0 || gap==Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } list.Add(DataSourceFactory.ConvertValue<" + cs_t + ">( __cell )); } } obj." + f.name + " = list; }\n";
            } else {
                let es = find_embedded_struct(s, inner);
                if es != () {
                    code += "            { var list = new System.Collections.Generic.List<global::" + es.fqn + ">(); if (__h != null && __h.IndexList != null) { for (int i=0;i<__h.IndexList.Count;i++) { var subH = __h.IndexList[i]; if (!Polygen.Common.CsvUtils.HeaderHasValues(subH, row)) { if (i==0 || gap==Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } var sub = global::" + ns_name + "." + es.name + "Csv.FromRowWithHeader(subH, row, gap); list.Add(sub); } } obj." + f.name + " = list; }\n";
                } else {
                    let ext = resolve_struct(all_files, inner, ns_name);
                    if ext != () {
                        let child = ext;
                        let call_ns = if inner.contains(".") { let __ns = ""; let __i=0; let parts=inner.split("."); for seg in parts { if __i<parts.len-1 { if __ns != "" { __ns += "."; } __ns += seg; } __i+=1; } __ns } else { ns_name };
                        code += "            { var list = new System.Collections.Generic.List<global::" + child.fqn + ">(); if (__h != null && __h.IndexList != null) { for (int i=0;i<__h.IndexList.Count;i++) { var subH = __h.IndexList[i]; if (!Polygen.Common.CsvUtils.HeaderHasValues(subH, row)) { if (i==0 || gap==Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } var sub = global::" + call_ns + "." + child.name + "Csv.FromRowWithHeader(subH, row, gap); list.Add(sub); } } obj." + f.name + " = list; }\n";
                    }
                }
            }
        } else {
            let es = find_embedded_struct(s, t);
            let ext = resolve_struct(all_files, t, ns_name);
            if es != () {
                code += "            if (!Polygen.Common.CsvUtils.HeaderHasValues(__h, row)) { obj." + f.name + " = null; } else { obj." + f.name + " = global::" + ns_name + "." + es.name + "Csv.FromRowWithHeader(__h, row, gap); }\n";
            } else if ext != () {
                let child = ext;
                let call_ns = if t.contains(".") { let __ns = ""; let __i=0; let parts=t.split("."); for seg in parts { if __i<parts.len-1 { if __ns != "" { __ns += "."; } __ns += seg; } __i+=1; } __ns } else { ns_name };
                code += "            if (!Polygen.Common.CsvUtils.HeaderHasValues(__h, row)) { obj." + f.name + " = null; } else { obj." + f.name + " = global::" + call_ns + "." + child.name + "Csv.FromRowWithHeader(__h, row, gap); }\n";
            }
            else {
                if t.ends_with("__Enum") || current_struct_inline_enums.contains(t) { let enum_t = if t.contains(".") { "global::" + t } else { "global::" + s.fqn + "." + t }; code += "            if (__h != null && __h.Index >= 0 && __h.Index < row.Length) { var __cell = row[__h.Index]; obj." + f.name + " = DataSourceFactory.ConvertValue<" + enum_t + ">( __cell ); }\n"; }
                else { let cs_t = if t.contains(".") { "global::" + t } else { type_utils::map_type(t) }; code += "            if (__h != null && __h.Index >= 0 && __h.Index < row.Length) { var __cell = row[__h.Index]; obj." + f.name + " = DataSourceFactory.ConvertValue<" + cs_t + ">( __cell ); }\n"; }
            }
        }
    } }
    code += "            return obj;\n        }\n";

    // Convenience: fast read using header tree
    code += "        public static System.Collections.Generic.IEnumerable<"+full_t+"> ReadCsvFast(string path, char sep = ',', Polygen.Common.CsvUtils.GapMode gap = Polygen.Common.CsvUtils.GapMode.Break)\n        {\n";
    code += "            var lines = File.ReadAllLines(path); if (lines.Length == 0) yield break;\n";
    code += "            var header = lines[0].Split(sep);\n";
    code += "            var h = BuildHeader(header, string.Empty);\n";
    code += "            for (int i=1;i<lines.Length;i++) { var row = lines[i].Split(sep); if (!Polygen.Common.CsvUtils.HeaderHasValues(h, row)) { if (gap == Polygen.Common.CsvUtils.GapMode.Break) break; else continue; } yield return FromRowWithHeader(h, row, gap); }\n";
    code += "        }\n";
    // Dictionary-based readers omitted (header-tree unified)
    code += "}\n"; // end class
    // Also emit CSV mappers for embedded structs within this namespace (after class)
    for it in s.items { if it.is_embedded_struct() { let es = it.as_embedded_struct(); code += emit_struct(ns_name, es, all_files); } }
    code += "}\n\n"; // end namespace
    code
}

fn walk_namespace(ns_obj, prefix, all_files) {
    // ns_obj.name is already fully qualified in IR; avoid double prefixing
    let full_ns_name = ns_obj.name;
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


