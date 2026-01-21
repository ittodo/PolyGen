# PolyGen 구현 로드맵

> 최종 업데이트: 2026-01-21

---

## 완료된 작업

### 리팩토링 (Phase 1-5) ✅

| Phase | 내용 | 상태 |
|-------|------|:----:|
| Phase 1 | 코드 품질 (Clippy, 패키지명) | ✅ |
| Phase 2 | 에러 처리 & 모듈화 | ✅ |
| Phase 3 | 아키텍처 (CodeGenerator, Pipeline) | ✅ |
| Phase 4 | 성능 & 확장성 (TypeRegistry) | ✅ |
| Phase 5 | 새 언어 지원 (C++, Rust, TypeScript) | ✅ |

### 문서화 ✅

- [x] `ANNOTATIONS_AND_ATTRIBUTES.md` - 어노테이션/어트리뷰트 가이드
- [x] 인덱스 메모리 정책 문서화
- [x] Output 카테고리 시스템 초안

---

## 구현 예정 작업

### 우선순위 1: 인덱스 시스템 통일

**목표:** 필드 레벨 `index` 제약조건을 `@index` 어노테이션으로 통일

| 작업 | 설명 | 파일 |
|------|------|------|
| 1-1 | `@index` 어노테이션 파싱 확장 (복합 필드 지원) | `polygen.pest` |
| 1-2 | AST에 IndexAnnotation 추가 | `ast_model.rs` |
| 1-3 | IR에 테이블 레벨 인덱스 목록 추가 | `ir_model.rs` |
| 1-4 | IR 빌더에서 `@index` → IndexDef 변환 | `ir_builder.rs` |
| 1-5 | 복합 인덱스 메모리 정책 적용 (2개까지) | `ir_builder.rs` |
| 1-6 | 템플릿에서 인덱스 생성 코드 업데이트 | `templates/` |
| 1-7 | 필드 레벨 `index` 제약조건 deprecated 경고 | `validation.rs` |

**의존성:** 없음
**예상 범위:** 중간

---

### 우선순위 2: SQLite 기본 지원

**목표:** SQLite DDL 생성 및 기본 CRUD

| 작업 | 설명 | 파일 |
|------|------|------|
| 2-1 | `templates/sqlite/` 디렉토리 생성 | - |
| 2-2 | `sqlite.toml` 설정 파일 | `templates/sqlite/` |
| 2-3 | DDL 생성 템플릿 (CREATE TABLE) | `sqlite_file.rhai` |
| 2-4 | 타입 매핑 (poly → SQLite) | `sqlite_file.rhai` |
| 2-5 | 인덱스 생성 (CREATE INDEX) | `sqlite_file.rhai` |
| 2-6 | 외래 키 지원 | `sqlite_file.rhai` |

**의존성:** 우선순위 1 (인덱스 통일)
**예상 범위:** 중간

---

### 우선순위 3: @datasource 어노테이션

**목표:** 테이블별 데이터소스 지정

| 작업 | 설명 | 파일 |
|------|------|------|
| 3-1 | `@datasource` 어노테이션 파싱 | `polygen.pest` |
| 3-2 | AST에 datasource 필드 추가 | `ast_model.rs` |
| 3-3 | IR에 datasource 정보 포함 | `ir_model.rs` |
| 3-4 | 네임스페이스 상속 로직 구현 | `ir_builder.rs` |
| 3-5 | 템플릿에서 datasource별 분기 | `templates/` |

**의존성:** 없음
**예상 범위:** 중간

---

### 우선순위 4: 마이그레이션 기본 지원

**목표:** 스키마 변경 감지 및 마이그레이션 SQL 생성

| 작업 | 설명 | 파일 |
|------|------|------|
| 4-1 | `@renamed_from` 어노테이션 파싱 | `polygen.pest` |
| 4-2 | AST/IR에 rename 정보 포함 | `ast_model.rs`, `ir_model.rs` |
| 4-3 | 스키마 diff 로직 구현 | 새 모듈 |
| 4-4 | 마이그레이션 SQL 생성 (SQLite) | 템플릿 |
| 4-5 | `_polygen_migrations` 테이블 관리 | 템플릿 |
| 4-6 | CLI 명령어 (`polygen migrate`) | `main.rs` |

**의존성:** 우선순위 2 (SQLite)
**예상 범위:** 큼

---

### 우선순위 5: @cache 전략

**목표:** 테이블별 캐시 전략 지정

