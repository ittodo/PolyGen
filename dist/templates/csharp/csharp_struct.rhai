using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;
using System.ComponentModel.DataAnnotations.Schema;

namespace ${ns.name}
{
    ${
        let header_content = ""; // 'comment' 변수명을 'header_content'로 변경하여 명확하게 함
        for header_item in s.header
        {
            if header_item.is_comment()
            {
                let c = header_item.as_comment();
                header_content += "    // " + c + "\r\n";
            }
            if header_item.is_annotation()
            {
                let annotation = header_item.as_annotation();
                // AnnotationDef는 더 이상 'comment' 필드를 가지지 않습니다.
                // 어노테이션에 대한 주석은 앞에 별개의 StructItem::Comment로 존재합니다.
                header_content += "    [" + annotation.name + "]\r\n";
            }
        }
        header_content // 수집된 헤더 내용 출력
    }


    public partial class ${s.name}
    {
        ${
            let field_content = "";
            for struct_item in s.items {
                if struct_item.is_field() {
                    let field = struct_item.as_field();
                    let attributes = {
                        let attr_content = "";
                        for attr in field.attributes {
                            attr_content += "        [" + attr + "]\r\n";
                        }
                        attr_content
                    };
                    field_content += attributes + "        public " + field.field_type + " " + field.name + " { get; set; }\r\n";
                } else if struct_item.is_comment() {
                    let c = struct_item.as_comment();
                    field_content += "        // " + c + "\r\n";
                } else if struct_item.is_annotation() {
                    let annotation = struct_item.as_annotation();
                    field_content += "        [" + annotation.name + "]\r\n";
                }
            }
            field_content
        }
    }
}