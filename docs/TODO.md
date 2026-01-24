# PolyGen TODO

> 최종 업데이트: 2026-01-24

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

#### SQLite 지원 (Phase 1-3)
- [x] DDL 생성 (CREATE TABLE, INDEX)
- [x] `.renames` 파일 문법
- [x] 마이그레이션 SQL 생성 (ALTER TABLE RENAME)
- [x] 네임스페이스 접두사 처리
- [x] @datasource 기반 자동 DDL 생성 연동
  - 모든 언어(C#, C++, Rust, TypeScript)에서 자동 DDL 생성
  - datasource별 테이블 필터링 (sqlite/mysql)

#### CLI 명령어 & 고급 어노테이션 (Phase 4)
- [x] CLI 서브커맨드 구조 (`polygen generate`, `polygen migrate`)
- [x] 마이그레이션 전용 CLI (`polygen migrate --baseline <old> --schema-path <new>`)
- [x] `@cache` 어노테이션 (full_load, on_demand, write_through)
- [x] `@readonly` 어노테이션
- [x] `@soft_delete` 어노테이션

---

## ❌ 미완료 (우선순위순)

### SQLite/DB 지원 (Phase 4) ✅ 완료
- [x] 마이그레이션 diff 로직 (`--baseline` 옵션으로 스키마 비교)
- [x] CLI 명령어 (`polygen migrate`) - 서브커맨드 구조 도입
- [x] 언어별 DB accessor 코드 생성 (C#, Rust, C++, TypeScript 완료)

### 고급 어노테이션 ✅ 완료
- [x] `@cache` 어노테이션 (full_load, on_demand, write_through 전략)
- [x] `@readonly` 어노테이션 (읽기 전용 테이블)
- [x] `@soft_delete` 어노테이션 (소프트 삭제 필드 지정)

### 추가 DB 지원
- [ ] MySQL/MariaDB 지원 (SQLite 기반 확장)
- [ ] PostgreSQL (옵션)
- [ ] Redis 캐시 (옵션)

### 남은 고급 기능
- [ ] 자동 타임스탬프 (`auto_create`, `auto_update`)
- [ ] 쿼리/뷰 지원 (검토 필요)
- [ ] 비동기 캐시 전략 (`on_demand_async`) - 언어별 패턴 복잡, 필요시 추가

### GUI / UX 개선
- [x] PolyGen GUI 도구 (Tauri + Svelte)
  - [x] 경로 설정: 스키마 파일, 템플릿 경로, 출력 경로
  - [x] 언어/DB 옵션 체크박스 선택
  - [x] 내장 `.poly` 에디터 (Monaco Editor, 신택스 하이라이팅)
  - [x] 스키마 검증 및 에러 표시 (실시간)
  - [x] 스키마 시각화 뷰 (테이블 관계, referencedBy/references)
  - [x] Go to Definition, Find References, Hover, Rename
  - [ ] DB 파일 경로 지정 (SQLite: .db 파일 위치)
  - [ ] 마이그레이션 baseline 파일 지정
  - [ ] 프로젝트별 프리셋 저장/불러오기
  - [ ] 최근 사용 경로 기억
  - [ ] 스키마 비교 뷰 (baseline vs 현재)
  - [ ] CLI 명령어 표시 (복사용)

### 스키마 관리 테이블
- [ ] `_polygen_schema` 테이블 자동 생성
  - 마이그레이션/DDL 실행 시 버전 기록
  - 스키마 해시로 변경 감지
  - 중복 마이그레이션 방지
  - import된 여러 파일 통합 해시 처리

### IDE 연동
- [ ] VS Code 확장 (.poly 신택스 하이라이팅)
- [x] LSP (Language Server Protocol) 지원
  - Go to Definition, Find References
  - Hover, Document Symbols, Rename
  - Completion (자동완성)

### 데이터 검증 확장
- [ ] Foreign Key 검증 (ValidateForeignKeys)
  - 컨테이너 참조로 FK 존재 여부 확인
  - SQLite: LEFT JOIN 쿼리로 orphan 탐지

---

## 🆕 신규 기능 제안

### 개발자 경험 (DX)

#### Watch 모드 (Hot Reload)
- [ ] `polygen watch --schema game.poly --lang csharp`
- [ ] 스키마 변경 감지 → 자동 재생성
- [ ] 파일 시스템 감시 (notify crate)

#### 스키마 Diff 도구
- [ ] `polygen diff --old v1.poly --new v2.poly`
- [ ] 두 스키마 비교, 변경사항 시각화
- [ ] 마이그레이션 SQL 자동 생성

### 문서화 & 시각화

#### 스키마 시각화 ⭐ ✅ 완료
- [x] `polygen visualize --format mermaid`
- [x] Mermaid ER 다이어그램 생성
- [x] JSON 출력 (GUI용)
- [x] 테이블 관계, FK 시각화 (referencedBy, references)
- [x] README/문서에 삽입 가능
- [x] GUI 통합 (Tauri + Svelte)
  - Detail View: 3열 레이아웃 (좌측 referencedBy / 중앙 테이블 상세 / 우측 references)
  - Diagram View: 모든 테이블을 박스로 표시, 화살표로 관계 시각화
  - 테이블 검색 및 네임스페이스별 그룹화
  - 클릭으로 테이블 간 네비게이션
  - 줌/팬 지원 (마우스 휠, 드래그)

#### 스키마 문서 생성
- [ ] `polygen docs --output docs/schema.md`
- [ ] Markdown 기반 API 문서
- [ ] 테이블/필드 설명, 제약조건 표
- [ ] 주석(doc comments) 활용

### 테스트 & 개발

#### Mock 데이터 생성
- [ ] `polygen mock --schema game.poly --count 100`
- [ ] 제약조건 기반 랜덤 데이터 생성
  - `range(1,100)` → 1~100 사이 랜덤
  - `regex("[A-Z]{5}")` → 패턴 매칭 문자열
  - `max_length(50)` → 길이 제한 준수
- [ ] CSV/JSON 출력

### 추가 언어 지원

#### Python
- [ ] dataclass 모델 생성
- [ ] Pydantic v2 모델
- [ ] SQLAlchemy ORM 모델
- [ ] FastAPI 스키마 연동

#### Kotlin
- [ ] data class 생성
- [ ] kotlinx.serialization 지원
- [ ] Room DB 엔티티

#### Swift
- [ ] Codable struct 생성
- [ ] SwiftData 모델

### 추가 직렬화 포맷

#### Protocol Buffers
- [ ] `polygen --lang protobuf`
- [ ] .proto 파일 생성
- [ ] gRPC 서비스 정의

#### MessagePack
- [ ] 바이너리 직렬화 대안
- [ ] 언어별 MessagePack 지원

### 스키마 분석

#### Import 최적화
- [ ] 순환 참조 감지 및 경고
- [ ] 미사용 import 경고
- [ ] 의존성 그래프 분석

#### 스키마 통계
- [ ] `polygen stats --schema game.poly`
- [ ] 테이블/필드/Enum 개수
- [ ] 복잡도 메트릭
- [ ] 네임스페이스별 분포

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
| `SQL_TODO.md` | ✅ 완료 - SQL/DB 지원 상세 |
| `CLAUDE.md` | 프로젝트 가이드 |

---

## 우선순위 매트릭스

| 우선순위 | 작업 | 난이도 | 가치 |
|---------|------|--------|------|
| 🔴 높음 | Foreign Key 검증 | 중 | 검증 시스템 완성 |
| 🔴 높음 | 자동 타임스탬프 | 하 | 실용적 기능 |
| ✅ 완료 | 스키마 시각화 | 중 | 문서화 가치 |
| 🟡 중간 | Watch 모드 | 중 | DX 향상 |
| 🟡 중간 | Mock 데이터 생성 | 중 | 테스트 편의 |
| 🟡 중간 | VS Code 확장 | 중 | DX 향상 |
| 🟢 낮음 | Python 지원 | 중 | 사용자층 확대 |
| 🟢 낮음 | MySQL 지원 | 중 | DB 확장 |
| 🟢 낮음 | Protocol Buffers | 중 | 직렬화 확장 |

---

*최종 업데이트: 2026-01-24*
