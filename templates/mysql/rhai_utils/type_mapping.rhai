// MySQL type mapping utilities

fn _as_type_string(x) { if type_of(x) == "TypeRef" { x.original } else { x } }

fn is_nullable(type_string) {
    let type_string = _as_type_string(type_string);
    type_string.starts_with("Option<")
}

fn unwrap_optional(type_string) {
    let type_string = _as_type_string(type_string);
    if type_string.starts_with("Option<") {
        let start = "Option<".len();
        let len = type_string.len() - start - 1;
        return type_string.sub_string(start, len);
    }
    type_string
}

fn is_list(type_string) {
    let type_string = _as_type_string(type_string);
    type_string.starts_with("List<")
}

fn unwrap_list(type_string) {
    let type_string = _as_type_string(type_string);
    let start = "List<".len();
    let len = type_string.len() - start - 1;
    type_string.sub_string(start, len)
}

// Returns a base MySQL type name. Caller appends length and NULL/NOT NULL.
fn base_mysql_type(type_string) {
    let type_string = _as_type_string(type_string);
    if is_list(type_string) { return "JSON"; }
    let t = unwrap_optional(type_string);

    switch t {
        "u8" => "TINYINT UNSIGNED",
        "u16" => "SMALLINT UNSIGNED",
        "u32" => "INT UNSIGNED",
        "u64" => "BIGINT UNSIGNED",
        "i8" => "TINYINT",
        "i16" => "SMALLINT",
        "i32" => "INT",
        "i64" => "BIGINT",
        "f32" => "FLOAT",
        "f64" => "DOUBLE",
        "string" => "VARCHAR",
        "bool" => "TINYINT(1)",
        "bytes" => "BLOB",
        "timestamp" => "DATETIME",
        _ => "INT" // enums/customs → integer by default
    }
}

// =============================================================================
// Timestamp Utilities
// =============================================================================

/// Check if a field has auto_create constraint
fn has_auto_create(field) {
    field.auto_create != ()
}

/// Check if a field has auto_update constraint
fn has_auto_update(field) {
    field.auto_update != ()
}

/// Get the MySQL DEFAULT clause for auto_create timestamp
fn get_auto_create_default() {
    "DEFAULT CURRENT_TIMESTAMP"
}

/// Get the MySQL ON UPDATE clause for auto_update timestamp
fn get_auto_update_clause() {
    "ON UPDATE CURRENT_TIMESTAMP"
}
