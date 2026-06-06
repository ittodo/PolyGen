# PolyGen 어노테이션 & 어트리뷰트 가이드

> 상태: 작성 중 (2026-06-03)

---

## 개요

PolyGen 스키마 언어는 두 가지 메타데이터 시스템을 제공합니다:

| 구분 | 어노테이션 (Annotation) | 어트리뷰트 (Attribute/Constraint) |
|------|------------------------|----------------------------------|
| **문법** | `@name(key: value)` | `constraint_name` 또는 `constraint(value)` |
| **목적** | 메타데이터, 런타임 힌트, 코드 생성 | 데이터 무결성, 스키마 정의, 검증 |
| **적용 대상** | 문법상 여러 declaration/member/variant, built-in은 target 제한 | field만 |
| **영향** | 로더, 캐시, 인덱스, 코드 생성 | DB 스키마, 유효성 검증 |

---

## 1. 어노테이션 (Annotation)

### 1.1 문법

```pest
annotation             = { "@" ~ IDENT ~ ("(" ~ annotation_params_list? ~ ")")? }
annotation_params_list = { annotation_arg ~ ("," ~ annotation_arg)* }
annotation_arg         = { annotation_param | literal }
annotation_param       = { IDENT ~ ":" ~ literal }
```

**지원 리터럴 타입:**
- 문자열: `"value"`
- 정수: `123`
- 부동소수점: `3.14`
- 불린: `true`, `false`
- 식별자: `on_demand`

### 1.2 Attachment target model

Annotation은 두 단계로 이해합니다.

1. **문법상 붙일 수 있는 위치**: parser가 metadata로 보존하는 위치입니다.
2. **built-in 의미가 있는 target**: PolyGen validation/codegen이 실제 의미를 해석하는 위치입니다.

문법상 attachment point:

| 위치 | 예 |
|------|----|
| namespace | `@x namespace game { ... }` |
| top-level table/embed/enum | `@x table Item { ... }` |
| nested embed/enum | `@x embed Position { ... }` |
| regular field | `@x name: string;` |
| inline embed field | `@x data: embed { ... };` |
| inline enum field | `@x state: enum { ... };` |
| enum variant | `@x Active;` |

Unknown/custom annotation은 AST metadata로 보존됩니다. Built-in validation이 의미를
모르는 annotation은 기본 동작을 바꾸지 않으며, template이나 도구가 custom metadata로
해석할 수 있습니다.

### 1.3 Canonical built-in annotation target matrix

| 어노테이션 | 파라미터 | Built-in target | 설명 |
|----------|---------|-----------------|------|
| `@index` | `(field1, field2, ...)`, `unique: true` | table | exact key lookup/DB index 생성을 위한 table-level index |
| `@search` | `mode`, `n`, `min`, `normalize`, `name`, `target` | searchable field, inline enum field | Container/BinaryRef/Registry 검색 인덱스 |
| `@pack` | `separator: ","` | named/nested embed | embed 필드를 단일 문자열로 직렬화 |
| `@readonly` | 없음 | table | generated mutable container에서 쓰기/저장 제외 |
| `@soft_delete` | `"deleted_at"` 또는 `field: deleted_at` | table | 논리 삭제 필드 지정 |
| `@taggable` | 없음 | table | 태그 지원 표시. 유지 여부는 별도 검토 가능 |

Enum variant에도 annotation을 붙일 수 있지만, 현재 built-in annotation은 enum variant를
semantic target으로 사용하지 않습니다.

### 1.4 Removed / externalized annotation candidates

아래 항목은 기존 코드가 일부 지원하더라도 canonical `.poly` 문법에서는 제거하거나 별도
설정으로 옮깁니다.

| 기존 항목 | 기존 target | 대체/정리 방향 | 이유 |
|-----------|-------------|----------------|------|
| `@datasource` | namespace/table | output/source sidecar config | DB/Redis 산출물 선택은 schema 선언부보다 배포 설정에 가까움 |
| `@load` | table | `*.sources.toml` | CSV/JSON runtime path는 타입 선언이 아님 |
| `@cache` | namespace/table 의도 | cache/output config 또는 재설계된 table policy | data/output 설정과 섞여 있고 현재 validation target이 넓음 |
| `@link_rows` | table | `foreign_key(... ) as ...` relation으로 흡수 검토 | relation 원천을 FK로 단일화 |
| `@renamed_from` | draft | `.renames` 파일 | 현재 미구현이며 migration rename은 별도 파일로 처리 |

