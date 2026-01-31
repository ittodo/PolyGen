# PolyGen 새 템플릿 DSL 설계 프롬프트

## 배경

PolyGen은 `.poly` 스키마 파일에서 여러 언어의 코드를 생성하는 폴리글랏 코드 생성기입니다. 현재 두 가지 템플릿 엔진이 공존합니다:

### 1. PolyTemplate (.ptpl) — 선언적 엔진
```
%for item in namespace.items
%if item.is_struct
%include "section/struct_block" with item.as_struct
%endif
%endfor
```
- 장점: 읽기 쉽다, 구조가 명확, 출력과 템플릿이 1:1 대응
- 한계: 복잡한 로직(재귀, 타입 분기, 런타임 lookup) 표현 불가
- 사용 가능한 디렉티브: `%if/%elif/%else/%endif`, `%for/%endfor`, `%include with indent`, `%blank`, `%--`, `{{expr | filter}}`

### 2. Rhai (.rhai) — 명령형 스크립팅 엔진
```rhai
for it in s.items {
    if it.is_field() {
        let f = it.as_field();
        if f.field_type.is_enum {
            code += `cols.Add(obj.${f.name}.ToString());\n`;
        } else if f.field_type.is_primitive {
            code += `cols.Add(CsvUtils.ToStringInvariant(obj.${f.name}));\n`;
        } else {
            let es = find_embedded_struct(s, f.field_type.type_name);
            if es != () {
                code += `Csv.${s.fqn}.${es.name}.AppendRow(obj.${f.name}, cols);\n`;
            } else {
                code += `Csv.${f.field_type.fqn}.AppendRow(obj.${f.name}, cols);\n`;
            }
        }
    }
}
```
- 장점: 표현력이 높다, 어떤 로직이든 가능
- 한계: `code +=` 패턴으로 생성 코드가 어디서 나왔는지 추적 어려움, 템플릿과 출력 구조 괴리

### 마이그레이션 결과

C# 템플릿 49개 중:
- **28개 → ptpl로 마이그레이션 성공** (Main 파일, Container)
- **21개 → Rhai로 유지** (CSV Mappers, JSON Mappers, Binary R/W, DataContext, SQLite Accessor)

ptpl로 변환 불가능한 이유: 이 템플릿들은 "템플릿"이 아니라 "코드 생성 프로그램"에 가까움.

---

## 목표

**표현력과 가독성을 모두 만족시키는 새 DSL을 설계하라.**

핵심 요구사항:
1. ptpl의 선언적 가독성을 유지하면서
2. Rhai의 복잡한 로직 표현력을 흡수
3. 생성된 코드의 각 줄이 어떤 템플릿 줄에서 나왔는지 추적 가능 (source mapping)
4. 기존 ptpl 문법과 하위 호환되면 이상적

---

## Rhai에서 발견된 명령형 패턴 분류

### Tier 1 — 모든 Rhai 파일에서 사용 (필수 대체 대상)

| 패턴 | 예시 | 빈도 |
|------|------|------|
| `code +=` 문자열 연결 | `code += \`public void Load()\n\`;` | 모든 파일 |
| `eval()` 동적 템플릿 실행 | `eval("\`" + writer_tmpl + "\`")` | 9+ 파일 |
| `!= ()` null 비교 | `if es != ()` | 대부분 |

### Tier 2 — 여러 파일에서 사용 (높은 우선순위)

| 패턴 | 예시 | 빈도 |
|------|------|------|
| FQN 문자열 분리 | `fqn.split(".")` → namespace + type_name 추출 | 5+ 파일 |
| 타입별 다중 분기 | `is_primitive / is_enum / is_list / is_option` 조합 | 4+ 파일 |
| 런타임 struct lookup | `find_embedded_struct(s, type_name)` | 3+ 파일 |
| 변수 save/restore | `let saved = s; s = new_val; ...; s = saved;` | 3+ 파일 |
| 해시맵 구성 | `let map = #{};` 동적 키 할당 | 3 파일 |
| PascalCase 변환 | `split("_")` + 첫글자 대문자 (8회 중복) | 3 파일 |

