# PolyGen Schema Language

> 상태: 초안 (2026-06-06)

이 문서는 `.poly` 파일을 사람이 읽고 작성하기 쉽게 설명하는 문법 가이드입니다.
정확한 parser 규칙의 원본은 `src/polygen.pest`이고, 어노테이션과 제약조건의 세부
의미는 [schema-annotations.md](schema-annotations.md)를 기준으로 합니다.

---

## 1. 파일 구조

`.poly` 파일은 file import와 top-level definition으로 구성됩니다.

```poly
import "common_types.poly";

namespace game.data {
    enum Rarity {
        Common = 1;
        Rare = 2;
    }

    table Item {
        id: u32 primary_key;
        name: string max_length(80);
        rarity: Rarity default(Common);
    }
}
```

Top-level definition은 다음 네 가지입니다.

| 정의 | 용도 |
|------|------|
| `namespace` | 타입을 논리적인 이름 공간으로 묶음 |
| `table` | 생성 대상 데이터 모델 |
| `embed` | 여러 table에서 재사용하는 값 객체 |
| `enum` | 이름 있는 enum 타입 |

### 1.1 선언부와 데이터 설정부

권장 구조는 schema declaration과 data/source 설정을 분리하는 것입니다.

`*.poly`는 타입, 필드, 제약조건, 관계 같은 선언부를 담습니다.

```poly
namespace demo.catalog {
    enum Rarity {
        Common = 1;
        Rare = 2;
        Epic = 3;
    }

    table Item {
        id: u32 primary_key;
        name: string max_length(80);
        rarity: Rarity default(Common);
        price: u32 default(0);
    }
}
```

CSV/JSON load path 같은 runtime data 설정은 sidecar `*.sources.toml`에 둡니다.

```toml
[tables."demo.catalog.Item".load]
csv = "examples/data/items.csv"
json = "examples/data/items.json"
```

기존 구현에는 DB/Redis 산출물 선택용 `@datasource("sqlite")` annotation이 남아 있지만,
canonical 문법에서는 이를 선언부에서 제외하고 별도 output/source 설정으로 옮기는
방향을 우선으로 둡니다.

---

## 2. 주석과 메타데이터

PolyGen은 줄 주석과 블록 주석을 지원합니다.

```poly
// 일반 주석
/// 문서 주석처럼 사용할 수 있는 줄 주석
/* 블록 주석 */
```

정의나 필드 앞에 있는 주석은 metadata로 붙을 수 있습니다.

```poly
/// 플레이어 데이터
@taggable
table Player {
    /// 고유 ID
    id: u32 primary_key;
}
```

Enum variant 뒤의 같은 줄 `//` 주석은 해당 variant의 inline comment로 보존됩니다.

```poly
enum Currency {
    Cash = 1; // 현금
    Gem = 2;  // 유료 재화
}
```

---

## 3. Import

### 3.1 File Import

파일 import는 다른 `.poly` 파일을 포함합니다.

```poly
import "common_types.poly";
import "shared/economy.poly";
```

문자열 경로를 사용하며 세미콜론으로 끝납니다.

### 3.2 Namespace Import

Namespace 안에서는 타입 참조용 import를 선언할 수 있습니다.

```poly
namespace game.character {
    import game.common.*;
    import game.item.ItemType;

    table Player {
        element: Element;
        favorite_type: ItemType;
    }
}
```

Wildcard import는 `.*`를 붙입니다.

---

## 4. Namespace

Namespace는 `.`으로 구분된 path를 이름으로 사용합니다.

```poly
namespace game.character {
    table Player {
        id: u32 primary_key;
    }
}
```

Namespace는 중첩할 수 있습니다.

```poly
namespace game {
    namespace character {
        table Player {
            id: u32 primary_key;
        }
    }
}
```

위 예제의 table FQN은 `game.character.Player`입니다.

---

## 5. Table

Table은 생성 대상 데이터 모델입니다.

```poly
table Player {
    id: u32 primary_key;
    name: string max_length(30);
    level: u16 default(1) range(1, 100);
}
```

