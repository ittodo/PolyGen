// MySQL/MariaDB type mapping utilities

fn base_mysql_type(type_ref, max_length) {
    let base = type_ref.type_name;

    switch base {
        "u8" => "TINYINT UNSIGNED",
        "i8" => "TINYINT",
        "u16" => "SMALLINT UNSIGNED",
        "i16" => "SMALLINT",
        "u32" => "INT UNSIGNED",
        "i32" => "INT",
        "u64" => "BIGINT UNSIGNED",
        "i64" => "BIGINT",
        "f32" => "FLOAT",
        "f64" => "DOUBLE",
        "bool" => "BOOLEAN",
        "bytes" => "BLOB",
        "timestamp" => "TIMESTAMP",
        "string" => {
            if max_length != () {
                "VARCHAR(" + max_length + ")"
            } else {
                "VARCHAR(255)"
            }
        },
        _ => "INT"
    }
}

fn is_nullable(type_ref) {
    type_ref.is_option
}

fn is_array(type_ref) {
    type_ref.is_list
}

fn quote_ident(name) {
    "`" + name + "`"
}
