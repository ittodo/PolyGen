${
    let content = "";
    let is_first_member = true;

        if item.is_comment() {
            let c = item.as_comment();
            // Add indentation for the comment
            content += "    //  " + c + "\r\n";
        } else if item.is_member() {
            if !is_first_member {
                // Add a comma before the next member
                content += ",\r\n";
            }
            let m = item.as_member();
            // Add indentation for the member
            content += "    " + m.name;
            content += "\r\n";
            is_first_member = false;
        }
    content
}