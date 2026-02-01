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

C# 템플릿 61개 중 (rhai_utils 2개 포함):
- **27개 → ptpl로 마이그레이션 성공** (Main 파일, Container, Detail, Section, Enum)
- **34개 → Rhai로 유지** (CSV Mappers, JSON Mappers, Binary R/W, DataContext, SQLite Accessor, using 헬퍼)

ptpl로 변환 불가능한 이유: 이 템플릿들은 "템플릿"이 아니라 "코드 생성 프로그램"에 가까움.

---

## 목표

**단일 통합 DSL로 모든 언어의 모든 코드 생성을 처리한다. Rhai를 완전히 대체한다.**

핵심 요구사항:
1. ptpl의 선언적 가독성을 유지하면서
2. Rhai의 복잡한 로직 표현력을 **완전히 흡수** (Tier 3 포함 — 재귀, 그래프, 스택 순회)
3. 생성된 코드의 각 줄이 어떤 템플릿 줄에서 나왔는지 추적 가능 (source mapping)
4. 기존 ptpl 문법과 하위 호환
5. **C#, C++, Rust, TypeScript, Go, Unreal, SQLite** 전 언어에서 이 하나의 DSL만 사용
6. Rhai의 역할을 계산 전용으로 제한 (출력 생성, eval, include, write_file 제거)

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