Table body에는 field, nested enum, nested embed를 둘 수 있습니다.

```poly
table Monster {
    enum Status {
        Active;
        Inactive;
    }

    embed DropRule {
        item_id: u32;
        chance: f32 range(0.0, 1.0);
    }

    id: u32 primary_key;
    status: Status;
    drops: DropRule[];
}
```

---

## 6. Embed

Embed는 재사용 가능한 값 객체입니다. Table과 같은 field 문법을 사용하지만, 독립적인
row/table이라기보다 다른 타입 안에 포함되는 구조입니다.

```poly
embed Position {
    x: f32;
    y: f32;
}

table SpawnPoint {
    id: u32 primary_key;
    position: Position;
}
```

Embed도 namespace 안에 둘 수 있고, table 안에 nested embed로 둘 수도 있습니다.

```poly
table Monster {
    embed DropItem {
        item_id: u32;
        count: u32 default(1);
    }

    drops: DropItem[];
}
```

---

## 7. Enum

Enum은 이름 있는 선택지 타입입니다.

```poly
enum ItemType {
    Weapon;
    Armor;
    Potion;
}
```

Variant에는 정수 값을 명시할 수 있습니다.

```poly
enum Rarity {
    Common = 1;
    Rare = 2;
    Epic = 3;
}
```

Variant 구분자는 `;` 또는 `,`를 권장합니다. 한 enum 안에서는 한 스타일로 맞추는 것이
좋습니다.

```poly
enum Status {
    Active;
    Inactive;
    Banned;
}
```

---

## 8. Field

Field는 table 또는 embed body 안에 작성합니다.

```poly
name: Type [cardinality] [constraint...] [= field_number];
```

예:

```poly
id: u32 primary_key = 1;
name: string max_length(30) = 2;
description: string?;
tags: string[];
rarity: Rarity default(Common);
owner_id: u32 foreign_key(Player.id) as items;
```

Field number는 serialization ordering을 위한 선택 값입니다.

```poly
table Item {
    id: u32 primary_key = 1;
    name: string = 2;
}
```

---

## 9. Type

### 9.1 Primitive Type

지원 primitive type은 다음과 같습니다.

| 그룹 | 타입 |
|------|------|
| 문자열/바이너리 | `string`, `bytes` |
| 정수 | `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64` |
| 실수 | `f32`, `f64` |
| 기타 | `bool`, `timestamp` |

### 9.2 Named Type

사용자 정의 타입은 이름 또는 FQN으로 참조합니다.

```poly
namespace game.common {
    embed StatBlock {
        hp: u32;
        attack: u32;
    }
}

namespace game.character {
    table Player {
        stats: game.common.StatBlock;
    }
}
```

같은 namespace 안에서는 짧은 이름을 사용할 수 있습니다.

```poly
namespace game.item {
    enum ItemType {
        Weapon;
        Armor;
    }

    table Item {
        item_type: ItemType;
    }
}
```

### 9.3 Cardinality

Cardinality는 타입 뒤에 붙입니다.

| 표기 | 의미 |
|------|------|
| 없음 | 단일 필수 값 |
| `?` | optional 값 |
| `[]` | list/array 값 |

```poly
name: string;
description: string?;
tags: string[];
positions: game.common.Position[];
```

`?`와 `[]`는 동시에 붙이지 않습니다. Optional list 같은 모델이 필요하면 별도 embed나
타겟별 표현 정책을 먼저 설계해야 합니다.

---

## 10. Inline Embed Field

Inline embed는 field 자리에서 익명 구조를 바로 정의합니다.

```poly
table Monster {
    drop_item: embed {
        item_id: u32;
        chance: f32 range(0.0, 1.0);
    };
}
```

Array cardinality도 사용할 수 있습니다.

```poly
table Monster {
    drop_items: embed {
        item_id: u32;
        count: u32 default(1);
    }[];
}
```

Inline embed는 field 자리에서 구조를 선언하는 shorthand입니다. 생성 embed 이름은 field
PascalCase에 `Embed`를 붙입니다. 위 `drop_item` field의 생성 이름은 `DropItemEmbed`이고,
C# 같은 타겟에서는 `Monster.DropItemEmbed`처럼 nested struct/class로 생성될 수 있습니다.