| 작업 | 설명 | 파일 |
|------|------|------|
| 5-1 | `@cache` 어노테이션 파싱 | `polygen.pest` |
| 5-2 | 캐시 전략 enum 정의 | `ast_model.rs` |
| 5-3 | IR에 캐시 정보 포함 | `ir_model.rs` |
| 5-4 | C# DataSource에 캐시 로직 생성 | 템플릿 |

**의존성:** 우선순위 3 (@datasource)
**예상 범위:** 중간

---

### 우선순위 6: MySQL 확장

**목표:** MySQL DDL 완성 및 마이그레이션

| 작업 | 설명 | 파일 |
|------|------|------|
| 6-1 | 기존 MySQL 템플릿 개선 | `templates/mysql/` |
| 6-2 | 외래 키 DDL 생성 | `mysql_file.rhai` |
| 6-3 | 인덱스/유니크 키 DDL | `mysql_file.rhai` |
| 6-4 | 마이그레이션 SQL 생성 | 템플릿 |
| 6-5 | 엔진/문자셋 설정 | `mysql.toml` |

**의존성:** 우선순위 4 (마이그레이션)
**예상 범위:** 중간

---

### 우선순위 7: 추가 어노테이션

**목표:** 테이블/필드 관리 어노테이션 확장

| 작업 | 설명 |
|------|------|
| 7-1 | `@readonly` - 읽기 전용 테이블 |
| 7-2 | `@soft_delete` - 논리 삭제 |
| 7-3 | `auto_create` / `auto_update` - 자동 타임스탬프 |

**의존성:** 우선순위 5 (@cache)
**예상 범위:** 작음

---

### 우선순위 8: C++ 복합 인덱스 지원

**목표:** C++ 템플릿에 튜플 키 해시 유틸리티 추가

| 작업 | 설명 | 파일 |
|------|------|------|
| 8-1 | `TupleHash` 유틸리티 작성 | `static/cpp/PolygenUtils.hpp` |
| 8-2 | 2필드 복합 인덱스 생성 | `cpp_file.rhai` |
| 8-3 | 3필드+ 필터 폴백 생성 | `cpp_file.rhai` |

**의존성:** 우선순위 1 (인덱스 통일)
**예상 범위:** 작음

---

## 검토 중 (실제 사용 후 결정)

### Output 카테고리 시스템

```poly
output static_data {
    server: binary("static.bin");
    client: binary("static.bin");
}

@output(static_data)
namespace game.data { ... }
```

**결정 보류 사유:**
- 실제 프로젝트 적용 후 패턴 확인 필요
- 문법 복잡도 검증 필요

### 필드 레벨 타겟 분기

```poly
table Item {
    id: u32 primary_key;
    @server drop_rate: f32;
    @client sprite_id: u32;
}
```

**결정 보류 사유:**
- 클라/서버 분기 빈도 측정 필요
- 3가지 문법 후보 중 선택 필요

---

## 의존성 그래프

```
[1. 인덱스 통일]
       ↓
       ├──────────────────┐
       ↓                  ↓
[2. SQLite]         [8. C++ 복합인덱스]
       ↓
[3. @datasource]
       ↓
       ├──────────────────┐
       ↓                  ↓
[4. 마이그레이션]    [5. @cache]
       ↓                  ↓
[6. MySQL 확장]     [7. 추가 어노테이션]
```

---

## 우선순위 요약

| 순위 | 작업 | 예상 범위 | 의존성 |
|:----:|------|:--------:|--------|
| 1 | 인덱스 시스템 통일 | 중간 | 없음 |
| 2 | SQLite 기본 지원 | 중간 | 1 |
| 3 | @datasource 어노테이션 | 중간 | 없음 |
| 4 | 마이그레이션 기본 | 큼 | 2 |
| 5 | @cache 전략 | 중간 | 3 |
| 6 | MySQL 확장 | 중간 | 4 |
| 7 | 추가 어노테이션 | 작음 | 5 |
| 8 | C++ 복합 인덱스 | 작음 | 1 |

---

## 관련 문서

| 문서 | 설명 |
|------|------|
| `ANNOTATIONS_AND_ATTRIBUTES.md` | 어노테이션/어트리뷰트 상세 가이드 |
| `SQL_TODO.md` | SQL 지원 확장 상세 계획 |
| `REFACTORING_TODO.md` | 완료된 리팩토링 기록 |

---

*최종 업데이트: 2026-01-21*
