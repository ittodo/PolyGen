# tests/ - Agent Documentation

## Scope
PolyGen의 테스트 코드가 위치한 폴더입니다. 스냅샷 테스트를 주로 사용하며, 생성된 코드가 기대한 형식을 출력하는지 검증합니다.

## Structure
```
tests/
├── snapshot_tests.rs        # 스냅샷 테스트 메인 파일
├── schemas/                 # 테스트용 스키마 파일
│   ├── basic_table.poly
│   ├── named_enum.poly
│   ├── inline_enum_test.poly
│   ├── named_embed.poly
│   ├── inline_embed_table.poly
│   ├── annotations_table.poly
│   ├── constraints_table.poly
│   ├── nested_namespaces.poly
│   ├── inline_enum_field_table.poly
│   ├── anonymous_enum_field_table.poly
│   ├── file_imports.poly
│   ├── imported_schema_a.poly
│   └── imported_schema_b.poly
├── snapshots/               # 테스트 스냅샷 결과
│   ├── snapshot_tests__basic_table_ast.snap
│   ├── snapshot_tests__basic_table_ir.snap
│   ├── snapshot_tests__named_enum_ast.snap
│   ├── snapshot_tests__named_enum_ir.snap
│   ├── snapshot_tests__inline_enum_test_ast.snap
│   ├── snapshot_tests__inline_enum_test_ir.snap
│   ├── snapshot_tests__named_embed_ast.snap
│   ├── snapshot_tests__named_embed_ir.snap
│   ├── snapshot_tests__inline_embed_table_ast.snap
│   ├── snapshot_tests__inline_embed_table_ir.snap
│   ├── snapshot_tests__annotations_table_ast.snap
│   ├── snapshot_tests__annotations_table_ir.snap
│   ├── snapshot_tests__constraints_table_ast.snap
│   ├── snapshot_tests__constraints_table_ir.snap
│   ├── snapshot_tests__nested_namespaces_ast.snap
│   ├── snapshot_tests__nested_namespaces_ir.snap
│   ├── snapshot_tests__inline_enum_field_table_ast.snap
│   ├── snapshot_tests__inline_enum_field_table_ir.snap
│   ├── snapshot_tests__anonymous_enum_field_table_ast.snap
│   ├── snapshot_tests__anonymous_enum_field_table_ir.snap
│   ├── snapshot_tests__file_imports_ast.snap
│   ├── snapshot_tests__file_imports_ir.snap
│   ├── snapshot_tests__imported_schema_a_ast.snap
│   ├── snapshot_tests__imported_schema_a_ir.snap
│   ├── snapshot_tests__imported_schema_b_ast.snap
│   ├── snapshot_tests__imported_schema_b_ir.snap
│   └── snapshot_tests__csv_mappers_csharp.snap
├── test_data/              # 테스트용 데이터 파일
│   ├── sample_input.json
│   ├── expected_output.csv
│   └── csv_test_schema.poly
├── game_schema_compile_test/  # C# 컴파일 테스트 프로젝트
├── game_schema_test/       # C# 테스트 프로젝트
├── csharp_test_runner/      # C# 테스트 러너
└── output/                 # 테스트 출력 디렉토리
```

## Files

### 테스트 코드
- **snapshot_tests.rs**: 스냅샷 테스트 메인 (1.6KB)
  - `test_ast_snapshots()`: 모든 스키마 파일에 대한 AST 스냅샷 테스트
  - `test_csv_mappers_snapshot()`: CSV 매퍼 생성 스냅샷 테스트
  - `insta` 매크로를 사용하여 스냅샷 비교

### 테스트 스키마 파일

#### 기본 구조 테스트
- **basic_table.poly**: 기본 테이블 구조
  - 기본 필드 타입 테스트 (u32, string, f32, bool)
  - 필수/선택 필드 테스트
  - 배열 필드 테스트

#### 열거형 테스트
- **named_enum.poly**: 명명된 열거형
  - 네임스페이스 레벨 열거형 정의
  - 값 목록 테스트

- **inline_enum_test.poly**: 인라인 열거형
  - 테이블 내부에 열거형 정의
  - 익명 열거형 사용

#### 임베드 타입 테스트
- **named_embed.poly**: 명명된 임베드 타입
  - 네임스페이스 레벨 임베드 정의
  - 여러 테이블에서 재사용

- **inline_embed_table.poly**: 인라인 임베드
  - 테이블 내부에 임베드 정의
  - 익명 임베드 사용

#### 어노테이션 및 제약조건 테스트
- **annotations_table.poly**: 어노테이션
  - `@taggable` 어노테이션 테스트
  - `@load`, `@save` 어노테이션 테스트

- **constraints_table.poly**: 제약조건
  - `primary_key`, `unique` 테스트
  - `max_length`, `default`, `range` 테스트
  - `regex` 제약조건 테스트

