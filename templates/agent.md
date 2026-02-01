# templates/ - Agent Documentation

## Scope
PolyTemplate (.ptpl) 엔진을 사용하여 다양한 타겟 언어의 코드를 생성하는 템플릿 파일들이 위치한 폴더입니다. 지원 언어: C#, C++, Rust, TypeScript, Go, Unreal, SQLite.

## Template Engine

PolyTemplate (.ptpl)은 디렉티브(`%`)로 제어 흐름을, 인터폴레이션(`{{}}`)으로 값 치환을 수행하는 선언적 코드 생성 DSL입니다.

- **디렉티브 라인** (`%`로 시작): `%if`, `%for`, `%include`, `%logic` 등
- **출력 라인** (그 외): 그대로 출력, `{{expr | filter}}` 부분만 치환
- **TOML 설정**: 각 언어의 `.toml` 파일에서 타입 매핑, 필터 동작, prelude 정의
- **Rhai 연동**: `%logic` 블록에서 Rhai 코드 실행, prelude `.rhai` 유틸리티 함수 사용

상세: `docs/PTPL_LANGUAGE_GUIDE.md`

## Structure
```
templates/
├── csharp/                    # C# 코드 생성
│   ├── csharp.toml            # 언어 설정 (type_map, binary_read 등)
│   ├── csharp_file.ptpl       # 메인 클래스/구조체 생성
│   ├── csharp_csv_columns_file.ptpl  # CSV 컬럼 정의
│   ├── csharp_sqlite_accessor_file.ptpl  # SQLite Accessor
│   ├── section/               # 섹션 단위 템플릿
│   │   ├── namespace_block.ptpl
│   │   ├── struct_block.ptpl
│   │   └── enum_block.ptpl
│   ├── detail/                # 세부 렌더링
│   │   ├── struct_header.ptpl
│   │   ├── struct_body.ptpl
│   │   ├── struct_field.ptpl
│   │   ├── struct_pack.ptpl       # @pack Pack/Unpack/TryUnpack
│   │   ├── struct_fk_nav.ptpl
│   │   ├── struct_relation_nav.ptpl
│   │   └── struct_auto_update.ptpl
│   └── rhai_utils/            # Rhai 유틸리티 (prelude)
│       ├── type_utils.rhai
│       └── binary_mapping.rhai
│
├── cpp/                       # C++ 헤더 전용 생성
│   ├── cpp.toml
│   ├── cpp_file.ptpl          # 구조체/Enum 헤더
│   ├── cpp_container_file.ptpl  # Container (인덱스, 검증)
│   ├── cpp_loaders_file.ptpl  # CSV/JSON/Binary 로더
│   ├── cpp_sqlite_accessor_file.ptpl
│   ├── section/
│   │   ├── namespace_block.ptpl
│   │   ├── struct_block.ptpl
│   │   └── enum_block.ptpl
│   ├── detail/
│   │   ├── struct_body.ptpl
│   │   └── pack_methods.ptpl  # @pack pack/unpack/try_unpack
│   └── rhai_utils/
│       └── type_mapping.rhai
│
├── rust/                      # Rust 모듈 생성
│   ├── rust.toml
│   ├── rust_file.ptpl
│   ├── rust_container_file.ptpl
│   ├── rust_loaders_file.ptpl
│   ├── rust_sqlite_accessor_file.ptpl
│   ├── section/
│   │   ├── namespace_block.ptpl
│   │   ├── struct_block.ptpl
│   │   └── enum_block.ptpl
│   ├── detail/
│   │   ├── struct_body.ptpl
│   │   └── pack_methods.ptpl  # @pack pack/unpack
│   └── rhai_utils/
│       └── type_mapping.rhai
│
├── typescript/                # TypeScript 인터페이스/Zod 생성
│   ├── typescript.toml
│   ├── typescript_file.ptpl
│   ├── typescript_zod_file.ptpl
│   ├── typescript_sqlite_accessor_file.ptpl
│   ├── section/
│   │   ├── namespace_block.ptpl
│   │   ├── struct_block.ptpl
│   │   └── enum_block.ptpl
│   └── rhai_utils/
│       └── type_mapping.rhai
│
├── go/                        # Go 패키지 생성
│   ├── go.toml
│   ├── go_file.rhai           # (아직 .rhai — 미마이그레이션)
│   ├── go_container_file.rhai
│   └── rhai_utils/
│       └── type_mapping.rhai
│
├── unreal/                    # Unreal Engine USTRUCT/UENUM
│   ├── unreal.toml
│   ├── unreal_file.ptpl
│   ├── unreal_loaders_file.ptpl
│   ├── unreal_hotreload_file.ptpl
│   ├── section/
│   │   ├── namespace_block.ptpl
│   │   ├── struct_block.ptpl
│   │   └── enum_block.ptpl
│   ├── detail/
│   │   └── struct_body.ptpl
│   └── rhai_utils/
│       └── type_mapping.rhai
│
├── sqlite/                    # SQLite DDL + Migration
│   ├── sqlite.toml
│   ├── sqlite_file.ptpl
│   ├── sqlite_migration_file.ptpl
│   └── rhai_utils/
│       └── type_mapping.rhai
│
├── mermaid/                   # Mermaid 다이어그램 (예정)
└── rhai_utils/                # 공통 Rhai 유틸리티
    └── indent.rhai
```

