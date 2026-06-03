# tests/ - Agent Documentation

## Scope
PolyGen의 테스트 코드가 위치한 폴더입니다. 스냅샷 테스트를 주로 사용하며, 생성된 코드가 기대한 형식을 출력하는지 검증합니다.

## Structure
```
tests/
├── snapshot_tests.rs        # 스냅샷/생성 smoke 테스트 메인 파일
├── schemas/                 # AST/IR 스냅샷용 스키마 파일
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
├── integration/             # 언어별 runner가 공유하는 01-10 통합 테스트 schema
├── runners/                 # C#/C++/Rust/TypeScript/Go/DB/descriptor 통합 runner
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
└── output/                 # 테스트 출력 디렉토리
```

## Files

### 테스트 코드
- **snapshot_tests.rs**: 스냅샷/생성 smoke 테스트 메인
  - `test_ast_snapshots()`: 모든 스키마 파일에 대한 AST 스냅샷 테스트
  - `test_csv_mappers_snapshot()`: 임시 출력 디렉터리에서 CSV 매퍼 생성 스냅샷 테스트
  - `test_redis_key_helpers_snapshot()`: Redis descriptor/Lua helper 및 C#/C++/Rust/TypeScript/Go/Python/Kotlin/Swift/Unreal key helper 생성 스냅샷 테스트
  - `test_additional_language_generation_smoke()`: Python/Kotlin/Swift/Protobuf/MessagePack 기본 schema 생성물의 핵심 구조 smoke 테스트
  - `test_db_auto_timestamp_ddl_generation()`: MySQL/PostgreSQL/SQLite DDL의 auto timestamp 생성 smoke 테스트
  - `test_csharp_auto_update_helpers_generation()`: C# `auto_update` helper, `auto_create` timezone, binary bytes/enum cardinality와 nullable value 생성 smoke 테스트, binary reader/writer와 mapping prelude의 TODO placeholder 부재 검증
  - `test_cpp_binary_enum_cardinality_generation()`: C++ binary loader의 `Enum?`, `Enum[]` 생성이 checked enum read/write helper와 validator를 사용하는지 검증
  - `test_rust_enum_try_from_generation()`: Rust enum `TryFrom<i32>`와 CSV/Binary loader의 checked enum 및 optional enum 변환 생성 검증
  - `test_rust_csv_list_parse_errors_generation()`: Rust CSV primitive 배열/optional primitive 파싱이 invalid item을 삭제하거나 `None`으로 바꾸지 않고 `LoadError`를 생성하는지, `bool[]`이 scalar bool과 같은 `yes/no`, `1/0` 규칙을 쓰는지 검증
  - `test_go_pack_methods_generation()`: Go @pack `Pack`, `Unpack<Type>`, `TryUnpack<Type>` 생성과 finite float/unsigned parser helper 생성 검증
  - `test_typescript_pack_unpack_validation_generation()`: TypeScript @pack unpack helper가 checked number/bool parser를 생성하고 `NaN`/invalid bool/negative unsigned를 침묵 변환하지 않는지 검증
  - `insta` 매크로를 사용하여 스냅샷 비교
- **src/lib.rs 단위 테스트**: CLI 라우팅/보조 함수 테스트
  - `watch` 명령의 `--schema` alias 파싱 검증
  - watch 대상 경로 중복 제거 및 관련 파일 확장자 필터 검증
  - 생성 결과 출력 순회는 `read_dir` 엔트리 오류를 숨기지 않고 상위 호출자에게 전파
- **src/codegen.rs 단위 테스트**: 출력 파일명/manifest/언어 탐색 보조 함수 테스트
  - `discover_languages()`가 유효한 main template만 수집하고 알파벳순으로 반환하는지 검증