## 가장 복잡한 파일 상세 (CSV Mappers Writer — 490줄)

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
    Conditional { line: usize, condition: CondExpr, then_body, elif_branches, else_body },
    ForLoop { line: usize, variable: String, collection: CollectionExpr, body: Vec<TemplateNode> },
    Include { line: usize, template_path: String, context_bindings: Vec<IncludeBinding>, indent: Option<usize> },
    BlankLine { line: usize },
    // 주석(%--) 은 파서 단계에서 건너뛰며 노드를 생성하지 않음
}
```

### 컨텍스트 (src/template/context.rs)
```rust
pub enum ContextValue {
    String(String), Bool(bool), Int(i64), Float(f64),
    Schema(SchemaContext), File(FileDef), Namespace(NamespaceDef),
    NamespaceItem(NamespaceItem), Struct(StructDef), StructItem(StructItem),
    Field(Box<FieldDef>), TypeRef(TypeRef),
    Enum(EnumDef), EnumItem(EnumItem), EnumMember(EnumMember),
    Annotation(AnnotationDef), AnnotationParam(AnnotationParam),
    Index(IndexDef), IndexField(IndexFieldDef),
    Relation(RelationDef), ForeignKey(ForeignKeyDef),
    Timezone(TimezoneRef), Range(RangeDef),
    List(Vec<ContextValue>), Null,
}
```

### 필터 (src/template/expr.rs — Filter enum)
케이스 변환: `PascalCase`, `SnakeCase`, `CamelCase`, `Upper`, `Lower`, `RemoveDots`
타입 매핑: `LangType`, `Format`, `BinaryRead`, `BinaryReadOption`, `BinaryReadList`, `BinaryReadStruct`, `CsvRead`
유틸리티: `Quote`, `Count`, `Join(separator)`, `IsEmbedded`, `Suffix(str)`, `Prefix(str)`

### 렌더러 (src/template/renderer.rs)
- `render()`: 재귀적으로 노드 트리를 순회하며 출력 생성
- `resolve_property()`: `context_value.property_name` 체인을 해석
- `apply_lang_type_filter()`: TypeRef → 언어별 타입 문자열 변환

---

## 설계 방향: ptpl + Rhai 임베딩

**ptpl이 출력을 소유하고, Rhai는 계산만 담당한다.**

- `%logic ... %endlogic` 블록 안은 **진짜 Rhai 코드**가 그대로 실행된다 (새 인터프리터 구현 불필요)
- Rhai에서 **뺏는 것**: `code +=` 출력, `eval()`, `include()` 템플릿 조합, 다줄 문자열 생성, `write_file()`
- Rhai에서 **유지하는 것**: `fn`, `for`, `if/else`, `while`, 배열/맵, 문자열 메서드, `return/break/continue`
- Rhai `import ... as` → **제거** (toml `[rhai] prelude`로 자동 로딩)
- `%logic` 안 Rhai 문자열은 **한 줄(개행 금지)** — 변수명, 타입명, 표현식 조각 같은 짧은 값만 허용
- 다줄 출력은 반드시 ptpl 출력 라인, `%include`, `%render`로 처리

---

## 평가 기준

새 DSL을 평가할 때 다음 기준을 사용하세요:

1. **가독성**: 출력 코드의 구조가 템플릿에서 보이는가?
2. **추적성**: 생성된 코드 줄 → 템플릿 줄 매핑이 가능한가?
3. **표현력**: CSV Mappers Writer (490줄)를 완전히 표현할 수 있는가?
4. **구현 복잡도**: Rust 엔진 변경량이 합리적인가?
5. **하위 호환**: 기존 ptpl 27개 파일이 그대로 동작하는가?
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
| `templates/csharp/class/csharp_csv_mappers_struct_writer.rhai` | 가장 복잡한 Rhai 템플릿 (490줄) |
| `templates/csharp/class/csharp_csv_mappers_struct.rhai` | 스택 기반 재귀 (85줄) |
| `templates/csharp/csharp_csv_columns_file.rhai` | 재귀 함수 + 사이클 감지 (199줄) |
| `templates/csharp/csharp_sqlite_accessor_file.rhai` | 해시맵/그래프 구성 (763줄) |
| `templates/csharp/csharp_file.ptpl` | ptpl 성공 사례 — 메인 파일 |
| `templates/csharp/csharp_container_file.ptpl` | ptpl 성공 사례 — Container |

---

## 새 DSL 설계안 v0.1 (Draft)

### 이름/포지션
- 가칭: **PolyTemplate v2 (ptpl2)**
- 목표: **출력 중심(ptpl) + 로직 표현력(Rhai) + 소스 매핑** 동시 만족

### 설계 원칙
1. **출력은 ptpl이 소유** — 템플릿 줄 ↔ 출력 줄 1:1 매핑
2. **로직은 Rhai가 소유** — `%logic` 안은 진짜 Rhai 코드 (새 인터프리터 없음)
3. **Rhai 문자열은 한 줄만** — `\n` 포함 시 런타임 에러 (다줄 출력은 ptpl 전용)
4. **ptpl 하위 호환** — 기존 27개 ptpl 파일 무수정 동작
5. **eval/include 제거** → `%include with`, `%render`, `%while`로 정적 구조화
6. **기존 Rhai 유틸리티 재활용** — toml `[rhai] prelude`로 자동 로딩, 점진적으로 Rust 네이티브/ptpl 필터로 전환

---

## 핵심 문법 (ptpl 확장)

### 1) 기존 ptpl 유지
- `%if / %elif / %else / %endif`
- `%for / %endfor`
- `%include "path" with ctx [indent N]`
- `%blank`, `%--` (주석)
- `{{ expr | filter }}` (인라인 표현식 + 필터)

### 2) 변수/대입
```
%let name = expr
%set name = expr
```
- `%let`과 `%set`은 **동일 동작** (플랫 스코프 — 파일 전체에서 유효)
- 둘 다 변수가 없으면 생성, 있으면 재할당

### 3) 매칭/분기 (복합 분기 대응)
```
%match expr
%when pattern [if guard]
  ...
%when pattern
  ...
%else
  ...
%endmatch
```
- `pattern`: 리터럴, `_` 와일드카드, 튜플 패턴 `(a, b, _)` 허용
- `guard`: 보조 조건 (선택)

### 4) 로직 블록 — 내부는 진짜 Rhai
```
%logic
  let es = find_embedded_struct(s, field.field_type.type_name);
  let call_prefix = if es != () {
    "Csv." + s.fqn + "." + es.name
  } else {
    "Csv." + field.field_type.fqn
  };