중요한 점은 이 이름이 `.poly`의 public type이 아니라는 것입니다. 즉 다른 field에서
`Monster.DropItemEmbed`처럼 참조할 수 없습니다. 또한 생성 이름이 schema 안의 named type
짧은 이름과 충돌하면 target code에서 shadowing이 생길 수 있으므로 validation 오류로
거부합니다.

```poly
table MonsterAudit {
    // 허용하지 않음: Monster.DropItemEmbed는 스키마에서 참조 가능한 타입이 아님
    item: Monster.DropItemEmbed;
}
```

여러 곳에서 재사용하거나 명시적으로 참조할 타입이면 inline embed보다 named `embed`로
분리해야 합니다.

---

## 11. Inline Enum Field

Inline enum은 특정 field에서만 쓰는 enum을 field 자리에서 직접 정의합니다.

```poly
table Task {
    state: enum {
        Todo;
        Done;
    };
}
```

Constraint가 필요한 경우 enum body 뒤에 붙입니다.

```poly
table Task {
    state: enum {
        Todo;
        Done;
    } default(Todo);
}
```

Cardinality도 enum body 뒤에 붙입니다.

```poly
table Task {
    previous_states: enum {
        Todo;
        Doing;
        Done;
    }[];
}
```

생성 enum 이름은 field PascalCase에 `Enum`을 붙입니다.

```poly
table Task {
    state: enum {
        Todo;
        Done;
    };
}
```

위 field의 생성 enum 이름은 `StateEnum`입니다. C# 같은 타겟에서는 `Task.StateEnum`
처럼 nested enum으로 생성될 수 있습니다.

중요한 점은 이 이름이 `.poly`의 public type이 아니라는 것입니다.
또한 생성 이름이 schema 안의 named enum 짧은 이름과 충돌하면 target code에서
shadowing이 생길 수 있으므로 validation 오류로 거부합니다.

```poly
table TaskAudit {
    // 허용하지 않음: Task.StateEnum은 스키마에서 참조 가능한 타입이 아님
    state: Task.StateEnum;
}
```

다른 곳에서 재사용해야 하는 enum이면 named enum으로 빼야 합니다.

```poly
enum TaskState {
    Todo;
    Done;
}

table Task {
    state: TaskState default(Todo);
}

table TaskAudit {
    state: TaskState;
}
```

---

## 12. Metadata And Policy

Metadata는 schema declaration에 붙는 부가 정보입니다. PolyGen 문법에서는 `@name(...)`
형태의 annotation으로 표현하고, 실제 의미는 annotation 이름과 target에 따라 달라집니다.

### 12.1 Attachment Points

문법상 annotation은 다음 위치에 붙을 수 있습니다.

| 위치 | 예 |
|------|----|
| namespace 앞 | `@x namespace game { ... }` |
| top-level table/embed/enum 앞 | `@x table Item { ... }` |
| nested embed/enum 앞 | `@x embed Position { ... }` |
| regular field 앞 | `@x name: string;` |
| inline embed field 앞 | `@x payload: embed { ... };` |
| inline enum field 앞 | `@x state: enum { ... };` |
| enum variant 앞 | `@x Active;` |

```poly
namespace demo.catalog {
    @index(rarity)
    @readonly
    table Item {
        @search
        name: string;

        @search(mode: exact)
        rarity: Rarity;
    }
}
```

### 12.2 Custom Annotation

Parser는 알 수 없는 annotation도 metadata로 보존합니다. Built-in validation과 codegen이
의미를 모르는 annotation은 기본 동작을 바꾸지 않지만, template이나 도구가 custom
metadata로 해석할 수 있습니다.

```poly
@designer_note("shown in editor")
table Item {
    @ui(label: "Display Name")
    name: string;
}
```

Enum variant에도 custom metadata를 붙일 수 있습니다. 현재 built-in annotation은 enum
variant를 의미 있는 target으로 사용하지 않습니다.

