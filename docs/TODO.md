# PolyGen TODO

> 최종 업데이트: 2026-05-23

---

## 현재 진행 상황

### ✅ 완료된 작업

#### 문서화 (2026-01-28)
- [x] `CUSTOMIZATION.md` - Rhai 템플릿 커스터마이징 가이드 (5개 언어)
- [x] `SOURCE_STRUCTURE.md` - 소스 코드 구조 문서화
- [x] `CLAUDE.md` 진입점 및 문서 동기화 규칙 추가

#### 코드 품질 개선 (2026-01-28)
- [x] Clippy 경고 수정: `strip_prefix()` 사용 (ast_parser/fields.rs)
- [x] Clippy 경고 수정: `Box<FieldDef>` 적용으로 enum 크기 최적화 (ir_model.rs)
- [x] Clippy 경고 수정: doc comment 형식 수정 (ir_builder.rs)
- [x] 위험한 `unwrap()`/`expect()` 10곳 → `unwrap_or_else()` 패턴으로 변경
- [x] `is_readonly()` 함수 간소화 (중복 로직 제거)
- [x] `rhai_generator.rs` 모듈 문서화 추가
- [x] `codegen.rs` 사용 예제 추가

#### 코어 리팩토링 (Phase 1-4)
- [x] Clippy 경고 수정, 패키지명 정리
- [x] 에러 처리 개선 (CodeGenError)
- [x] AST Parser 모듈화 (8개 하위 모듈)
- [x] CodeGenerator, CompilationPipeline 아키텍처
- [x] TypeRegistry 중앙화
- [x] 언어별 설정 파일 (`<lang>.toml`)
- [x] 언어 자동 탐색 결과 정렬로 다중 언어 생성 순서 결정성 보장
- [x] 생성 결과 출력 순회 중 파일시스템 엔트리 오류 전파
- [x] 코드 문서화 (doc comments)

> 상세 내용: [archive/REFACTORING_TODO.md](archive/REFACTORING_TODO.md), [archive/PHASE4_TODO.md](archive/PHASE4_TODO.md)

#### 언어 지원 (Phase 5)
- [x] C# - 클래스, Enum, CSV/JSON/Binary 로더, Container, Validation
- [x] C++ - 헤더 전용, 구조체, Enum, CSV/JSON/Binary 로더, Container, Validation, Redis key helper
- [x] Rust - 모듈, Struct, Enum, CSV/Binary 로더, Container, Validation
- [x] TypeScript - 인터페이스, Enum, Zod 스키마, Validation
- [x] Go - Struct, Enum, CSV/JSON/Binary 로더, Container, Validation, Redis key helper
- [x] Unreal Engine - USTRUCT/UENUM 매크로, CSV/JSON 로더, Hot Reload, Redis key helper
- [x] 통합 테스트 프레임워크 (10개 테스트 케이스)
- [x] C#/C++/Rust 통합 runner 필수 테스트 디렉터리/파일 누락 시 실패 처리 (Windows/POSIX)
- [x] 통합 runner 실패 전환 후 잔여 `Skipped` 요약/카운터 제거
- [x] TypeScript 통합 테스트 런타임 assertion 실행 (`tsx`, `console.assert` 실패 시 exit 1)
- [x] TypeScript Zod 중첩 namespace schema 회귀 테스트
- [x] Go 통합 테스트 범위 보강 (`01`-`10` 전체 `go test ./...`, inline enum/index/FK smoke test)
- [x] Go 통합 runner 고정 테스트 케이스 누락 시 실패 처리 (Windows/POSIX)
- [x] TypeScript/Python/Kotlin/Swift/Unreal/Protobuf/MessagePack 통합 runner 고정 테스트 케이스 누락 시 실패 처리 (Windows/POSIX)
- [x] Go container 중첩 namespace table 생성 회귀 테스트
- [x] Python/Kotlin/Swift/Protobuf/MessagePack 기본 생성 smoke 테스트
- [x] Python 통합 runner 추가 (`01`-`10` 전체 dataclass/Pydantic/SQLAlchemy/Redis helper `py_compile`)
- [x] MessagePack 통합 runner 추가 (`01`-`10` 전체 descriptor JSON 구조 검증)
- [x] SQLite DDL 통합 runner 범위 확대 (`01`-`10` 전체 SQL 실행/구조 검증, 중첩 namespace DDL 회귀 검증)
- [x] Mermaid 통합 runner 추가 (`01`-`10` 전체 ER diagram 구조 검증, 중첩 namespace 회귀 검증)
- [x] Redis 통합 runner 추가 (`01`-`10` + cache fixture descriptor/Lua helper 구조 검증)
- [x] MySQL/MariaDB DDL 통합 runner 추가 (`01`-`10` 전체 SQL 구조 검증, 중첩 namespace DDL 회귀 검증)
- [x] PostgreSQL DDL 통합 runner 추가 (`01`-`10` 전체 SQL 구조 검증, 중첩 namespace DDL 회귀 검증)
- [x] DDL/descriptor 통합 runner 고정 테스트 케이스 누락 시 실패 처리 (Windows/POSIX)
- [x] Protobuf 통합 runner 추가 (`01`-`10` 전체 proto3 구조 검증)
- [x] Swift 통합 runner 추가 (`01`-`10` 전체 Codable/SwiftData/Redis helper 구조 검증)
- [x] Unreal 통합 runner 추가 (`01`-`10` 전체 USTRUCT/UENUM/loader/hot reload/Redis helper 구조 검증)
- [x] 전체 통합 runner 진입점 추가 (`tests/runners/run_all.bat`, `tests/runners/run_all.sh`, 지정 runner subset 실행 지원)
- [x] 전체 통합 runner matrix 검증 추가 (`run_all --verify`, ordered Python availability/fallback/selected-Python live-regression invocation/pre-invocation no-bytecode/runtime runner-argument/`--list`/`--help`/live-regression `--verify` guard check, empty/malformed/invalid runner list/dir 이름, 중복/누락/extra/한쪽 script/목록 mismatch 및 Windows `--list`/`--help`/`py -3` fallback/no-Python failure/live-failure short-circuit/default/default-validation/subset/failure/unknown/invalid/metachar runner 회귀 테스트)