%endlogic
{{call_prefix}}.AppendRow(obj.{{field.name}}, cols);
```
- `%logic ... %endlogic` 안은 **Rhai 코드가 그대로 실행**됨 (Rhai 파서/인터프리터 재활용)
- 로직 블록은 **출력을 만들지 않고**, 변수/상태 계산만 수행
- 로직 블록에서 선언한 변수는 이후 ptpl `{{ }}` 표현식에서 접근 가능
- **핵심 제약: 문자열 값에 `\n` 금지** — 한 줄짜리 값만 허용 (위반 시 런타임 에러)

### 5) 블록/렌더 (eval 대체)
```
%block append_row(field)
  ... 템플릿 본문 ...
%endblock

%render append_row with field
```
- 블록은 **정적 템플릿 조각**으로, eval 없이 재사용 가능
- `%render`는 블록을 호출하며, 소스 매핑은 블록 정의 라인 기준
- 동적 선택: `%render $var_name with field` (`$` 접두사로 변수 참조 구분)

### 6) Rhai `fn` 정의 — `%logic` 안에서 그대로 사용
```
%logic
  fn find_embedded(s, name) {
    for it in s.items {
      if it.is_embedded_struct() {
        let es = it.as_embedded_struct();
        if es.name == name { return es; }
      }
    }
    ()
  }

  fn is_sqlite_table(struct_ds, ns_ds) {
    if struct_ds != () && struct_ds != "" { return struct_ds == "sqlite"; }
    if ns_ds != () && ns_ds != "" { return ns_ds == "sqlite"; }
    false
  }
%endlogic
```
- Rhai의 `fn` 정의를 그대로 사용 — 새 문법 없음
- 재귀, return, 배열/맵 조작 모두 Rhai 네이티브
- 대부분 **간단한 조회/판별 함수** (5~15줄) — 200+ 함수가 이 패턴
- 복잡한 함수 (재귀 + 사이클 감지)도 Rhai `fn`으로 충분

### 7) `%while` 디렉티브 — 출력 라인과 혼합 가능
```
%while stack.len > 0
%logic
  let frame = stack.pop();
  let def = frame[0];
  let stage = frame[1];
  let depth = frame[2];
%endlogic
%if stage == 0
{{indent(depth)}}public static class {{def.name}}
{{indent(depth)}}{
  %include "csharp/class/csv_mappers_writer" with def [indent depth + 1]
%else
{{indent(depth)}}}
%endif
%endwhile
```
- `%while expr ... %endwhile`: `%for`와 같은 레벨의 독립 디렉티브
- 내부에 출력 라인, `%include`, `%if` 등 사용 가능 (소스 매핑 유지)
- `%logic` 내부의 `while`은 계산 전용 (출력 없음)

### 8) 컬렉션 필터 (`| where`)
```
%for field in struct.fields | where !field.is_list
  ... non-list 필드만 처리 ...
%endfor

%for field in struct.fields | where field.is_list
  ... list 필드만 처리 ...
%endfor
```
- 다중 패스 반복 패턴 대응 (non-list 먼저, list 나중)
- `| where` 뒤에는 boolean 표현식

---

## 표현식: 두 세계의 공존

### ptpl 표현식 (`{{ }}` 안) — 기존 ptpl 유지 + 확장
```
{{field.name}}                       %-- 프로퍼티 접근
{{field.name | PascalCase}}          %-- 필터 적용
{{field.field_type | LangType}}      %-- 타입 매핑
{{call_prefix}}                      %-- %logic에서 선언한 변수 참조
```
- 기존 ptpl 필터 시스템 그대로 유지
- `%logic`에서 선언한 Rhai 변수를 `{{ }}`에서 참조 가능 (Rhai 값 → ContextValue 변환)

### `%logic` 안 표현식 — Rhai 문법 그대로
```
%logic
  let parts = fqn.split(".");                    // 문자열 메서드
  let ns = parts[0..parts.len-1].join(".");      // 배열 슬라이스 + join
  let type_name = parts[parts.len - 1];          // 인덱싱
  let map = #{};                                 // 해시맵 (Rhai 문법)
  map[type_name] = ns;                           // 동적 키 할당
  let is_opt = type_name.starts_with("Option<"); // 불리언 결과
