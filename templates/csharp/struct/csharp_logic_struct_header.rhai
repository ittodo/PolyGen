${
    let header = "";
    for header_item in s.header
    {
        if header_item.is_comment()
        {
            let c = header_item.as_comment();
            header += `// ${c}
`;
        }
        if header_item.is_annotation()
        {
            let a = header_item.as_annotation();
            if(a.params.len()!= 0)
            {
                let is_first = true;
                header += `[${a.name}`;
                header += "(";
                for param in a.params
                {
                    if !is_first
                    {
                        header += ','
                    }
                    header += `${param.key} : ${param.value}` ;
                    is_first = false;
                }
                header+= ")]";
            }
            else
            {
                header += `[${a.name}]
`;

            }
        }
    }
    header
}
