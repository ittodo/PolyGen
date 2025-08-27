${    let header = "";
    for header_item in s.header
    {
        if header_item.is_comment()
        {
            let c = header_item.as_comment();
            header += "/// " + c + "\n";
        }
        if header_item.is_annotation()
        {
            let a = header_item.as_annotation();
            if(a.params.len()!= 0)
            {
                let params_str = "";
                let is_first = true;
                for param in a.params
                {
                    if !is_first {
                        params_str += ", ";
                    }
                    // Assuming values should be treated as strings in the attribute.
                    params_str += `${param.key} = "${param.value}"`;
                    is_first = false;
                }
                header += `[${a.name}(${params_str})]` + "\n";
            }
            else
            {
                header += `[${a.name}]` + "\n";
            }
        }
    }
    header
}
