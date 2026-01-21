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
@index(guild_id, level)         // 복합 인덱스
table Player {
    id: u32 primary_key;
    name: string;
    guild_id: u32;
    level: u16;
}
```

### 3.2 자동 인덱스 생성

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

### 3.3 인덱스 이름 규칙

| 소스 | 생성되는 인덱스 이름 |
|------|-------------------|
| `@index(name)` | `ByName` |
| `@index(guild_id, level)` | `ByGuildIdLevel` |
| `primary_key` on `id` | `ById` |
| `unique` on `code` | `ByCode` |
| `foreign_key` on `player_id` | `ByPlayerId` |

### 3.4 기존 `index` 제약조건 제거

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

*최종 업데이트: 2026-01-21*
