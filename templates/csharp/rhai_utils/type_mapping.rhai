// Rhai script for C# type mapping

// Function to map language-agnostic types to C# types.
// Handles primitives, Option<T>, and List<T>.
// Coerce a possibly-TypeRef argument into a string type name
fn _as_type_string(t) {
    if type_of(t) == "TypeRef" { return t.original; }
    t
}

fn map_type(type_string) {
    let type_string = _as_type_string(type_string);
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
        _ => if type_string.contains(".") && !type_string.starts_with("global::") { "global::" + type_string } else { type_string } // custom types => fully-qualify
    };
    return mapped_type;
}
