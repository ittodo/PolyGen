# PolyTemplate (.ptpl) Language Guide

PolyTemplate은 PolyGen의 코드 생성 템플릿 언어입니다. 디렉티브(`%`)로 제어 흐름을, 인터폴레이션(`{{}}`)으로 값 치환을 수행합니다.

---

## 기본 구조

```ptpl
%logic
// Rhai 코드 (데이터 준비)
let greeting = "Hello";
%endlogic
%if show_header
// {{greeting}}, {{name}}!
%endif
```

- **디렉티브 라인** (`%`로 시작): 제어 흐름, 출력하지 않음
- **출력 라인** (그 외): 그대로 출력, `{{expr}}` 부분만 치환
- **`%blank`**: 빈 줄 출력

---

## 디렉티브

### %logic / %endlogic

Rhai 코드 블록. 데이터 준비, 함수 정의, 복잡한 로직 처리.

```ptpl
%logic
import "templates/cpp/rhai_utils/type_mapping" as cpp_types;

fn get_cpp_type(field) {
    cpp_types::map_primitive_type(field.field_type.type_name)
}

let type_name = get_cpp_type(field);
let has_content = items.len() > 0;
%endlogic
```

- prelude로 등록된 `.rhai` 함수 사용 가능
- `import` 문으로 다른 `.rhai` 유틸리티 불러오기
- 변수는 `{{}}` 인터폴레이션에서 사용 가능
- `set_output("text")`: 직접 출력 설정 (특수한 경우)

### %if / %elif / %else / %endif

조건부 출력. 조건은 Rhai 엔진으로 평가.

```ptpl
%if field.is_primary_key
    [PrimaryKey]
%elif field.is_optional
    {{field.field_type | lang_type}}? {{field.name}};
%else
    {{field.field_type | lang_type}} {{field.name}};
%endif
```

지원하는 조건 표현식:
- 프로퍼티: `field.is_primary_key`, `struct.has_pk`
- 비교: `==`, `!=`, `>`, `<`, `>=`, `<=`
- 논리: `&&`, `||`, `!`
- 메서드 호출: `items.len() > 0`
- 문자열 비교: `field.name == "id"`

### %for / %endfor

컬렉션 반복.

```ptpl
%for field in struct.fields
    public {{field.field_type | lang_type}} {{field.name}};
%endfor
```

선택적 where 필터:
```ptpl
%for field in struct.fields | where field.is_primary_key
    // PK: {{field.name}}
%endfor
```

### %while / %endwhile

조건 반복 (최대 10,000회).

```ptpl
%let count = 0
%while count < 5
    Item {{count}}
    %set count = count + 1
%endwhile
```

### %match / %when / %endmatch

패턴 매칭.

```ptpl
%match field.field_type.type_name
%when "u32"
    uint
%when "string"
    string
%when "bool" if field.is_optional
    bool?
%else
    object
%endmatch
```

- 문자열 패턴: `"value"`
- 와일드카드: `_`
- 가드 조건: `%when "pattern" if condition`
- 첫 번째 매칭에서 중단

### %block / %render

재사용 가능한 블록 정의 및 렌더링.

```ptpl
%block field_line(f)
    public {{f.field_type | lang_type}} {{f.name}};
%endblock

%for field in struct.fields
%render field_line with field
%endfor
```

동적 블록 이름:
```ptpl
%let block_name = "field_line"
%render $block_name with field
```

### %include

다른 `.ptpl` 파일 포함.

```ptpl
%include "detail/field_info" with field
%include "section/header" with struct, prefix="game"
%include "common/footer" indent=2
```

- `with binding`: 포커스 바인딩 (컨텍스트 객체)
- `key=value`: 추가 변수 전달
- `indent=N`: N레벨 들여쓰기 (4스페이스/레벨)
- 포함된 파일의 `%let`/`%logic` 는 부모와 격리됨
- 최대 깊이: 16

