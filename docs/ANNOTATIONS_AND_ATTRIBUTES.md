# PolyGen 어노테이션 & 어트리뷰트 가이드

> 상태: 작성 중 (2026-01-21)

---

## 개요

PolyGen 스키마 언어는 두 가지 메타데이터 시스템을 제공합니다:

| 구분 | 어노테이션 (Annotation) | 어트리뷰트 (Attribute/Constraint) |
|------|------------------------|----------------------------------|
| **문법** | `@name(key: value)` | `constraint_name` 또는 `constraint(value)` |
| **목적** | 메타데이터, 런타임 힌트, 코드 생성 | 데이터 무결성, 스키마 정의, 검증 |
| **적용 대상** | table, embed, enum, field | field만 |
| **영향** | 로더, 캐시, 인덱스, 코드 생성 | DB 스키마, 유효성 검증 |

---

## 1. 어노테이션 (Annotation)

### 1.1 문법

```pest
annotation             = { "@" ~ IDENT ~ ("(" ~ annotation_params_list? ~ ")")? }
annotation_params_list = { annotation_param ~ ("," ~ annotation_param)* }
annotation_param       = { IDENT ~ ":" ~ literal }
```

**지원 리터럴 타입:**
- 문자열: `"value"`
- 정수: `123`
- 부동소수점: `3.14`
- 불린: `true`, `false`
- 식별자: `on_demand`

### 1.2 구현된 어노테이션

| 어노테이션 | 파라미터 | 적용 대상 | 설명 |
|----------|---------|----------|------|
| `@load` | `csv: "path"`, `json: "path"` | table | 데이터 로더 지정 |
| `@taggable` | 없음 | table | 태그 지원 표시 |
| `@link_rows` | `(TypeName)` | table | 행 연결 (Cross-reference) |
| `@index` | `(field1, field2, ...)` | table | 인덱스 생성 (단일/복합) |

**사용 예제:**
```poly
@load(csv: "data/players.csv", json: "data/players.json")
@taggable
@index(name)
@index(guild_id, level)  // 복합 인덱스
table Player {
    id: u32 primary_key;
    name: string;
    guild_id: u32;
    level: u16;
}
```

### 1.3 계획된 어노테이션

#### @datasource - 데이터소스 지정

```poly
@datasource("static")
namespace data {
    @datasource("cache")
    table HotData { ... }
}
```

| 파라미터 | 설명 |
|---------|------|
| `"main"` | 기본 DB (MySQL 등) |
| `"static"` | 정적 데이터 (SQLite 등) |
| `"cache"` | 캐시 (Redis 등) |

**우선순위:** 테이블 직접 지정 > 가장 가까운 namespace > 상위 namespace > 기본값

#### @cache - 캐시 전략

```poly
@cache(strategy: full_load)
table ItemTable { ... }

@cache(strategy: on_demand, ttl: 300)
table Player { ... }
```

| 전략 | 설명 | 용도 |
|------|------|------|
| `full_load` | 시작시 전체 로드 | 정적 데이터, 설정 테이블 |
| `on_demand` | 필요시 로드 | 유저 데이터 |
| `write_through` | 쓰기시 즉시 DB 반영 | 중요 데이터 |
| `write_back` | 지연 쓰기 (배치) | 로그, 통계 |

| 파라미터 | 타입 | 설명 |
|---------|------|------|
| `strategy` | 식별자 | 캐시 전략 |
| `ttl` | 정수 | 만료 시간 (초) |

#### @readonly - 읽기 전용

```poly
@readonly
table ItemTable { ... }
```

- `SaveChanges()`에서 무시
- 수정 시도시 예외 발생

#### @soft_delete - 논리 삭제

```poly
@soft_delete("deleted_at")
table Player {
    deleted_at: timestamp?;
}
```

- DELETE → `UPDATE deleted_at = NOW()`
- SELECT시 자동으로 `deleted_at IS NULL` 조건 추가

#### @renamed_from - 이름 변경 (마이그레이션)

