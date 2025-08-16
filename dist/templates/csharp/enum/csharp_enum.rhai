// This template generates a C# enum definition.
// It expects 'e' to be an EnumDef object.

public enum ${e.name}
{
    ${
        let content = "";
        let is_first_member = true; // 멤버 사이에 쉼표를 처리하기 위함

        for item in e.items {
            if item.is_comment() {
                let c = item.as_comment();
                content += "    // " + c + "\r\n"; // 주석 들여쓰기
            } else if item.is_member() {
                if !is_first_member {
                    content += ",\r\n"; // 첫 멤버가 아니면 앞에 쉼표 추가
                }
                let m = item.as_member();
                content += "    " + m.name + ""; // 멤버 이름
                is_first_member = false;
            }
        }
        content
    }
}