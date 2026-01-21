${
    // C++ enum generator template
    // Expects 'e' to be an EnumDef object

    let result = "";

    // Enum declaration with underlying type (i32 for binary compatibility)
    result += "enum class " + e.name + " : int32_t {\n";

    // Members
    for item in e.items {
        if item.is_member() {
            let m = item.as_member();

            // Member with optional value
            if m.value != () {
                result += "    " + m.name + " = " + m.value + ",\n";
            } else {
                result += "    " + m.name + ",\n";
            }
        } else if item.is_comment() {
            result += "    // " + item.as_comment() + "\n";
        }
    }

    result += "};\n";

    result
}