**사용 예제:**
```poly
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

CSV/JSON 경로는 `.sources.toml`에 작성합니다. 자세한 형식은
[sources-config.md](sources-config.md)를 참고합니다.
`@taggable`은 table 정의에만 사용할 수 있으며 인자를 받지 않습니다.

#### @taggable - 태그 지원 marker

`@taggable`은 table이 외부 tag 시스템과 연결될 수 있음을 표시하는 marker입니다. 인자를
받지 않고, field 구조나 데이터 무결성에는 영향을 주지 않습니다. tag 저장 방식이나 tag
데이터의 source path는 schema declaration 밖에서 결정하는 것이 원칙입니다.

```poly
@taggable
table Item {
    id: u32 primary_key;
    name: string;
}
```

#### @load 제거 방향

`@load(csv: "...", json: "...")`는 schema 안에 runtime file path를 넣는 방식이라
선언부/데이터 설정부 분리 원칙과 맞지 않습니다.

```poly
// 제거 대상
@load(csv: "data/items.csv")
table Item {
    id: u32 primary_key;
}
```

```toml
# canonical: schema sidecar 설정
[tables."game.Item".load]
csv = "data/items.csv"
```

#### @link_rows 제거/흡수 방향

`@link_rows(Target)`는 relation marker로 도입되었지만, 실제 관계의 원천은
`foreign_key(... ) as ...`가 더 명확합니다. relation 생성은 field-level FK에서
파생하는 방향으로 통일합니다.

```poly
// 제거/흡수 대상
@link_rows(Skill)
table PlayerSkill {
    player_id: u32 foreign_key(Player.id);
    skill_id: u32 foreign_key(Skill.id);
}
```

```poly
// canonical: FK가 relation의 원천
table PlayerSkill {
    player_id: u32 foreign_key(Player.id) as skills;
    skill_id: u32 foreign_key(Skill.id) as users;
}
```

#### @pack - embed 직렬화

```poly
@pack
embed Position {
    x: f32;
    y: f32;
}

@pack(separator: "|")
embed ColorAlpha {
    r: u8;
    g: u8;
    b: u8;
    a: u8;
}
```

`@pack`은 named/nested `embed` 정의에만 사용할 수 있습니다. `separator`를 생략하면
기본값은 `;`입니다. `separator`를 지정할 때는 named parameter `separator`만 허용되며,
값은 한 글자 문자열이어야 합니다. 코드 생성 대상 언어들이 char delimiter를 사용하므로
빈 문자열, 여러 글자 문자열, 작은따옴표, 백슬래시는 검증 단계에서 오류로 처리됩니다.
Go/Python/Kotlin/Swift 생성물은 `Pack`/`pack`, `Unpack`/`unpack`, `TryUnpack`/`try_unpack`
계열 API를 만들며, 숫자 unpack은 범위 오류와 `NaN`/`Inf`를 실패로 처리합니다.
Unreal 생성물은 packed embed USTRUCT에 `Pack`, `Unpack`, `TryUnpack`을 만들고 field count,
finite float, unsigned negative 입력을 방어합니다.

#### @datasource - 데이터소스 지정 (제거/외부화 대상)

`@datasource`는 현재 구현에서 DB/Redis 산출물 생성을 트리거하는 annotation입니다.
새 canonical 문법에서는 schema 선언부에서 제거하고, output/source sidecar config로
옮길 대상입니다.

```poly
@datasource("sqlite")
namespace data {
    @datasource("cache")
    table HotData { ... }

    @datasource(value: postgres)
    table AuditLog { ... }
}
```

| 파라미터 | 설명 |
|---------|------|
| `sqlite` | SQLite DDL/Accessor 대상 |
| `mysql` | MySQL DDL 대상 |
| `mariadb` | MariaDB DDL 대상 (`mysql` 템플릿 사용) |
| `postgresql` | PostgreSQL DDL 대상 |
| `postgres` | PostgreSQL alias |
| `redis` | Redis cache schema descriptor 대상 |
| `cache` | Redis cache schema descriptor alias |

**우선순위:** 테이블 직접 지정 > 가장 가까운 namespace > 상위 namespace > 기본값
`@datasource`는 namespace/table에만 사용할 수 있습니다. positional 값 1개 또는
`value: ...` named parameter 1개만 허용되며, 값은 식별자 또는 문자열이어야 합니다.

### 1.5 데이터/캐시 어노테이션

#### @cache - 캐시 전략 (재설계 대상)

```poly
@cache(strategy: full_load)
table ItemTable { ... }