### Tier 3 — 특수한 파일에서 사용 (난이도 높음)

| 패턴 | 예시 | 파일 |
|------|------|------|
| 스택 기반 재귀 | `stack.push([def, stage, depth])` while loop | csv_mappers_struct |
| 다중 패스 반복 | non-list 필드 먼저, list 필드 나중 | csv_mappers_writer |
| 재귀 함수 + 사이클 감지 | `visited` 배열 + `depth >= 10` 가드 | csv_columns |
| eval + 변수 주입 | `eval("let x = \"val\";\n" + template)` | binary/json struct |
| 역방향 의존성 그래프 | FK 대상 → 참조하는 테이블 맵 | sqlite_accessor |
| 타입→메서드 디스패치 | C# 타입별 SqliteDataReader.GetXxx 매핑 | sqlite_accessor |

---

## 가장 복잡한 파일 상세 (CSV Mappers Writer — 491줄)

이 파일 하나에 거의 모든 Tier 1~3 패턴이 동시에 사용됩니다:

```
AppendRowWithHeader (직렬화)
├── Non-list 필드 순회
│   ├── enum → .ToString()
│   ├── primitive/string → CsvUtils.ToStringInvariant()
│   └── struct → embedded struct인지 확인
│       ├── embedded → 소유자.Csv.자식.AppendRow()
│       └── 독립 → Csv.네임스페이스.타입.AppendRow()
├── List 필드 순회
│   ├── enum[] → 인덱스별 .ToString()
│   ├── primitive[] → 인덱스별 ToStringInvariant()
│   └── struct[] → embedded 여부로 또 분기
BuildWriteHeaderFromItems (헤더 트리 구성)
├── 위와 동일한 타입 분기
├── Boolean pre-scan (has_list_items)
├── 재귀적 헤더 구성
CollectWriteHeaderNames (헤더 이름 수집)
├── 위와 동일한 타입 분기
BuildHeader (읽기용 헤더 파싱)
├── 위와 동일한 타입 분기
WriteCsv, WriteCsvWithHeader (고수준 API)
```

**핵심 난제**: 하나의 필드에 대해 `is_list × (is_primitive | is_enum | is_struct) × (is_embedded | is_external)` 조합으로 최대 6가지 코드 경로가 존재하며, 이것이 7개 메서드에 걸쳐 반복됩니다.

---

## 현재 ptpl 엔진 구현 참고

### 파서 (src/template/parser.rs)
```rust
pub enum TemplateNode {
    OutputLine { line: usize, segments: Vec<LineSegment> },
    Conditional { condition: CondExpr, then_body, elif_branches, else_body },
    ForLoop { variable: String, collection: CollectionExpr, body: Vec<TemplateNode> },
    Include { template_path: String, context_bindings: Vec<IncludeBinding>, indent: Option<usize> },
    BlankLine,
    Comment,
}
```

### 컨텍스트 (src/template/context.rs)
```rust
pub enum ContextValue {
    File(FileDef), Namespace(NamespaceDef), NamespaceItem(NamespaceItem),
    Struct(StructDef), Field(FieldDef), Enum(EnumDef), EnumVariant(EnumVariantDef),
    Annotation(AnnotationDef), Attribute(AttributeDef), TypeRef(TypeRef),
    Relation(RelationDef), ForeignKey(ForeignKeyDef), Index(IndexDef),
    IndexField(IndexFieldDef), EmbeddedStruct(StructDef),
    String(String), Bool(bool), Integer(i64), List(Vec<ContextValue>),
}
```

### 필터 (src/template/filters.rs)
현재 지원: `PascalCase`, `SnakeCase`, `CamelCase`, `UpperCase`, `LowerCase`, `RemoveDots`, `LangType`

### 렌더러 (src/template/renderer.rs)
- `render()`: 재귀적으로 노드 트리를 순회하며 출력 생성
- `resolve_property()`: `context_value.property_name` 체인을 해석
- `apply_lang_type_filter()`: TypeRef → 언어별 타입 문자열 변환

