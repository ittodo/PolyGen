// SQLite type mapping utilities

// Map poly types to SQLite types
fn base_sqlite_type(type_ref) {
    let base = type_ref.type_name;

    switch base {
        // Integer types - SQLite uses INTEGER for all
        "u8" | "u16" | "u32" | "i8" | "i16" | "i32" => "INTEGER",
        "u64" | "i64" => "INTEGER",

        // Float types
        "f32" | "f64" => "REAL",

        // String and text
        "string" => "TEXT",

        // Boolean - SQLite stores as INTEGER (0/1)
        "bool" => "INTEGER",

        // Binary data
        "bytes" => "BLOB",

        // Default for unknown/custom types (enums, structs as references)
        _ => "INTEGER"
    }
}

// Check if type is nullable (optional)
fn is_nullable(type_ref) {
    type_ref.is_option
}

// Check if type is an array/list
fn is_array(type_ref) {
    type_ref.is_list
}

// Get default value for SQLite
fn get_default_value(attributes) {
    for attr in attributes {
        if attr.starts_with("Default(") && attr.ends_with(")") {
            let start = "Default(".len();
            let value = attr.sub_string(start, attr.len() - start - 1);
            return value;
        }
    }
    ()
}

// Check if field is primary key
fn is_primary_key(attributes) {
    for attr in attributes {
        if attr == "Key" || attr == "PrimaryKey" {
            return true;
        }
    }
    false
}

// Check if field is unique
fn is_unique(attributes) {
    for attr in attributes {
        if attr.contains("IsUnique") && attr.contains("true") {
            return true;
        }
    }
    false
}

// Get max length constraint
fn get_max_length(attributes) {
    for attr in attributes {
        if attr.starts_with("MaxLength(") && attr.ends_with(")") {
            let start = "MaxLength(".len();
            return attr.sub_string(start, attr.len() - start - 1);
        }
    }
    ()
}

// Get foreign key reference
fn get_foreign_key(attributes) {
    for attr in attributes {
        if attr.starts_with("ForeignKey(") && attr.ends_with(")") {
            let start = "ForeignKey(".len();
            return attr.sub_string(start, attr.len() - start - 1);
        }
    }
    ()
}