- **tests/runners/rust/run_tests.bat**, **tests/runners/rust/run_tests.sh**: Rust 생성 코드 통합 테스트
  - 고정 `TEST_CASES`의 테스트 디렉터리, schema 또는 Rust test file이 누락되면 skip이 아니라 실패로 처리
  - 요약은 실제 결과 카운터인 passed/failed만 출력
  - Windows/POSIX runner는 generation 실패와 생성된 `.rs` 파일 존재를 컴파일 전에 사전 검증
  - Windows/POSIX runner는 생성 파일과 test `main.rs` 복사 실패도 즉시 실패로 처리
  - Windows/POSIX runner는 Rust 빌드/런타임 실패 로그를 숨기지 않고 출력해 원인을 바로 확인할 수 있게 함
  - POSIX runner는 `pipefail`로 `cargo build | sed`와 `polygen | sed`의 앞 명령 실패를 보존
  - 생성된 `schema_redis_keys.rs` 모듈도 `lib.rs`에 포함해 컴파일 검증
  - `03_nested_namespaces`에서 깊은 namespace table에도 `BinaryIO` impl과 container table/index가 생성되는지 검증
  - `09_sqlite`에서 중첩 namespace table이 상위 `@datasource("sqlite")`를 상속해 accessor에 포함되는지 검증
  - `04_inline_enums`에서 invalid binary enum discriminant가 `InvalidData`로 거부되는지 검증
  - `06_arrays_and_optionals`에서 invalid CSV primitive 배열 item 및 optional primitive 값이 `LoadError`로 거부되는지, `bool[]`의 `yes/no`, `1/0` 입력이 유지되는지 검증
  - Windows/Linux runner 모두 `09_sqlite` 케이스와 `schema_sqlite_accessor.rs` 포함 컴파일을 검증
- **tests/runners/typescript/run_tests.bat**, **tests/runners/typescript/run_tests.sh**: TypeScript 생성 코드 통합 테스트
  - 고정 `TEST_CASES`의 테스트 디렉터리, schema 또는 test file이 누락되면 skip이 아니라 실패로 처리
  - 요약은 실제 결과 카운터인 passed/failed만 출력
  - `npm`/`tsx` 의존성 설치 실패는 이후 type-check 단계로 넘기지 않고 즉시 실패로 처리하며 설치 로그를 출력
  - Windows/POSIX runner 모두 각 케이스의 생성된 `.ts` 파일 존재를 사전 검증
  - Windows/POSIX runner는 generation/typecheck/runtime 실패 로그를 generated 루트에 남기고 실패 시 즉시 출력
  - `schema_redis_keys.ts`도 `tsconfig.json` include에 포함해 type-check 검증
  - `tests/run_all.ts`를 `tsx`로 실행해 10개 테스트 모듈의 runtime assertion까지 검증
  - `console.assert`를 throw 방식으로 재정의해 실패가 exit code 1로 전파되도록 검증
  - `03_nested_namespaces`에서 중첩 namespace Zod schema가 valid/invalid 값을 런타임에서 판별하는지 검증
  - `09_sqlite`에서 중첩 namespace SQLite accessor 타입이 생성되는지 type-check 검증
  - `10_pack_embed` 케이스의 Zod pack/unpack helper가 invalid input을 런타임에서 거부하는지 검증
- **tests/runners/go/run_tests.bat**, **tests/runners/go/run_tests.sh**: Go 생성 코드 통합 테스트
  - `01`-`10` 통합 케이스의 `schema_redis_keys.go`와 단일 `polygen` 패키지 생성물을 함께 `go test ./...`로 검증
  - 고정 `TEST_CASES`의 schema가 누락되면 skip이 아니라 실패로 처리
  - Windows/POSIX runner는 생성 output 디렉터리와 `.go` 파일 존재를 사전 검증하고, `go mod init`/smoke test 복사 실패를 실패로 처리
  - Windows/POSIX runner는 `go mod init`과 `go test` 실패 로그를 케이스 output에 남기고 재실행 없이 출력
  - POSIX runner는 `go` 도구와 release `polygen` 바이너리도 사전 검증하고 `env bash`/`errexit nounset pipefail`로 실행 전제를 명확히 함
  - `tests/runners/go/tests/<case>_test.go`가 있으면 생성 Go 패키지에 복사해 runtime smoke test로 실행
  - `03_nested_namespaces`에서 깊은 namespace table과 sibling table이 `NewSchemaContainer()`에 포함되는지 검증
  - `04_inline_enums`에서 inline enum 상수/컨테이너 인덱스 smoke test 실행
  - `07_indexes`에서 unique/group index와 foreign key validation 성공/실패 경로 검증
  - `10_pack_embed`에서 Go @pack `Pack`, `Unpack<Type>`, `TryUnpack<Type>` roundtrip과 invalid input 거부 검증
