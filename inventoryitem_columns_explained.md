# `InventoryItem` 테이블의 데이터 컬럼 구조 설명

이 문서는 `game_schema.poly`에 정의된 `game.junction.InventoryItem` 테이블이 데이터 구조로 변환될 때 생성되는 컬럼 목록과 그 의미를 설명합니다.

---

| 컬럼 이름 (Column Name) | 데이터 타입 (Data Type) | 스키마 원본 (Schema Source) | 설명 (Description) |
| :--- | :--- | :--- | :--- |
| `id` | `u32` | `InventoryItem.id` | 인벤토리 항목의 고유 식별자입니다. (기본 키) |
| `player_id` | `u32` | `InventoryItem.player_id` | 이 아이템을 소유한 플레이어의 ID입니다. (`Player` 테이블 외래 키) |
| `item_id` | `u32` | `InventoryItem.item_id` | 아이템의 ID입니다. (`Item` 테이블 외래 키) |
| `quantity` | `u32` | `InventoryItem.quantity` | 소유한 아이템의 수량입니다. 스키마에 `default(1)` 및 `range(1, 999)` 제약조건이 설정되어 있습니다. |
| `_tags[0]` | `string` | `@taggable` on `InventoryItem` | 인벤토리 항목에 부여된 첫 번째 태그입니다. (예: "장착중") |
| `...` | `string` | `@taggable` on `InventoryItem` | 태그는 여러 개가 될 수 있습니다. |

### 컬럼으로 생성되지 않는 필드

`InventoryItem` 테이블은 다른 테이블에 관계를 생성하는 역할을 하므로, 자신에게 추론되는 관계 필드는 없습니다. 대신 이 테이블의 `foreign_key` 선언은 `Player` 테이블에 `inventory` 관계를 생성합니다.