# SQL 지원 확장 계획

> 상태: 검토 중 (2026-01-21)

---

## 우선순위

1. **SQLite** (먼저) - 단순, 테스트 쉬움, 임베디드/게임에 적합
2. **MySQL/MariaDB** (나중) - SQLite 기반 확장
3. **PostgreSQL** (옵션) - 필요시

---

## 현재 상태

- 기본 MySQL 템플릿 존재 (`templates/mysql/`)
- CREATE TABLE DDL 생성 가능
- SQLite 템플릿 없음

---

## Phase 1: SQLite 기본 지원

### 1.1 DDL 생성

```sql
CREATE TABLE Player (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    level INTEGER DEFAULT 1,
    email TEXT UNIQUE,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_player_level ON Player(level);
```

**타입 매핑**:
| .poly 타입 | SQLite 타입 |
|-----------|-------------|
| u8, u16, u32, u64, i8, i16, i32, i64 | INTEGER |
| f32, f64 | REAL |
| string | TEXT |
| bool | INTEGER (0/1) |
| bytes | BLOB |

### 1.2 마이그레이션 테이블

```sql
CREATE TABLE _polygen_migrations (
    version INTEGER PRIMARY KEY AUTOINCREMENT,
    poly_schema TEXT NOT NULL,
    checksum TEXT,
    applied_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

### 1.3 SQLite ALTER TABLE 제한 대응

SQLite는 ALTER TABLE이 제한적:
- ✅ ADD COLUMN
- ❌ DROP COLUMN (3.35.0+ 에서 지원)
- ❌ RENAME COLUMN (3.25.0+ 에서 지원)
- ❌ MODIFY COLUMN (불가)

**해결책**: 테이블 재생성 패턴
```sql
-- 1. 새 테이블 생성
CREATE TABLE Player_new (...);
-- 2. 데이터 복사
INSERT INTO Player_new SELECT ... FROM Player;
-- 3. 기존 테이블 삭제
DROP TABLE Player;
-- 4. 이름 변경
ALTER TABLE Player_new RENAME TO Player;
```

---

## Phase 2: 이름 변경 지원

```poly
@renamed_from("OldPlayer")
table Player {
    @renamed_from("user_name")
    name: string;
}
```

생성 SQL (SQLite 3.25.0+):
```sql
ALTER TABLE OldPlayer RENAME TO Player;
ALTER TABLE Player RENAME COLUMN user_name TO name;
```

---

## Phase 3: DDL 확장

| 기능 | SQLite | MySQL |
|------|--------|-------|
| 인덱스 | ✅ CREATE INDEX | ✅ |
| 유니크 인덱스 | ✅ CREATE UNIQUE INDEX | ✅ |
| 풀텍스트 | ✅ FTS5 (별도 문법) | ✅ FULLTEXT |
| 외래키 | ✅ REFERENCES (PRAGMA 필요) | ✅ |
| Auto increment | ✅ AUTOINCREMENT | ✅ AUTO_INCREMENT |
| 엔진 | ❌ 해당 없음 | ✅ ENGINE=InnoDB |
| 파티셔닝 | ❌ 해당 없음 | ✅ |
| 문자셋 | ❌ 해당 없음 | ✅ CHARSET |

---

## Phase 4: 쿼리/뷰 (검토 필요)

### 쿼리 정의

```poly
query GetPlayerById(id: u32) -> Player {
    SELECT * FROM Player WHERE id = ?
}
```

### 뷰

```poly
view PlayerSummary {
    SELECT id, name, level FROM Player
}
```

---

## 설계 고려사항

### 대안 A: SQL 전용 블록 분리

```poly
namespace game {
    table Player { ... }
}

sql {
    @index Player(name, level);
    query GetPlayer(id: u32) -> Player { ... }
}
```

### 대안 B: 별도 파일

```
game.poly        -- 공통 스키마
game.sql.poly    -- SQL 전용 확장
```

### 대안 C: 범위 축소

- DDL + 마이그레이션만
- 쿼리/뷰는 수동 작성

---

## 변경 감지 매트릭스

| 변경 유형 | 감지 방법 | SQLite SQL |
|----------|----------|------------|
| 테이블 추가 | 새 이름 | `CREATE TABLE` |
| 테이블 삭제 | 이름 사라짐 | `DROP TABLE` (경고) |
| 테이블 이름변경 | `@renamed_from` | `ALTER TABLE RENAME TO` |
| 컬럼 추가 | 새 필드 | `ALTER TABLE ADD COLUMN` |
| 컬럼 삭제 | 필드 사라짐 | 테이블 재생성 (경고) |
| 컬럼 이름변경 | `@renamed_from` | `ALTER TABLE RENAME COLUMN` (3.25+) |
| 타입 변경 | 동일 이름, 다른 타입 | 테이블 재생성 |

---

## 구현 순서

- [ ] `templates/sqlite/` 디렉토리 생성
- [ ] `sqlite.toml` 설정 파일
- [ ] 기본 DDL 생성 (CREATE TABLE)
- [ ] 인덱스 생성
- [ ] `@renamed_from` 어노테이션 파싱 (polygen.pest)
- [ ] IR에 rename 정보 포함
- [ ] 마이그레이션 diff 로직
- [ ] 마이그레이션 SQL 생성
- [ ] CLI 명령어 (`polygen migrate`)

---

## 결정 필요 사항

- [ ] SQL 전용 문법 분리 방식 결정
- [ ] 마이그레이션 CLI 설계
- [ ] 쿼리/뷰 지원 범위
- [ ] 파괴적 변경 처리 정책
- [ ] SQLite 최소 지원 버전 (3.25.0? 3.35.0?)

---

*최종 업데이트: 2026-01-21*