%endlogic
```
- Rhai 문법/연산자/메서드 그대로 사용: `+`, `==`, `!=`, `&&`, `||`, `!`
- Rhai 문자열: `+`로 결합 (Rhai 네이티브 연산자)
- Rhai 해시맵: `#{}` (Rhai 네이티브 문법)
- `import ... as` 제거됨 — 유틸 함수는 toml prelude로 자동 주입

### Rhai ↔ ptpl 값 변환
| Rhai 타입 | ptpl ContextValue | 변환 |
|----------|------------------|------|
| `String` | `ContextValue::String` | 직접 (`\n` 포함 시 에러) |
| `bool` | `ContextValue::Bool` | 직접 |
| `i64` | `ContextValue::Int` | 직접 |
| `f64` | `ContextValue::Float` | 직접 |
| `Array` | `ContextValue::List` | 재귀 변환 |
| `Map` | `ContextValue::Map` (신규) | 재귀 변환 |
| `()` | `ContextValue::Null` | 직접 |
| IR 타입 (`StructDef` 등) | 기존 ContextValue 변형 | 기존 Rhai 래퍼 재활용 |

### Rhai에서 금지되는 것 (런타임 에러)
```
%logic
  let multi = "line1\nline2"; // ⚠️ Rhai 안에서는 허용, 단 {{multi}}로 ptpl에 노출 시 에러
  code += "something\n";      // ❌ 에러: code 변수 사용 금지 (출력은 ptpl이 담당)
  eval("`template`");         // ❌ 에러: eval 금지
  let tmpl = include("...");  // ❌ 에러: include 금지 (%include 사용)
  write_file(path, content);  // ❌ 에러: write_file 금지 (엔진이 담당)
%endlogic
```
- `eval`, `include`, `write_file`: Rhai 엔진에서 **등록 해제**
- `\n` 검사: `%endlogic` 시점에 Rhai Scope → ContextValue 변환할 때 **String에 `\n` 포함 시 에러** (Rhai 내부에서는 자유)

---

## 소스 매핑 규칙

1. **OutputLine**: 템플릿 라인 번호가 그대로 출력 라인에 매핑
2. **Include/Render**: 호출 스택을 기록하되, 실 매핑은 포함된 라인의 위치
3. **Inline expr**: 동일 템플릿 라인의 segment로 기록

---

## CSV Mappers Writer 패턴 대응 예시

```
%for field in struct.fields
%match (field.is_list, field.type_category)
%when (false, "primitive")
  cols.Add(CsvUtils.ToStringInvariant(obj.{{field.name}}));
%when (false, "enum")
  cols.Add(obj.{{field.name}}.ToString());
%when (false, "struct")
  %let es = find_embedded_struct(struct, field.type_name)
  %if es != none
    Csv.{{struct.fqn}}.{{es.name}}.AppendRow(obj.{{field.name}}, cols);
  %else
    Csv.{{field.type_fqn}}.AppendRow(obj.{{field.name}}, cols);
  %endif
%when (true, _)
  %render append_list_field with field
%endmatch
%endfor
```

---

## 하위 호환 전략
- 기존 ptpl 문법/필터는 **그대로 동작**
- 새 키워드 추가: `%let`, `%set`, `%match/%when/%endmatch`, `%logic/%endlogic`, `%block/%endblock`, `%render`, `%while/%endwhile`
- 템플릿 파일 확장자는 기존 `.ptpl` 유지
- 기존 27개 ptpl 파일은 **무수정으로 동작**해야 함

---

## 구현 단계 (러스트 엔진 변경)

### Rhai 임베딩이므로 표현식 엔진 구현 불필요 — 핵심은 ptpl 파서/렌더러 확장과 Rhai 통합

1. **ptpl Parser 확장** (src/template/parser.rs)
   - `TemplateNode` 추가: `Let`, `Set`, `Match/When`, `LogicBlock`, `Block`, `Render`, `While`
   - `CollectionExpr` 확장: `| where` 필터 조건
   - `LogicBlock`의 본문은 **파싱하지 않고 원본 문자열로 보존** → Rhai에 넘김

