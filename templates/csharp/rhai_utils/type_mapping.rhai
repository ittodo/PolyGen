// Rhai script for C# type mapping

// Function to map language-agnostic types to C# types.
// Handles primitives, Option<T>, and List<T>.
fn map_type(type_string) {
    if type_string.starts_with("Option<") {
        let start = "Option<".len();
        let len = type_string.len() - start - 1;
        let inner_type = type_string.sub_string(start, len);
        let mapped_inner = map_type(inner_type);
        // In C#, both value and reference types can use the '?' for nullability.
        return mapped_inner + "?";
    }

    if type_string.starts_with("List<") {
        let start = "List<".len();
        let len = type_string.len() - start - 1;
        let inner_type = type_string.sub_string(start, len);
        let mapped_inner = map_type(inner_type);
        return "List<" + mapped_inner + ">";
    }

    let mapped_type = switch type_string {
        "u8" => "byte",
        "u16" => "ushort",
        "u32" => "uint",
        "u64" => "ulong",
        "i8" => "sbyte",
        "i16" => "short",
        "i32" => "int",
        "i64" => "long",
        "f32" => "float",
        "f64" => "double",
        "string" => "string",
        "bool" => "bool",
        _ => type_string // Default case for custom types
    };
    return mapped_type;
}