### %let / %set

변수 할당.

```ptpl
%let name = "hello"
%let count = 42
%let pi = 3.14
%let active = true
%set name = struct.name
```

- `%let`: 새 변수 생성
- `%set`: 기존 변수 재할당
- 지원 값: 문자열 리터럴, 정수, 실수, 불린, 프로퍼티 경로

### %blank

빈 줄 출력.

```ptpl
#pragma once
%blank
#include <string>
```

### %-- (주석)

주석 (출력하지 않음).

```ptpl
%-- 이 줄은 무시됩니다
output line
```

---

## 인터폴레이션

`{{expr}}` 또는 `{{expr | filter1 | filter2}}` 형식.

```ptpl
{{struct.name}}                          // 프로퍼티 접근
{{field.field_type | lang_type}}         // 필터 적용
{{items | join(", ")}}                   // 파라미터 있는 필터
{{name | snake_case | upper}}            // 필터 체이닝 (왼→오)
```

### 프로퍼티 경로

점(`.`)으로 구분된 경로로 값 접근:

```
struct.name                  → 구조체 이름
field.field_type.type_name   → 필드 타입의 타입명
field.field_type.is_option   → Optional 여부
namespace.items              → 네임스페이스 아이템 목록
```

우선순위: `%let`/`%set` 바인딩 → `%logic` 변수 → 템플릿 컨텍스트

---

## 필터

### 문자열 케이스 변환

| 필터 | 입력 → 출력 | 용도 |
|------|------------|------|
| `upper` | hello → HELLO | 대문자 |
| `lower` | HELLO → hello | 소문자 |
| `pascal_case` | hello_world → HelloWorld | 클래스명 |
| `snake_case` | HelloWorld → hello_world | 변수명 |
| `camel_case` | hello_world → helloWorld | JS 변수명 |

### 문자열 조작

| 필터 | 설명 | 예시 |
|------|------|------|
| `quote` | 쌍따옴표 감싸기 | `value` → `"value"` |
| `suffix("str")` | 문자열 뒤에 추가 | `name\|suffix("Type")` → `nameType` |
| `prefix("str")` | 문자열 앞에 추가 | `type\|prefix("I")` → `IType` |
| `remove_dots` | `.` 제거 | `a.b.c` → `abc` |

### 컬렉션

| 필터 | 설명 | 예시 |
|------|------|------|
| `count` | 길이 | `fields\|count` → `5` |
| `join("sep")` | 구분자로 연결 | `names\|join(", ")` → `a, b, c` |

### 타입 시스템 (TOML 설정 기반)

| 필터 | 용도 |
|------|------|
| `lang_type` | IR 타입 → 언어별 타입 매핑 |
| `binary_read` | 바이너리 읽기 표현식 |
| `binary_read_option` | Optional 바이너리 읽기 |
| `binary_read_list` | List 바이너리 읽기 |
| `binary_read_struct` | Struct 바이너리 읽기 |
| `csv_read` | CSV 읽기 표현식 |
| `is_embedded` | 임베디드 여부 체크 |

---

## TOML 연동

각 언어의 `.toml` 설정 파일에서 타입 매핑과 필터 동작 정의:

```toml
[type_map]
u32 = "uint"
string = "string"
optional = "{{type}}?"
list = "List<{{type}}>"

[binary_read]
u32 = "reader.ReadUInt32()"
string = "reader.ReadString()"

[csv_read]
u32 = "uint.Parse(row[col])"

[rhai]
prelude = ["rhai_utils/type_mapping.rhai"]
```

- `type_map`: `lang_type` 필터 동작 정의
- `binary_read`, `csv_read`: 해당 필터의 출력 정의
- `prelude`: `%logic` 블록에서 사용 가능한 Rhai 함수 사전 로드

---

## 컨텍스트 변수

### 루트 바인딩