- **tests/runners/python/run_tests.bat**, **tests/runners/python/run_tests.sh**: Python 생성 코드 통합 테스트
  - 고정 `TEST_CASES`의 schema가 누락되면 skip이 아니라 실패로 처리
  - Windows/POSIX runner는 생성된 `.py` 파일 존재를 사전 검증하고, POSIX runner는 `python3`/`python` fallback과 release `polygen` 바이너리도 검증
  - Windows/POSIX runner는 generation/validation 실패 로그를 케이스 output에 남기고 실패 시 즉시 출력
  - `01`-`10` 통합 케이스의 dataclass, Pydantic, SQLAlchemy, Redis key helper 산출물을 `py_compile`로 문법 검증
- **tests/runners/messagepack/run_tests.bat**, **tests/runners/messagepack/run_tests.sh**: MessagePack descriptor 통합 테스트
  - 고정 `TEST_CASES`의 schema가 누락되면 skip이 아니라 실패로 처리
  - Windows/POSIX runner는 생성된 `.messagepack.json` 파일 존재를 사전 검증하고, POSIX runner는 `python3`/`python` fallback과 release `polygen` 바이너리도 검증
  - Windows/POSIX runner는 generation/validation 실패 로그를 케이스 output에 남기고 실패 시 즉시 출력
  - `01`-`10` 통합 케이스의 `.messagepack.json` 산출물을 JSON 파싱하고 `format`/`version`/array encoding/type kind/field index/foreignKey shape을 검증
- **tests/runners/sqlite/run_tests.bat**, **tests/runners/sqlite/run_tests.sh**: SQLite DDL 통합 테스트
  - `01`-`10` 통합 케이스의 `schema.sql` 산출물을 Python `sqlite3` in-memory DB에 실행해 문법과 metadata/user table 생성을 검증
  - 고정 `TEST_CASES`의 schema가 누락되면 skip이 아니라 실패로 처리
  - Windows/POSIX runner는 생성된 `schema.sql` 파일 존재를 사전 검증하고, POSIX runner는 `python3`/`python` fallback도 검증
  - Windows/POSIX runner는 generation/validation 실패 로그를 케이스 output에 남기고 실패 시 즉시 출력
  - table/column/FK/index/trigger 구조를 검증하고 `08_complex_schema` 중첩 namespace table/FK/index 및 `09_sqlite` 중첩 datasource 상속 회귀를 확인
- **tests/runners/mermaid/run_tests.bat**, **tests/runners/mermaid/run_tests.sh**: Mermaid ER diagram 통합 테스트
  - `01`-`10` 통합 케이스의 `schema.mmd` 산출물을 파싱해 entity/field/relation 구조를 검증
  - 고정 `TEST_CASES`의 schema가 누락되면 skip이 아니라 실패로 처리
  - Windows/POSIX runner는 생성된 `schema.mmd` 파일 존재를 사전 검증하고, POSIX runner는 `python3`/`python` fallback도 검증
  - Windows/POSIX runner는 generation/validation 실패 로그를 케이스 output에 남기고 실패 시 즉시 출력
  - `08_complex_schema`에서 중첩 namespace entity와 FK relation이 생성되고 embed가 entity로 나오지 않는지 회귀 검증
- **tests/runners/redis/run_tests.bat**, **tests/runners/redis/run_tests.sh**: Redis cache schema 통합 테스트
  - `01`-`10` 통합 케이스와 `tests/test_data/redis_cache_schema.poly`의 `schema.redis.json`/`schema.redis.lua` 산출물을 검증
  - 고정 `TEST_CASES` 또는 Redis cache fixture가 누락되면 skip이 아니라 실패로 처리
  - Windows/POSIX runner는 Redis descriptor/Lua 파일 존재를 사전 검증하고, POSIX runner는 `python3`/`python` fallback과 release `polygen` 바이너리도 검증
  - Windows/POSIX runner는 generation/validation 실패 로그를 케이스 output에 남기고 실패 시 즉시 출력
  - cache table descriptor의 key pattern/index/ttl/cache strategy와 Lua helper 함수 본문이 일치하는지 검증
