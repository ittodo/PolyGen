// PostgreSQL type mapping utilities

fn base_postgresql_type(type_ref, max_length) {
    let base = type_ref.type_name;

    switch base {
        "u8" | "i8" | "u16" | "i16" => "SMALLINT",
        "u32" | "i32" => "INTEGER",
        "u64" | "i64" => "BIGINT",
        "f32" => "REAL",
        "f64" => "DOUBLE PRECISION",
        "bool" => "BOOLEAN",
        "bytes" => "BYTEA",
        "timestamp" => "TIMESTAMPTZ",
        "string" => {
            if max_length != () {
                "VARCHAR(" + max_length + ")"
            } else {
                "TEXT"
            }
        },
        _ => "INTEGER"
    }
}

fn is_nullable(type_ref) {
    type_ref.is_option
}

fn is_array(type_ref) {
    type_ref.is_list
}

fn quote_ident(name) {
    "\"" + name + "\""
}
