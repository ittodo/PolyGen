# `PlayerSkill` 테이블의 데이터 컬럼 구조 설명

이 문서는 `game_schema.poly`에 정의된 `game.junction.PlayerSkill` 테이블이 데이터 구조로 변환될 때 생성되는 컬럼 목록과 그 의미를 설명합니다.

---

| 컬럼 이름 (Column Name) | 데이터 타입 (Data Type) | 스키마 원본 (Schema Source) | 설명 (Description) |
| :--- | :--- | :--- | :--- |
| `player_id` | `u32` | `PlayerSkill.player_id` | 플레이어의 고유 식별자입니다. (`Player` 테이블 외래 키) |
| `skill_id` | `u32` | `PlayerSkill.skill_id` | 스킬의 고유 식별자입니다. (`Skill` 테이블 외래 키) |
| `skill_level` | `u16` | `PlayerSkill.skill_level` | 플레이어가 습득한 스킬의 레벨입니다. 스키마에 `default(1)` 및 `range(1, 20)` 제약조건이 설정되어 있습니다. |
| `next_skill_id` | `u32?` | `@link_rows` 어노테이션 | **`@link_rows`에 의해 자동 생성된 컬럼입니다.** 동일한 `player_id`를 가진 그룹 내에서, 현재 행 다음 순서에 있는 행의 `skill_id`를 가리킵니다. 그룹의 마지막 행일 경우 이 값은 `null`이 됩니다. |
| `_tags[0]` | `string` | `@taggable` on `PlayerSkill` | `PlayerSkill` 관계에 부여된 첫 번째 태그입니다. (예: "주력스킬") |
| `...` | `string` | `@taggable` on `PlayerSkill` | 태그는 여러 개가 될 수 있습니다. |

### 컬럼으로 생성되지 않는 필드

`PlayerSkill` 테이블은 다른 테이블에 관계를 생성하는 역할을 하므로, 자신에게 추론되는 관계 필드는 없습니다. 대신 이 테이블의 `foreign_key` 선언은 `Player` 테이블에 `skills` 관계를, `Skill` 테이블에 `users` 관계를 생성합니다.