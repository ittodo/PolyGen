# `Player` 테이블의 데이터 컬럼 구조 설명

이 문서는 `game_schema.poly`에 정의된 `game.character.Player` 테이블이 CSV나 데이터베이스 테이블과 같은 평탄한(flat) 데이터 구조로 변환될 때 생성되는 컬럼 목록과 그 의미를 설명합니다.

---

| 컬럼 이름 (Column Name) | 데이터 타입 (Data Type) | 스키마 원본 (Schema Source) | 설명 (Description) |
| :--- | :--- | :--- | :--- |
| `id` | `u32` | `Player.id` | 플레이어의 고유 식별자입니다. (기본 키) |
| `name` | `string` | `Player.name` | 플레이어의 이름입니다. 스키마에 `unique`로 지정되어 중복될 수 없습니다. |
| `level` | `u16` | `Player.level` | 플레이어의 현재 레벨입니다. 스키마에 `default(1)` 및 `range(1, 99)` 제약조건이 설정되어 있습니다. |
| `stats.health` | `u32` | `Player.stats` -> `StatBlock.health` | 플레이어의 체력입니다. `stats` 필드는 `StatBlock` 타입을 **임베드(embed)**하고 있으므로, 그 하위 필드인 `health`가 `.` 구분자와 함께 평탄화(flatten)되어 컬럼이 됩니다. |
| `stats.mana` | `u32` | `Player.stats` -> `StatBlock.mana` | 플레이어의 마나입니다. (`StatBlock` 임베드) |
| `stats.attack` | `u32` | `Player.stats` -> `StatBlock.attack` | 플레이어의 공격력입니다. (`StatBlock` 임베드) |
| `stats.defense` | `u32` | `Player.stats` -> `StatBlock.defense` | 플레이어의 방어력입니다. (`StatBlock` 임베드) |
| `main_hand.item_id` | `u32?` | `Player.main_hand` -> `EquipmentSlot.item_id` | 주무기 슬롯의 아이템 ID입니다. `EquipmentSlot`은 `Player` 테이블 내부에 정의된 명명된 임베드 타입입니다. |
| `main_hand.enchant_level` | `u8` | `Player.main_hand` -> `EquipmentSlot.enchant_level` | 주무기 슬롯의 강화 레벨입니다. |
| `off_hand.item_id` | `u32?` | `Player.off_hand` -> `EquipmentSlot.item_id` | 보조무기 슬롯의 아이템 ID입니다. `off_hand` 필드는 선택적(`?`)이지만, 그 하위 필드들도 컬럼으로 생성됩니다. |
| `off_hand.enchant_level` | `u8` | `Player.off_hand` -> `EquipmentSlot.enchant_level` | 보조무기 슬롯의 강화 레벨입니다. |
| `_tags[0]` | `string` | `@taggable` on `Player` | 플레이어에게 부여된 첫 번째 태그입니다. `Player` 테이블에 붙은 **`@taggable` 어노테이션**에 의해 생성되는 특수 컬럼입니다. |
| `_tags[1]` | `string` | `@taggable` on `Player` | 플레이어에게 부여된 두 번째 태그입니다. |
| `...` | `string` | `@taggable` on `Player` | 태그는 여러 개가 될 수 있으며, `_tags[2]`, `_tags[3]`... 와 같이 인덱스가 증가하며 컬럼이 추가될 수 있습니다. |

### 컬럼으로 생성되지 않는 필드
*   **`inventory`** (`InventoryItem` 테이블과의 관계)
*   **`skills`** (`Skill` 테이블과의 관계)

`Player` 테이블의 스키마 정의에는 `inventory`나 `skills`라는 필드가 직접 보이지 않을 수 있습니다. 이들은 **추론된 관계(inferred relations)**입니다.

Polygen 컴파일러는 다른 테이블(예: `InventoryItem`, `PlayerSkill`)에 정의된 `foreign_key(Player.id)` 선언을 분석하여 `Player`가 어떤 테이블들과 관계를 맺는지 자동으로 파악합니다. 예를 들어, `PlayerSkill` 테이블에 `player_id: u32 foreign_key(Player.id) as skills` 와 같이 정의되어 있다면, 컴파일러는 `Player` 모델에 `skills` 라는 이름의 관계 필드를 자동으로 생성해줍니다.

이 관계 필드들은 `Player` 테이블 자체에 물리적인 데이터 컬럼을 만들지는 않지만, ORM이나 데이터 조회 코드에서 편리하게 관련 데이터에 접근할 수 있도록 해주는 중요한 역할을 합니다.