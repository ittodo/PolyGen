# SQL 지원 확장 계획

> 상태: Phase 1-4 완료 (2026-01-22)

---

## 목표

**DB 캐시 전용 라이브러리 세트 생성**
- 기존 DataSource 스타일 유지/확장
- 여러 DB를 키-밸류로 연결
- 정적 데이터 / 유저 데이터 / 캐시 통합 관리

---

## 아키텍처 결정: @datasource 기반 자동 생성 (B안 채택)

SQL은 별도 `--lang` 타겟이 아닌, **언어별 플러그인**으로 동작:

```bash
# 실행
polygen --lang csharp --schema game.poly

# @datasource 어노테이션에 따라 자동 생성
```

### 스키마 예시

```poly
@datasource("sqlite")
namespace game.data {
    table ItemTable { ... }    // → SQLite DDL 생성
    table SkillTable { ... }
}

@datasource("mysql")
namespace game.user {
    table Player { ... }       // → MySQL DDL 생성
}
```

### 생성 결과

```
output/
├── csharp/
│   ├── Game/Data/ItemTable.cs
│   ├── Game/User/Player.cs
│   └── DataSource.cs          # 통합 accessor
│
├── sqlite/                     # @datasource("sqlite") 테이블만
│   └── game_data.sql
│
└── mysql/                      # @datasource("mysql") 테이블만
    └── game_user.sql
```

### 장점
- CLI 플래그 없이 스키마가 모든 것을 결정
- namespace 단위로 다른 DB 지정 가능
- 언어 코드와 DDL이 항상 동기화
- 스키마가 "진실의 원천" 역할

---

## 현재 상태

### ✅ 완료
- [x] `templates/sqlite/` 디렉토리 생성
- [x] `sqlite.toml` 설정 파일
- [x] 기본 DDL 생성 (CREATE TABLE)
- [x] 인덱스 생성 (CREATE INDEX, CREATE UNIQUE INDEX)
- [x] `.renames` 파일 문법 (polygen.pest)
- [x] IR에 rename 정보 포함 (`RenameInfo`, `RenameKind`)
- [x] 마이그레이션 SQL 생성 (ALTER TABLE RENAME)
- [x] 네임스페이스 접두사 처리 (`game.data` → `game_data_`)
- [x] @datasource 기반 자동 DDL 생성 연동
  - C#, C++, Rust, TypeScript 모두 지원
  - SQLite/MySQL datasource 필터링

