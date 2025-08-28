// Rhai script for C# BinaryReader read-expression mapping (little-endian)
// Supports primitives only for initial scope.

fn is_primitive(type_string) {
    switch type_string {
        "u8" => true,
        "i8" => true,
        "u16" => true,
        "i16" => true,
        "u32" => true,
        "i32" => true,
        "u64" => true,
        "i64" => true,
        "f32" => true,
        "f64" => true,
        "bool" => true,
        _ => false
    }
}

// Returns a C# expression string to read the given type from a BinaryReader identifier.
// Example: read_expr("i32", "br") => "br.ReadInt32()"
fn read_expr(type_string, br_ident) {
    switch type_string {
        "u8" => `${br_ident}.ReadByte()`,
        "i8" => `${br_ident}.ReadSByte()`,
        "u16" => `${br_ident}.ReadUInt16()`,
        "i16" => `${br_ident}.ReadInt16()`,
        "u32" => `${br_ident}.ReadUInt32()`,
        "i32" => `${br_ident}.ReadInt32()`,
        "u64" => `${br_ident}.ReadUInt64()`,
        "i64" => `${br_ident}.ReadInt64()`,
        "f32" => `${br_ident}.ReadSingle()`,
        "f64" => `${br_ident}.ReadDouble()`,
        "bool" => `${br_ident}.ReadBoolean()`,
        _ => "/* TODO: Unsupported type for binary read */"
    }
}

