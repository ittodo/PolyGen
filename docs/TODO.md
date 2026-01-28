# PolyGen TODO

> 최종 업데이트: 2026-01-28

---

## 현재 진행 상황

### ✅ 완료된 작업

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
- [x] 코드 문서화 (doc comments)

> 상세 내용: [archive/REFACTORING_TODO.md](archive/REFACTORING_TODO.md), [archive/PHASE4_TODO.md](archive/PHASE4_TODO.md)

#### 언어 지원 (Phase 5)
- [x] C# - 클래스, Enum, CSV/JSON/Binary 로더, Container, Validation
- [x] C++ - 헤더 전용, 구조체, Enum, CSV/JSON/Binary 로더, Container, Validation
- [x] Rust - 모듈, Struct, Enum, CSV/Binary 로더, Container, Validation
- [x] TypeScript - 인터페이스, Enum, Zod 스키마, Validation
- [x] Go - Struct, Enum, CSV/JSON/Binary 로더, Container, Validation
- [x] 통합 테스트 프레임워크 (8개 테스트 케이스)

#### 데이터 무결성 검증 시스템
- [x] ValidationResult, ValidationError, ValidationException 인프라
- [x] 필드 제약조건 검증 (max_length, range, regex)
- [x] Default 값 지원 (모든 언어)
- [x] 컨테이너 레벨 ValidateAll(), ValidateOrThrow()
- [x] Foreign Key 검증 (ValidateForeignKeys) - C#, C++, Rust, Go

#### SQLite 지원 (Phase 1-4)
- [x] DDL 생성 (CREATE TABLE, INDEX)
- [x] `.renames` 파일 문법
- [x] 마이그레이션 SQL 생성 (ALTER TABLE RENAME)
- [x] 네임스페이스 접두사 처리
- [x] @datasource 기반 자동 DDL 생성 연동
- [x] 마이그레이션 diff 로직 (`--baseline` 옵션)
- [x] CLI 명령어 (`polygen generate`, `polygen migrate`)

> 상세 내용: [SQL_TODO.md](SQL_TODO.md)

#### 고급 어노테이션
- [x] `@cache` 어노테이션 (full_load, on_demand, write_through)
- [x] `@readonly` 어노테이션
- [x] `@soft_delete` 어노테이션
- [x] timestamp 타입 및 auto_create/auto_update 제약조건

#### GUI / UX
- [x] PolyGen GUI 도구 (Tauri + Svelte)
- [x] 경로 설정: 스키마 파일, 템플릿 경로, 출력 경로
- [x] 언어/DB 옵션 체크박스 선택
- [x] 내장 `.poly` 에디터 (Monaco Editor, 신택스 하이라이팅)
- [x] 스키마 검증 및 에러 표시 (실시간)
- [x] 스키마 시각화 뷰 (테이블 관계, referencedBy/references)
- [x] Mermaid ER 다이어그램 생성

#### LSP 지원
- [x] Go to Definition, Find References
- [x] Hover, Document Symbols, Rename
- [x] Completion (자동완성)

---

## ❌ 미완료 (우선순위순)

### 🔴 높은 우선순위

#### GUI 개선
- [ ] DB 파일 경로 지정 (SQLite: .db 파일 위치)
- [ ] 마이그레이션 baseline 파일 지정

#### 스키마 관리 테이블
- [ ] `_polygen_schema` 테이블 자동 생성
- [ ] 마이그레이션/DDL 실행 시 버전 기록
- [ ] 스키마 해시로 변경 감지

### 🟡 중간 우선순위

#### IDE 연동
- [ ] VS Code 확장 (.poly 신택스 하이라이팅)

#### Watch 모드 (Hot Reload)
- [ ] `polygen watch --schema game.poly --lang csharp`
- [ ] 스키마 변경 감지 → 자동 재생성
- [ ] 파일 시스템 감시 (notify crate)

#### GUI 시각화 개선 (우선)
- [ ] 테이블 상세 정보 패널 확장
- [ ] 필드 제약조건 시각적 표시 개선
- [ ] 관계 그래프 인터랙션 강화

#### 스키마 문서 생성 (낮은 우선순위)
- [ ] `polygen docs --output docs/schema.md` - CI/CD용
- [ ] GUI 시각화로 대체 가능, 필요시 구현

#### ~~Mock 데이터 생성~~ (삭제)
- AI가 더 현실적인 데이터 생성 가능
- 복잡한 분포/관계 표현에 한계

#### GUI 추가 기능
- [ ] 프로젝트별 프리셋 저장/불러오기
- [ ] 최근 사용 경로 기억
- [ ] 스키마 비교 뷰 (baseline vs 현재)
- [ ] CLI 명령어 표시 (복사용)

### 🟢 낮은 우선순위

#### 추가 DB 지원
- [ ] MySQL/MariaDB 지원 (SQLite 기반 확장)
- [ ] PostgreSQL (옵션)
- [ ] Redis 캐시 (옵션)

#### 추가 언어 지원
- [ ] Python (dataclass, Pydantic, SQLAlchemy)
- [ ] Kotlin (data class, kotlinx.serialization)
- [ ] Swift (Codable struct, SwiftData)

#### 추가 직렬화 포맷
- [ ] Protocol Buffers (.proto 파일)
- [ ] MessagePack

#### 스키마 분석
- [ ] `polygen diff --old v1.poly --new v2.poly`
- [ ] 순환 참조 감지 및 경고
- [ ] 미사용 import 경고
- [ ] `polygen stats` (테이블/필드/Enum 통계)

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
다른 언어(C++, Rust, TypeScript, Go)는 Rhai 템플릿으로 구현됨.

```
C#:    src/rhai/csharp/type_mapping.rs       (Rust)
C++:   templates/cpp/rhai_utils/type_mapping.rhai    (Rhai)
Rust:  templates/rust/rhai_utils/type_mapping.rhai   (Rhai)
TS:    templates/typescript/rhai_utils/type_mapping.rhai (Rhai)
Go:    templates/go/rhai_utils/type_mapping.rhai     (Rhai)
```

**향후 계획**: 일관성 문제나 유지보수 이슈 발생 시 C#도 Rhai로 통일

---

## 참고 문서

| 문서 | 설명 |
|------|------|
| `archive/REFACTORING_TODO.md` | 코어 리팩토링 Phase 1-5 (완료) |
| `archive/PHASE4_TODO.md` | 성능 & 확장성 개선 (완료) |
| `SQL_TODO.md` | SQL/DB 지원 상세 |
| `CLAUDE.md` | 프로젝트 가이드 |

---

## 우선순위 매트릭스

| 우선순위 | 작업 | 난이도 | 가치 |
|---------|------|--------|------|
| 🔴 높음 | DB 파일 경로 UI | 하 | UX 개선 |
| 🟡 중간 | Watch 모드 | 중 | DX 향상 |
| 🟡 중간 | Mock 데이터 생성 | 중 | 테스트 편의 |
| 🟡 중간 | VS Code 확장 | 중 | DX 향상 |
| 🟢 낮음 | Python 지원 | 중 | 사용자층 확대 |
| 🟢 낮음 | MySQL 지원 | 중 | DB 확장 |
| 🟢 낮음 | Protocol Buffers | 중 | 직렬화 확장 |

---

*최종 업데이트: 2026-01-26*
