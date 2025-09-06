// Render CREATE TABLE for a StructDef s within namespace ns.
${
    import "templates/mysql/rhai_utils/type_mapping" as type_utils;

    // Table name (no namespace prefix in this minimal version)
    let table_name = s.name;

    let ddl = "CREATE TABLE " + table_name + " (\n";
    let first = true;
    let pk_cols = [];
    let uniq_cols = [];

    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            let col_name = f.name; // embedded struct flattening: TODO

            // constraints
            for a in f.attributes {
                if a == "Key" { pk_cols.push(col_name); }
                if a.contains("IsUnique") && a.contains("true") { uniq_cols.push(col_name); }
            }

            let base_type = type_utils::base_mysql_type(f.field_type);
            let is_null = type_utils::is_nullable(f.field_type);
            let line = if base_type == "VARCHAR" {
                // derive MaxLength
                let ml_str = "255";
                for a in f.attributes {
                    if a.starts_with("MaxLength(") && a.ends_with(")") {
                        let start = "MaxLength(".len();
                        ml_str = a.sub_string(start, a.len() - start - 1);
                    }
                }
                col_name + " " + base_type + "(" + ml_str + ")" + (if is_null { " NULL" } else { " NOT NULL" })
            } else {
                col_name + " " + base_type + (if is_null { " NULL" } else { " NOT NULL" })
            };

            if !first { ddl += ",\n"; } else { first = false; }
            ddl += "  " + line;
        }
        // ignore comments/enums/embedded structs for now
    }

    if pk_cols.len() > 0 {
        let cols = "";
        let i = 0;
        for c in pk_cols { if i > 0 { cols += ", "; } cols += c; i += 1; }
        if !first { ddl += ",\n"; } else { first = false; }
        ddl += "  PRIMARY KEY (" + cols + ")";
    }

    for u in uniq_cols {
        if !first { ddl += ",\n"; } else { first = false; }
        ddl += "  UNIQUE KEY " + u + "_uniq (" + u + ")";
    }

    ddl += "\n) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;\n";
    ddl
}