### ✅ 추가 완료
- [x] 마이그레이션 diff 로직 (`--baseline` 옵션으로 스키마 비교)
- [x] CLI 명령어 (`polygen migrate`) - 서브커맨드 구조로 구현
- [x] 언어별 DB accessor 생성 (C#, Rust, C++, TypeScript 완료)
- [x] 고급 어노테이션: `@cache`, `@readonly`, `@soft_delete`

---

## 우선순위

1. **SQLite** (현재) - 단순, 테스트 쉬움, 임베디드/게임에 적합
2. **MySQL/MariaDB** (나중) - SQLite 기반 확장
3. **PostgreSQL** (옵션) - 필요시
4. **Redis** (옵션) - 캐시 전용

---

## DataSource 아키텍처

### @datasource 어노테이션

```poly
// 데이터소스 선언 (파일 상단)
datasource main = mysql;
datasource cache = redis;
datasource static = sqlite;

// namespace 단위 기본값 설정
@datasource("main")
namespace game {
    table Player { ... }           // → main (상속)
    table Guild { ... }            // → main (상속)

    @datasource("static")
    namespace data {
        table ItemTable { ... }    // → static (상속)
        table SkillTable { ... }   // → static (상속)

        @datasource("cache")
        table HotData { ... }      // → cache (오버라이드)
    }

    @datasource("cache")
    table Session { ... }          // → cache (오버라이드)
}
```

### 우선순위 규칙

```
테이블 직접 지정 > 가장 가까운 namespace > 상위 namespace > 기본값
```

### 생성되는 코드 (C# 예시)

```csharp
var data = new GameDataSource();

// 키-밸류로 DB 연결
data.Connect("static", "game_data.db");           // SQLite 파일
data.Connect("main", "server=localhost;db=game"); // MySQL
data.Connect("cache", "localhost:6379");          // Redis

// 사용 (기존 DataSource 스타일)
var item = data.ItemTables.GetById(1001);    // → static (SQLite)
var player = data.Players.GetById(123);       // → main (MySQL)
var session = data.Sessions.Get("token_xx");  // → cache (Redis)

// 변경 후 저장
player.Level = 50;
data.SaveChanges();  // 각 DB에 맞게 저장
```

### DB별 지원 기능

| 기능 | SQLite | MySQL | Redis |
|------|--------|-------|-------|
| DDL 생성 | ✅ | ✅ | ❌ |
| 마이그레이션 | ✅ | ✅ | ❌ |
| CRUD | ✅ | ✅ | ✅ (Key-Value) |
| 트랜잭션 | ✅ | ✅ | MULTI/EXEC |
| 캐시 전략 | 전체 로드 | 온디맨드 | 네이티브 |
| TTL/만료 | 수동 | 수동 | 네이티브 |

---

## Phase 1: SQLite 기본 지원 ✅ 완료

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

## Phase 2: 이름 변경 지원 ✅ 완료

### 구현 방식: `.renames` 파일

어노테이션 대신 별도 파일로 관리 (버전 관리 용이):

```
# migrations/v1_to_v2.renames
game.Player -> User;
game.User.user_name -> name;
game.User.hp -> health;
```

```poly
// schema.poly
import "migrations/v1_to_v2.renames";

namespace game {
    table User { ... }  // 이전: Player
}
```

### 생성 SQL (SQLite 3.25.0+)

```sql
ALTER TABLE game_Player RENAME TO game_User;
ALTER TABLE game_User RENAME COLUMN user_name TO name;
ALTER TABLE game_User RENAME COLUMN hp TO health;
```

### 경로 형식

| 세그먼트 수 | 의미 | 예시 |
|------------|------|------|
| 2개 | 테이블 rename | `namespace.OldTable -> NewTable;` |
| 3개+ | 필드 rename | `namespace.Table.old_field -> new_field;` |

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

## 테이블/필드 어노테이션 (구현 완료)

### @cache - 캐시 전략 ✅

```poly
@cache(strategy: full_load)     // 시작시 전체 로드 (정적 데이터)
table ItemTable { ... }

@cache(strategy: on_demand)     // 필요시 로드
@cache(ttl: 300)                // 5분 후 만료
table Player { ... }
```

| 전략 | 설명 | 용도 |
|------|------|------|
| `full_load` | 시작시 전체 로드 | 정적 데이터, 설정 테이블 |
| `on_demand` | 필요시 로드 | 유저 데이터 |
| `write_through` | 쓰기시 즉시 DB 반영 | 중요 데이터 |
| `write_back` | 지연 쓰기 (배치) | 로그, 통계 |

### @readonly - 읽기 전용 ✅

```poly
@readonly
table ItemTable { ... }  // 수정 불가 (정적 데이터)
```

- SaveChanges()에서 무시
- 수정 시도시 예외 발생

### @soft_delete - 논리 삭제 ✅

```poly
@soft_delete("deleted_at")
table Player {
    deleted_at: timestamp?;  // NULL이면 활성, 값 있으면 삭제됨
}
```

- DELETE → UPDATE deleted_at = NOW()
- SELECT시 자동으로 deleted_at IS NULL 조건 추가
- 필요시 삭제된 데이터도 조회 가능

### 자동 타임스탬프

```poly
table Player {
    created_at: timestamp auto_create;   // INSERT시 자동 설정
    updated_at: timestamp auto_update;   // UPDATE시 자동 갱신
}
```

- `auto_create`: INSERT시 현재 시간
- `auto_update`: UPDATE시 현재 시간

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

### Phase 1: 기본 지원 ✅
- [x] `templates/sqlite/` 디렉토리 생성
- [x] `sqlite.toml` 설정 파일
- [x] 기본 DDL 생성 (CREATE TABLE)
- [x] 인덱스 생성

### Phase 2: 마이그레이션 ✅
- [x] `.renames` 파일 문법 (polygen.pest)
- [x] IR에 rename 정보 포함
- [x] 마이그레이션 SQL 생성

### Phase 3: @datasource 연동 ✅
- [x] @datasource 어노테이션 기반 DDL 자동 생성
- [x] 언어 코드 생성 시 해당 DB DDL도 함께 출력
- [x] datasource별 테이블 필터링

### Phase 4: 고급 기능 ✅ 완료
- [x] 마이그레이션 diff 로직 (`--baseline` 옵션으로 스키마 비교)
- [x] CLI 명령어 (`polygen migrate`) - 서브커맨드 구조로 구현
- [x] 언어별 DB accessor 코드 생성 (C#, Rust, C++, TypeScript 완료)
- [x] 고급 어노테이션 지원:
  - `@cache` (full_load, on_demand, write_through)
  - `@readonly` (읽기 전용 테이블)
  - `@soft_delete` (소프트 삭제 필드 지정)

### Phase 5: DB 기반 마이그레이션 ✅ 완료 (2026-01-26)
- [x] SQLite DB introspection 모듈 (`src/db_introspection.rs`)
- [x] DB-to-Schema 비교 로직 (`MigrationDiff::compare_db`)
- [x] CLI `--db` 옵션 추가 (SQLite 파일 경로 지정)
- [x] 통합 테스트 작성 (`tests/db_migration_tests.rs`)

---

## DB 기반 마이그레이션 (Phase 5)

### 개요

기존 스키마-투-스키마 비교 방식 대신, **실제 DB에 연결하여 현재 상태를 읽어오는 방식** 지원.

### 사용법

```bash
# 기존 방식: 스키마 파일 비교
polygen migrate --baseline old.poly --schema-path new.poly

# 새 방식: DB 파일에서 직접 스키마 읽기
polygen migrate --db game.db --schema-path schema.poly
```

### 장점

| 기존 (--baseline) | 새 방식 (--db) |
|-------------------|----------------|
| baseline.poly 파일 관리 필요 | 실제 DB 상태 반영 |
| 수동 변경 감지 불가 | 모든 변경 감지 |
| 파일 동기화 필요 | 항상 최신 상태 |

### 지원 기능

| 변경 유형 | 감지 | 생성 SQL |
|----------|------|----------|
| 테이블 추가 | ✅ | `CREATE TABLE` |
| 테이블 삭제 | ✅ | `DROP TABLE` (경고) |
| 컬럼 추가 | ✅ | `ALTER TABLE ADD COLUMN` |
| 컬럼 삭제 | ✅ | `DROP COLUMN` (경고) |
| 타입 변경 | ✅ | 테이블 재생성 (경고) |

### 모듈 구조

```
src/
├── db_introspection.rs   # DB 스키마 읽기
│   ├── SqliteIntrospector  # SQLite 연결 및 introspection
│   ├── DbSchema            # DB 스키마 구조체
│   ├── DbTable             # 테이블 정보
│   └── DbColumn            # 컬럼 정보
│
└── migration.rs          # 마이그레이션 diff
    ├── MigrationDiff::compare()     # 스키마-투-스키마
    └── MigrationDiff::compare_db()  # DB-투-스키마 (NEW)
```

### 테스트

```bash
# DB 마이그레이션 테스트 실행
cargo test --test db_migration_tests

# 모듈 단위 테스트
cargo test db_introspection
```

---

## 결정 완료

- [x] ~~SQL 전용 문법 분리 방식~~ → `.renames` 파일 방식 채택
- [x] ~~아키텍처~~ → B안 (@datasource 기반 자동 생성) 채택
- [x] SQLite 최소 지원 버전 → 3.25.0 (RENAME COLUMN 지원)
- [x] ~~마이그레이션 diff 자동 감지~~ → DB introspection 방식 채택

## 결정 필요 사항

- [ ] 쿼리/뷰 지원 범위
- [ ] 파괴적 변경 처리 정책 (DROP TABLE 경고 등)
- [ ] MySQL DB introspection 지원

---

*최종 업데이트: 2026-01-26*