- **tests/runners/mysql/run_tests.bat**, **tests/runners/mysql/run_tests.sh**: MySQL/MariaDB DDL 통합 테스트
  - `01`-`10` 통합 케이스의 `schema.sql` 산출물을 서버 없이 파싱해 metadata table, datasource 필터링, table/column/FK/index 구조를 검증
  - 고정 `TEST_CASES`의 schema가 누락되면 skip이 아니라 실패로 처리
  - Windows/POSIX runner는 생성된 `schema.sql` 파일 존재를 사전 검증하고, POSIX runner는 `python3`/`python` fallback도 검증
  - Windows/POSIX runner는 generation/validation 실패 로그를 케이스 output에 남기고 실패 시 즉시 출력
  - `08_complex_schema`에서 중첩 namespace table과 FK/index가 생성되는지 회귀 검증
- **tests/runners/postgresql/run_tests.bat**, **tests/runners/postgresql/run_tests.sh**: PostgreSQL DDL 통합 테스트
  - `01`-`10` 통합 케이스의 `schema.sql` 산출물을 서버 없이 파싱해 metadata table, datasource 필터링, table/column/FK/index/trigger 구조를 검증
  - 고정 `TEST_CASES`의 schema가 누락되면 skip이 아니라 실패로 처리
  - Windows/POSIX runner는 생성된 `schema.sql` 파일 존재를 사전 검증하고, POSIX runner는 `python3`/`python` fallback도 검증
  - Windows/POSIX runner는 generation/validation 실패 로그를 케이스 output에 남기고 실패 시 즉시 출력
  - `08_complex_schema`에서 중첩 namespace table과 FK/index가 생성되는지 회귀 검증
- **tests/runners/protobuf/run_tests.bat**, **tests/runners/protobuf/run_tests.sh**: Protobuf 생성 코드 통합 테스트
  - 고정 `TEST_CASES`의 schema가 누락되면 skip이 아니라 실패로 처리
  - Windows/POSIX runner는 생성된 `.proto` 파일 존재를 사전 검증하고, POSIX runner는 `python3`/`python` fallback과 release `polygen` 바이너리도 검증
  - Windows/POSIX runner는 generation/validation 실패 로그를 케이스 output에 남기고 실패 시 즉시 출력
  - `01`-`10` 통합 케이스의 `.proto` 산출물을 파싱해 `syntax = "proto3"`, package, import, enum 첫 값 0, message field 번호/타입을 검증
- **tests/runners/kotlin/run_tests.bat**, **tests/runners/kotlin/run_tests.sh**: Kotlin 생성 코드 통합 테스트
  - 고정 `TEST_CASES`의 schema가 누락되면 skip이 아니라 실패로 처리
  - Windows/POSIX runner는 생성된 `.kt` 파일 존재를 사전 검증하고, POSIX runner는 `python3`/`python` fallback과 release `polygen` 바이너리도 검증
  - Windows/POSIX runner는 generation/validation 실패 로그를 케이스 output에 남기고 실패 시 즉시 출력
  - `01`-`10` 통합 케이스의 `.kt` 산출물을 파싱해 `@Serializable`, data class field 타입/default, enum `value`/`fromValue` helper를 검증
- **tests/runners/swift/run_tests.bat**, **tests/runners/swift/run_tests.sh**: Swift 생성 코드 통합 테스트
  - 고정 `TEST_CASES`의 schema가 누락되면 skip이 아니라 실패로 처리
  - Windows/POSIX runner는 생성된 `.swift` 파일 존재를 사전 검증하고, POSIX runner는 `python3`/`python` fallback과 release `polygen` 바이너리도 검증
  - Windows/POSIX runner는 generation/validation 실패 로그를 케이스 output에 남기고 실패 시 즉시 출력
  - `01`-`10` 통합 케이스의 `.swift` 산출물을 파싱해 Codable struct, SwiftData `@Model` class, Redis helper 구조를 검증