## Template Architecture

### 파일 구성 패턴

각 언어 템플릿은 동일한 구조를 따릅니다:

1. **`<lang>.toml`**: 언어 설정 (타입 매핑, 필터 정의, 템플릿 목록)
2. **`<lang>_file.ptpl`**: 메인 엔트리 포인트 (per-file 렌더링)
3. **`<lang>_<feature>_file.ptpl`**: 추가 기능 템플릿 (container, loaders, sqlite_accessor 등)
4. **`section/`**: 네임스페이스, 구조체, enum 블록 단위 분리
5. **`detail/`**: 필드, 헤더, 팩 등 세부 렌더링
6. **`rhai_utils/`**: `%logic` 블록에서 import하는 Rhai 헬퍼 함수

### 렌더링 흐름

```
<lang>_file.ptpl
  └─> %for ns in file.namespaces
      └─> %include "section/namespace_block" with ns
          └─> %for item in namespace.items
              └─> %include "section/struct_block" with struct
                  └─> %include "detail/struct_body" with struct
                      └─> %include "detail/struct_pack" with struct  (조건부)
              └─> %include "section/enum_block" with enum
```

### TOML 설정 구조

```toml
[templates]
main = "<lang>_file.ptpl"
extension = "cs"

[[templates.extra]]
template = "<lang>_container_file.ptpl"
per_file = false
output = "<lang>/<file>.Container.<ext>"

[type_map]
u32 = "uint"
string = "string"

[binary_read]
u32 = "reader.ReadUInt32()"

[rhai]
prelude = ["rhai_utils/type_utils.rhai"]
```

## Key Concepts

### PolyTemplate 디렉티브
- `%logic` / `%endlogic`: Rhai 코드 블록 (데이터 준비)
- `%if` / `%elif` / `%else` / `%endif`: 조건부 출력
- `%for item in collection` / `%endfor`: 반복
- `%include "path" with binding`: 하위 템플릿 포함
- `%match` / `%when` / `%endmatch`: 패턴 매칭
- `%block` / `%render`: 재사용 가능 블록
- `%let` / `%set`: 변수 할당
- `%blank`: 빈 줄 출력
- `%--`: 주석 (출력 안 함)

### 인터폴레이션 & 필터
```
{{struct.name}}                    → 프로퍼티 접근
{{field.field_type | lang_type}}   → TOML type_map 기반 타입 변환
{{name | pascal_case}}             → 케이스 변환
{{items | join(", ")}}             → 컬렉션 조인
```

### 컨텍스트 변수
- `schema`: 전체 스키마 (SchemaContext)
- `file`: 현재 파일 (FileDef)
- `namespace`: 네임스페이스 (NamespaceDef)
- `struct`: 구조체 (StructDef) — `is_embed`, `has_pk`, `pack_separator` 등
- `field`: 필드 (FieldDef) — `field_type`, `is_primary_key`, `is_optional` 등
- `enum`: 열거형 (EnumDef) — `members`

## Development Guidelines

### 새로운 타겟 언어 추가
1. `templates/<lang>/` 폴더 생성
2. `<lang>.toml` 설정 파일 작성 (type_map 필수)
3. `<lang>_file.ptpl` 메인 템플릿 작성
4. `section/`, `detail/` 하위 템플릿 분리
5. (선택) `rhai_utils/type_mapping.rhai`에 Rhai 헬퍼 추가

### 템플릿 작성 규칙
- 복잡한 로직은 `%logic` 블록에서 문자열로 조립, 출력 라인에서 `{{var}}`로 렌더링
- `%include`로 모듈화하여 재사용성 확보
- 들여쓰기는 출력 라인에 직접 작성 (템플릿이 곧 출력)
- 타입 변환은 TOML `type_map` + `lang_type` 필터 활용

### 테스트
- `cargo test` — 스냅샷 테스트 검증
- `tests/runners/<lang>/run_tests.bat` — 통합 테스트 (컴파일 + 실행)
- 템플릿 변경 시 반드시 해당 언어 통합 테스트 실행

*최종 업데이트: 2026-02-02*
