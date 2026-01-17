# src/ - Agent Documentation

## Scope
PolyGen의 핵심 Rust 소스 코드가 위치한 폴더입니다. 스키마 파싱, AST/IR 변환, 유효성 검사, 코드 생성 등 전체 파이프라인을 구현합니다.

## Structure
```
src/
├── ast_model.rs          # AST 데이터 모델 정의
├── ast_parser.rs         # Pest 기반 파서 구현
├── ir_model.rs           # IR 데이터 모델 정의
├── ir_builder.rs         # AST → IR 변환 로직
├── validation.rs         # AST 유효성 검사
├── error.rs              # 에러 타입 정의
├── rhai_generator.rs     # Rhai 템플릿 엔진 연동
├── polygen.pest          # Pest 문법 정의
├── lib.rs                # 라이브러리 엔트리 + polygen.pest 매크로
├── main.rs               # CLI 엔트리 포인트
└── rhai/                 # Rhai 템플릿용 사용자 정의 함수
    ├── mod.rs            # Rhai 모듈 정의
    ├── registry.rs       # Rhai 함수 레지스트리
    ├── csharp.rs         # C# 코드 생성용 함수
    └── csv.rs           # CSV 매퍼 생성용 함수
```

## Files

### Core Parser & AST
- **polygen.pest**: Pest 파서 생성기 문법 정의
  - `.poly` 파일의 문법 규칙을 정의합니다
  - Pest가 이 파일에서 파서 코드를 자동 생성합니다

- **ast_model.rs**: AST (Abstract Syntax Tree) 데이터 모델
  - `AstRoot`: 스키마 파일 전체 내용
  - `Definition`: 네임스페이스, 테이블, 열거형, 임베드 등
  - `Namespace`, `Table`, `Enum`, `Embed`, `FieldDefinition` 등
  - 스키마 파일의 구조를 Rust 타입으로 표현합니다

- **ast_parser.rs**: Pest 파서 구현 (35KB)
  - Pest 파서를 사용하여 `.poly` 파일을 AST로 변환
  - `build_ast_from_pairs()`: Pest 파싱 결과를 AST로 빌드
  - 파일 임포트 처리 (`import` 문)
  - 주석 및 어노테이션 처리

### IR (Intermediate Representation)
- **ir_model.rs**: IR 데이터 모델 (JSON 직렬화 가능)
  - `SchemaContext`: 루트 컨텍스트
  - `FileDef`: 단일 소스 파일
  - `NamespaceDef`: 네임스페이스 정의
  - `StructDef`: 구조체/클래스 정의
  - `EnumDef`: 열거형 정의
  - `FieldDef`: 필드 정의
  - `TypeRef`: 타입 참조 (원본, 변환, FQN 등)

- **ir_builder.rs**: AST → IR 변환 (29KB)
  - `build_ir()`: 여러 AST에서 IR로 변환
  - 네임스페이스 구조화
  - 타입 FQN (Fully Qualified Name) 생성
  - 임베드 처리 (중첩 구조체)
  - 열거형 처리 및 타입 추론
  - 관계 필드 생성 (`foreign_key` → 역참조)
  - 필드 번호 자동 할당

### Validation & Error Handling
- **validation.rs**: AST 유효성 검사 (8.8KB)
  - `validate_ast()`: 전체 AST 유효성 검사
  - 중복 이름 검사 (같은 네임스페이스 내)
  - 순환 참조 감지
  - 외래 키 타겟 존재 확인
  - 필드 제약조건 유효성 검사

- **error.rs**: 커스텀 에러 타입
  - `PolygenError`: 프로젝트 전체 에러 타입
  - `ParseError`, `ValidationError`, `GenerationError` 등

### Code Generation
- **rhai_generator.rs**: Rhai 템플릿 엔진 연동 (1.1KB)
  - `generate_code_with_rhai()`: IR과 Rhai 템플릿으로 코드 생성
  - Rhai 엔진 설정 및 컨텍스트 주입
  - 템플릿 파일 로드 및 실행

### Rhai Functions
- **rhai/mod.rs**: Rhai 모듈 정의
  - 사용자 정의 함수 모듈 내보내기

- **rhai/registry.rs**: Rhai 함수 레지스트리 (13KB)
  - Rhai 엔진에 커스텀 함수 등록
  - `register_rhai_functions()`: 모든 함수 등록
  - 타입 변환, 문자열 조작, FQN 처리 등 유틸리티

- **rhai/csharp.rs**: C# 코드 생성용 함수 (4KB)
  - C# 타입 매핑 (`map_type()`)
  - C# 네임스페이스 처리
  - 접근 제어자 생성

- **rhai/csv.rs**: CSV 매퍼 생성용 함수 (51KB)
  - CSV 컬럼 이름 생성
  - CSV 헤더 생성
  - CSV 읽기/쓰기 코드 생성
  - 중첩 구조체 처리
  - 리스트/배열 처리