```poly
enum Currency {
    @display_name("Gold")
    Gold = 1;
}
```

### 12.3 Canonical Built-in Annotation Targets

Canonical `.poly` 문법에 남기는 built-in annotation은 schema declaration 자체에
가까운 기능만 포함합니다.

| Annotation | Target | 설명 |
|------------|--------|------|
| `@index` | table | exact key lookup과 DB index 생성을 위한 table-level index |
| `@search` | searchable field, inline enum field | text/exact 검색 API를 위한 generated search index |
| `@pack` | named/nested embed | embed 값을 compact string으로 pack/unpack하는 helper 생성 |
| `@readonly` | table | generated mutable container에서 쓰기/저장 대상에서 제외 |
| `@soft_delete` | table | delete를 timestamp update로 바꾸는 logical delete 정책 |
| `@taggable` | table | tag 지원 marker. 계속 유지할지 별도 검토 가능 |

세부 parameter와 validation 규칙은 [schema-annotations.md](schema-annotations.md)를
기준으로 합니다.

### 12.4 Index And Search

`@index`와 `@search`는 둘 다 조회용 구조를 만들지만 목적이 다릅니다.

| 구분 | `@index` | `@search` |
|------|----------|-----------|
| 목적 | 정확한 key lookup | 검색 API와 inverted index |
| 위치 | table 앞 | field 앞 |
| 입력 | field 이름 목록 | 검색 mode/options |
| 대표 사용 | `ByGuildId(10)`, `ByName("sword")` | `SearchByName("fire")`, `SearchByDescription("ice")` |
| 데이터 의미 | equality key, unique/composite key, DB index | text token/ngram/word/exact search |
| 제약 여부 | uniqueness/index metadata와 연결 가능 | 데이터 무결성 제약 아님 |

```poly
@index(item_code, unique: true)
@index(category_id, rarity)
table Item {
    item_code: string;
    category_id: u32;
    rarity: Rarity;

    @search(mode: ngram, n: 3)
    name: string;

    @search(mode: word, normalize: lower_trim)
    description: string?;
}
```

### 12.5 Removed Or Externalized Metadata

아래 항목은 canonical `.poly` 문법에서 빼고, 별도 설정이나 기존 attribute로 대체합니다.
현재 코드가 일부를 계속 받아들이더라도 새 스키마에서는 사용하지 않는 방향입니다.

| 기존 항목 | 대체/정리 방향 | 이유 |
|-----------|----------------|------|
| `@datasource` | output/source sidecar config | schema declaration과 배포/출력 설정 분리 |
| `@load` | `*.sources.toml` | CSV/JSON runtime path는 schema declaration이 아님 |
| `@cache` | cache/output config 또는 table policy로 재설계 | datasource/output 설정과 섞여 있고 target 제한이 넓음 |
| `@link_rows` | `foreign_key(... ) as ...` relation으로 흡수 검토 | relation 원천을 FK로 단일화 |
| `index` field attribute | `@index(field)` | index 문법을 table-level로 통일 |

---

## 13. Field Attributes And Constraints

Field attribute/constraint는 field type 뒤에 공백으로 나열합니다. 데이터 무결성,
기본값, lifecycle, relation 같은 field 자체의 의미를 정의합니다.

```poly
table Player {
    id: u32 primary_key;
    name: string unique max_length(30);
    level: u16 default(1) range(1, 100);
    email: string? regex("^[^@]+@[^@]+$");
    guild_id: u32 foreign_key(Guild.id);
    owner_id: u32 foreign_key(User.id) as owned_items;
    created_at: timestamp auto_create;
    updated_at: timestamp auto_update(utc);
}
```

### 13.1 Identity

| Attribute | 예 | 의미 |
|-----------|----|------|
| `primary_key` | `id: u32 primary_key;` | table의 primary identity field |

### 13.2 Uniqueness And Indexing

| Attribute | 예 | 의미 |
|-----------|----|------|
| `unique` | `name: string unique;` | 단일 field unique constraint |