2. **Context 확장** (src/template/context.rs)
   - `ContextValue::Map(HashMap<String, ContextValue>)` 추가

3. **Rhai 통합 계층** (신규: src/template/rhai_bridge.rs)
   - `%logic` 본문을 Rhai 엔진에 전달하여 실행
   - Rhai 스코프 ↔ ptpl TemplateContext 양방향 변환
   - Rhai 결과 문자열의 `\n` 검사 (위반 시 에러)
   - Rhai에서 `eval()`, `include()`, `write_file()`, `import` **등록 해제/비활성화**
   - 기존 Rust 등록 함수 (`cs_map_type` 등)는 유지
   - toml `[rhai] prelude` 파일의 fn들을 Rhai Scope에 미리 등록 (import 대체)

4. **Renderer 확장** (src/template/renderer.rs)
   - `%logic`: Rhai 실행 → 변수를 ptpl 컨텍스트에 병합
   - `%let/%set`: 간단한 표현식 평가 (Rhai에 위임 가능)
   - `%match/%when`: 패턴 매칭 분기
   - `%block/%render`: 블록 저장소 + 동적 렌더
   - `%while`: 조건 루프 (ptpl 디렉티브 레벨, 소스 매핑 유지)
   - `| where`: 컬렉션 필터링
   - 소스 매핑 스택 유지

---

## 결정 사항

### 확정
1. `%logic` 내부는 **Rhai 코드 그대로** 실행 (새 인터프리터 구현 안 함)
2. `%logic` 안 문자열은 **한 줄 제한 (`\n` 금지)** — 위반 시 런타임 에러
3. 다줄 출력은 **ptpl 출력 라인, `%include`, `%render`만** 허용
4. Rhai에서 **제거**: `eval()`, `include()`, `write_file()`, `code +=` 패턴, `import ... as`
5. Rhai에서 **유지**: `fn`, `for`, `if/else`, `while`, 배열/맵, 문자열 메서드, `return/break/continue` (`import` 제거됨)
6. `%while ... %endwhile`: 독립 디렉티브 (출력 라인 + `%include` 혼합 가능)
7. `%for ... | where expr`: 컬렉션 필터링
8. block/render: `%block name(param)` + `%render name with expr`, 동적 render는 `%render $var with expr`
9. 함수 정의: Rhai `fn`을 `%logic` 안에서 정의 (별도 `%fn` 디렉티브 불필요), **파일 상단 `%logic` 블록에 모아서 정의 권장**
10. 들여쓰기: `indent_text()` 대신 `%include ... [indent N]` 사용
11. `%let`/`%set`: **플랫 스코프** — 파일 전체에서 유효, 두 키워드 구분 없이 동일 동작
12. `emit` **삭제** — 모든 출력은 ptpl 출력 라인만 허용, `%logic`에서 직접 출력 불가
13. `%match/%when`: **튜플 + 리터럴 + 와일드카드(`_`)** 지원, 변수 바인딩/가드 조건 포함
14. `%include`된 파일의 `%logic`은 **완전 격리** — `with`로 전달된 값만 접근 가능, 부모 스코프 접근 불가
15. `\n` 검사: **ptpl 경계에서만** — `%endlogic` 시점에 Rhai Scope → ContextValue 변환할 때 String에 `\n` 포함 시 에러. Rhai 내부에서는 자유
16. `import ... as` **제거** — 유틸리티 함수는 언어별 toml의 `[rhai] prelude` 설정으로 자동 로딩 (템플릿에서 import 구문 불필요)
17. 기존 `rhai_utils/` 파일: **점진적 제거** — 마이그레이션하면서 하나씩 Rust 네이티브 등록 또는 ptpl 필터로 전환

---

## 확정된 설계 과제

### 과제 1: Rhai 스코프 ↔ ptpl 컨텍스트 동기화 → **확정**

