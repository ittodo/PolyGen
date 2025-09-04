import "templates/rhai_utils/indent" as indent_utils;

fn unwrap_option(t) {
    let t = if type_of(t) == "TypeRef" { t.original } else { t };
    if t.starts_with("Option<") {
        let s = "Option<".len();
        let l = t.len() - s - 1;
        return t.sub_string(s, l);
    }
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

// find embedded struct by name within a struct 's'
fn find_embedded_struct(s, name) {
    for it in s.items { if it.is_embedded_struct() { let es = it.as_embedded_struct(); if es.name == name { return es; } } }
    ()
}

// Recursively collect columns according to rules
// Lookup helpers across schema
fn find_struct_in_ns(ns, target_name) {
    for item in ns.items {
        if item.is_struct() {
            let s = item.as_struct();
            if s.name == target_name { return s; }
        }
    }
    ()
}

fn find_struct_in_tree(ns, prefix, target_ns, target_name) {
    let fqn = if prefix == "" { ns.name } else { prefix + "." + ns.name };
    if fqn == target_ns {
        let s = find_struct_in_ns(ns, target_name);
        if s != () { return s; }
    }
    for item in ns.items {
        if item.is_namespace() {
            let child = item.as_namespace();
            let s = find_struct_in_tree(child, fqn, target_ns, target_name);
            if s != () { return s; }
        }
    }
    ()
}

fn get_struct_at(all_files, target_ns, target_name) {
    for file in all_files {
        for root_ns in file.namespaces {
            let s = find_struct_in_tree(root_ns, "", target_ns, target_name);
            if s != () { return s; }
        }
    }
    ()
}

fn any_struct_named(all_files, target_name) {
    for file in all_files {
        for root_ns in file.namespaces {
            let s = find_struct_in_ns(root_ns, target_name);
            if s != () { return s; }
            for item in root_ns.items { if item.is_namespace() { let child = item.as_namespace(); let t = any_struct_named_in(child, target_name, root_ns.name); if t != () { return t; } } }
        }
    }
    ()
}

fn any_struct_named_in(ns, target_name, prefix) {
    let s = find_struct_in_ns(ns, target_name);
    if s != () { return s; }
    for item in ns.items { if item.is_namespace() { let child = item.as_namespace(); let t = any_struct_named_in(child, target_name, if prefix == "" { ns.name } else { prefix + "." + ns.name }); if t != () { return t; } } }
    ()
}

fn resolve_struct(all_files, type_string, current_ns_name) {
    let core = unwrap_option(type_string);
    if core.starts_with("List<") {
        let ls = "List<".len(); let ll = core.len() - ls - 1; let inner = core.sub_string(ls, ll);
        return resolve_struct(all_files, inner, current_ns_name);
    }
    if core.contains(".") {
        // fully qualified via split
        let parts = core.split(".");
        let name = parts[parts.len-1];
        let ns = ""; let i = 0;
        for seg in parts { if i < parts.len-1 { if ns != "" { ns += "."; } ns += seg; } i += 1; }
        return get_struct_at(all_files, ns, name);
    } else {
        // try current namespace, then any
        if current_ns_name != () && current_ns_name != "" {
            let s = get_struct_at(all_files, current_ns_name, core);
            if s != () { return s; }
        }
        return any_struct_named(all_files, core);
    }
}

fn collect_columns_with(ctx_struct, prefix, type_string, visited, depth, current_ns_name, all_files) {
    let cols = [];
    let t = unwrap_option(type_string);

    if depth >= 10 {
        // Depth guard reached; stop here.
        cols.push(prefix);
        return cols;
    }

    if t.starts_with("List<") {
        let ls = "List<".len();
        let ll = t.len() - ls - 1;
        let inner = t.sub_string(ls, ll);
        let np = if prefix == "" { "[0]" } else { prefix + "[0]" };
        let sub = collect_columns_with(ctx_struct, np, inner, visited, depth + 1, current_ns_name, all_files);
        for c in sub { cols.push(c); }
        return cols;
    }

    // Embedded struct flattening using dot; detect cycles
    let es = find_embedded_struct(ctx_struct, t);
    if es != () {
        if visited.contains(es.name) {
            print("Error: Circular embed detected at '" + es.name + "' while collecting CSV columns for struct '" + ctx_struct.name + "'.");
            return cols; // stop on cycle
        }
        let next_visited = visited + [es.name];
        for it in es.items {
            if it.is_field() {
                let f = it.as_field();
                let np = if prefix == "" { f.name } else { prefix + "." + f.name };
                let sub = collect_columns_with(es, np, f.field_type, next_visited, depth + 1, current_ns_name, all_files);
                for c in sub { cols.push(c); }
            }
        }
        return cols;
    }

    // External struct flattening
    let ext = resolve_struct(all_files, t, current_ns_name);
    if ext != () {
        if visited.contains(ext.name) {
            print("Error: Circular type reference detected at '" + ext.name + "' while collecting CSV columns for struct '" + ctx_struct.name + "'.");
            return cols;
        }
        let next_visited = visited + [ext.name];
        for it in ext.items {
            if it.is_field() {
                let f = it.as_field();
                let np = if prefix == "" { f.name } else { prefix + "." + f.name };
                let sub = collect_columns_with(ext, np, f.field_type, next_visited, depth + 1, current_ns_name, all_files);
                for c in sub { cols.push(c); }
            }
        }
        return cols;
    }

    // Base case: primitive/enum/external types
    cols.push(prefix);
    cols
}

for file in schema.files {
    if file.path == () || file.path == "" { continue; }
    for ns in file.namespaces {
        for item in ns.items {
            if item.is_struct() {
                let s = item.as_struct();
                // Build header following rules: embed uses dot, list uses [0], option is transparent, enums included
                let headers = [];
                for it in s.items {
                    if it.is_field() {
                        let f = it.as_field();
                        let current_ns_name = ns.name;
                        let cols = collect_columns_with(s, f.name, f.field_type, [s.name], 0, current_ns_name, schema.files);
                        for c in cols { headers.push(c); }
                    }
                }
                let header_line = "";
                let first = true;
                for h in headers { if !first { header_line += ","; } header_line += h; first = false; }

                let out_dir = "output/csharp/CsvSchemas";
                let out_name = ns.name + "." + s.name + ".csv";
                let final_path = out_dir + "/" + out_name;
                write_file(final_path, header_line + "\n");
            }
        }
    }
}

""