#### 데이터 무결성 검증 시스템
- [x] ValidationResult, ValidationError, ValidationException 인프라
- [x] 필드 제약조건 검증 (max_length, range, regex)
- [x] 스키마 단계 필드 제약조건 의미 검증 (max_length 타입/값, range 타입/bound, regex 패턴)
- [x] 스키마 단계 primary_key/unique 의미 검증 (중복, cardinality, indexable type)
- [x] 스키마 단계 field-level index 의미 검증 (deprecated constraint, indexable type)
- [x] 스키마 단계 default 제약조건 의미 검증 (타입 호환성, 정수 범위, range 연동)
- [x] 스키마 단계 foreign_key 의미 검증 (대상 table/PK/unique field 존재, 타입 호환성)
- [x] Default 값 지원 (모든 언어)
- [x] 컨테이너 레벨 ValidateAll(), ValidateOrThrow()
- [x] Foreign Key 검증 (ValidateForeignKeys) - C#, C++, Rust, Go

#### SQLite 지원 (Phase 1-4)
- [x] DDL 생성 (CREATE TABLE, INDEX)
- [x] DDL auto timestamp 생성 (MySQL/MariaDB auto_create/auto_update, PostgreSQL/SQLite auto_create/auto_update trigger)
- [x] `.renames` 파일 문법
- [x] 마이그레이션 SQL 생성 (ALTER TABLE RENAME)
- [x] 네임스페이스 접두사 처리
- [x] @datasource 기반 자동 DDL 생성 연동
- [x] 마이그레이션 diff 로직 (`--baseline` 옵션)
- [x] CLI 명령어 (`polygen generate`, `polygen migrate`)

> 상세 내용: [SQL_TODO.md](SQL_TODO.md)

