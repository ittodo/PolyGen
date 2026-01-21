${
    // Go enum generator template
    // Expects 'e' to be an EnumDef object
    // Go doesn't have native enums, so we use const + iota or explicit values

    // Check if all values are sequential starting from 0 (can use iota)
    let use_iota = true;
    let expected_value = 0;
    for item in e.items {
        if item.is_member() {
            let m = item.as_member();
            if m.value != () {
                if m.value != expected_value {
                    use_iota = false;
                }
            }
            expected_value += 1;
        }
    }

    // Generate enum type and constants
    // Prefix enum values with enum name to avoid conflicts (Go convention)
    let members = "";
    let first = true;
    for item in e.items {
        if item.is_member() {
            let m = item.as_member();
            let const_name = e.name + m.name;  // e.g., "ItemTypeWeapon"
            if use_iota {
                // Use iota pattern for sequential enums
                if first {
                    members += "\t" + const_name + " " + e.name + " = iota\n";
                    first = false;
                } else {
                    members += "\t" + const_name + "\n";
                }
            } else {
                // Use explicit values with type for each constant
                let val = if m.value != () { m.value } else { 0 };
                members += "\t" + const_name + " " + e.name + " = " + val + "\n";
            }
        } else if item.is_comment() {
            members += "\t// " + item.as_comment() + "\n";
        }
    }

    let result = "";
    result += "type " + e.name + " int32\n\n";
    result += "const (\n";
    result += members;
    result += ")\n";

    result
}
