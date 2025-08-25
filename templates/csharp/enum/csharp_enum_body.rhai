${
    let content = "";
    if item.is_comment() {
        let c = item.as_comment();
        // Add indentation for the comment
        content += "    //  " + c + "\n";
    } else if item.is_member() {        
        let m = item.as_member();
        // Add indentation for the member
        content += "    " + m.name;
        content += ",\n";
    }
    content
}