| 상황 | 동작 |
|------|------|
| `%logic`에서 `let x = "val";` | `%endlogic` 이후 `{{x}}`로 접근 가능 → **Yes** |
| 두 번째 `%logic` 블록 | 이전 블록의 변수 유지 → **Yes** (동일 Rhai Scope 재사용) |
| `%for` 안의 `%logic` | 루프 반복마다 Rhai Scope 리셋 → **No** (누적) |
| `%include`된 파일의 `%logic` | 부모 스코프 접근 → **No** (`with`로 전달된 값만 접근, 완전 격리) |

**구현**: 하나의 ptpl 파일 렌더링 동안 **하나의 Rhai Scope**를 유지. `%include`된 파일은 **별도 Scope** 생성 (전달된 바인딩만 포함).

### 과제 2: Rhai `fn` 정의의 위치 → **확정 (상단 권장)**

**컨벤션**: 파일 상단에 `%logic` 블록으로 모든 `fn`을 미리 정의:
```
%logic
  fn find_embedded(s, name) { ... }
  fn is_sqlite_table(ds, ns_ds) { ... }
  fn collect_fk_deps(s) { ... }
%endlogic
%-- 이후부터 출력 시작
```
- 이 패턴이면 대부분의 파일이 **상단 fn 정의 + 하단 ptpl 출력** 구조
- `%for` 안 `%logic`에서 `fn` 정의 시 Rhai 엔진이 에러를 내면 에러 그대로 전파 (별도 제약 불필요)

### 과제 3: `import ... as` 처리 → **확정 (제거, toml prelude로 대체)**

Rhai `import ... as` 구문을 **제거**하고, 유틸리티 함수는 **언어별 toml 설정으로 자동 로딩**:

```toml
# templates/csharp/csharp.toml
[rhai]
prelude = [
  "rhai_utils/type_utils.rhai",
  "rhai_utils/type_mapping.rhai",
]
```

- 엔진이 `%logic` 실행 전에 prelude 파일의 `fn`들을 Rhai Scope에 미리 등록
- 템플릿에서는 바로 `find_embedded(s, name)` 호출 가능 (import 구문 불필요)
- `indent_text()` 등 `\n` 반환 함수는 prelude에서 제외하고 점진적으로 ptpl 필터/`%include`로 대체

### 과제 4: `\n` 검사 → **확정 (ptpl 경계에서만)**

- `%endlogic` 시점에 Rhai Scope → ContextValue 변환할 때, **String 값에 `\n` 포함 시 런타임 에러**
- Rhai 내부에서는 `\n` 자유 (`split()`, `join()` 등 내부 처리 허용)
- ptpl `{{ }}` 표현식에 노출되는 최종 값만 한 줄이면 됨

### 과제 5: 기존 `rhai_utils/` 재활용 → **확정 (점진적 제거)**

- 마이그레이션 초기: toml prelude로 기존 `.rhai` 유틸 파일을 그대로 로딩하여 재활용
- 점진적으로: Rust 네이티브 등록 또는 ptpl 필터로 전환
- `indent_text()`: 즉시 제거 (`%include ... [indent N]`으로 대체)
- 나머지 200+ 함수: 하나씩 전환하면서 `.rhai` 유틸 파일 의존도 줄이기

---

## Tier 3 패턴 대응 예시 (Rhai 임베딩 방식)

### csv_columns_file.rhai → ptpl (재귀 함수 + 사이클 감지)

