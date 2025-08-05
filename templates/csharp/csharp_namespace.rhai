 `using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

${
    let ns_block = "";
    if ns.name != "" {
        ns_block = `namespace ${ns.name}
{
`;
    }
    ns_block
}${
    let content = "";
    for item in ns.items {
        if item.is_struct() {
            let s = item.as_struct();
            let fields = {
                let field_content = "";
                for struct_item in s.items {
                    if struct_item.is_field() {
                        let field = struct_item.as_field();
                        let attributes = {
                            let attr_content = "";
                            for attr in field.attributes {
                                attr_content += `        [${attr}]
`;
                            }
                            attr_content
                        };
                        field_content += `${attributes}        public ${field.field_type} ${field.name} { get; set; }
`;
                    }
                }
                field_content
            };
            content += `    public partial class ${s.name}
    {
${fields}    }

`;
        } else if item.is_enum() {
            let e = item.as_enum();
            let members = {
                let member_content = "";
                for enum_item in e.items {
                    if enum_item.is_member() {
                        let member = enum_item.as_member();
                        member_content += `        ${member.name},
`;
                    }
                }
                member_content
            };
            content += `    public enum ${e.name}
    {
${members}    }

`;
        }
    }
    content
}${
    let closing_brace = "";
    if ns.name != "" {
        closing_brace = "}";
    }
    closing_brace
}
`