@cache(strategy: on_demand, ttl: 300)
table Player { ... }

@cache("write_through")
table Account { ... }
```

| 전략 | 설명 | 용도 |
|------|------|------|
| `full_load` | 시작시 전체 로드 | 정적 데이터, 설정 테이블 |
| `on_demand` | 필요시 로드 | 유저 데이터 |
| `write_through` | 쓰기시 즉시 DB 반영 | 중요 데이터 |
| `write_back` | 지연 쓰기 (배치) | 로그, 통계 |

| 파라미터 | 타입 | 설명 |
|---------|------|------|
| `strategy` | 식별자 또는 문자열 | 캐시 전략 |
| `ttl` | 0 이상의 정수 | 만료 시간 (초) |

Redis descriptor 생성 시 `ttl`은 `ttlSeconds`로 출력됩니다. `strategy` 없이
`@cache(ttl: 300)`만 지정하면 Redis descriptor의 기본 전략은 `on_demand`입니다.
검증 단계에서 지원하지 않는 전략, 음수/비정수 TTL, 중복 strategy/ttl, 알 수 없는
파라미터는 오류로 처리됩니다.

의도상 `@cache`는 namespace/table 정책 metadata입니다. 다만 cache strategy와 datasource
선택은 output/source 설정에 가까워서, canonical `.poly` annotation으로 남길지 별도
cache/output config로 옮길지 재설계 대상입니다. 현재 validation은 target 제한도 넓어서
구현 정리 후보로 둡니다.

#### @readonly - 읽기 전용

```poly
@readonly
table ItemTable { ... }
```

`@readonly`는 table 정의에만 사용할 수 있으며 인자를 받지 않습니다.

- `SaveChanges()`에서 무시
- 수정 시도시 예외 발생

#### @soft_delete - 논리 삭제

```poly
@soft_delete("deleted_at")
table Player {
    deleted_at: timestamp?;
}
```

`@soft_delete`는 table 정의에만 사용할 수 있습니다. 지정한 필드는 같은 table의 regular field여야 하며
타입은 `timestamp?`이어야 합니다. positional field name 또는 `field: deleted_at` named parameter를
허용합니다.

- DELETE → `UPDATE deleted_at = NOW()`
- SELECT시 자동으로 `deleted_at IS NULL` 조건 추가

#### @renamed_from - 이름 변경 (Draft / 미구현)

```poly
@renamed_from("OldPlayer")
table Player {
    @renamed_from("user_name")
    name: string;
}
```

이 annotation은 현재 built-in annotation target matrix에 포함하지 않습니다. 이름 변경
마이그레이션은 현재 `.renames` 파일을 사용합니다.

### 1.6 @search - 필드별 역색인 (C#/C++/Rust/TypeScript/Go/Python/Kotlin/Swift Container, Unreal Registry, C#/C++/TypeScript/Go/Python/Kotlin/Swift BinaryRef 구현)

`@search`는 각 searchable field에 붙는 역색인(inverted index) 생성 힌트입니다. 데이터 무결성 제약이
아니라 Container/BinaryRef 같은 산출물에서 검색용 파생 인덱스를 만들기 위한 metadata이므로
attribute/constraint가 아니라 field-level annotation으로 둡니다.

#### 지원 문법

| 문법 | 의미 | 기본/제약 |
|------|------|-----------|
| `@search` | 타입별 기본 검색 인덱스 생성 | string은 `ngram`, 나머지 scalar/enum은 `exact` |
| `@search(n: 3)` | string n-gram 크기 지정 | `mode` 생략 시 string 기본 `ngram` |
| `@search(mode: exact)` | 정확히 같은 값으로 역색인 | string/number/bool/enum/timestamp |
| `@search(mode: ngram)` | n-gram 역색인 | string/string? 전용 |
| `@search(mode: word)` | 단어 토큰 역색인 | string/string? 전용 |
| `@search(normalize: lower_trim)` | 정규화 정책 지정 | string 계열 mode에 적용 |
| `@search(name: "DisplayName")` | 생성 API/index 이름 지정 | 실제 문자열 이름이므로 따옴표 사용 |

옵션 값이 enum-like인 경우 따옴표 없는 identifier를 권장합니다.

```poly
table Item {
    id: u32 primary_key;

    @search
    name: string;

    @search(n: 3, normalize: lower_trim)
    description: string?;

    @search(mode: exact)
    item_code: string;

    @search
    grade: u8;

    @search
    rarity: Rarity;

    @search
    enabled: bool;
}
```

#### 타입별 기본 동작

| field 타입 | 기본 mode | 허용 mode | 색인 제외 조건 |
|-----------|-----------|-----------|----------------|
| `string` | `ngram` | `exact`, `ngram`, `word` | 빈 문자열은 mode 정책에 따름 |
| `string?` | `ngram` | `exact`, `ngram`, `word` | `null` |
| 정수 타입 | `exact` | `exact` | optional이면 `null` |
| `bool` | `exact` | `exact` | optional이면 `null` |
| named enum field | `exact` | `exact` | optional이면 `null` |
| inline enum field | `exact` | `exact` | optional이면 `null` |
| `timestamp` | `exact` | `exact` | optional이면 `null` |
| float 타입 | 없음 | 향후 `bucket`/`range` 검토 | v1에서는 금지 권장 |
| `bytes` | 없음 | 없음 | 항상 금지 |
| embed/struct 참조 | 없음 | 없음 | 항상 금지 |
| array | 없음 | 향후 `each: true` 검토 | v1에서는 금지 권장 |

#### 옵션

| 옵션 | 타입 | 기본값 | 설명 |
|------|------|--------|------|
| `mode` | identifier/string | 타입별 기본값 | `exact`, `ngram`, `word` |
| `n` | integer | `2` | `ngram` token 크기 |
| `min` | integer | `n` | 검색어 최소 길이 |
| `normalize` | identifier/string | `lower_trim` for string | `none`, `lower`, `trim`, `lower_trim` |
| `name` | string/identifier | `By<Field>` 또는 `Search<Field>` | 생성 API/index 이름 |
| `target` | identifier/string | `csharp` | 산출물 제한. `csharp`, `csharp_container`, `csharp_binary_ref`, `rust`, `rust_container`, `cpp`, `cpp_container`, `cpp_binary_ref`, `typescript`, `typescript_container`, `typescript_binary_ref`, `go`, `go_container`, `go_binary_ref`, `python`, `python_container`, `python_binary_ref`, `kotlin`, `kotlin_container`, `kotlin_binary_ref`, `swift`, `swift_container`, `swift_binary_ref`, `unreal`, `unreal_registry` |

#### 생성 API

C# 일반 Container와 BinaryRef는 같은 `SearchBy<Field>` API를 생성합니다. TypeScript
Container/BinaryRef는 `searchBy<Field>` API를 생성하고, Go Container/BinaryRef는 `SearchBy<Field>` API를
생성합니다. C++ Container/BinaryRef, Rust Container, Python Container/BinaryRef는 언어 명명 규칙에 맞춰
`search_by_<field>` API를 생성합니다. Kotlin/Swift Container/BinaryRef는 `searchBy<Field>` API를
생성합니다. Unreal Registry는 `Search<Table>By<Field>` Blueprint query helper를 생성합니다.

- 일반 Container: row를 `Add`/`add_row`할 때 메모리 postings를 갱신하고 row 객체 참조를 반환합니다.
- BinaryRef: 파일에 row ordinal 기반 postings를 저장하고 lazy row ref를 반환합니다.

BinaryRef 파일에는 row offset 대신 row ordinal 기반 postings를 저장합니다.

```text
token/value -> row ordinal[]
row ordinal -> rowOffsets[ordinal] -> RowRef
```

예상 API:

```csharp
ctx.Items.SearchByName("화염검");      // ngram/word
ctx.Items.SearchByItemCode("ITEM_001"); // exact string
ctx.Items.SearchByGrade(5);             // exact number
ctx.Items.SearchByRarity(Rarity.Rare);  // exact enum
ctx.Items.SearchByEnabled(true);        // exact bool
```

`ngram` 검색은 query를 같은 정책으로 token화한 뒤 postings를 교집합(AND)으로 결합합니다.
`word`도 기본은 AND 검색으로 두고, OR 검색은 API 확장 시 별도 메서드로 추가합니다.

---

## 2. Field Attributes / Constraints

### 2.1 문법

```pest
constraint      = { primary_key | unique | max_length | default_val | range_val | regex_val | foreign_key_val | auto_create | auto_update }
primary_key     = { "primary_key" }
unique          = { "unique" }
max_length      = { "max_length" ~ "(" ~ INTEGER ~ ")" }
default_val     = { "default" ~ "(" ~ literal ~ ")" }
range_val       = { "range" ~ "(" ~ literal ~ "," ~ literal ~ ")" }
regex_val       = { "regex" ~ "(" ~ STRING_LITERAL ~ ")" }
foreign_key_val = { "foreign_key" ~ "(" ~ path ~ ")" ~ ("as" ~ IDENT)? }
auto_create     = { "auto_create" ~ ("(" ~ timezone ~ ")")? }
auto_update     = { "auto_update" ~ ("(" ~ timezone ~ ")")? }
```

**특징:**
- 필드 타입 뒤에 공백으로 구분하여 나열
- `@` 접두사 없음
- 여러 제약조건 조합 가능
- inline enum field도 제약조건이 필요하면 `state: enum { Todo; Done; } default(Todo);`처럼 작성할 수 있으며, AST에서는 regular field의 inline enum 타입으로 정규화됩니다. 생성 enum 이름은 field PascalCase에 `Enum`을 붙인 `StateEnum` 형식입니다. 이 이름은 생성 코드 내부 이름일 뿐 `.poly`의 다른 필드에서 `Task.StateEnum`처럼 참조할 수 없습니다. 같은 이름의 named enum을 가릴 수 있는 경우에는 validation 오류로 거부합니다.
- inline embed field의 생성 embed 이름은 field PascalCase에 `Embed`를 붙인 `ProfileEmbed` 형식입니다. 이 이름도 생성 코드 내부 이름일 뿐 `.poly`의 다른 필드에서 `User.ProfileEmbed`처럼 참조할 수 없습니다. 같은 이름의 named type을 가릴 수 있는 경우에는 validation 오류로 거부합니다.

### 2.2 구현된 field attributes / constraints

| Attribute | Category | 문법 | 파라미터 | 상태 | 설명 |
|-----------|----------|------|---------|------|------|
| `primary_key` | identity | `primary_key` | 없음 | canonical | 기본 키 |
| `unique` | uniqueness | `unique` | 없음 | canonical | 단일 field 고유 값 제약 |
| `max_length` | validation | `max_length(n)` | 정수 | canonical | 문자열/바이트 최대 길이 |
| `range` | validation | `range(min, max)` | 리터럴 2개 | canonical | 값 범위 제한 |
| `regex` | validation | `regex("pattern")` | 문자열 | canonical | 정규식 검증 |
| `default` | default/lifecycle | `default(value)` | 리터럴 | canonical | 기본값 |
| `auto_create` | default/lifecycle | `auto_create[(timezone)]` | timezone(선택) | canonical | 생성 시각 자동 설정 |
| `auto_update` | default/lifecycle | `auto_update[(timezone)]` | timezone(선택) | canonical | 갱신 시각 자동 설정 |
| `foreign_key` | relation | `foreign_key(Table.field) [as alias]` | 경로, 별칭(선택) | canonical | 외래 키 참조 |

제거 대상 field attribute:

| Attribute | 기존 문법 | 대체 |
|-----------|-----------|------|
| `index` | `name: string index;` | `@index(name)` |

**검증 규칙:**
- `primary_key`는 table당 하나만 허용됩니다. optional/array 필드, `bytes`, struct/embed 필드에는 사용할 수 없습니다.
- `unique`는 optional scalar 필드에는 사용할 수 있지만, array 필드, `bytes`, struct/embed 필드에는 사용할 수 없습니다.
- `index` field constraint는 제거되었습니다. 새 schema에서는 `@index(field)`로 작성하며, 파서는 `name: string index;`를 거부합니다.
- `max_length`는 `string`/`bytes` 필드에만 사용할 수 있으며 값은 1 이상이어야 합니다.
- `default`는 배열 필드에는 사용할 수 없습니다. 기본 타입은 타입에 맞는 리터럴만 허용하고, 정수 기본값은 해당 정수 타입의 범위 안에 있어야 합니다. `range`와 함께 쓰는 경우 기본값도 범위 안에 있어야 합니다.
- `range`는 숫자 필드에만 사용할 수 있습니다. 정수 필드는 정수 bound만 허용하며, unsigned 정수의 최소값은 0 이상이어야 합니다.
- `regex`는 `string` 필드에만 사용할 수 있고, 패턴은 Rust `regex` 문법으로 컴파일 가능해야 합니다.
- `foreign_key`는 `Table.field` 형태여야 하며, 대상은 실제 table의 `primary_key` 또는 `unique` regular field여야 합니다. FK 필드와 대상 필드는 배열이 아니어야 하고, optional 여부를 제외한 타입이 같아야 합니다.
- `auto_create`와 `auto_update`는 `timestamp` 필드에만 사용할 수 있습니다. table당 `auto_create`는 하나만 허용하고, `auto_update`는 여러 필드에 사용할 수 있습니다.
- timezone 인자는 생략 시 UTC이며, `utc`, `local`, UTC offset(`+9`, `-5`, `+5:30`), 문자열 timezone name을 지원합니다.

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

#### auto_create / auto_update - 자동 타임스탬프

```poly
table Player {
    created_at: timestamp auto_create;   // INSERT시 자동 설정
    updated_at: timestamp auto_update;   // UPDATE시 자동 갱신
    synced_at: timestamp auto_update(+9);
}
```

파싱/AST/IR/검증은 구현되어 있습니다. 코드 생성은 타겟별로 차이가 있으며, 예를 들어 C# DataTable은
timezone 설정을 반영해 `auto_create` 값을 `Add()` 시점에 설정하고, C#/Rust/C++/TypeScript 계열 템플릿은 `auto_update`
helper를 생성합니다. DB DDL은 MySQL/MariaDB에서 `auto_create`/`auto_update`를 반영하고,
PostgreSQL/SQLite에서도 `auto_create` 기본값과 `auto_update` 기본값/trigger를 생성합니다.

---

## 3. 인덱스 설계 (통일)

`@index`는 정확히 일치하는 key lookup을 위한 구조입니다. 테이블의 특정 field 조합을
key로 보고, generated container에서는 `ByName(...)`, `ByGuildIdLevel(...)` 같은 조회
API를 만들고, DB 타겟에서는 일반/복합/unique index DDL로 이어집니다.

`@search`는 검색용 역색인입니다. 문자열을 n-gram 또는 word token으로 쪼개거나,
enum/scalar 값을 exact search bucket으로 묶어서 `SearchByName(...)` 같은 검색 API를
만듭니다. 데이터 무결성 제약이 아니며, DB unique/index와도 같은 의미가 아닙니다.

| 구분 | `@index` | `@search` |
|------|----------|-----------|
| 붙는 위치 | table | field |
| 목적 | exact key lookup, DB index | generated search API, inverted index |
| 입력 | field 이름 목록 | mode/options |
| 결과 API 예 | `ByCode("ITEM_001")`, `ByGuildIdLevel(3, 10)` | `SearchByName("fire")`, `SearchByDescription("ice sword")` |
| 복합 조건 | field 목록으로 표현 | 검색 옵션과 query token으로 표현 |
| unique 의미 | `unique: true`로 복합 unique 표현 가능 | 없음 |
| DB DDL 의미 | 있음 | 없음 또는 target별 검색 구조 |

같은 field에 둘 다 필요할 수 있습니다. 예를 들어 `item_code`는 정확한 코드 조회를 위해
`@index(item_code, unique: true)`가 맞고, `name`/`description`은 사람이 입력하는 검색어를
받기 위해 `@search`가 맞습니다.

### 3.1 인덱스 생성 방법

**Canonical `@index` 사용:**

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

`@index`는 table 정의에만 사용할 수 있습니다. positional 인자는 같은 table의 regular field를
참조해야 하며, 한 annotation 안에서 같은 필드를 중복 지정할 수 없습니다. 인덱스 필드는
array/bytes/struct/embed 타입일 수 없고, scalar 또는 enum 타입이어야 합니다. named parameter는
`unique`만 허용되며 값은 boolean 또는 `true`, `false`, `1`, `0`입니다.

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

### 3.6 제거 대상: 기존 `index` 제약조건

**제거 대상:**
```poly
table Player {
    name: string index;
}
```

**Canonical:**
```poly
@index(name)
table Player {
    name: string;
}
```

**이유:**
- 복합 인덱스 지원 불가 문제 해결
- 테이블 레벨에서 인덱스 관리 일원화
- 새 schema에서는 `index` field constraint를 사용하지 않습니다.
- 파서는 더 이상 `index` field constraint를 받지 않으며, table-level `@index(...)`로 통일합니다.
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
| `@taggable` | `[Taggable]` |
| `@index(name)` | 인덱스 딕셔너리 생성 |

```csharp
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

