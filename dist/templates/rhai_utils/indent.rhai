// Function to indent a multi-line string.
// - text: the string to indent.
// - indent_level: how many tabs to prepend to each line.
fn indent_text(text, indent_level) {
    if text == () || text.len == 0 {
        return "";
    }

    let indentation = "";
    for i in 0..indent_level {
        indentation += "	";
    }

    let lines = text.split("\n");
    let new_lines = [];
    for line in lines {
        if line.len > 0 {
            new_lines.push(indentation + line);
        } else {
            new_lines.push("");
        }
    }

    let result = "";
    let first = true;
    for line in new_lines {
        if !first {
            result += "\n";
        }
        result += line;
        first = false;
    }

    result
}