#### 고급 어노테이션
- [x] `@load` 대상/인자 의미 검증 (table 전용, csv/json named string)
- [x] `@taggable` 대상/인자 의미 검증 (table 전용, no args)
- [x] `@link_rows` 대상/인자 의미 검증 (table 전용, positional target 1개)
- [x] `@cache` 어노테이션 (full_load, on_demand, write_through, write_back)
- [x] `@cache` strategy/ttl 의미 검증 (알 수 없는 전략, 음수 TTL, 중복 파라미터 차단)
- [x] `@datasource` 대상/값 의미 검증 (namespace/table 전용, 지원 datasource 제한)
- [x] `@index` 대상/필드 의미 검증 (table 전용, 같은 table의 indexable regular field)
- [x] `@pack` 대상/구분자 의미 검증 (embed 전용, 한 글자 separator)
- [x] `@readonly` 어노테이션
- [x] `@readonly` 대상/인자 의미 검증 (table 전용, no args)
- [x] `@soft_delete` 어노테이션
- [x] `@soft_delete` 대상/필드 의미 검증 (table 전용, 같은 table의 timestamp? field)
- [x] timestamp 타입 및 auto_create/auto_update 제약조건 (파싱/AST/IR/검증, 타겟별 생성 부분 지원)
- [x] C# auto_update helper 생성 (`OnUpdate<Field>`, `OnUpdateAll`)
- [x] C# auto_create timezone 생성 보강 (`local`, UTC offset minute, named timezone)
- [x] C# binary bytes cardinality 처리 보강 (`bytes?`, `bytes[]`)
- [x] C# binary optional value-type null 보존 (`i32?`, `bool?`, `DateTime?`, enum? 등)
- [x] C# binary enum cardinality smoke coverage (`Enum?`, `Enum[]`)
- [x] C# binary enum 직렬화/역직렬화 안전화 (`Enum.IsDefined`, invalid discriminant 에러 처리)
- [x] C# binary reader 생성물의 오해성 TODO 주석 제거
- [x] C# binary mapping 비지원 타입 placeholder 제거 (`NotSupportedException` 생성)
- [x] C# Container 중첩 namespace table/index 회귀 테스트
- [x] C# DataContext 중첩 namespace datasource table 회귀 테스트
- [x] C++ binary enum cardinality 처리 보강 (`Enum?`, `Enum[]`)
- [x] C++ binary enum 직렬화/역직렬화 안전화 (`enum_validator`, invalid discriminant 에러 처리)
- [x] C++ loader 중첩 namespace table 수집 보강 및 Linux runner `09_sqlite` parity 보정
- [x] C++ Container 중첩 namespace indexed table 수집 보강
- [x] Rust enum 역직렬화 안전화 (`TryFrom<i32>`, invalid discriminant 에러 처리)
- [x] Rust CSV primitive 배열 파싱 오류 처리 (`filter_map` 침묵 삭제 제거)
- [x] Rust CSV optional primitive 파싱 오류 처리 (invalid 값을 `None`으로 침묵 변환하지 않음)
- [x] Rust CSV optional enum 파싱 오류 처리 (non-integer 값을 `None`으로 침묵 변환하지 않음)
- [x] Rust CSV bool 배열 파싱 일관화 (`true/false`, `yes/no`, `1/0`)
- [x] Rust loader 중첩 namespace table 수집 보강 및 Linux runner `09_sqlite` parity 보정
- [x] Rust Container 중첩 namespace table/relation 수집 보강
- [x] C#/C++/Rust/TypeScript SQLite accessor 중첩 namespace table 수집 및 `@datasource("sqlite")` 상속 보강
- [x] TypeScript @pack unpack 파싱 검증 보강 (`NaN`, invalid bool, negative unsigned 차단)
- [x] Go @pack 메서드 생성 (`Pack`, `Unpack<Type>`, `TryUnpack<Type>`, finite float/unsigned 검증)
- [x] Unreal loader/hot reload 중첩 namespace/inline embed 수집 보강
- [x] SQLite DDL 중첩 namespace table 수집 및 foreign key 생성 보강
- [x] MySQL/MariaDB DDL 중첩 namespace table 수집 보강
- [x] PostgreSQL DDL 중첩 namespace table 수집 보강
- [x] Mermaid ER diagram 중첩 namespace entity/relation 수집 보강

