${
    // Unreal UENUM generator template
    // Expects 'e' to be an EnumDef object

    let result = "";

    // Add E prefix for Unreal enum naming convention
    let base_name = e.name;
    let first_char = base_name.sub_string(0, 1);
    let enum_name = if first_char == "E" { base_name } else { "E" + base_name };

    // UENUM declaration with BlueprintType for Blueprint exposure
    result += "UENUM(BlueprintType)\n";
    result += "enum class " + enum_name + " : uint8\n";
    result += "{\n";

    // Members
    let member_count = 0;
    for item in e.items {
        if item.is_member() {
            let m = item.as_member();
            member_count += 1;

            // Add UMETA for display name
            let display_name = m.name;

            // Member with optional value and UMETA
            if m.value != () {
                result += "    " + m.name + " = " + m.value + " UMETA(DisplayName = \"" + display_name + "\"),\n";
            } else {
                result += "    " + m.name + " UMETA(DisplayName = \"" + display_name + "\"),\n";
            }
        } else if item.is_comment() {
            result += "    // " + item.as_comment() + "\n";
        }
    }

    result += "};\n";

    result
}