복합 index나 복합 unique index는 table-level `@index`를 사용합니다.

```poly
@index(name)
@index(region, shard)
@index(server_id, external_id, unique: true)
table Player {
    name: string;
    region: string;
    shard: u16;
    server_id: u32;
    external_id: string;
}
```

### 13.3 Validation

| Attribute | 예 | 의미 |
|-----------|----|------|
| `max_length` | `name: string max_length(80);` | string/bytes length bound |
| `range` | `chance: f32 range(0.0, 1.0);` | numeric range bound |
| `regex` | `email: string regex("...");` | string regex validation |

### 13.4 Defaults And Lifecycle

| Attribute | 예 | 의미 |
|-----------|----|------|
| `default` | `level: u16 default(1);` | generated/runtime default value |
| `auto_create` | `created_at: timestamp auto_create;` | create timestamp lifecycle |
| `auto_update` | `updated_at: timestamp auto_update(local);` | update timestamp lifecycle |

### 13.5 Relations

| Attribute | 예 | 의미 |
|-----------|----|------|
| `foreign_key` | `user_id: u32 foreign_key(User.id);` | reference another table key |
| `foreign_key ... as` | `user_id: u32 foreign_key(User.id) as items;` | generated reverse relation alias |

### 13.6 Removed Legacy Field Attributes

`index` field attribute는 문법에서 제거되었습니다. 파서는 더 이상
`name: string index;`를 받지 않으며, 새 스키마에서는 항상 table-level
`@index(...)`를 사용합니다.

```poly
// 제거 대상
table Player {
    name: string index;
}

// canonical
@index(name)
table Player {
    name: string;
}
```

---

## 14. Literal

Literal은 annotation argument와 constraint value에서 사용합니다.

| Literal | 예 |
|---------|----|
| String | `"data/items.csv"` |
| Integer | `123`, `-5` |
| Float | `3.14`, `-0.5` |
| Boolean | `true`, `false` |
| Identifier | `sqlite`, `Todo`, `on_demand` |

Enum default는 보통 identifier literal을 사용합니다.

```poly
enum RecurrenceType {
    None = 0;
    Daily = 1;
}

table Schedule {
    recurrence: RecurrenceType default(None);
}
```

---

## 15. 이름과 경로

Identifier는 알파벳 또는 `_`로 시작하고, 이후에는 알파벳/숫자/`_`를 사용할 수
있습니다.

```text
IDENT = letter_or_underscore (letter_or_digit_or_underscore)*
```

Path는 identifier를 `.`으로 연결합니다.

```poly
game.character.Player
game.common.StatBlock
```

Schema 안에서 public type으로 참조할 수 있는 것은 named `table`, named `embed`,
named `enum`, 그리고 table/embed body 안에 직접 선언한 nested `embed`/`enum`입니다.
Inline enum에서 생성되는 `StateEnum` 같은 이름은 target code를 위한 내부 이름으로
취급하며 `.poly` 타입 참조에는 등록하지 않습니다.

---

## 16. Renames 파일

`.renames` 파일은 schema migration에서 이름 변경을 알리는 별도 파일입니다.
주석은 `#`를 사용합니다.

```text
# table rename
Player -> User;

# field rename
User.user_name -> name;

# namespace 포함
game.Player -> game.User;
game.User.old_field -> new_field;
```

오른쪽 값은 새 identifier 하나입니다.

---

## 17. 권장 스타일

- Field, constraint, field number 순서를 유지합니다: `name: Type constraint... = number;`
- Enum variant 구분자는 `;` 또는 `,` 중 하나로 통일합니다.
- 재사용하는 enum/embed는 inline으로 두지 말고 named type으로 분리합니다.
- Cross-namespace 참조는 FQN을 쓰면 가장 명확합니다.
- 새 인덱스는 field-level `index`보다 table-level `@index(...)`를 우선 사용합니다.
- CSV/JSON load path는 새 스키마에서 `.sources.toml`을 우선 사용합니다.
- `.poly`에는 타입 선언을 중심으로 두고, data/source/output 설정은 가능한 별도 설정부로 분리합니다.