| 변수 | 타입 | 설명 |
|------|------|------|
| `schema` | SchemaContext | 전체 스키마 |
| `file` | FileDef | 현재 처리 중인 파일 |

### file 프로퍼티

```
file.path                        파일 경로
file.namespaces                  네임스페이스 목록
file.all_tables                  모든 테이블 (flat)
file.container_name              PascalCase 파일명
file.all_tables_interface_list   인터페이스 목록 문자열
```

### namespace 프로퍼티

```
namespace.name             "game.character"
namespace.datasource       "sqlite" 등
namespace.items            구조체, enum, 하위 네임스페이스
namespace.tables           테이블만 (embed 제외)
namespace.enums            enum 목록
```

### struct 프로퍼티

```
struct.name                "Player"
struct.fqn                 "game.character.Player"
struct.namespace_fqn       "game.character"
struct.is_table            true/false
struct.has_pk              primary key 유무
struct.fields              필드 목록
struct.items               필드 + enum + embed + 주석
struct.indexes             인덱스 목록
struct.annotations         어노테이션 목록
struct.pack_separator      @pack separator (있으면)
```

### field 프로퍼티

```
field.name                 "id"
field.field_type           TypeRef 객체
field.is_primary_key       PK 여부
field.is_unique            UNIQUE 여부
field.is_optional          ? 여부
field.is_array             [] 여부
field.has_default_value    기본값 유무
field.default_value        기본값
field.max_length           max_length 제약
field.range                range 제약 (.min, .max)
field.regex_pattern        regex 패턴
field.foreign_key          FK 정보
```

### field.field_type (TypeRef) 프로퍼티

```
field.field_type.type_name       "u32", "string", "Player" 등
field.field_type.is_primitive    기본 타입 여부
field.field_type.is_option       Optional 여부
field.field_type.is_list         배열 여부
field.field_type.namespace_fqn   타입의 네임스페이스
```

### enum 프로퍼티

```
enum.name                  "PlayerClass"
enum.fqn                   "game.character.PlayerClass"
enum.members               멤버 목록
  member.name              "Warrior"
  member.value             1
  member.comment           주석
```

---

## 실전 예시

### C# 클래스 생성

```ptpl
%logic
import "templates/csharp/rhai_utils/type_utils" as types;
let has_fields = struct.fields.len() > 0;
%endlogic
%if has_fields
public class {{struct.name}}
{
%for field in struct.fields
    public {{field.field_type | lang_type}} {{field.name | pascal_case}} { get; set; }
%endfor
}
%endif
```

### C++ 헤더 (네임스페이스 + 조건부)

```ptpl
%logic
let ns_parts = namespace.name.split(".");
let ns_open = "";
let ns_close = "";
for part in ns_parts {
    ns_open += "namespace " + part + " {\n";
    ns_close += "} ";
}
%endlogic
#pragma once
%blank
{{ns_open}}
%for item in namespace.items
%if item.is_struct()
%include "detail/struct_def" with item.as_struct()
%elif item.is_enum()
%include "detail/enum_def" with item.as_enum()
%endif
%endfor
{{ns_close}}
```

### 패턴 매칭으로 바이너리 읽기

```ptpl
%match field.field_type.type_name
%when "u32"
    let {{field.name}} = reader.read_u32()?;
%when "string"
    let {{field.name}} = reader.read_string()?;
%when "bool"
    let {{field.name}} = reader.read_u8()? != 0;
%else
    let {{field.name}} = {{field.field_type.type_name}}::read_binary(reader)?;
%endmatch
```

---

## 소스 맵

모든 출력 라인은 다음 정보를 추적합니다:
- 템플릿 파일 경로 및 라인 번호
- Include 스택 (부모 템플릿)
- IR 경로 (어떤 struct/field에서 생성되었는지)

이를 통해 생성된 코드에서 원본 템플릿으로 역추적이 가능합니다.

---

*최종 업데이트: 2026-02-01*
