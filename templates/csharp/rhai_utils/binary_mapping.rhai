// Unified binary read/write expression mapping for C# BinaryReader/BinaryWriter
// Supports primitives, strings, enums, and custom types

// =============================================================================
// Primitive Type Checking
// =============================================================================

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
        "timestamp" => true,
        _ => false
    }
}

// =============================================================================
// C# Type Mapping (poly type → C# type)
// =============================================================================

fn cs_type(poly_type) {
    switch poly_type {
        "u8" => "byte",
        "i8" => "sbyte",
        "u16" => "ushort",
        "i16" => "short",
        "u32" => "uint",
        "i32" => "int",
        "u64" => "ulong",
        "i64" => "long",
        "f32" => "float",
        "f64" => "double",
        "bool" => "bool",
        "string" => "string",
        "bytes" => "byte[]",
        "timestamp" => "DateTime",
        _ => poly_type
    }
}

// =============================================================================
// Read Expressions
// =============================================================================

/// Returns a C# expression string to read the given type from a BinaryReader.
/// Example: read_expr("i32", "br") => "br.ReadInt32()"
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
        "timestamp" => `new DateTime(${br_ident}.ReadInt64(), DateTimeKind.Utc)`,
        _ => "/* TODO: Unsupported type for binary read */"
    }
}

// =============================================================================
// Write Expressions
// =============================================================================

/// Returns a C# expression string to write the given type to a BinaryWriter.
/// Example: write_expr("i32", "bw", "value") => "bw.Write(value)"
fn write_expr(type_string, bw_ident, value_expr) {
    switch type_string {
        "u8" => `${bw_ident}.Write(${value_expr})`,
        "i8" => `${bw_ident}.Write(${value_expr})`,
        "u16" => `${bw_ident}.Write(${value_expr})`,
        "i16" => `${bw_ident}.Write(${value_expr})`,
        "u32" => `${bw_ident}.Write(${value_expr})`,
        "i32" => `${bw_ident}.Write(${value_expr})`,
        "u64" => `${bw_ident}.Write(${value_expr})`,
        "i64" => `${bw_ident}.Write(${value_expr})`,
        "f32" => `${bw_ident}.Write(${value_expr})`,
        "f64" => `${bw_ident}.Write(${value_expr})`,
        "bool" => `${bw_ident}.Write(${value_expr})`,
        "timestamp" => `${bw_ident}.Write(${value_expr}.Ticks)`,
        _ => "/* TODO: Unsupported type for binary write */"
    }
}

/// Returns a C# lambda expression for writing the given primitive type.
/// Example: write_lambda("i32") => "(bw2,v)=>bw2.Write(v)"
fn write_lambda(type_string) {
    if type_string == "timestamp" {
        "(bw2,v)=>bw2.Write(v.Ticks)"
    } else if is_primitive(type_string) {
        "(bw2,v)=>bw2.Write(v)"
    } else {
        "/* TODO: Unsupported type for binary write lambda */"
    }
}

/// Returns a C# lambda expression for reading the given primitive type.
/// Example: read_lambda("i32", "br") => "_ => br.ReadInt32()"
fn read_lambda(type_string, br_ident) {
    if type_string == "timestamp" {
        `_ => new DateTime(${br_ident}.ReadInt64(), DateTimeKind.Utc)`
    } else if is_primitive(type_string) {
        `_ => ${read_expr(type_string, br_ident)}`
    } else {
        "/* TODO: Unsupported type for binary read lambda */"
    }
}
