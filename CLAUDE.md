# CLAUDE.md - PolyGen AI Assistant Guide

이 문서는 Claude 및 기타 AI 어시스턴트가 PolyGen 프로젝트를 이해하고 효과적으로 작업하기 위한 가이드입니다.

---

## 프로젝트 개요

**PolyGen**은 `.poly` 스키마 파일을 단일 진실 공급원(Single Source of Truth)으로 사용하여 여러 프로그래밍 언어의 코드를 생성하는 **폴리글랏 코드 생성기**입니다.

### 핵심 개념

- **입력**: `.poly` 스키마 파일 (선언적 데이터 모델 정의)
- **출력**: 타겟 언어 코드 (C#, C++, Rust, TypeScript 지원, MySQL 확장 예정)
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
[5. Code Generator] ─── src/rhai_generator.rs + templates/ (코드 생성)
       ↓
Generated Code + Static Utilities
```

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
│   ├── ir_builder.rs         # AST → IR 변환 (가장 큰 모듈)
│   ├── type_registry.rs      # 타입 레지스트리 (Phase 4)
│   ├── pipeline.rs           # 컴파일 파이프라인
│   ├── codegen.rs            # 코드 생성 유틸리티
│   ├── rhai_generator.rs     # Rhai 템플릿 엔진
│   ├── migration.rs          # 마이그레이션 diff 생성
│   ├── db_introspection.rs   # DB 스키마 introspection (SQLite)
│   ├── error.rs              # 에러 타입 정의
│   └── rhai/                 # Rhai 함수 모듈
│       ├── registry.rs       # 함수 등록
│       ├── common/           # 공통 함수
│       └── csharp/           # C# 전용 함수
│
├── templates/                # Rhai 템플릿 (60+ 파일)
│   ├── csharp/               # C# 템플릿
│   │   ├── csharp.toml       # 언어 설정
│   │   ├── csharp_file.rhai  # 메인 클래스 생성
│   │   ├── csharp_csv_mappers_file.rhai
│   │   ├── csharp_json_mappers_file.rhai
│   │   ├── csharp_binary_*.rhai
│   │   ├── class/            # 클래스 템플릿
│   │   ├── enum/             # Enum 템플릿
│   │   └── rhai_utils/       # 유틸리티 스크립트
│   ├── cpp/                  # C++ 템플릿
│   │   ├── cpp.toml          # 언어 설정
│   │   ├── cpp_file.rhai     # 메인 헤더 생성
│   │   ├── cpp_loaders_file.rhai  # CSV/JSON 로더
│   │   └── rhai_utils/       # 유틸리티 스크립트
│   ├── rust/                 # Rust 템플릿
│   │   ├── rust.toml         # 언어 설정
│   │   ├── rust_file.rhai    # 메인 모듈 생성
│   │   ├── rust_loaders_file.rhai  # CSV 로더
│   │   └── rhai_utils/       # 유틸리티 스크립트
│   ├── typescript/           # TypeScript 템플릿
│   │   ├── typescript.toml   # 언어 설정
│   │   ├── typescript_file.rhai  # 인터페이스 생성
│   │   ├── typescript_zod_file.rhai  # Zod 스키마 생성
│   │   └── rhai_utils/       # 유틸리티 스크립트
│   └── mysql/                # MySQL 템플릿
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
│   ├── integration/          # 통합 테스트 스키마 (8개 케이스)
│   │   ├── 01_basic_types/   # 기본 타입 테스트
│   │   ├── 02_enums/         # Enum 테스트
│   │   ├── 03_nested_namespaces/  # 중첩 네임스페이스
│   │   ├── 04_optional_fields/    # Optional 필드
│   │   ├── 05_arrays/        # 배열 테스트
│   │   ├── 06_annotations/   # 어노테이션
│   │   ├── 07_cross_references/   # 타입 간 참조
│   │   └── 08_complex_schema/     # 종합 테스트
│   └── runners/              # 언어별 테스트 러너
│       ├── cpp/              # C++ 테스트 (CMake)
│       ├── csharp/           # C# 테스트 (.NET)
│       ├── rust/             # Rust 테스트 (Cargo)
│       └── typescript/       # TypeScript 테스트 (npm/tsc)
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
| 생성 코드 변경 | `templates/<lang>/` (Rhai 템플릿) |
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

### 통합 테스트

통합 테스트는 생성된 코드가 각 언어에서 올바르게 컴파일되고 동작하는지 검증합니다.

```bash
# C++ 테스트 실행
cd tests/runners/cpp && ./run_tests.sh

# C# 테스트 실행
cd tests/runners/csharp && dotnet test

# Rust 테스트 실행
cd tests/runners/rust && cargo test

# TypeScript 테스트 실행
cd tests/runners/typescript && ./run_tests.sh
```

| 테스트 케이스 | 검증 내용 |
|--------------|----------|
| 01_basic_types | 기본 타입 (u8-u64, i8-i64, f32/f64, string, bool) |
| 02_enums | Enum 정의 및 직렬화 |
| 03_nested_namespaces | 중첩 네임스페이스와 cross-namespace 타입 참조 |
| 04_optional_fields | Optional 필드 (`?`) 처리 |
| 05_arrays | 배열 타입 (`[]`) 처리 |
| 06_annotations | @load, @taggable 어노테이션 |
| 07_cross_references | 외래 키 및 타입 간 참조 |
| 08_complex_schema | 게임 데이터 종합 테스트 (RPG 시스템) |

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

### 템플릿 (Rhai)

- **파일명**: `<lang>_<purpose>.rhai`
- **언어 코드**: 소문자 (`csharp`, `mysql`, `typescript`)

### Git 커밋

- 커밋 메시지는 영어로 작성
- 변경 유형 prefix 사용: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`

---

## 새 언어 지원 추가

1. `templates/<new_lang>/` 디렉토리 생성
2. `<new_lang>.toml` 설정 파일 작성
3. `<new_lang>_file.rhai` 메인 템플릿 작성
4. (선택) 정적 유틸리티 파일을 `static/<new_lang>/`에 추가
5. `src/codegen.rs`에 정적 파일 복사 로직 추가

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
| C# | ✅ 완료 | 클래스, Enum, CSV/JSON/Binary 로더 |
| C++ | ✅ 완료 | 헤더 전용, 구조체, Enum, CSV/JSON/Binary 로더 |
| Rust | ✅ 완료 | 모듈, Struct, Enum, CSV/Binary 로더 |
| MySQL | 🚧 진행중 | DDL 스크립트 생성 |
| TypeScript | ✅ 완료 | 인터페이스, Enum, Zod 스키마 검증 |

---

## 관련 문서

| 문서 | 설명 |
|------|------|
| `development_guide.md` | 개발 워크플로우 가이드 |
| `REFACTORING_TODO.md` | 리팩토링 진행 상황 |
| `RHAI_REFACTOR_PLAN.md` | Rhai 모듈 리팩토링 계획 |
| `TEMPLATE_REFACTOR_PLAN.md` | 템플릿 통합 계획 |
| `PHASE4_TODO.md` | Phase 4 상세 계획 |
| `agent.md` | 에이전트용 빠른 인덱스 |
| `src/agent.md` | 소스 코드 구조 |
| `templates/agent.md` | 템플릿 시스템 가이드 |
| `tests/agent.md` | 테스트 구조 |

---

## 주의사항

1. **output/ 디렉토리**: 실행 시 재생성되므로 중요한 파일을 저장하지 마세요
2. **스냅샷 변경**: 코어 로직 변경 시 `cargo insta review`로 스냅샷 업데이트 필요
3. **인코딩**: UTF-8 사용
4. **명시적 요청 없이 코드 변경 금지**: 사용자의 명시적 지시가 있을 때만 코드 수정

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

*최종 업데이트: 2026-01-26*
