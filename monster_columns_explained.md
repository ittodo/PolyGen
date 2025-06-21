# `Monster` 테이블의 데이터 컬럼 구조 설명

이 문서는 `game_schema.poly`에 정의된 `game.character.Monster` 테이블이 CSV나 데이터베이스 테이블과 같은 평탄한(flat) 데이터 구조로 변환될 때 생성되는 컬럼 목록과 그 의미를 설명합니다.

---

| 컬럼 이름 (Column Name) | 데이터 타입 (Data Type) | 스키마 원본 (Schema Source) | 설명 (Description) |
| :--- | :--- | :--- | :--- |
| `id` | `u32` | `Monster.id` | 몬스터의 고유 식별자입니다. (기본 키) |
| `name` | `string` | `Monster.name` | 몬스터의 이름입니다. |
| `stats.health` | `u32` | `Monster.stats` -> `StatBlock.health` | 몬스터의 체력입니다. `stats` 필드는 `StatBlock` 타입을 **임베드(embed)**하고 있으므로, 그 하위 필드가 `.` 구분자와 함께 평탄화(flatten)되어 컬럼이 됩니다. |
| `stats.mana` | `u32` | `Monster.stats` -> `StatBlock.mana` | 몬스터의 마나입니다. (`StatBlock` 임베드) |
| `stats.attack` | `u32` | `Monster.stats` -> `StatBlock.attack` | 몬스터의 공격력입니다. (`StatBlock` 임베드) |
| `stats.defense` | `u32` | `Monster.stats` -> `StatBlock.defense` | 몬스터의 방어력입니다. (`StatBlock` 임베드) |
| `spawn_point.x` | `f32` | `Monster.spawn_point` -> `Position.x` | 몬스터의 생성 지점 X좌표입니다. (`Position` 임베드) |
| `spawn_point.y` | `f32` | `Monster.spawn_point` -> `Position.y` | 몬스터의 생성 지점 Y좌표입니다. (`Position` 임베드) |
| `patrol_points[0].x` | `f32` | `Monster.patrol_points` -> `Position.x` | 첫 번째 순찰 지점의 X좌표입니다. `patrol_points`는 **임베드 타입의 배열**이므로, `[인덱스]`와 `.` 구분자를 사용하여 각 요소의 하위 필드가 컬럼으로 생성됩니다. |
| `patrol_points[0].y` | `f32` | `Monster.patrol_points` -> `Position.y` | 첫 번째 순찰 지점의 Y좌표입니다. |
| `patrol_points[1].x` | `f32` | `Monster.patrol_points` -> `Position.x` | 두 번째 순찰 지점의 X좌표입니다. |
| `...` | `...` | `...` | 순찰 지점의 개수만큼 `patrol_points[N].x`, `patrol_points[N].y` 컬럼이 계속 생성됩니다. |
| `drop_items[0].item_id` | `u32` | `Monster.drop_items` -> `embed.item_id` | 첫 번째 드랍 아이템의 ID입니다. `drop_items`는 테이블 내부에 정의된 **임베드 타입의 배열**이며, `patrol_points`와 동일한 방식으로 평탄화됩니다. |
| `drop_items[0].drop_chance` | `f32` | `Monster.drop_items` -> `embed.drop_chance` | 첫 번째 드랍 아이템의 드랍 확률입니다. |
| `drop_items[1].item_id` | `u32` | `Monster.drop_items` -> `embed.item_id` | 두 번째 드랍 아이템의 ID입니다. |
| `...` | `...` | `...` | 드랍 아이템의 종류만큼 `drop_items[N].item_id`, `drop_items[N].drop_chance` 컬럼이 계속 생성됩니다. |
| `_tags[0]` | `string` | `@taggable` on `Monster` | 몬스터에게 부여된 첫 번째 태그입니다. `Monster` 테이블에 붙은 **`@taggable` 어노테이션**에 의해 생성되는 특수 컬럼입니다. |
| `_tags[1]` | `string` | `@taggable` on `Monster` | 몬스터에게 부여된 두 번째 태그입니다. |
| `...` | `string` | `@taggable` on `Monster` | 태그는 여러 개가 될 수 있으며, `_tags[2]`, `_tags[3]`... 와 같이 인덱스가 증가하며 컬럼이 추가될 수 있습니다. |

### 컬럼으로 생성되지 않는 필드

현재 `game_schema.poly` 스키마에서는 다른 테이블이 `Monster` 테이블을 참조하는 `foreign_key` 관계를 정의하고 있지 않습니다. 따라서 `Monster` 테이블에는 ORM 모델 생성을 위해 추론되는 관계 필드가 없습니다.