- **tests/runners/unreal/run_tests.bat**, **tests/runners/unreal/run_tests.sh**: Unreal 생성 코드 통합 테스트
  - 고정 `TEST_CASES`의 schema가 누락되면 skip이 아니라 실패로 처리
  - Windows/POSIX runner는 생성된 `.h` 파일 존재를 사전 검증하고, POSIX runner는 `python3`/`python` fallback과 release `polygen` 바이너리도 검증
  - Windows/POSIX runner는 generation/validation 실패 로그를 케이스 output에 남기고 실패 시 즉시 출력
  - `01`-`10` 통합 케이스의 `.h` 산출물을 파싱해 USTRUCT/UENUM, loader 함수, hot reload delegate/load 함수, Redis helper 구조를 검증
- **tests/runners/cpp/run_tests.bat**, **tests/runners/cpp/run_tests.sh**: C++ 생성 코드 통합 테스트
  - 고정 `TEST_CASES`의 테스트 디렉터리, schema 또는 C++ test file이 누락되면 skip이 아니라 실패로 처리
  - 요약은 실제 결과 카운터인 passed/failed만 출력
  - Windows/POSIX runner는 generation 실패와 생성된 `.hpp` 파일 존재를 컴파일 전에 사전 검증
  - Windows/POSIX runner는 `polygen_support.hpp` 복사 실패도 컴파일 전 명시 실패로 처리
  - Windows/POSIX runner는 C++ 컴파일 실패 로그를 숨기지 않고 출력해 원인을 바로 확인할 수 있게 함
  - `compile_test.bat` 보조 경로도 MSVC/g++ fallback, test/generated/header 누락 실패 처리, Redis key helper smoke compile을 공유
  - POSIX runner는 `pipefail`로 `cargo build | sed`와 `polygen | sed`의 앞 명령 실패를 보존
  - CMake 보조 경로와 `run_all_tests.bat`도 `01`-`10` 전체 케이스를 대상으로 하며 test source/generated header 누락 시 skip이 아니라 실패로 처리
  - `schema_redis_keys.hpp`를 smoke translation unit에 include해 helper 헤더 컴파일 검증
  - `03_nested_namespaces`에서 깊은 namespace table에도 binary read/write helper와 container table/index가 생성되는지 검증
  - `09_sqlite`에서 중첩 namespace SQLite table 타입이 메인 헤더에 생성되는지 검증
  - `04_inline_enums`에서 invalid binary enum discriminant read/write가 `runtime_error`로 거부되는지 검증
  - Windows/Linux runner 모두 `09_sqlite` 케이스를 포함해 컴파일/실행 검증
- **tests/runners/csharp/run_tests.bat**, **tests/runners/csharp/run_tests.sh**: C# 생성 코드 통합 테스트
  - 고정 `TEST_CASES`의 테스트 디렉터리, schema 또는 C# test file이 누락되면 skip이 아니라 실패로 처리
  - 요약은 실제 결과 카운터인 passed/failed만 출력
  - Windows/POSIX runner는 generation 실패와 생성된 `.cs` 파일 존재를 컴파일 전에 사전 검증
  - Windows/POSIX runner는 생성 `.cs`, 하위 Common/Data 파일, test `Program.cs` 복사 실패도 즉시 실패로 처리
  - Windows/POSIX runner는 `dotnet build`/`dotnet run` 실패 로그를 숨기지 않고 출력해 원인을 바로 확인할 수 있게 함
  - POSIX runner는 `pipefail`로 `cargo build | sed`와 `polygen | sed`의 앞 명령 실패를 보존
  - `03_nested_namespaces`에서 root `SchemaDataContainer`가 깊은 namespace table/index를 포함하는지 검증
  - `04_inline_enums`에서 invalid binary enum discriminant read/write가 `InvalidDataException`으로 거부되는지 검증
  - `06_arrays_and_optionals`에서 binary optional null roundtrip 검증
  - `07_indexes`에서 C# Container/BinaryRef `@search` ngram/exact postings와 string/number/enum 조회 API 검증
  - `09_sqlite`에서 중첩 namespace table이 상위 SQLite datasource를 상속해 `SqliteDbContext`에 포함되는지 검증
  - `09_sqlite`에서 중첩 namespace table이 `Schema.DataContext.DataContext.Sqlite`에도 포함되는지 검증

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
  - `@load` 어노테이션 테스트

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