---

## 설계 탐색 방향 (제안)

아래는 탐색할 수 있는 방향들입니다. 하나를 선택하거나 조합하세요.

### 방향 A: ptpl 확장 — 최소 추가
기존 ptpl에 몇 가지 디렉티브만 추가:
```
%let full_type = "global::" ~ struct.fqn
%match field.type_category
%when "primitive"
  cols.Add(CsvUtils.ToStringInvariant(obj.{{field.name}}));
%when "enum"
  cols.Add(obj.{{field.name}}.ToString());
%when "struct"
  %include "csv_append_struct" with field
%endmatch
%for field in struct.fields | where !field.is_list
```

### 방향 B: 새 DSL — 출력 중심 + 로직 블록
출력 줄은 그대로 쓰되, 로직은 별도 블록:
```
@logic {
  let es = find_embedded_struct(struct, field.type_name)
  let call_prefix = es ? "Csv.{struct.fqn}.{es.name}" : "Csv.{field.fqn}"
}
{call_prefix}.AppendRowWithHeader(__h, obj.{field.name}, cols, gap);
```

### 방향 C: 컨텍스트 풍부화 — Rust에서 선처리
복잡한 로직을 모두 Rust IR 빌더로 이동하고, 템플릿은 단순 렌더링만:
```rust
// Rust IR에서 미리 계산
struct CsvFieldInfo {
    name: String,
    category: FieldCategory, // Primitive, Enum, EmbeddedStruct, ExternalStruct
    append_call: String,     // 완성된 호출 문자열
    reader_call: String,
}
```
```
%for field in struct.csv_fields
{{field.append_call}}
%endfor
```

### 방향 D: 하이브리드 — ptpl 안에 인라인 스크립트
```
%for field in struct.fields
%script {
  // Rhai-like 로직 블록
  if field.is_embedded {
    emit "Csv.${struct.fqn}.${field.embedded_name}.AppendRow(obj.${field.name}, cols);"
  } else {
    emit "Csv.${field.type_fqn}.AppendRow(obj.${field.name}, cols);"
  }
}
%endfor
```

---

## 평가 기준

새 DSL을 평가할 때 다음 기준을 사용하세요:

1. **가독성**: 출력 코드의 구조가 템플릿에서 보이는가?
2. **추적성**: 생성된 코드 줄 → 템플릿 줄 매핑이 가능한가?
3. **표현력**: CSV Mappers Writer (491줄)를 완전히 표현할 수 있는가?
4. **구현 복잡도**: Rust 엔진 변경량이 합리적인가?
5. **하위 호환**: 기존 ptpl 28개 파일이 그대로 동작하는가?
6. **다언어 확장**: C# 외에 C++, Rust, TypeScript, Go에도 적용 가능한가?

---

## 프로젝트 참조 파일

| 파일 | 용도 |
|------|------|
| `CLAUDE.md` | 프로젝트 전체 가이드 |
| `src/template/parser.rs` | 현재 ptpl 파서 |
| `src/template/renderer.rs` | 현재 ptpl 렌더러 |
| `src/template/context.rs` | 컨텍스트 값 시스템 |
| `src/template/expr.rs` | 표현식/필터 정의 |
| `src/template/filters.rs` | 필터 구현 |
| `src/ir_model.rs` | IR 데이터 모델 |
| `templates/csharp/class/csharp_csv_mappers_struct_writer.rhai` | 가장 복잡한 Rhai 템플릿 (491줄) |
| `templates/csharp/class/csharp_csv_mappers_struct.rhai` | 스택 기반 재귀 (86줄) |
| `templates/csharp/csharp_csv_columns_file.rhai` | 재귀 함수 + 사이클 감지 (200줄) |
| `templates/csharp/csharp_sqlite_accessor_file.rhai` | 해시맵/그래프 구성 (764줄) |
| `templates/csharp/csharp_file.ptpl` | ptpl 성공 사례 — 메인 파일 |
| `templates/csharp/csharp_container_file.ptpl` | ptpl 성공 사례 — Container |