```
%-- 상단: Rhai로 모든 계산 함수 정의 + 데이터 수집
%logic
  fn find_embedded(s, name) {
    for it in s.items {
      if it.is_embedded_struct() { let es = it.as_embedded_struct(); if es.name == name { return es; } }
    }
    ()
  }

  fn collect_columns(ctx_struct, prefix, type_str, visited, depth, ns_name, all_files) {
    let cols = [];
    if depth >= 10 { cols.push(prefix); return cols; }

    let t = unwrap_option(type_str);
    if t.starts_with("List<") {
      let inner = t.sub_string(5, t.len - 6);
      let np = if prefix == "" { "[0]" } else { prefix + "[0]" };
      let sub = collect_columns(ctx_struct, np, inner, visited, depth + 1, ns_name, all_files);
      for c in sub { cols.push(c); }
      return cols;
    }

    let es = find_embedded(ctx_struct, t);
    if es != () {
      if visited.contains(es.name) { return cols; }
      let next = visited + [es.name];
      for it in es.items { if it.is_field() {
        let f = it.as_field();
        let np = if prefix == "" { f.name } else { prefix + "." + f.name };
        let sub = collect_columns(es, np, f.field_type, next, depth + 1, ns_name, all_files);
        for c in sub { cols.push(c); }
      }}
      return cols;
    }

    cols.push(prefix);
    cols
  }

  let columns = collect_columns(s, "", s.fqn, [], 0, ns.name, schema.files);
%endlogic

%-- 하단: 수집된 columns로 선언적 출력
%for col in columns
    "{{col}}",
%endfor
```

**포인트**: 재귀 함수는 Rhai `fn` 그대로. 기존 Rhai 코드와 거의 동일하되, `code +=` 대신 ptpl 출력 라인 사용.

### csv_mappers_struct.rhai → ptpl (스택 기반 중첩 클래스)

```
%logic
  let stack = [[s, 0, 0]];
%endlogic
%while stack.len > 0
%logic
  let frame = stack.pop();
  let def = frame[0];
  let stage = frame[1];
  let depth = frame[2];
%endlogic
%if stage == 0
%include "csharp/detail/class_open" with def [indent depth]
%include "csharp/class/csv_mappers_writer" with def [indent depth + 1]
%include "csharp/class/csv_mappers_reader" with def [indent depth + 1]
%logic
  stack.push([def, 1, depth]);
  let children = [];
  for it in def.items { if it.is_embedded_struct() { children.push(it.as_embedded_struct()); } }
  let i = children.len;
  while i > 0 { i -= 1; stack.push([children[i], 0, depth + 1]); }
%endlogic
%else
%include "csharp/detail/class_close" with depth
%endif
%endwhile
```

**포인트**: `eval()` + `include()` → `%include with [indent]`. 스택 조작은 Rhai `while` 그대로.

### sqlite_accessor_file.rhai → ptpl (역방향 의존성 그래프)

```
%-- 상단: 순수 Rhai로 데이터 수집
%logic
  fn is_sqlite_table(struct_ds, ns_ds) {
    if struct_ds != () && struct_ds != "" { return struct_ds == "sqlite"; }
    if ns_ds != () && ns_ds != "" { return ns_ds == "sqlite"; }
    false
  }

  fn collect_fk_deps(s) {
    let deps = [];
    for it in s.items { if it.is_field() {
      let f = it.as_field();
      if f.has_foreign_key {
        let parts = f.foreign_key.target_table_fqn.split(".");
        deps.push(parts[parts.len - 1]);
      }
    }}
    deps
  }

  let sqlite_tables = [];
  let table_by_name = #{};
  for file in schema.files {
    for ns_item in file.namespaces {
      for item in ns_item.items { if item.is_struct() {
        let st = item.as_struct();
        if is_sqlite_table(st.datasource, ns_item.datasource) {
          let info = #{ name: st.name, fqn: st.fqn, struct_def: st, deps: collect_fk_deps(st) };
          sqlite_tables.push(info);
          table_by_name[st.name] = info;
        }
      }}
    }
  }

  let reverse_deps = #{};
  for tbl in sqlite_tables { reverse_deps[tbl.name] = []; }
  for tbl in sqlite_tables {
    for dep in tbl.deps {
      if reverse_deps[dep] != () { reverse_deps[dep].push(tbl.name); }
    }
  }
%endlogic

%-- 하단: 선언적 출력 (소스 매핑 유지)
%if sqlite_tables.len == 0
%-- 빈 출력 (SQLite 테이블 없음)
%else
// Generated by PolyGen - SQLite Database Accessor
using System;
using System.Collections.Generic;
using System.Data;
using System.Linq;
using Microsoft.Data.Sqlite;
using Polygen.Common;
%blank
namespace Polygen.Data
{
%for tbl in sqlite_tables
    %include "csharp/sqlite/table_accessor" with tbl, reverse_deps [indent 1]
%endfor
}
%endif
```