- **redis_key_helpers.snap**: Redis key helper 생성 스냅샷
  - `@cache`와 `@datasource("cache")` fixture에 대해 Redis descriptor/Lua 및 언어별 key helper 출력 검증

- **DB auto timestamp DDL smoke test**: 임시 스키마로 MySQL/PostgreSQL/SQLite DDL 생성 결과 검증
  - MySQL/MariaDB: `auto_create`/`auto_update`
  - PostgreSQL/SQLite: `auto_create`와 `auto_update` trigger

### 테스트 데이터
- **sample_input.json**: JSON 테스트 입력
  - JSON → CSV 변환 테스트용

- **expected_output.csv**: 예상 CSV 출력
  - 변환 테스트의 기대 결과

- **csv_test_schema.poly**: CSV 테스트 스키마
  - CSV 관련 기능 테스트용 스키마

### 통합 테스트 Runner
- **runners/**: 언어별 생성 코드와 DB/descriptor 산출물을 검증하는 runner 모음
  - Windows runner는 `.bat`, POSIX runner는 `.sh`를 기준으로 유지
  - `tests/runners/run_all.bat`와 `tests/runners/run_all.sh`는 C#/C++/Rust/TypeScript/Go/DB/descriptor runner 전체 또는 인자로 지정한 일부 runner를 순차 실행하고 실패 runner를 집계
  - `run_all`은 `--list`, `--verify`, `--help`로 지원 runner 목록, runner matrix 동기화 및 verifier 회귀 테스트, 사용법을 출력/검증하며 `--verify`는 Python availability/fallback을 먼저 확인하고 live matrix와 regression test 단계를 분리해 표시
  - `tests/runners/verify_runner_matrix.py`는 `run_all.bat`, `run_all.sh`, 실제 runner 디렉터리 목록이 동기화되어 있는지와 ordered Python availability/fallback/selected-Python live-regression invocation/pre-invocation no-bytecode/런타임 runner 인자/`--list`/`--help`/`--verify` live-regression guard가 유지되는지 검증하며 중복 runner 이름과 `.bat`/`.sh` 한쪽만 있는 runner 디렉터리를 실패로 처리
  - `tests/runners/test_verify_runner_matrix.py`는 임시 runner tree로 matrix verifier의 정상/empty/malformed/중복/invalid list/dir name/누락/extra/한쪽 script/순서 mismatch/Python guard/fallback 누락/순서 오류/selected-Python invocation 누락/no-bytecode guard 누락/순서 오류/runtime guard 누락/`--list` guard 누락/`--help` guard 누락/`--verify` 단계 누락과 Windows `--list`/`--help`/`py -3` fallback/no-Python failure/live-failure short-circuit/default/default-validation/subset/failure/unknown/invalid/metachar runner 실행 회귀를 검증
  - 생성 실패, 필수 산출물 누락, 컴파일/런타임 실패는 skip이 아니라 실패로 집계
  - 실패 로그는 각 runner output 아래에 남기고 실패 시 즉시 출력

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
5. **어노테이션**: @taggable, @load, @datasource, @cache
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
4. 필요한 경우 `tests/runners/<lang>/tests/` 또는 validator runner에 통합 테스트 추가

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
- C# runner는 생성된 테스트 프로젝트를 `dotnet build`, `dotnet run`으로 실행
- 생성된 코드의 컴파일과 runtime smoke test를 함께 검증

### 테스트 커버리지
- `snapshot_tests.rs`는 AST/IR 스냅샷과 언어/DB/descriptor 생성 smoke 테스트를 함께 포함
- 핵심 Rust 모듈은 파서, 검증, IR, 템플릿, 코드 생성, CLI 보조 함수 단위 테스트를 보유
- `tests/runners/`는 공유 통합 schema 01-10을 기준으로 생성 코드 컴파일/실행과 DB/descriptor 산출물 검증을 담당

### 주의사항
- **스냅샷 크기**: 스냅샷이 너무 크면 가독성 저하
- **테스트 격리**: 각 테스트는 독립적이어야 함
- **실패한 스냅샷**: 원인을 명확히 파악 후 업데이트
- **삭제된 기능**: 해당 스냅샷도 함께 삭제