### Entry Points
- **lib.rs**: 라이브러리 엔트리 (9.2KB)
  - 모든 모듈 재내보내
  - `#[derive(Parser)]`와 `#[grammar = "polygen.pest"]` 매크로
  - `Cli`: CLI 인자 구조체
  - `parse_and_merge_schemas()`: 여러 스키마 파일 병합
  - `run()`: 메인 실행 로직
  - `build_ir_from_asts()`: AST → IR 변환 진입점

- **main.rs**: CLI 엔트리 포인트 (139바이트)
  - `main()` 함수에서 `lib.rs::run()` 호출
  - CLI 파싱 및 에러 핸들링

## Key Concepts

### 파이프라인
```
.poly 파일 → Pest Parser → AST → Validation → IR → Rhai Template → Generated Code
```

### 데이터 흐름
1. **파싱**: Pest가 `.poly` 파일을 파싱하여 AST 생성
2. **검사**: Validation 단계에서 AST 구조 확인
3. **변환**: AST를 언어 독립적인 IR로 변환
4. **생성**: Rhai 템플릿 엔진이 IR을 타겟 언어 코드로 변환

### 타입 시스템
- **AST**: 스키마 원본 구조 (`.poly` 파일 그대로)
- **IR**: 언어 독립적인 중간 표현 (JSON 직렬화 가능)
  - FQN (Fully Qualified Name)으로 타입 식별
  - 타입 참조 체인 유지
  - 생성된 코드에 필요한 모든 메타데이터 포함

### 네임스페이스
- 네임스페이스는 논리적 그룹핑 단위
- `path: Vec<String>`으로 중첩 구조 표현
- FQN 형식: `game.common.StatBlock`

### 임베드 타입
- 명명된 임베드: 네임스페이스 레벨에서 정의, 여러 곳에서 재사용
- 익명 임베드: 테이블 내부에 직접 정의, 해당 테이블에서만 사용
- 중첩 임베드: 임베드 내부에 또 다른 임베드 가능

## Dependencies

### 외부 라이브러리
- `pest` 2.7 + `pest_derive`: 파서 생성기
- `rhai` 1.22.2: 템플릿 엔진
- `serde`: IR → JSON 직렬화
- `thiserror`: 커스텀 에러 타입
- `anyhow`: 에러 핸들링
- `heck`: 케이스 변환
- `regex`: 정규식 처리

### 내부 의존성
- `ast_parser.rs` → `ast_model.rs`, `error.rs`
- `ir_builder.rs` → `ast_model.rs`, `ir_model.rs`, `error.rs`
- `validation.rs` → `ast_model.rs`, `error.rs`
- `rhai_generator.rs` → `ir_model.rs`, `rhai/*`
- `lib.rs` → 모든 모듈
- `rhai/*` → `ir_model.rs`

## Development Guidelines

### 코딩 스타일
- Rust 2021 에디션
- 4-스페이스 인덴트
- 최대 ~100칼럼 너비
- 모듈/파일: `snake_case`
- 타입/트레이트: `PascalCase`
- 함수/변수: `snake_case`

### 테스트
- `insta` 스냅샷 테스트 사용
- 테스트 스키마: `tests/schemas/*.poly`
- 스냅샷: `tests/snapshots/*_ast.snap`, `*_ir.snap`
- 스냅샷 업데이트: `cargo insta review` 또는 `$env:INSTA_UPDATE='auto'; cargo test`

### 빌드 및 린트
```bash
cargo build            # 빌드
cargo test             # 테스트 실행
cargo clippy -- -D warnings  # 린트 (경고가 에러로 처리됨)
cargo fmt --all        # 포맷팅
```

### 새로운 기능 추가 시
1. `polygen.pest`에 문법 추가
2. `ast_model.rs`에 AST 타입 추가
3. `ast_parser.rs`에 파싱 로직 추가
4. `validation.rs`에 검사 로직 추가 (필요한 경우)
5. `ir_builder.rs`에 IR 변환 로직 추가
6. `ir_model.rs`에 IR 타입 추가
7. `rhai_generator.rs` 또는 `rhai/*.rs`에 코드 생성 로직 추가
8. 템플릿 파일에 새로운 코드 생성 패턴 추가
9. 테스트 스키마 및 스냅샷 추가

### 주의사항
- **불필요한 import 제거**: 현재 15개의 경고 존재 (unused imports, unused variables 등)
- **크레이트 이름**: `PolyGen` → `poly_gen`으로 변경 권장 (Rust 관례)
- **mutable 변수**: 필요하지 않은 경우 제거 (4개 경고)
- **사용하지 않는 함수**: 제거하거나 `#[allow(dead_code)]` 추가

### 디버깅
- 출력 디렉토리에 디버그 파일 생성:
  - `debug/parse_tree.txt`: Pest 파싱 트리
  - `ast_debug.txt`: 전체 AST 덤프
  - `ir_debug.txt`: 전체 IR 덤프
