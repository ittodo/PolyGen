// Type inspection utilities for readers generation
// Requires global `schema` (SchemaContext) and a provided `current_ns_name` string in scope

fn strip_wrappers(type_string) {
    let t = type_string;
    if t.starts_with("Option<") {
        let start = "Option<".len();
        let len = t.len() - start - 1;
        return t.sub_string(start, len);
    }
    if t.starts_with("List<") {
        let start = "List<".len();
        let len = t.len() - start - 1;
        return t.sub_string(start, len);
    }
    t
}

fn short_name(type_string) {
    let parts = type_string.split(".");
    parts[parts.len - 1]
}

fn ns_prefix(type_string) {
    if !type_string.contains(".") { return ""; }
    let idx = type_string.last_index_of(".");
    type_string.sub_string(0, idx)
}

fn is_inline_enum_name(name) {
    name.ends_with("__Enum")
}

fn find_enum_in_ns(ns, target_name) {
    for item in ns.items {
        if item.is_enum() {
            let e = item.as_enum();
            if e.name == target_name { return true; }
        }
    }
    false
}

fn find_enum_in_tree(ns, prefix, target_ns, target_name) {
    // compute full namespace
    let fqn = if prefix == "" { ns.name } else { prefix + "." + ns.name };
    if fqn == target_ns {
        if find_enum_in_ns(ns, target_name) { return true; }
    }
    // recurse into nested namespaces
    for item in ns.items {
        if item.is_namespace() {
            let child = item.as_namespace();
            if find_enum_in_tree(child, fqn, target_ns, target_name) { return true; }
        }
    }
    false
}

fn enum_exists_at(target_ns, target_name) {
    for file in schema.files {
        for root_ns in file.namespaces {
            if find_enum_in_tree(root_ns, "", target_ns, target_name) { return true; }
        }
    }
    false
}

fn any_enum_named(target_name) {
    for file in schema.files {
        for root_ns in file.namespaces {
            if find_enum_in_ns(root_ns, target_name) { return true; }
            // traverse children
            for item in root_ns.items {
                if item.is_namespace() {
                    let child = item.as_namespace();
                    if any_enum_named_in(child, target_name, root_ns.name) { return true; }
                }
            }
        }
    }
    false
}

fn any_enum_named_in(ns, target_name, prefix) {
    if find_enum_in_ns(ns, target_name) { return true; }
    for item in ns.items {
        if item.is_namespace() {
            let child = item.as_namespace();
            if any_enum_named_in(child, target_name, if prefix == "" { ns.name } else { prefix + "." + ns.name }) { return true; }
        }
    }
    false
}

// Public API: detect if a type_string represents an enum
fn is_enum_type(type_string, current_ns_name) {
    if is_inline_enum_name(type_string) { return true; }
    let core = strip_wrappers(type_string);
    if core.contains(".") {
        let ns = ns_prefix(core);
        let name = short_name(core);
        return enum_exists_at(ns, name);
    } else {
        if current_ns_name != () && current_ns_name != "" {
            if enum_exists_at(current_ns_name, core) { return true; }
        }
        return any_enum_named(core);
    }
}