#### GUI / UX
- [x] PolyGen GUI 도구 (Tauri + Svelte)
- [x] 경로 설정: 스키마 파일, 템플릿 경로, 출력 경로
- [x] 언어/DB 옵션 체크박스 선택
- [x] 내장 `.poly` 에디터 (Monaco Editor, 신택스 하이라이팅)
- [x] 스키마 검증 및 에러 표시 (실시간)
- [x] 스키마 시각화 뷰 (테이블 관계, referencedBy/references)
- [x] Mermaid ER 다이어그램 생성 (`polygen visualize`, `--lang mermaid`)
- [x] 템플릿 에디터 + 실시간 생성 프리뷰
- [x] 템플릿 파일 트리, Rhai/PTPL 편집, 컨텍스트 레퍼런스 패널
- [x] 새 언어 생성 마법사 (`templates/<lang>/` + `<lang>.toml` 생성)
- [x] 최근 프로젝트 저장/불러오기
- [x] 마이그레이션 baseline/SQLite DB 경로 지정
- [x] Svelte 접근성 경고 정리 (label/ARIA)

#### LSP 지원
- [x] Go to Definition, Find References
- [x] Hover, Document Symbols, Rename
- [x] Completion (자동완성)
- [x] Cross-file Go to Definition URI 반환 보정

#### IDE 연동
- [x] VS Code 확장 기본 구현 (`polygen-vscode`)
- [x] `.poly` TextMate 신택스 하이라이팅
- [x] LSP 연결 설정 (`polygen.lsp.path`, `polygen.lsp.enabled`)

---

## ❌ 미완료 (우선순위순)

### 🔴 높은 우선순위

#### 스키마 관리 테이블
- [x] `_polygen_schema` 테이블 자동 생성
- [x] DDL/마이그레이션 SQL에 `schema_hash`, `schema_json` upsert 생성
- [x] `migrate --db`에서 DB 저장 hash와 현재 IR hash 비교 출력
- [x] hash 불일치 정책 옵션 추가 (`warn`, `fail`, `force`)
- [x] 파괴적 변경 정책 옵션 추가 (`warn`, `fail`, `allow`)
- [x] 마이그레이션/DDL 실행 시 `_polygen_migrations` 버전 기록
- [x] GUI에서 hash 정책 설명/상태 표시 강화

#### GUI/LSP 안정화
- [x] 템플릿 파일 이름 변경 기능
- [x] 템플릿 프리뷰 에러 위치와 원본 `.ptpl` 라인 연결 강화
- [x] VS Code 확장 패키징/배포 절차 검증
- [x] LSP cross-file reference/rename 통합 테스트 확대

### 🟡 중간 우선순위

#### Watch 모드 (Hot Reload)
- [x] `polygen watch --schema game.poly --lang csharp`
- [x] 스키마 변경 감지 → 자동 재생성
- [x] 파일 시스템 감시 (notify crate)

#### GUI 시각화 개선 (우선)
- [x] 테이블 상세 정보 패널 확장
- [x] 필드 제약조건 시각적 표시 개선
- [x] 관계 그래프 인터랙션 강화

#### 스키마 문서 생성 (낮은 우선순위)
- [x] `polygen docs --output docs/schema.md` - CI/CD용
- [x] GUI 시각화로 대체 가능, 필요시 구현

#### ~~Mock 데이터 생성~~ (삭제)
- AI가 더 현실적인 데이터 생성 가능
- 복잡한 분포/관계 표현에 한계

#### GUI 추가 기능
- [x] 프로젝트별 프리셋 저장/불러오기
- [x] 스키마 비교 뷰 (baseline vs 현재)
- [x] CLI 명령어 표시 (복사용)

### 🟢 낮은 우선순위

