// Rhai helpers for generating CSV mappers without defining functions inside eval blocks

fn unwrap_option(t) {
    let t = if type_of(t) == "TypeRef" { t.original } else { t };
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

fn find_embedded_struct(cur_s, name) {
    for it in cur_s.items { if it.is_embedded_struct() { let es = it.as_embedded_struct(); if es.name == name { return es; } } }
    ()
}

fn find_struct_in_ns(ns_obj, target_name) {
    for item in ns_obj.items { if item.is_struct() { let s = item.as_struct(); if s.name == target_name { return s; } } }
    ()
}

fn find_struct_in_tree(ns_obj, prefix, target_ns, target_name) {
    let fqn = if prefix == "" { ns_obj.name } else { prefix + "." + ns_obj.name };
    if fqn == target_ns {
        let s = find_struct_in_ns(ns_obj, target_name);
        if s != () { return s; }
    }
    for item in ns_obj.items { if item.is_namespace() { let child = item.as_namespace(); let s = find_struct_in_tree(child, fqn, target_ns, target_name); if s != () { return s; } } }
    ()
}

fn get_struct_at(all_files, target_ns, target_name) {
    for file in all_files { for root_ns in file.namespaces { let s = find_struct_in_tree(root_ns, "", target_ns, target_name); if s != () { return s; } } }
    ()
}

fn any_struct_named_in(ns_obj, target_name, prefix) {
    let s = find_struct_in_ns(ns_obj, target_name);
    if s != () { return s; }
    for item in ns_obj.items { if item.is_namespace() { let child = item.as_namespace(); let t = any_struct_named_in(child, target_name, if prefix == "" { ns_obj.name } else { prefix + "." + ns_obj.name }); if t != () { return t; } } }
    ()
}

fn any_struct_named(all_files, target_name) {
    for file in all_files { for root_ns in file.namespaces { let s = find_struct_in_ns(root_ns, target_name); if s != () { return s; } for item in root_ns.items { if item.is_namespace() { let child = item.as_namespace(); let t = any_struct_named_in(child, target_name, root_ns.name); if t != () { return t; } } } } }
    ()
}

// Variant helpers that also return the fully qualified namespace string
fn find_struct_with_ns_in_tree(ns, prefix, target_ns, target_name) {
    let fqn = if prefix == "" { ns.name } else { prefix + "." + ns.name };
    if fqn == target_ns {
        let s = find_struct_in_ns(ns, target_name);
        if s != () { return [s, fqn]; }
    }
    for item in ns.items {
        if item.is_namespace() {
            let child = item.as_namespace();
            let res = find_struct_with_ns_in_tree(child, fqn, target_ns, target_name);
            if res != () { return res; }
        }
    }
    ()
}

fn get_struct_with_ns_at(all_files, target_ns, target_name) {
    for file in all_files {
        for root_ns in file.namespaces {
            let res = find_struct_with_ns_in_tree(root_ns, "", target_ns, target_name);
            if res != () { return res; }
        }
    }
    ()
}

fn any_struct_named_with_ns_in(ns_obj, target_name, prefix) {
    let s = find_struct_in_ns(ns_obj, target_name);
    if s != () {
        let fqn = if prefix == "" { ns_obj.name } else { prefix + "." + ns_obj.name };
        return [s, fqn];
    }
    for item in ns_obj.items {
        if item.is_namespace() {
            let child = item.as_namespace();
            let next_prefix = if prefix == "" { ns_obj.name } else { prefix + "." + ns_obj.name };
            let res = any_struct_named_with_ns_in(child, target_name, next_prefix);
            if res != () { return res; }
        }
    }
    ()
}

fn any_struct_named_with_ns(all_files, target_name) {
    for file in all_files {
        for root_ns in file.namespaces {
            let s = find_struct_in_ns(root_ns, target_name);
            if s != () { return [s, root_ns.name]; }
            for item in root_ns.items {
                if item.is_namespace() {
                    let child = item.as_namespace();
                    let res = any_struct_named_with_ns_in(child, target_name, root_ns.name);
                    if res != () { return res; }
                }
            }
        }
    }
    ()
}

fn resolve_struct_with_ns(all_files, type_string, current_ns_name) {
    let core = unwrap_option(type_string);
    if core.starts_with("List<") {
        let ls = "List<".len(); let ll = core.len() - ls - 1; let inner = core.sub_string(ls, ll);
        return resolve_struct_with_ns(all_files, inner, current_ns_name);
    }
    if core.contains(".") {
        let parts = core.split("."); let name = parts[parts.len-1];
        let ns = ""; let i = 0; for seg in parts { if i < parts.len-1 { if ns != "" { ns += "."; } ns += seg; } i += 1; }
        let res = get_struct_with_ns_at(all_files, ns, name);
        if res != () { return res; }
        return [(), ns];
    } else {
        if current_ns_name != () && current_ns_name != "" {
            let res = get_struct_with_ns_at(all_files, current_ns_name, core);
            if res != () { return res; }
        }
        return any_struct_named_with_ns(all_files, core);
    }
}

fn resolve_struct(all_files, type_string, current_ns_name) {
    let core = unwrap_option(type_string);
    if core.starts_with("List<") { let ls="List<".len(); let ll=core.len()-ls-1; let inner=core.sub_string(ls,ll); return resolve_struct(all_files, inner, current_ns_name); }
    if core.contains(".") {
        let parts = core.split("."); let name = parts[parts.len-1]; let ns=""; let i=0; for seg in parts { if i<parts.len-1 { if ns != "" { ns += "."; } ns += seg; } i+=1; }
        return get_struct_at(all_files, ns, name);
    } else {
        if current_ns_name != () && current_ns_name != "" { let s = get_struct_at(all_files, current_ns_name, core); if s != () { return s; } }
        return any_struct_named(all_files, core);
    }
}

// Collect headers for a type (ported from columns generator)
fn csv_collect_headers(ctx_struct, prefix, type_string, visited, depth, current_ns_name, all_files) {
    let cols = [];
    let t = unwrap_option(type_string);
    if depth >= 10 {
        cols.push(prefix);
        return cols;
    }
    if t.starts_with("List<") {
        let ls = "List<".len();
        let ll = t.len() - ls - 1;
        let inner = t.sub_string(ls, ll);
        let np = if prefix == "" { "[0]" } else { prefix + "[0]" };
        let sub = csv_collect_headers(ctx_struct, np, inner, visited, depth + 1, current_ns_name, all_files);
        for c in sub { cols.push(c); }
        return cols;
    }
    let es = find_embedded_struct(ctx_struct, t);
    if es != () {
        if visited.contains(es.name) {
            return cols;
        }
        let next_visited = visited + [es.name];
        for it in es.items {
            if it.is_field() {
                let f = it.as_field();
                let np = if prefix == "" { f.name } else { prefix + "." + f.name };
                let sub = csv_collect_headers(es, np, f.field_type, next_visited, depth + 1, current_ns_name, all_files);
                for c in sub { cols.push(c); }
            }
        }
        return cols;
    }
    let ext = resolve_struct(all_files, t, current_ns_name);
    if ext != () {
        if visited.contains(ext.name) {
            return cols;
        }
        let next_visited = visited + [ext.name];
        for it in ext.items {
            if it.is_field() {
                let f = it.as_field();
                let np = if prefix == "" { f.name } else { prefix + "." + f.name };
                let sub = csv_collect_headers(ext, np, f.field_type, next_visited, depth + 1, current_ns_name, all_files);
                for c in sub { cols.push(c); }
            }
        }
        return cols;
    }
    cols.push(prefix);
    cols
}

fn csv_headers_for_struct(cur_struct, current_ns_name, all_files) {
    let headers = [];
    for it in cur_struct.items {
        if it.is_field() {
            let f = it.as_field();
            let sub = csv_collect_headers(cur_struct, f.name, f.field_type, [cur_struct.name], 0, current_ns_name, all_files);
            for c in sub { headers.push(c); }
        }
    }
    headers
}

// Generate C# append code for a type (adds to variable 'cols')
fn csv_append_code_for(ctx_struct, type_string, expr_prefix, current_ns_name, all_files, visited, depth) {
    let code = "";
    let t = unwrap_option(type_string);
    if depth >= 10 { code += "cols.Add(string.Empty);\n"; return code; }
    if t.starts_with("List<") {
        let ls="List<".len(); let ll=t.len()-ls-1; let inner=t.sub_string(ls,ll);
        code += "if ("+expr_prefix+" != null && "+expr_prefix+".Count > 0) {\n";
        code += csv_append_code_for(ctx_struct, inner, expr_prefix+"[0]", current_ns_name, all_files, visited, depth+1);
        code += "} else {\n";
        // empty slots for inner columns
        let tmp_headers = csv_collect_headers(ctx_struct, "", inner, visited, depth+1, current_ns_name, all_files);
        for _h in tmp_headers { code += "cols.Add(string.Empty);\n"; }
        code += "}\n";
        return code;
    }
    if is_primitive_like(t) { code += "cols.Add(CsvUtils.ToStringInvariant("+expr_prefix+"));\n"; return code; }
    if t.ends_with("__Enum") { code += "cols.Add(("+expr_prefix+").ToString());\n"; return code; }
    let es = find_embedded_struct(ctx_struct, t);
    if es != () { code += "if ("+expr_prefix+" == null) { for (int i=0;i< "+es.name+"Csv.ColumnCount_"+es.name+"(); i++) cols.Add(string.Empty);} else { "+es.name+"Csv.AppendRow("+expr_prefix+", cols); }\n"; return code; }
    let res = resolve_struct_with_ns(all_files, t, current_ns_name);
    if res != () {
        let ext = res[0]; let ns_fqn = res[1];
        if ext != () && ns_fqn != () {
            code += "if ("+expr_prefix+" == null) { for (int i=0;i< "+ns_fqn+"."+ext.name+"Csv.ColumnCount_"+ext.name+"(); i++) cols.Add(string.Empty);} else { "+ns_fqn+"."+ext.name+"Csv.AppendRow("+expr_prefix+", cols); }\n";
            return code;
        }
    }
    // default
    code += "cols.Add(string.Empty);\n";
    code
}
