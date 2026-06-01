// Redis key schema helper functions.

fn join_with(arr, sep) {
    let result = "";
    let i = 0;
    for item in arr {
        if i > 0 { result += sep; }
        result += item;
        i += 1;
    }
    result
}

fn json_string(value) {
    let escaped = value;
    escaped.replace("\\", "\\\\");
    escaped.replace("\"", "\\\"");
    escaped.replace("\r", "\\r");
    escaped.replace("\n", "\\n");
    "\"" + escaped + "\""
}

fn should_include_for_redis(struct_datasource, ns_datasource, cache_strategy) {
    if struct_datasource != () && struct_datasource != "" {
        return struct_datasource == "redis" || struct_datasource == "cache";
    }
    if ns_datasource != () && ns_datasource != "" {
        return ns_datasource == "redis" || ns_datasource == "cache";
    }
    cache_strategy != () && cache_strategy != ""
}

fn has_cache_annotation(s) {
    for item in s.header {
        if item.is_annotation {
            let ann = item.as_annotation();
            if ann.name == "cache" {
                return true;
            }
        }
    }
    false
}

fn annotation_param(ann, key) {
    for param in ann.params {
        if param.key == key {
            return param.value;
        }
    }
    ""
}

fn cache_strategy(s) {
    for item in s.header {
        if item.is_annotation {
            let ann = item.as_annotation();
            if ann.name == "cache" {
                if ann.positional_args.len() > 0 {
                    return ann.positional_args[0];
                }

                let strategy = annotation_param(ann, "strategy");
                if strategy != "" {
                    return strategy;
                }
            }
        }
    }
    ""
}

fn cache_ttl_seconds(s) {
    for item in s.header {
        if item.is_annotation {
            let ann = item.as_annotation();
            if ann.name == "cache" {
                return annotation_param(ann, "ttl");
            }
        }
    }
    ""
}

fn table_name_from_fqn(fqn) {
    join_with(fqn.split("."), "_")
}

fn namespace_from_fqn(fqn) {
    let parts = fqn.split(".");
    if parts.len() <= 1 {
        return "";
    }
    let ns_parts = [];
    for i in 0..(parts.len() - 1) {
        ns_parts.push(parts[i]);
    }
    join_with(ns_parts, ".")
}

fn simple_name_from_fqn(fqn) {
    let parts = fqn.split(".");
    if parts.len() == 0 {
        return fqn;
    }
    parts[parts.len() - 1]
}

fn lua_name(value) {
    let result = to_snake_case(value);
    result.replace(".", "_");
    result.replace("-", "_");
    result
}

fn redis_key_prefix(fqn) {
    "polygen:" + namespace_from_fqn(fqn) + ":" + simple_name_from_fqn(fqn)
}

fn primary_key_field(s) {
    for item in s.items {
        if item.is_field {
            let f = item.as_field();
            if f.is_primary_key {
                return f.name;
            }
        }
    }
    "id"
}