#### 네임스페이스 및 필드 테스트
- **nested_namespaces.poly**: 중첩 네임스페이스
  - 깊은 네임스페이스 구조 테스트
  - `game.core.character.player` 등

- **inline_enum_field_table.poly**: 인라인 열거형 필드
  - 테이블 내부에 열거형 정의 후 필드에서 사용

- **anonymous_enum_field_table.poly**: 익명 열거형 필드
  - 필드 타입으로 직접 익명 열거형 정의

#### 파일 임포트 테스트
- **file_imports.poly**: 파일 임포트
  - `import` 문 테스트
  - 여러 파일 병합 테스트

- **imported_schema_a.poly**: 임포트된 스키마 A
  - 파일 임포트용 소스 스키마

- **imported_schema_b.poly**: 임포트된 스키마 B
  - 파일 임포트용 소스 스키마

### 스냅샷 파일
- **_ast.snap 파일들**: AST 스냅샷 (13개)
  - 각 스키마 파일에 대한 AST 구조 기록
  - `snapshot_tests__<schema>_ast.snap` 형식

- **_ir.snap 파일들**: IR 스냅샷 (13개)
  - 각 스키마 파일에 대한 IR 구조 기록
  - `snapshot_tests__<schema>_ir.snap` 형식

- **csv_mappers_csharp.snap**: CSV 매퍼 생성 스냅샷
  - C# CSV 매퍼 코드 생성 결과

### 테스트 데이터
- **sample_input.json**: JSON 테스트 입력
  - JSON → CSV 변환 테스트용

- **expected_output.csv**: 예상 CSV 출력
  - 변환 테스트의 기대 결과

- **csv_test_schema.poly**: CSV 테스트 스키마
  - CSV 관련 기능 테스트용 스키마

### C# 테스트 프로젝트
- **game_schema_compile_test/**: C# 컴파일 테스트
  - 생성된 C# 코드가 컴파일되는지 검증

- **game_schema_test/**: C# 기능 테스트
  - 생성된 C# 코드의 기능 테스트
  - CSV 읽기/쓰기 테스트
  - 바이너리 직렬화 테스트

- **csharp_test_runner/**: C# 테스트 러너
  - C# 테스트 실행 프로그램

## Key Concepts

### 스냅샷 테스트
- **Insta 프레임워크**: 스냅샷 기반 테스트
- **AST 스냅샷**: 파싱 결과가 올바른지 검증
- **IR 스냅샷**: 변환된 IR이 올바른지 검증
- **코드 생성 스냅샷**: 생성된 코드가 기대한 형식인지 검증

### 테스트 명명 규칙
- 스키마 파일: `<feature>.poly`
- 스냅샷 파일: `snapshot_tests__<feature>_<type>.snap`
  - `<type>`: `ast`, `ir`, `csharp` 등

### 테스트 카테고리
1. **기본 구조**: 테이블, 필드, 타입
2. **열거형**: 명명된, 인라인, 익명
3. **임베드**: 명명된, 인라인
4. **제약조건**: primary_key, unique, range 등
5. **어노테이션**: @taggable, @load, @save
6. **네임스페이스**: 중첩 구조
7. **파일 임포트**: import 문

## Dependencies

### 외부 라이브러리
- **insta 1.34**: 스냅샷 테스트 프레임워크
- **walkdir 2.5**: 파일 시스템 순회

### 내부 의존성
- **src/lib.rs**: 테스트에서 호출하는 핵심 함수
- **examples/**: 참고용 스키마

## Development Guidelines

### 새로운 테스트 추가 시
1. `tests/schemas/<new_feature>.poly` 생성
2. `cargo test` 실행하여 스냅샷 자동 생성
3. 스냅샷 검토: `cargo insta review`
4. 필요한 경우 C# 테스트 프로젝트에 테스트 추가

### 스냅샷 업데이트
```bash
# 모든 스냅샷 업데이트
$env:INSTA_UPDATE='auto'; cargo test

# 또는
cargo insta review
```

### 테스트 실행
```bash
# 전체 테스트 실행
cargo test

# 특정 테스트만 실행
cargo test test_ast_snapshots
cargo test test_csv_mappers_snapshot
```

### 스냅샷 관리
- 스냅샷은 git에 커밋됩니다
- 의도적인 변경이 아닌 경우 스냅샷을 업데이트하지 마세요
- PR에서 스냅샷 변경을 주의 깊게 검토하세요

### C# 테스트
- C# 프로젝트는 `dotnet build`, `dotnet test`로 실행
- 생성된 코드의 기능을 직접 테스트

### 테스트 커버리지
- 현재 2개의 스냅샷 테스트만 존재
- 유닛 테스트가 부족 (개선 필요)

### 주의사항
- **스냅샷 크기**: 스냅샷이 너무 크면 가독성 저하
- **테스트 격리**: 각 테스트는 독립적이어야 함
- **실패한 스냅샷**: 원인을 명확히 파악 후 업데이트
- **삭제된 기능**: 해당 스냅샷도 함께 삭제
