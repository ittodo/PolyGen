# PolyGen TODO

> 최종 업데이트: 2026-01-22

---

## 현재 진행 상황

### ✅ 완료된 작업

#### 코어 리팩토링 (Phase 1-4)
- [x] Clippy 경고 수정, 패키지명 정리
- [x] 에러 처리 개선 (CodeGenError)
- [x] AST Parser 모듈화 (8개 하위 모듈)
- [x] CodeGenerator, CompilationPipeline 아키텍처
- [x] TypeRegistry 중앙화
- [x] 언어별 설정 파일 (`<lang>.toml`)
- [x] 코드 문서화 (doc comments)

#### 언어 지원 (Phase 5)
- [x] C# - 클래스, Enum, CSV/JSON/Binary 로더, Container
- [x] C++ - 헤더 전용, 구조체, Enum, CSV/JSON/Binary 로더, Container
- [x] Rust - 모듈, Struct, Enum, CSV/Binary 로더
- [x] TypeScript - 인터페이스, Enum, Zod 스키마
- [x] 통합 테스트 프레임워크 (8개 테스트 케이스)

#### SQLite 지원 (Phase 1-3)
- [x] DDL 생성 (CREATE TABLE, INDEX)
- [x] `.renames` 파일 문법
- [x] 마이그레이션 SQL 생성 (ALTER TABLE RENAME)
- [x] 네임스페이스 접두사 처리
- [x] @datasource 기반 자동 DDL 생성 연동
  - 모든 언어(C#, C++, Rust, TypeScript)에서 자동 DDL 생성
  - datasource별 테이블 필터링 (sqlite/mysql)

---

## ❌ 미완료 (우선순위순)

### SQLite/DB 지원 (Phase 4)
- [ ] 마이그레이션 diff 로직 (자동 감지)
- [ ] CLI 명령어 (`polygen migrate`)
- [x] 언어별 DB accessor 코드 생성 (C#, Rust 완료)

### 추가 DB 지원
- [ ] MySQL/MariaDB 지원 (SQLite 기반 확장)
- [ ] PostgreSQL (옵션)
- [ ] Redis 캐시 (옵션)

### 고급 기능
- [ ] `@cache`, `@readonly`, `@soft_delete` 어노테이션
- [ ] 자동 타임스탬프 (`auto_create`, `auto_update`)
- [ ] 쿼리/뷰 지원 (검토 필요)

---

## 아키텍처 결정

| 항목 | 결정 |
|------|------|
| SQL 지원 방식 | @datasource 기반 자동 생성 (B안) |
| Rename 지원 | `.renames` 파일 방식 |
| SQLite 최소 버전 | 3.25.0 (RENAME COLUMN 지원) |

---

## 참고 문서

| 문서 | 설명 |
|------|------|
| `REFACTORING_TODO.md` | ✅ 완료 - 코어 리팩토링 Phase 1-5 |
| `PHASE4_TODO.md` | ✅ 완료 - 성능 & 확장성 개선 |
| `SQL_TODO.md` | 🚧 진행 중 - SQL/DB 지원 상세 |
| `CLAUDE.md` | 프로젝트 가이드 |

---

*최종 업데이트: 2026-01-22*
