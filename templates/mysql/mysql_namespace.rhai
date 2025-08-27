// Recursively render DDL for a namespace

${
    let ddl = "";
    for item in ns.items {
        if item.is_struct() {
            let part = render_items([item.as_struct()], "templates/mysql/mysql_struct.rhai", "s");
            if part.trim() != "" { ddl += part + "\n"; }
        } else if item.is_namespace() {
            let part = render_items([item.as_namespace()], "templates/mysql/mysql_namespace.rhai", "ns");
            if part.trim() != "" { ddl += part + "\n"; }
        }
    }
    ddl
}