| 어노테이션 | 스펙 상태 | 파싱 | AST | IR | 템플릿 |
|----------|----------|:---:|:---:|:---:|:------:|
| `@index` | canonical | ✅ | ✅ | ✅ | ✅ |
| `@search` | canonical | ✅ | ✅ | ✅ | ⚠️ C# Container/BinaryRef, C++ Container/BinaryRef, Rust Container, TypeScript Container/BinaryRef, Go Container/BinaryRef, Python/Kotlin/Swift Container/BinaryRef, Unreal Registry |
| `@pack` | canonical | ✅ | ✅ | ✅ | ✅ C#/C++/Rust/TypeScript/Go/Python/Kotlin/Swift/Unreal |
| `@readonly` | canonical | ✅ | ✅ | ✅ | ✅ |
| `@soft_delete` | canonical | ✅ | ✅ | ✅ | ✅ |
| `@taggable` | current | ✅ | ✅ | ✅ | ✅ |
| `@load` | 제거/외부화 대상 | ✅ | ✅ | ✅ | ✅ |
| `@datasource` | 제거/외부화 대상 | ✅ | ✅ | ✅ | ✅ |
| `@cache` | 재설계 대상 | ✅ | ✅ | ✅ | ✅ |
| `@link_rows` | 제거/흡수 대상 | ✅ | ✅ | ✅ | ✅ |
| `@renamed_from` | 미구현 | ❌ | ❌ | ❌ | ❌ |