**포인트**: 763줄의 `code +=` 파일이 → 상단 Rhai 로직(fn 정의 + 데이터 수집) + 하단 ptpl 출력으로 깔끔하게 분리. `using` 문, `namespace` 여닫이 등 보일러플레이트가 ptpl 출력 라인이 되면서 소스 매핑 자동 확보.

---

## Rhai 사용 현황 요약 (전수 조사 결과)

### 파일 분포 (전체 76 Rhai 파일)

| 언어 | Rhai 파일 수 | ptpl 파일 수 | 비고 |
|------|------------|------------|------|
| C# | 34 | 27 | 부분 마이그레이션 완료 |
| C++ | 7 | 0 | 전체 Rhai |
| Rust | 7 | 0 | 전체 Rhai |
| TypeScript | 9 | 0 | 전체 Rhai |
| Go | 0 | 6+ | **전체 ptpl 마이그레이션 완료** |
| Unreal | 6 | 0 | 전체 Rhai |
| SQLite | 2 | 0 | 전체 Rhai |
| MySQL | 4 | 0 | 전체 Rhai (미완성) |

### Rhai에서 제거할 패턴 (ptpl이 대체)

| 패턴 | 파일 수 | 대체 |
|------|---------|------|
| `code +=` / `result +=` / `body +=` 다줄 출력 | 29 | ptpl 출력 라인 |
| `eval()` + `include()` 템플릿 조합 | 33 | `%include with` / `%render` |
| `write_file()` | 24 | 엔진이 담당 |
| `indent_text()` 다줄 들여쓰기 | 11 | `%include ... [indent N]` |
| `return code` / `return ""` | 11 | 불필요 (ptpl이 출력 소유) |

### Rhai에서 유지할 패턴 (`%logic` 안에서 그대로)

| 패턴 | 파일 수 | 대표 용도 |
|------|---------|----------|
| `fn` 정의 | 21 (200+ 함수) | 타입 판별, struct 검색, FQN 분리 |
| `for..in` 루프 | 68 (500+ 사용) | 스키마 순회 |
| `if/else` 분기 | 68 (400+ 사용) | 타입 체크, null 체크 |
| 배열 조작 `.push()/.pop()/.len` | 36 | 데이터 수집 |
| 해시맵 `#{}` | 13 | 메타데이터 구축 |
| 문자열 메서드 `.split()/.starts_with()` 등 | 45 | FQN 파싱, 타입 추출 |
| ~~`import ... as`~~ | 32 → 제거 | toml `[rhai] prelude`로 대체 |
| `continue/break` | 33 | 루프 제어 |
| 재귀 함수 | 3 | 트리 탐색, 컬럼 수집 |
| `type_of()` | 3 | TypeRef vs string 구분 |

### `%logic` 안 Rhai 함수의 실제 모습 — 대부분 단순

Rhai 파일에 정의된 200+ 함수 중 대다수는 **5~15줄의 간단한 조회/판별 함수**:
```rhai
fn is_option(t) { t.starts_with("Option<") }
fn unwrap_option(t) { if t.starts_with("Option<") { t.sub_string(7, t.len - 8) } else { t } }
fn is_primitive_like(t) { switch t { "u8"=>true, "i8"=>true, "string"=>true, _ => false } }
fn find_embedded(s, name) { for it in s.items { if it.is_embedded_struct() { let e = it.as_embedded_struct(); if e.name == name { return e; } } } () }
```

복잡한 함수 (30줄 이상)는 **3~5개 파일에만** 존재:
- `collect_columns_with()` (csv_columns) — 재귀 + 사이클 감지
- `resolve_struct()` (csv_columns) — 크로스 네임스페이스 타입 해석
- 역방향 의존성 그래프 구축 (sqlite_accessor) — `#{}` 맵 조작

---

*Updated: 2026-02-01*