#### 추가 DB 지원
- [x] MySQL/MariaDB 지원 (DDL 생성, @datasource 자동 연동)
- [x] PostgreSQL 지원 (DDL 생성, @datasource 자동 연동)
- [x] Redis 캐시 지원 (key schema descriptor, ttlSeconds, Lua/C#/C++/Rust/TypeScript/Go/Python/Kotlin/Swift/Unreal key helper, @datasource("redis"/"cache") 자동 연동)

#### 추가 언어 지원
- [x] Python 지원 (dataclass, Pydantic, SQLAlchemy, Redis key helper)
- [x] Kotlin 지원 (data class, kotlinx.serialization, Redis key helper)
  - [x] Kotlin 생성물 통합 runner 추가 (`01`-`10` 케이스 구조 검증)
- [x] Swift 지원 (Codable struct, SwiftData, Redis key helper)
  - [x] Swift 생성물 통합 runner 추가 (`01`-`10` 케이스 구조 검증)

#### 추가 직렬화 포맷
- [x] Protocol Buffers 지원 (proto3 `.proto` 파일)
- [x] MessagePack 지원 (array encoding schema descriptor)

#### 스키마 분석
- [x] `polygen diff --old v1.poly --new v2.poly` (text/json 출력)
- [x] 순환 참조 감지 및 경고 (`polygen lint`, generate 파이프라인 경고)
- [x] 미사용 import 경고 (`polygen lint`, generate 파이프라인 경고)
- [x] `polygen stats` (테이블/필드/Enum 통계, text/json 출력)

---

## 아키텍처 결정

| 항목 | 결정 |
|------|------|
| SQL 지원 방식 | @datasource 기반 자동 생성 |
| Rename 지원 | `.renames` 파일 방식 |
| SQLite 최소 버전 | 3.25.0 (RENAME COLUMN 지원) |
| 타입 매핑 통일 | 문제 발생 시 C# Rust 헬퍼 → Rhai 템플릿으로 이관 |

### 타입 매핑 현황

현재 C#만 Rust 헬퍼(`src/rhai/csharp/type_mapping.rs`)로 구현되어 있고,
다른 언어(C++, Rust, TypeScript, Go, Unreal, SQLite)는 Rhai 템플릿으로 구현됨.

```
C#:     src/rhai/csharp/type_mapping.rs                  (Rust)
C++:    templates/cpp/rhai_utils/type_mapping.rhai        (Rhai)
Rust:   templates/rust/rhai_utils/type_mapping.rhai       (Rhai)
TS:     templates/typescript/rhai_utils/type_mapping.rhai  (Rhai)
Go:     templates/go/rhai_utils/type_mapping.rhai         (Rhai)
Python: templates/python/rhai_utils/type_mapping.rhai     (Rhai)
Kotlin: templates/kotlin/rhai_utils/type_mapping.rhai     (Rhai)
Swift:  templates/swift/rhai_utils/type_mapping.rhai      (Rhai)
Unreal: templates/unreal/rhai_utils/type_mapping.rhai     (Rhai)
SQLite: templates/sqlite/rhai_utils/type_mapping.rhai     (Rhai)
```

Python/Kotlin/Swift의 FQN 클래스명 변환은 Rhai `split` 순회 대신 `replace(".", "_")`
기반으로 처리해 복잡 스키마에서 `%logic` 렌더링 stack overflow를 피합니다.

**향후 계획**: 일관성 문제나 유지보수 이슈 발생 시 C#도 Rhai로 통일

---

## 참고 문서

| 문서 | 설명 |
|------|------|
| `CUSTOMIZATION.md` | Rhai 템플릿 커스터마이징 가이드 |
| `SOURCE_STRUCTURE.md` | 소스 코드 구조 |
| `archive/REFACTORING_TODO.md` | 코어 리팩토링 Phase 1-5 (완료) |
| `archive/PHASE4_TODO.md` | 성능 & 확장성 개선 (완료) |
| `SQL_TODO.md` | SQL/DB 지원 상세 |
| `CLAUDE.md` | 프로젝트 가이드 |

---

## 우선순위 매트릭스

| 우선순위 | 작업 | 난이도 | 가치 |
|---------|------|--------|------|
| 🔴 높음 | GUI/LSP 안정화 | 중 | 제품 완성도 |
| 🟡 중간 | GUI 시각화 개선 | 중 | UX 향상 |
| 🟡 중간 | Watch 모드 | 중 | DX 향상 |
| 🟡 중간 | VS Code 확장 배포 품질 | 중 | DX 향상 |
| ✅ 완료 | Python 지원 | 중 | 사용자층 확대 |
| ✅ 완료 | MySQL/MariaDB 지원 | 중 | DB 확장 |
| ✅ 완료 | Protocol Buffers | 중 | 직렬화 확장 |
| ✅ 완료 | MessagePack | 중 | 직렬화 확장 |

---

*최종 업데이트: 2026-05-23*
