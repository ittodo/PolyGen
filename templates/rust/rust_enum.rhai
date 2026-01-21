${
    // Rust enum generator template
    // Expects 'e' to be an EnumDef object

    // Generate enum members
    let members = "";
    let first_member = true;
    for item in e.items {
        if item.is_member() {
            let m = item.as_member();
            // Add #[default] attribute to first member
            if first_member {
                members += "    #[default]\n";
                first_member = false;
            }
            if m.value != () {
                members += "    " + m.name + " = " + m.value + ",\n";
            } else {
                members += "    " + m.name + ",\n";
            }
        } else if item.is_comment() {
            members += "    // " + item.as_comment() + "\n";
        }
    }

    let result = "";
    result += "#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]\n";
    result += "#[repr(i32)]\n";
    result += "pub enum " + e.name + " {\n";
    result += members;
    result += "}\n";

    result
}