### 5.2 어트리뷰트

| 어트리뷰트 | 스펙 상태 | 파싱 | AST | IR | C# | MySQL |
|-----------|----------|:---:|:---:|:---:|:---:|:-----:|
| `primary_key` | canonical | ✅ | ✅ | ✅ | ✅ | ✅ |
| `unique` | canonical | ✅ | ✅ | ✅ | ✅ | ✅ |
| `max_length` | canonical | ✅ | ✅ | ✅ | ✅ | ✅ |
| `default` | canonical | ✅ | ✅ | ✅ | ⚠️ | ⚠️ |
| `range` | canonical | ✅ | ✅ | ✅ | ✅ | ✅ |
| `regex` | canonical | ✅ | ✅ | ✅ | ✅ | ✅ |
| `foreign_key` | canonical | ✅ | ✅ | ✅ | ✅ | ⚠️ |
| `auto_create` | canonical | ✅ | ✅ | ✅ | ⚠️ | ✅ |
| `auto_update` | canonical | ✅ | ✅ | ✅ | ⚠️ | ✅ |
| `index` | 제거됨 | ❌ | ❌ | ❌ | ❌ | ❌ |

**범례:** ✅ 완료 | 🚧 진행중 | ⚠️ 부분 구현 | ❌ 미구현

---

## 6. 파일 위치

| 구성 요소 | 파일 |
|----------|------|
| 어노테이션 문법 | `src/polygen.pest` |
| 어트리뷰트 문법 | `src/polygen.pest` |
| AST 어노테이션 | `src/ast_model.rs` |
| AST 어트리뷰트 | `src/ast_model.rs` |
| 어노테이션 파싱 | `src/ast_parser/metadata.rs` |
| 어트리뷰트 파싱 | `src/ast_parser/fields.rs` |
| 공통 토큰 파싱 helper | `src/ast_parser/helpers.rs` |
| IR 어노테이션 | `src/ir_model.rs` |
| IR 어트리뷰트 | `src/ir_model.rs` |
| Rhai 등록 | `src/rhai/registry.rs` |

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

*최종 업데이트: 2026-06-03*
