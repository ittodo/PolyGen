${  
    import "templates/rhai_utils/indent" as indent_utils;
    import "templates/csharp/rhai_utils/type_mapping" as type_utils;
    let body = "";
    for item in s.items 
    {
        if item.is_comment() 
        {
            let c = item.as_comment();
            body += "   // " + c + "\n";
        }
        else if item.is_field()
        {
            let f = item.as_field();

            for attributes in f.attributes
            {
                body += "   [ " + attributes + " ]\n";
            }

            body += "   public " + type_utils::map_type(f.field_type.original) + " " + f.name + ";\n";
        }
        else if item.is_embedded_struct()
        {
            let structString = render_items([item.as_embedded_struct()], "templates/csharp/struct/csharp_logic_struct.rhai", "s");

            body += indent_utils::indent_text(structString , 1) + "\n";
        }
        else if item.is_inline_enum()
        {
            let enumString = render_items([item.as_inline_enum()], "templates/csharp/enum/csharp_enum.rhai", "e");
            body += indent_utils::indent_text(enumString , 1) + "\n";
        }
    }
    body
}
