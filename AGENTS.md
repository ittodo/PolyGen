# AGENTS.md - PolyGen AI Assistant Guide

이 문서는 Codex 및 기타 AI 어시스턴트가 PolyGen 프로젝트를 이해하고 효과적으로 작업하기 위한 가이드입니다.

---

## 프로젝트 개요

**PolyGen**은 `.poly` 스키마 파일을 단일 진실 공급원(Single Source of Truth)으로 사용하여 여러 프로그래밍 언어의 코드를 생성하는 **폴리글랏 코드 생성기**입니다.

### 핵심 개념

- **입력**: `.poly` 스키마 파일 (선언적 데이터 모델 정의)
- **출력**: 타겟 언어 코드 (C#, C++, Rust, TypeScript, Go, Unreal, SQLite)
- **템플릿 엔진**: PolyTemplate (.ptpl) — 선언적 코드 생성 DSL + Rhai 스크립팅
- **목적**: 데이터 모델을 한 번 정의하고 모든 플랫폼에서 일관된 코드 생성

---

## 컴파일 파이프라인

```
.poly Schema Files
       ↓
[1. Parser] ─────────── src/polygen.pest (Pest 문법 정의)
       ↓
[2. AST Builder] ────── src/ast_parser/ (파스 트리 → AST 변환)
       ↓
[3. Validation] ─────── src/validation.rs (논리적 검증)
       ↓
[4. IR Builder] ─────── src/ir_builder.rs (AST → IR 변환)
       ↓
[5. Code Generator] ─── src/template/ + templates/ (PolyTemplate 엔진)
       ↓
Generated Code + Static Utilities
```

---

## 소스 코드 진입점

### 주요 진입점

| 파일 | 역할 | 설명 |
|------|------|------|
| `src/main.rs` | CLI 진입점 | `lib.rs` 호출 |
| `src/lib.rs` | 라이브러리 진입점 | Clap CLI 정의, 명령어 라우팅 |
| `src/pipeline.rs` | 파이프라인 조율 | 전체 컴파일 플로우 관리 |

### 레이어별 핵심 파일

| 레이어 | 핵심 파일 | 역할 |
|--------|----------|------|
| **파싱** | `ast_parser/mod.rs` | Pest → AST 변환 |
| **검증** | `validation.rs` | AST 유효성 검사 |
| **IR 변환** | `ir_builder.rs` + `ir_builder/` | AST → IR 변환 |
| **코드 생성** | `template/renderer.rs` | PolyTemplate (.ptpl) 렌더링 |
| **템플릿 파서** | `template/parser.rs` | .ptpl 파일 파싱 (디렉티브, 인터폴레이션) |
| **Rhai 브릿지** | `template/rhai_bridge.rs` | %logic 블록, %if 조건 Rhai 평가 |
| **함수 등록** | `rhai/registry.rs` | Rhai 헬퍼 함수 등록 |
| **마이그레이션** | `migration.rs` | 스키마 diff, ALTER 생성 |

> 상세 구조: [docs/SOURCE_STRUCTURE.md](docs/SOURCE_STRUCTURE.md)

---

## 디렉토리 구조

```
PolyGen/
├── src/                      # Rust 소스 코드 (6,400+ 줄)
│   ├── main.rs               # CLI 진입점
│   ├── lib.rs                # 라이브러리 진입점
│   ├── polygen.pest          # Pest 문법 정의
│   ├── ast_model.rs          # AST 데이터 구조
│   ├── ast_parser/           # AST 파서 모듈 (8개 하위 모듈)
│   │   ├── mod.rs            # 메인 엔트리포인트
│   │   ├── types.rs          # 타입 파싱
│   │   ├── fields.rs         # 필드 정의 파싱
│   │   ├── definitions.rs    # table/enum/embed 파싱
│   │   ├── metadata.rs       # 주석/어노테이션 파싱
│   │   ├── literals.rs       # 리터럴 파싱
│   │   ├── helpers.rs        # 유틸리티 함수
│   │   └── macros.rs         # 파싱 매크로
│   ├── validation.rs         # AST 유효성 검사
│   ├── ir_model.rs           # IR 데이터 구조
│   ├── ir_builder.rs         # AST → IR 변환
│   ├── ir_builder/           # IR 빌더 하위 헬퍼
│   │   └── type_names.rs     # 타입/FQN 이름 헬퍼
│   ├── type_registry.rs      # 타입 레지스트리 (Phase 4)
│   ├── pipeline.rs           # 컴파일 파이프라인
│   ├── codegen.rs            # 코드 생성 유틸리티
│   ├── rhai_generator.rs     # Rhai 템플릿 엔진 (레거시, ptpl 전환 완료)
│   ├── template/             # PolyTemplate 엔진
│   │   ├── parser.rs         # .ptpl 파서 (디렉티브, 인터폴레이션)
│   │   ├── renderer.rs       # 템플릿 렌더러 (출력 생성)
│   │   ├── rhai_bridge.rs    # Rhai 연동 (%logic, %if 조건 평가)
│   │   └── expr.rs           # 표현식/필터 파싱
│   ├── migration.rs          # 마이그레이션 diff 생성
│   ├── db_introspection.rs   # DB 스키마 introspection (SQLite)
│   ├── error.rs              # 에러 타입 정의
│   └── rhai/                 # Rhai 함수 모듈
│       ├── registry.rs       # 함수 등록
│       ├── common/           # 공통 함수
│       └── csharp/           # C# 전용 함수
│
├── templates/                # PolyTemplate (.ptpl) 템플릿 (83+ 파일)
│   ├── csharp/               # C# 템플릿 (51 파일)
│   │   ├── csharp.toml       # 언어 설정 + 타입 매핑
│   │   ├── csharp_file.ptpl  # 메인 클래스 생성
│   │   ├── csharp_csv_columns_file.ptpl  # CSV 컬럼 매핑
│   │   ├── csharp_sqlite_accessor_file.ptpl  # SQLite Accessor
│   │   ├── class/            # 클래스 상세 템플릿
│   │   ├── container/        # Container 템플릿
│   │   ├── csv/              # CSV 관련 템플릿
│   │   ├── enum/             # Enum 템플릿
│   │   └── rhai_utils/       # Rhai 유틸리티 (prelude)
│   ├── cpp/                  # C++ 템플릿
│   │   ├── cpp.toml          # 언어 설정
│   │   ├── cpp_file.ptpl     # 메인 헤더 생성
│   │   ├── cpp_loaders_file.ptpl      # CSV/JSON/Binary 로더
│   │   ├── cpp_container_file.ptpl    # Container + 인덱스
│   │   ├── cpp_sqlite_accessor_file.ptpl  # SQLite Accessor
│   │   ├── detail/           # 상세 템플릿 (pack, auto_update)
│   │   └── rhai_utils/       # Rhai 유틸리티 (prelude)
│   ├── rust/                 # Rust 템플릿
│   │   ├── rust.toml         # 언어 설정
│   │   ├── rust_file.ptpl    # 메인 모듈 생성
│   │   ├── rust_loaders_file.ptpl     # CSV/Binary 로더
│   │   ├── rust_container_file.ptpl   # Container
│   │   ├── rust_sqlite_accessor_file.ptpl  # SQLite Accessor
│   │   ├── detail/           # 상세 템플릿 (pack, auto_update)
│   │   └── rhai_utils/       # Rhai 유틸리티 (prelude)
│   ├── typescript/           # TypeScript 템플릿
│   │   ├── typescript.toml   # 언어 설정
│   │   ├── typescript_file.ptpl       # 인터페이스 생성
│   │   ├── typescript_zod_file.ptpl   # Zod 스키마 생성
│   │   ├── typescript_sqlite_accessor_file.ptpl  # SQLite Accessor
│   │   ├── detail/           # 상세 템플릿
│   │   └── rhai_utils/       # Rhai 유틸리티 (prelude)
│   ├── go/                   # Go 템플릿
│   │   ├── go.toml           # 언어 설정
│   │   ├── go_file.ptpl      # 메인 패키지 생성
│   │   └── go_container_file.ptpl     # Container
│   ├── unreal/               # Unreal Engine 템플릿
│   │   ├── unreal.toml       # 언어 설정
│   │   ├── unreal_file.ptpl           # USTRUCT/UENUM 생성
│   │   ├── unreal_loaders_file.ptpl   # CSV/JSON 로더
│   │   ├── unreal_hotreload_file.ptpl # Hot Reload
│   │   └── rhai_utils/       # Rhai 유틸리티 (prelude)
│   ├── sqlite/               # SQLite 템플릿
│   │   ├── sqlite.toml       # 언어 설정
│   │   ├── sqlite_file.ptpl           # DDL 생성
│   │   ├── sqlite_migration_file.ptpl # 마이그레이션
│   │   └── rhai_utils/       # Rhai 유틸리티 (prelude)
│   ├── mermaid/              # Mermaid 다이어그램 (예정)
│   └── rhai_utils/           # 공유 Rhai 유틸리티 (indent 등)
│
├── static/                   # 런타임 정적 파일
│   └── csharp/               # C# 유틸리티
│       ├── DataSource.cs
│       ├── CsvUtils.cs
│       ├── JsonUtils.cs
│       ├── BinaryUtils.cs
│       └── PolygenAttributes.cs
│
├── tests/                    # 테스트
│   ├── snapshot_tests.rs     # 스냅샷 테스트
│   ├── schemas/              # 테스트용 스키마 (13+ 파일)
│   ├── snapshots/            # Insta 스냅샷
│   ├── output/               # 테스트 출력
│   ├── integration/          # 통합 테스트 스키마 (10개 케이스)
│   │   ├── 01_basic_types/   # 기본 타입 테스트
│   │   ├── 02_imports/       # 크로스 네임스페이스 임포트
│   │   ├── 03_nested_namespaces/  # 중첩 네임스페이스
│   │   ├── 04_inline_enums/  # 인라인 Enum
│   │   ├── 05_embedded_structs/   # 임베디드 구조체
│   │   ├── 06_arrays_and_optionals/  # 배열 + Optional
│   │   ├── 07_indexes/       # 인덱스 + 외래 키
│   │   ├── 08_complex_schema/     # 종합 테스트 (RPG 시스템)
│   │   ├── 09_sqlite/        # SQLite DDL + Accessor
│   │   └── 10_pack_embed/    # @pack 직렬화 (테스트 파일 미작성)
│   └── runners/              # 언어별 테스트 러너
│       ├── cpp/              # C++ 테스트 (MSVC)
│       ├── csharp/           # C# 테스트 (.NET 8.0)
│       ├── rust/             # Rust 테스트 (Cargo)
│       ├── typescript/       # TypeScript 테스트 (tsc --noEmit)
│       └── sqlite/           # SQLite DDL 검증
│
├── examples/                 # 예제 스키마
│   └── game_schema.poly      # 게임 데이터 예제
│
├── docs/                     # 설계 문서
└── output/                   # 생성된 코드 출력 (git ignore)
```

---

## 빠른 참조 - 작업별 파일 위치

| 작업 | 파일 위치 |
|------|----------|
| 문법/파싱 문제 | `src/polygen.pest` → `src/ast_parser/` |
| 이름/타입/제약 검증 | `src/validation.rs` |
| 타입 해석/IR 구조 | `src/ir_builder.rs` → `src/ir_model.rs` |
| 생성 코드 변경 | `templates/<lang>/` (PolyTemplate .ptpl) |
| 런타임 유틸리티 | `static/<lang>/` |
| DB 마이그레이션 | `src/migration.rs` → `src/db_introspection.rs` |
| 회귀 테스트 | `tests/` |

---

## 필수 명령어

### 빌드 & 실행

```bash
# 릴리즈 빌드
cargo build --release

# 실행 (기본)
cargo run -- --schema-path examples/game_schema.poly --lang csharp

# 전체 옵션
cargo run -- \
  --schema-path <SCHEMA_PATH> \
  --lang <LANGUAGE> \
  --templates-dir <TEMPLATES_DIR> \
  --output-dir <OUTPUT_DIR>
```

### 마이그레이션

```bash
# 스키마 비교 방식 (baseline .poly 파일 사용)
cargo run -- migrate --baseline old.poly --schema-path new.poly

# DB 비교 방식 (SQLite 파일에서 직접 스키마 읽기)
cargo run -- migrate --db game.db --schema-path schema.poly

# 출력 디렉토리 지정
cargo run -- migrate --db game.db --schema-path schema.poly --output-dir migrations/
```

### 테스트

```bash
# 모든 테스트 실행
cargo test

# 스냅샷 검토 (변경 승인)
cargo insta review

# 특정 테스트 실행
cargo test test_name

# DB 마이그레이션 테스트
cargo test --test db_migration_tests
```

### 코드 품질

```bash
# 린트 (경고를 에러로 처리)
cargo clippy -- -D warnings

# 포맷팅
cargo fmt --all

# 포맷 검사만
cargo fmt --all -- --check
```

---

## 스키마 언어 (.poly) 문법

### 기본 구조

```poly
// 파일 임포트
import "other_schema.poly";

// 네임스페이스 정의 (중괄호 필수)
namespace game.character {

    // 테이블 정의 (클래스/구조체)
    table Player {
        id: u32 primary_key;
        name: string max_length(100);
        level: u16 default(1) range(1, 100);
        email: string? unique;  // optional
        skills: Skill[];        // array
    }

    // Enum 정의 (값 할당 및 인라인 주석 지원)
    enum PlayerClass {
        Warrior = 1;  // 전사
        Mage = 2;     // 마법사
        Rogue = 3;    // 도적
    }

    // Embed 정의 (재사용 가능한 필드 그룹)
    embed Stats {
        hp: u32;
        mp: u32;
        attack: u32;
    }
}
```

### 주석 규칙 (위치 기반)

`//`와 `///`는 **동일하게 처리**됩니다. 주석의 의미는 **위치**에 따라 결정됩니다:

| 위치 | 의미 | 예시 |
|------|------|------|
| 항목 **앞** (별도 줄) | Doc Comment → 다음 항목에 붙음 | `// 전사 클래스`<br>`Warrior = 1;` |
| 항목 **뒤** (같은 줄) | Inline Comment → 현재 항목에 붙음 | `Warrior = 1; // 전사 클래스` |

```poly
// 이 주석은 AccountType enum의 doc comment가 됨
enum AccountType {
    Cash = 1;        // 이 주석은 Cash의 inline comment
    BankAccount = 2; // 이 주석은 BankAccount의 inline comment
}
```

### 지원 타입

- **기본 타입**: `string`, `bool`, `bytes`
- **정수**: `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`
- **부동소수점**: `f32`, `f64`
- **카디널리티**: `?` (optional), `[]` (array)

### 제약조건

제약조건은 `@` 없이 공백으로 구분하여 나열합니다:

```poly
id: u32 primary_key;
name: string unique max_length(100);
level: u16 default(1) range(1, 100);
```

| 제약조건 | 설명 | 예시 |
|---------|------|------|
| `primary_key` | 기본 키 | `id: u32 primary_key;` |
| `unique` | 고유 값 | `email: string unique;` |
| `max_length(n)` | 최대 길이 | `name: string max_length(50);` |
| `default(value)` | 기본값 | `level: u16 default(1);` |
| `range(min, max)` | 범위 제한 | `hp: u32 range(0, 9999);` |
| `regex("pattern")` | 정규식 검증 | `email: string regex(".*@.*");` |
| `foreign_key(path)` | 외래 키 | `user_id: u32 foreign_key(User.id);` |

### 어노테이션

```poly
@load(csv: "data/players.csv", json: "data/players.json")
@taggable
@link_rows(Character)
table Player {
    // ...
}
```

| 어노테이션 | 적용 대상 | 설명 | 예시 |
|-----------|----------|------|------|
| `@load` | table | CSV/JSON 데이터 로드 경로 | `@load(csv: "data.csv")` |
| `@taggable` | table | 행 태깅 활성화 | `@taggable` |
| `@link_rows` | table | 다른 테이블과 행 연결 | `@link_rows(Character)` |
| `@readonly` | table | 읽기 전용 테이블 | `@readonly` |
| `@cache` | table | 캐시 전략 설정 | `@cache("full_load")` |
| `@datasource` | namespace/table | 데이터 소스 지정 | `@datasource("sqlite")` |
| `@soft_delete` | table | 소프트 삭제 필드 지정 | `@soft_delete("deleted_at")` |
| `@pack` | embed | 필드를 단일 문자열로 직렬화 | `@pack` 또는 `@pack(separator: ",")` |

### @pack 어노테이션

`embed` 타입에 `@pack`을 붙이면 모든 필드를 단일 문자열로 직렬화/역직렬화하는 메서드가 생성됩니다.

```poly
// 기본 구분자: ;
@pack
embed Position {
    x: f32;
    y: f32;
}

// 커스텀 구분자: ,
@pack(separator: ",")
embed Color {
    r: u8;
    g: u8;
    b: u8;
}

table Player {
    id: u32 primary_key;
    position: Position;  // CSV에서 "100;200" 형태로 저장
    color: Color;        // CSV에서 "255,128,64" 형태로 저장
}
```

**생성되는 메서드:**
- C#: `Pack()`, `Unpack(string)`, `TryUnpack(string, out T)`
- C++: `pack()`, `unpack(string)`, `try_unpack(string, T&)`
- Rust: `pack()`, `unpack(&str) -> Result<Self, String>`
- TypeScript: `packX()`, `unpackX()`, `tryUnpackX()`

---

## 주요 데이터 구조

### AST (Abstract Syntax Tree)

`src/ast_model.rs`에 정의:

- `AstRoot` - 루트 노드
- `Definition` - table/enum/embed 정의
- `FieldDefinition` - 필드 정의
- `TypeWithCardinality` - 타입 + 카디널리티
- `Constraint` - 제약조건

### IR (Intermediate Representation)

`src/ir_model.rs`에 정의:

- `SchemaContext` - 전체 스키마 컨텍스트
- `FileDef` - 단일 파일
- `NamespaceDef` - 네임스페이스
- `StructDef` - 구조체/클래스 정의
- `FieldDef` - 필드 정의 (타입 해석 완료)
- `EnumDef` - Enum 정의
- `TypeRef` - 타입 참조 (FQN 포함)

---

## 테스트 전략

### 스냅샷 테스트

- `tests/schemas/`의 `.poly` 파일에 대해 AST/IR 생성 검증
- `cargo insta review`로 변경 사항 승인

### 테스트 스키마

| 파일 | 테스트 목적 |
|------|------------|
| `basic_table.poly` | 기본 테이블 정의 |
| `constraints_table.poly` | 제약조건 파싱 |
| `annotations_table.poly` | 어노테이션 파싱 |
| `inline_embed_table.poly` | 인라인 embed |
| `inline_enum_*.poly` | 인라인 enum |
| `nested_namespaces.poly` | 중첩 네임스페이스 |
| `file_imports.poly` | 파일 임포트 |

### 단위 테스트

- `validation.rs` - 24개 테스트 (중복 정의, 타입 참조 등)
- `ir_builder.rs` - 20개 테스트 (타입 해석, 카디널리티 등)
- `ast_parser/` - 24개 테스트 (파싱 검증)

#### 통합 테스트

통합 테스트는 생성된 코드가 각 언어에서 올바르게 컴파일되고 동작하는지 검증합니다.

```bash
# Windows (.bat)
tests\runners\sqlite\run_tests.bat
tests\runners\typescript\run_tests.bat
tests\runners\csharp\run_tests.bat
tests\runners\cpp\run_tests.bat
tests\runners\rust\run_tests.bat
```

| 테스트 케이스 | 검증 내용 |
|--------------|----------|
| 01_basic_types | 기본 타입 (u8-u64, i8-i64, f32/f64, string, bool) |
| 02_imports | 크로스 네임스페이스 임포트 및 타입 참조 |
| 03_nested_namespaces | 중첩 네임스페이스 |
| 04_inline_enums | 인라인 Enum 정의 |
| 05_embedded_structs | Embed 정의, 중첩 embed |
| 06_arrays_and_optionals | 배열 + Optional 필드 |
| 07_indexes | 인덱스, 외래 키, Container 검증 |
| 08_complex_schema | 게임 데이터 종합 테스트 (RPG 시스템) |
| 09_sqlite | SQLite DDL 생성 + Accessor 컴파일 |
| 10_pack_embed | @pack 직렬화 (테스트 파일 미작성) |

---

## 디버깅

실행 시 `output/` 디렉토리에 디버그 파일 생성:

- `output/debug/parse_tree.txt` - Pest 파스 트리
- `output/ast_debug.txt` - AST 덤프
- `output/ir_debug.txt` - IR 덤프

문제 발생 시 이 파일들을 순서대로 확인하여 어느 단계에서 문제가 발생했는지 추적합니다.

---

## 코드 컨벤션

### Rust 코드

- **모듈/파일**: `snake_case`
- **타입/트레이트**: `PascalCase`
- **함수/변수**: `snake_case`
- **최대 컬럼**: ~100자
- **들여쓰기**: 4 스페이스

### 템플릿 (PolyTemplate)

- **파일명**: `<lang>_<purpose>.ptpl` (메인/엑스트라 템플릿)
- **상세 파일**: `detail/<purpose>.ptpl` (서브 템플릿, %include로 사용)
- **Rhai 유틸리티**: `rhai_utils/<purpose>.rhai` (prelude 스크립트)
- **언어 코드**: 소문자 (`csharp`, `typescript`, `sqlite`)
- **템플릿 문법**: [docs/PTPL_LANGUAGE_GUIDE.md](docs/PTPL_LANGUAGE_GUIDE.md)

### Git 커밋

- 커밋 메시지는 영어로 작성
- 변경 유형 prefix 사용: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`

---

## 새 언어 지원 추가

1. `templates/<new_lang>/` 디렉토리 생성
2. `<new_lang>.toml` 설정 파일 작성 (타입 매핑, binary_read, csv_read 등)
3. `<new_lang>_file.ptpl` 메인 템플릿 작성
4. (선택) 엑스트라 템플릿 추가 (loaders, container, sqlite_accessor 등)
5. (선택) `rhai_utils/type_mapping.rhai` prelude 스크립트 작성
6. (선택) 정적 유틸리티 파일을 `static/<new_lang>/`에 추가
7. `src/codegen.rs`에 정적 파일 복사 로직 추가

---

## 현재 개발 상태

### 리팩토링 진행률: 100%

| Phase | 상태 | 설명 |
|-------|------|------|
| Phase 1 | ✅ 완료 | 코드 품질 (Clippy 경고, 패키지명) |
| Phase 2 | ✅ 완료 | 에러 처리 & 모듈화 |
| Phase 3 | ✅ 완료 | 아키텍처 개선 (CodeGenerator, Pipeline) |
| Phase 4 | ✅ 완료 | 성능 & 확장성 (TypeRegistry, 언어 설정) |

### 지원 언어

| 언어 | 상태 | 기능 |
|------|------|------|
| C# | ✅ 완료 | 클래스, Enum, CSV/JSON/Binary 로더, Container, SQLite Accessor |
| C++ | ✅ 완료 | 헤더 전용, 구조체, Enum, CSV/JSON/Binary 로더, Container, SQLite Accessor |
| Rust | ✅ 완료 | 모듈, Struct, Enum, CSV/Binary 로더, Container, SQLite Accessor |
| TypeScript | ✅ 완료 | 인터페이스, Enum, Zod 스키마 검증, SQLite Accessor |
| Go | ✅ 완료 | 패키지, Struct, Enum, Container |
| Unreal | ✅ 완료 | USTRUCT/UENUM 매크로, CSV/JSON 로더, Hot Reload |
| SQLite | ✅ 완료 | DDL 생성, Migration 스크립트 |
| Mermaid | 📝 예정 | 빈 디렉토리 (다이어그램 생성 예정) |

---

## 관련 문서

| 문서 | 설명 |
|------|------|
| `docs/SOURCE_STRUCTURE.md` | **소스 코드 구조 (모듈별 역할)** |
| `docs/PTPL_LANGUAGE_GUIDE.md` | **PolyTemplate (.ptpl) 언어 레퍼런스** |
| `docs/CUSTOMIZATION.md` | **Rhai 템플릿 커스터마이징 가이드** |
| `docs/TODO.md` | 할일 목록 및 진행 상황 |
| `docs/TEMPLATE_REFACTOR_PLAN.md` | 템플릿 리팩토링 계획 |
| `docs/SQL_TODO.md` | SQL/DB 지원 상세 |
| `docs/LANGUAGE_SUPPORT_GUIDE.md` | 언어 지원 가이드 |
| `docs/agent.md` | 에이전트용 빠른 인덱스 |
| `src/agent.md` | 소스 코드 구조 (간략) |
| `templates/agent.md` | 템플릿 시스템 가이드 |
| `tests/agent.md` | 테스트 구조 |

---

## 주의사항

1. **output/ 디렉토리**: 실행 시 재생성되므로 중요한 파일을 저장하지 마세요
2. **스냅샷 변경**: 코어 로직 변경 시 `cargo insta review`로 스냅샷 업데이트 필요
3. **인코딩**: UTF-8 사용
4. **명시적 요청 없이 코드 변경 금지**: 사용자의 명시적 지시가 있을 때만 코드 수정

---

## 문서 동기화 규칙

**소스 파일 추가/변경/삭제 시 반드시 관련 문서를 업데이트하세요.**

| 변경 유형 | 업데이트 대상 문서 |
|----------|-------------------|
| `.rs` 파일 추가/삭제 | `docs/SOURCE_STRUCTURE.md`, `AGENTS.md` (디렉토리 구조) |
| 새 모듈 추가 | `docs/SOURCE_STRUCTURE.md` (모듈별 상세) |
| 공개 API 변경 | 해당 `.rs` 파일의 doc comment |
| 새 기능 완료 | `docs/TODO.md` (완료 항목 체크) |
| 새 언어 지원 | `AGENTS.md` (지원 언어), `docs/LANGUAGE_SUPPORT_GUIDE.md` |
| 새 어노테이션 | `AGENTS.md` (어노테이션 표), `docs/ANNOTATIONS_AND_ATTRIBUTES.md` |
| 템플릿 변경 | `templates/agent.md` |
| 테스트 추가 | `tests/agent.md` |

### 문서화 체크리스트

```
□ 새 파일 추가 시 → SOURCE_STRUCTURE.md 업데이트
□ 함수 시그니처 변경 → doc comment 업데이트
□ 기능 완료 시 → TODO.md 체크
□ 주요 변경 시 → AGENTS.md 업데이트 날짜 변경
```

---

## 의존성 요약

| 크레이트 | 버전 | 용도 |
|---------|------|------|
| pest | 2.7 | PEG 파서 생성 |
| rhai | 1.22.2 | 템플릿 스크립팅 엔진 |
| serde | 1.0 | 직렬화 (IR JSON 출력) |
| thiserror | 1.0 | 에러 타입 정의 |
| heck | 0.5 | 케이스 변환 |
| clap | 4.5 | CLI 인자 파싱 |
| rusqlite | 0.31 | SQLite DB introspection |
| insta | 1.34 | 스냅샷 테스트 (dev) |

---

*최종 업데이트: 2026-05-22*
