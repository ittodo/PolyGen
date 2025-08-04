let result = "";

for ns in schema.namespaces {
    result += `using System;\n`;
    result += `using System.Collections.Generic;\n`;
    result += `using System.ComponentModel.DataAnnotations;\n`;
    result += `using System.ComponentModel.DataAnnotations.Schema;\n\n`;
    result += `namespace ${ns.name}\n{\n`;

    for item in ns.items {
        if item.is_struct() {
            let s = item.as_struct();
            result += `    public partial class ${s.name}\n    {\n`;

            for struct_item in s.items {
                if struct_item.is_field() {
                    let field = struct_item.as_field();
                    for attr in field.attributes {
                        result += `        [${attr}]\n`;
                    }
                    result += `        public ${field.field_type} ${field.name} { get; set; }\n`;
                }
            }

            result += `    }\n\n`;
        }
    }

    result += `}\n`;
}

result