```poly
@renamed_from("OldPlayer")
table Player {
    @renamed_from("user_name")
    name: string;
}
```

- 테이블/필드 이름 변경 추적
- 마이그레이션 SQL 자동 생성

---

## 2. 어트리뷰트 (Attribute/Constraint)

### 2.1 문법

```pest
constraint      = { primary_key | unique | max_length | default_val | range_val | regex_val | foreign_key_val }
primary_key     = { "primary_key" }
unique          = { "unique" }
max_length      = { "max_length" ~ "(" ~ INTEGER ~ ")" }
default_val     = { "default" ~ "(" ~ literal ~ ")" }
range_val       = { "range" ~ "(" ~ literal ~ "," ~ literal ~ ")" }
regex_val       = { "regex" ~ "(" ~ STRING_LITERAL ~ ")" }
foreign_key_val = { "foreign_key" ~ "(" ~ path ~ ")" ~ ("as" ~ IDENT)? }
```

**특징:**
- 필드 타입 뒤에 공백으로 구분하여 나열
- `@` 접두사 없음
- 여러 제약조건 조합 가능

### 2.2 구현된 어트리뷰트

| 어트리뷰트 | 문법 | 파라미터 | 설명 |
|-----------|------|---------|------|
| `primary_key` | `primary_key` | 없음 | 기본 키 |
| `unique` | `unique` | 없음 | 고유 값 제약 |
| `max_length` | `max_length(n)` | 정수 | 문자열/바이트 최대 길이 |
| `default` | `default(value)` | 리터럴 | 기본값 |
| `range` | `range(min, max)` | 리터럴 2개 | 값 범위 제한 |
| `regex` | `regex("pattern")` | 문자열 | 정규식 검증 |
| `foreign_key` | `foreign_key(Table.field) [as alias]` | 경로, 별칭(선택) | 외래 키 참조 |

**사용 예제:**
```poly
table Player {
    id: u32 primary_key;
    code: string unique max_length(10) regex("^[A-Z0-9]{5,10}$");
    level: u16 default(1) range(1, 100);
    guild_id: u32 foreign_key(Guild.id);
    owner_id: u32 foreign_key(User.id) as OwnedItems;
}
```

### 2.3 계획된 어트리뷰트

#### auto_create / auto_update - 자동 타임스탬프

```poly
table Player {
    created_at: timestamp auto_create;   // INSERT시 자동 설정
    updated_at: timestamp auto_update;   // UPDATE시 자동 갱신
}
```

---

## 3. 인덱스 설계 (통일)

### 3.1 인덱스 생성 방법

**어노테이션 `@index` 사용 (권장):**

```poly
@index(name)                    // 단일 필드 인덱스
@index(name, unique: true)      // 유니크 인덱스
@index(guild_id, level)         // 복합 인덱스 (2개)
@index(region, guild_id, level) // 복합 인덱스 (3개)
table Player {
    id: u32 primary_key;
    name: string;
    region: u8;
    guild_id: u32;
    level: u16;
}
```

### 3.2 타겟별 인덱스 지원 정책

복합 인덱스는 메모리 오버헤드가 크므로, 타겟별로 지원 범위가 다릅니다:

| 필드 수 | 인메모리 (C#/Rust) | Redis | DB (MySQL) |
|:------:|:-----------------:|:-----:|:----------:|
| 1개 | ✅ 인덱스 | ✅ 인덱스 | ✅ 인덱스 |
| 2개 | ✅ 복합 인덱스 | ✅ 복합 키 | ✅ 인덱스 |
| 3개+ | ⚠️ 2개 인덱스 + 필터 | ⚠️ 2개 + 필터 | ✅ 인덱스 |

**메모리 오버헤드 예시:**
```
단일 인덱스: HashMap<guild_id, Vec<&Player>>
- 길드 100개 → 엔트리 ~100개

2필드 복합: HashMap<(guild_id, level), Vec<&Player>>
- 길드 100개 × 레벨 100개 → 엔트리 최대 ~10,000개 (관리 가능)

3필드 복합: HashMap<(region, guild_id, level), Vec<&Player>>
- 지역 10개 × 길드 100개 × 레벨 100개 → 엔트리 최대 ~100,000개 (비효율)
```

### 3.3 생성 코드 예시

**스키마:**
```poly
@index(guild_id)                    // 1개: 모든 타겟
@index(guild_id, level)             // 2개: 모든 타겟
@index(region, guild_id, level)     // 3개: DB만 인덱스, 나머진 필터
table Player {
    id: u32 primary_key;
    region: u8;
    guild_id: u32;
    level: u16;
}
```

**C# 생성 코드:**
```csharp
public class PlayerTable
{
    // 1개 필드 - 직접 인덱스
    private Dictionary<uint, List<Player>> _byGuildId;

    // 2개 필드 - 복합 인덱스 (튜플 키)
    private Dictionary<(uint, ushort), List<Player>> _byGuildIdLevel;

    // 3개 필드 - 인덱스 생성 안함 (DB에서만 사용)

    // 조회 메서드
    public IReadOnlyList<Player> ByGuildId(uint guildId)
        => _byGuildId.TryGetValue(guildId, out var list) ? list : Empty;

    public IReadOnlyList<Player> ByGuildIdLevel(uint guildId, ushort level)
        => _byGuildIdLevel.TryGetValue((guildId, level), out var list) ? list : Empty;

    // 3개 필드 - 2개 인덱스 + 필터로 대체
    public IEnumerable<Player> ByRegionGuildIdLevel(byte region, uint guildId, ushort level)
        => ByGuildIdLevel(guildId, level).Where(p => p.Region == region);
}
```

**MySQL DDL:**
```sql
CREATE TABLE Player (
    id INT UNSIGNED NOT NULL,
    region TINYINT UNSIGNED NOT NULL,
    guild_id INT UNSIGNED NOT NULL,
    level SMALLINT UNSIGNED NOT NULL,
    PRIMARY KEY (id),
    INDEX idx_player_guild_id (guild_id),
    INDEX idx_player_guild_id_level (guild_id, level),
    INDEX idx_player_region_guild_id_level (region, guild_id, level)  -- DB는 3개도 OK
);
```

### 3.4 자동 인덱스 생성

다음 어트리뷰트는 자동으로 인덱스를 생성합니다:

| 어트리뷰트 | 인덱스 타입 | 반환 타입 |
|-----------|-----------|----------|
| `primary_key` | UniqueIndex | `Option<&T>` |
| `unique` | UniqueIndex | `Option<&T>` |
| `foreign_key` | GroupIndex | `&[T]` |

```poly
table Item {
    id: u32 primary_key;              // → ById: UniqueIndex (자동)
    code: string unique;              // → ByCode: UniqueIndex (자동)
    player_id: u32 foreign_key(Player.id);  // → ByPlayerId: GroupIndex (자동)
}
```

### 3.5 인덱스 이름 규칙

| 소스 | 생성되는 인덱스 이름 |
|------|-------------------|
| `@index(name)` | `ByName` |
| `@index(guild_id, level)` | `ByGuildIdLevel` |
| `@index(region, guild_id, level)` | `ByRegionGuildIdLevel` (DB만) |
| `primary_key` on `id` | `ById` |
| `unique` on `code` | `ByCode` |
| `foreign_key` on `player_id` | `ByPlayerId` |

### 3.6 기존 `index` 제약조건 제거

**변경 전 (deprecated):**
```poly
table Player {
    name: string index;  // ❌ 제거 예정
}
```

**변경 후:**
```poly
@index(name)
table Player {
    name: string;        // ✅ 권장
}
```

**이유:**
- 복합 인덱스 지원 불가 문제 해결
- 테이블 레벨에서 인덱스 관리 일원화
- 어노테이션/어트리뷰트 역할 명확화
- 타겟별 지원 정책 적용 가능

---

## 4. 언어별 변환

### 4.1 C# 변환

| 소스 | C# 어트리뷰트 |
|------|--------------|
| `primary_key` | `[Key]` |
| `unique` | `[Index(IsUnique = true)]` |
| `max_length(n)` | `[MaxLength(n)]` |
| `@load(csv: "...", json: "...")` | `[Load(csv = "...", json = "...")]` |
| `@taggable` | `[Taggable]` |
| `@index(name)` | 인덱스 딕셔너리 생성 |

```csharp
[Load(csv = "players.csv", json = "players.json")]
[Taggable]
public class Player
{
    [Key]
    public uint Id;

    [Index(IsUnique = true)]
    [MaxLength(10)]
    public string Code;

    [MaxLength(100)]
    public string Name;
}
```

### 4.2 MySQL DDL 변환

| 소스 | MySQL |
|------|-------|
| `primary_key` | `PRIMARY KEY (col)` |
| `unique` | `UNIQUE KEY (col)` |
| `max_length(n)` | `VARCHAR(n)` |
| `@index(col)` | `INDEX idx_table_col (col)` |
| `@index(col1, col2)` | `INDEX idx_table_col1_col2 (col1, col2)` |
| `foreign_key(Table.field)` | `FOREIGN KEY (col) REFERENCES Table(field)` |

```sql
CREATE TABLE Player (
    id INT UNSIGNED NOT NULL,
    code VARCHAR(10) NOT NULL,
    name VARCHAR(100) NOT NULL,
    guild_id INT UNSIGNED,
    level SMALLINT UNSIGNED DEFAULT 1,
    PRIMARY KEY (id),
    UNIQUE KEY (code),
    INDEX idx_player_name (name),
    INDEX idx_player_guild_id_level (guild_id, level),
    FOREIGN KEY (guild_id) REFERENCES Guild(id)
);
```

---

## 5. 구현 현황

### 5.1 어노테이션

| 어노테이션 | 파싱 | AST | IR | 템플릿 |
|----------|:---:|:---:|:---:|:------:|
| `@load` | ✅ | ✅ | ✅ | ✅ |
| `@taggable` | ✅ | ✅ | ✅ | ✅ |
| `@link_rows` | ✅ | ✅ | ✅ | ✅ |
| `@index` | 🚧 | 🚧 | 🚧 | 🚧 |
| `@datasource` | ❌ | ❌ | ❌ | ❌ |
| `@cache` | ❌ | ❌ | ❌ | ❌ |
| `@readonly` | ❌ | ❌ | ❌ | ❌ |
| `@soft_delete` | ❌ | ❌ | ❌ | ❌ |
| `@renamed_from` | ❌ | ❌ | ❌ | ❌ |

### 5.2 어트리뷰트

| 어트리뷰트 | 파싱 | AST | IR | C# | MySQL |
|-----------|:---:|:---:|:---:|:---:|:-----:|
| `primary_key` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `unique` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `max_length` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `default` | ✅ | ✅ | ❌ | ❌ | ❌ |
| `range` | ✅ | ✅ | ❌ | ❌ | ❌ |
| `regex` | ✅ | ✅ | ❌ | ❌ | ❌ |
| `foreign_key` | ✅ | ✅ | ✅ | ✅ | ❌ |
| `index` | ✅ | ✅ | ✅ | ⚠️ | ⚠️ |
| `auto_create` | ❌ | ❌ | ❌ | ❌ | ❌ |
| `auto_update` | ❌ | ❌ | ❌ | ❌ | ❌ |

**범례:** ✅ 완료 | 🚧 진행중 | ⚠️ 부분 구현 | ❌ 미구현

---

## 6. 파일 위치

| 구성 요소 | 파일 |
|----------|------|
| 어노테이션 문법 | `src/polygen.pest:90-94` |
| 어트리뷰트 문법 | `src/polygen.pest:78-88` |
| AST 어노테이션 | `src/ast_model.rs:116-134` |
| AST 어트리뷰트 | `src/ast_model.rs:211-230` |
| 어노테이션 파싱 | `src/ast_parser/metadata.rs:12-82` |
| 어트리뷰트 파싱 | `src/ast_parser/fields.rs:106-147` |
| IR 어노테이션 | `src/ir_model.rs:189-205` |
| IR 어트리뷰트 | `src/ir_model.rs:125-154` |
| Rhai 등록 | `src/rhai/registry.rs:228-277, 394-405` |

---

## 7. 마이그레이션 가이드

### 7.1 `index` 제약조건 → `@index` 어노테이션

**Before:**
```poly
table Player {
    name: string index;
    guild_id: u32 index;
}
```

**After:**
```poly
@index(name)
@index(guild_id)
table Player {
    name: string;
    guild_id: u32;
}
```

### 7.2 복합 인덱스 추가

**Before (불가능):**
```poly
// 복합 인덱스를 표현할 방법이 없었음
```

**After:**
```poly
@index(guild_id, level)
table Player {
    guild_id: u32;
    level: u16;
}
```

---

## 8. 검토 중 (Draft)

> 아래 기능들은 실제 프로젝트 적용 후 문법을 확정할 예정입니다.

### 8.1 Output 카테고리 시스템

테이블의 데이터 소스와 직렬화 방식을 카테고리로 관리하는 방안입니다.

**목표:**
- 데이터 모델과 배포 설정 분리
- 테이블마다 서버/클라이언트 데이터 소스 지정
- 네임스페이스 레벨 기본값 + 테이블 오버라이드

**문법 초안:**
```poly
// 1. 카테고리 정의 (프로젝트당 한번)
output static_data {
    server: binary("data/static.bin");
    client: binary("data/static.bin");
}

output db_only {
    server: mysql("main");
    client: none;
}

output synced {
    server: mysql("main");
    client: json;  // 서버에서 수신
}

// 2. 네임스페이스 레벨 기본값
@output(static_data)
namespace game.data {

    // 상속받음
    table Item {
        id: u32 primary_key;
        name: string;
    }

    // 오버라이드
    @output(db_only)
    table Player { ... }
}
```

**일반적인 카테고리 패턴:**

| 카테고리 | 서버 | 클라이언트 | 용도 |
|---------|------|-----------|------|
| `static_data` | 바이너리 | 바이너리 | 아이템, 스킬 정의 |
| `db_only` | DB | 없음 | 플레이어, 인벤토리 |
| `synced` | DB | 서버에서 수신 | 내 캐릭터 정보 |
| `log` | DB (쓰기) | 없음 | 로그, 통계 |

### 8.2 필드 레벨 타겟 분기

같은 테이블인데 서버/클라이언트에서 필드가 약간 다른 경우를 처리하는 방안입니다.

**문법 후보 1: 단순 태그**
```poly
table Item {
    id: u32 primary_key;
    name: string;              // 공통

    @server drop_rate: f32;    // 서버만
    @client sprite_id: u32;    // 클라만
}
```

**문법 후보 2: 조건부 블록**
```poly
table Item {
    id: u32 primary_key;
    name: string;

    #server {
        drop_rate: f32;
        spawn_weight: u32;
    }

    #client {
        sprite_id: u32;
        sound_id: u32;
    }
}
```

**문법 후보 3: Embed 확장**
```poly
embed ServerItemData {
    drop_rate: f32;
    spawn_weight: u32;
}

embed ClientItemData {
    sprite_id: u32;
}

table Item {
    id: u32 primary_key;
    name: string;

    @server ...ServerItemData;
    @client ...ClientItemData;
}
```

### 8.3 결정 보류 사유

- 실제 게임 프로젝트에서 사용 패턴 확인 필요
- 클라/서버 분기가 얼마나 자주 발생하는지 측정 필요
- 문법 복잡도 vs 실용성 트레이드오프 검증 필요

---

*최종 업데이트: 2026-01-21*
