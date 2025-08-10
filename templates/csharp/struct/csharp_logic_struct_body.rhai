${
    let body = "";
    for item in s.items 
    {    
        if item.is_comment() 
        {        
            let c = item.as_comment();
            body += "   // " + c + "\n";
        }
        if item.is_field()
        {
            let f = item.as_field();

            for attributes in f.attributes
            {
                body += "   [ " + attributes + " ]\n";
            }

            body += "   public " + f.field_type + " " + f.name + ";\n";
        }    // 현재 어노테이션(@)은 생성 코드에 포함하지 않음
    }
    body
}