### **개요 (Overview)**

**Polygen**은 데이터 모델이 데이터베이스 스키마, 백엔드 모델, 프론트엔드 타입 등 여러 곳에 흩어져 발생하는 불일치와 반복 작업을 근절하기 위해 탄생했습니다.

Polygen은 **간결하고 강력한 스키마(`.poly`)**를 프로젝트의 "단일 진실 공급원(Single Source of Truth)"으로 삼습니다. 개발자는 최소한의 문법으로 데이터의 구조와 핵심 제약 조건을 정의합니다.

그러면 Polygen의 지능형 컴파일러가 이 간결한 소스를 분석하여 두 가지 핵심 결과물을 생성합니다:
1.  **완벽하게 일관된 코드:** 데이터베이스, 백엔드, 프론트엔드에 필요한 모든 코드.
2.  **살아있는 설계 청사진(Living Blueprint):** 테이블 간의 모든 관계(자동 추론된 관계 포함)를 명확하게 시각화하고 설명하는 종합적인 문서.

이 접근 방식을 통해 개발자는 **작성할 때는 간결함의 이점**을 누리고, **이해할 때는 풍부한 문서의 도움**을 받을 수 있습니다. Polygen은 단순한 코드 생성기를 넘어, 스키마 정의의 부담을 줄이고 생성된 아키텍처에 대한 깊은 통찰력을 제공하는 개발 파트너입니다.

### 생성되는 언어 
✅ C# - 지원 (현재)
📋 Prisma (DB) - 미지원 (향후 지원 예정)
📋 TypeScript - 미지원 ( NA )  
📋 Unreal - 미지원 (향후 지원 예정)

### **정의 (Definitions)**

Polygen의 스키마는 `table`, `field`, `enum`, `embed` 등 몇 가지 핵심적인 구성 요소로 이루어집니다. 각 요소는 데이터 모델을 명확하고 구조적으로 표현하기 위한 고유한 역할을 가집니다.

---

#### **`namespace` (네임스페이스)**

*   **목적 (Purpose):**
    스키마가 복잡해질 때, 관련된 `table`, `enum`, `embed` 등을 논리적인 그룹으로 묶어주는 컨테이너 역할을 합니다. 네임스페이스는 이름 충돌을 방지하고, 스키마의 가독성과 모듈성을 크게 향상시킵니다.

*   **문법 (Syntax):**    namespace <NamespaceName> {
        // table, enum, embed 등 다른 정의들이 위치합니다.
    }
    
    ```poly
    namespace <NamespaceName> {
        // table, enum, embed 등 다른 정의들이 위치합니다.
    }
    ```

*   **예제 (Example):**
    ```poly
    // 게임의 핵심 데이터를 위한 네임스페이스
    namespace game.core {
        table Player { id: u32 primary_key; }
        table Monster { id: u32 primary_key; }
    }
    ```

---

#### **`table`**

*   **목적 (Purpose):**
    데이터의 핵심적인 독립 단위(Entity)를 정의합니다. `table`은 관계형 데이터베이스의 '테이블', 객체 지향 언어의 '클래스', 또는 하나의 Excel '시트'나 CSV 파일에 해당합니다.

*   **문법 (Syntax):**
    ```poly
    table <TableName> {
        // 필드(field) 및 관계(relation) 정의
    }
    ```

*   **예제 (Example):**
    ```poly
    /// 캐릭터의 기본 정보를 담는 테이블
    table Character {
        id: u32 primary_key;
        name: string unique;
    }
    ```

---

#### **`field` (필드)**

*   **목적 (Purpose):**
    `table`이 가지는 구체적인 속성(Attribute)을 정의합니다. 데이터베이스의 '컬럼' 또는 클래스의 '멤버 변수'에 해당합니다.

*   **문법 (Syntax):**
    `field_name: [타입][필수성_기호] [제약조건들] [= 필드_번호]`

*   **핵심 요소:**
    *   **필수성 (Cardinality):** 타입 뒤에 붙는 기호로 간단하게 표현합니다.
        *   **(기본값) 필수:** `name: string` - 이 필드는 반드시 값을 가져야 합니다.
        *   **`?` (선택적):** `description: string?` - 이 필드는 값이 없거나 `null`일 수 있습니다.
        *   **`[]` (배열):** `tags: string[]` - 이 필드는 해당 타입의 값을 0개 이상 가질 수 있습니다.
    *   **필드 번호 (선택적):** `= 1`, `= 2` 와 같이 번호를 명시합니다. Protobuf와 같은 바이너리 직렬화 포맷과의 호환성을 위한 옵션이며, 생략 시 1부터 자동으로 할당됩니다.

*   **제약조건 (Constraints):**
    *   **`primary_key`**: 테이블의 기본 키로 사용됩니다. 고유하며 `null`이 될 수 없습니다.
    *   **`unique`**: 필드의 모든 값이 고유해야 합니다.
    *   **`max_length(N)`**: `string` 타입 필드의 최대 길이를 제한합니다.
    *   **`default(value)`**: 필드의 기본값을 지정합니다. 데이터 생성 시 값이 주어지지 않으면 이 값이 사용됩니다.
    *   **`range(min, max)`**: 숫자 타입 필드의 유효 범위를 제한합니다.
    *   **`regex("pattern")`**: 문자열 필드가 특정 정규표현식 패턴을 따라야 함을 명시합니다.
    *   **`foreign_key(TargetTable.target_field) [as <RelationName>]`**: 이 필드가 `TargetTable`의 `target_field`를 참조하는 외래 키임을 선언합니다. `target_field`는 일반적으로 `TargetTable`의 기본 키입니다. 선택적으로 `as <RelationName>`을 추가하여, `TargetTable`에 이 관계를 역으로 참조할 수 있는 필드(예: `user.posts`)를 자동으로 생성하도록 지시할 수 있습니다.

*   **예제 (Example):**
    ```poly
    table Player {
        id: u32 primary_key;
        // 이름은 3~30자 사이여야 하며, 알파벳과 숫자만 허용
        name: string unique max_length(30) regex("^[a-zA-Z0-9]{3,}$");
        // 레벨은 1~99 사이, 기본값은 1
        level: u16 default(1) range(1, 99);
        // 설명은 선택 사항
        description: string?;
    }
    ```

---

#### **기본 타입 (Built-in Types)**

Polygen은 다양한 프로그래밍 언어와 데이터베이스에서 공통으로 사용되는 핵심 데이터 타입들을 지원합니다.

*   **`string`**
    *   **설명:** 가변 길이의 텍스트 데이터를 나타냅니다.
    *   **대응:** Rust/Java의 `String`, C#의 `string`, SQL의 `VARCHAR` 또는 `TEXT`에 해당합니다.
    *   **사용 가능한 제약조건:**
        *   `max_length(N)`: 최대 길이를 제한합니다.
        *   `default("value")`: 기본값을 지정합니다.
        *   `regex("pattern")`: 값이 특정 정규표현식 패턴을 따르도록 강제합니다.
    *   **예시:** `email: string max_length(100) regex("...") default("no-reply@example.com");`

*   **정수 타입 (Integer Types)**
    *   **설명:** 부호 있는 정수(`i`)와 부호 없는 정수(`u`)를 지원하며, 비트 단위로 크기를 지정할 수 있습니다.
    *   **종류:** `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`
    *   **대응:** Rust의 `i32`/`u32`, Java의 `int`/`long`, SQL의 `INTEGER`/`BIGINT` 등에 해당합니다.
    *   **사용 가능한 제약조건:**
        *   `range(min, max)`: 값의 유효 범위를 제한합니다.
        *   `default(N)`: 기본값을 지정합니다.
    *   **예시:** `level: u16 default(1) range(1, 99);`

*   **부동소수점 타입 (Floating-Point Types)**
    *   **설명:** 소수점을 포함하는 숫자를 나타냅니다.
    *   **종류:** `f32` (단정밀도), `f64` (배정밀도)
    *   **대응:** Rust의 `f32`/`f64`, Java의 `float`/`double`, SQL의 `REAL`/`DOUBLE PRECISION`에 해당합니다.
    *   **사용 가능한 제약조건:**
        *   `range(min, max)`: 값의 유효 범위를 제한합니다.
        *   `default(N)`: 기본값을 지정합니다.
    *   **예시:** `drop_chance: f32 default(0.1) range(0.0, 1.0);`

*   **`bool`**
    *   **설명:** `true` 또는 `false` 값을 가지는 논리 타입입니다.
    *   **대응:** 대부분의 언어와 데이터베이스에서 `boolean` 타입에 해당합니다.
    *   **사용 가능한 제약조건:**
        *   `default(true/false)`: 기본값을 지정합니다.
    *   **예시:** `is_active: bool default(true);`

*   **`bytes`**
    *   **설명:** 이미지, 파일 등 원시 이진(binary) 데이터를 저장하는 데 사용됩니다.
    *   **대응:** Rust의 `Vec<u8>`, Java/C#의 `byte[]`, SQL의 `BLOB`/`BYTEA`에 해당합니다.
    *   **사용 가능한 제약조건:**
        *   `max_length(N)`: 최대 바이트 크기를 제한합니다.
    *   **예시:** `avatar_image: bytes max_length(1048576);` // 1MB

---

#### **`enum` & `embed` (재사용 가능한 타입)**

*   **`enum` (열거형):**
    *   **목적:** `FIRE`, `ICE`와 같이 미리 정해진 값의 목록을 정의하여, 타입 안정성을 높이고 오타를 방지합니다.
    *   **문법:** `enum <EnumName> { VALUE_1; VALUE_2; ... }`
    *   **예제:** `enum Element { FIRE; ICE; LIGHTNING; }`

*   **`embed` (임베드 타입):**
    *   **목적:** 여러 필드를 묶어 재사용 가능한 복합 구조체를 만듭니다. `embed` 타입은 다른 `table`이나 `embed` 내부에 물리적으로 포함(중첩)될 수 있습니다.
    *   **문법:**
        - `embed <EmbedName> { ... }` (재사용을 위해 명명된 embed)
        - `field_name: embed { ... }` (특정 테이블 내에서만 사용하기 위해 직접 정의)
    *   **예제 1: 재사용 가능한 `embed` 정의**

        `Position`과 같이 여러 곳에서 사용될 구조체는 네임스페이스에 직접 정의합니다.
        ```poly
        // 월드 좌표를 나타내는 Position 타입을 정의
        embed Position { x: f32; y: f32; }

        table Monster {
            // Position 타입이 Monster 테이블의 일부로 포함됨
            spawn_point: Position;
            // Position 타입의 배열을 포함할 수도 있음
            patrol_points: Position[];
        }
        ```
    *   **예제 2: 테이블 내부에 직접 `embed` 정의**

        `Monster` 테이블에서만 사용되는 드랍 아이템 정보처럼, 특정 테이블에 종속적인 구조체는 내부에 직접 정의할 수 있습니다.
        ```poly
        table Monster {
            id: u32 primary_key;
            name: string;

            // 이 몬스터에서만 사용되는 드랍 아이템 정보를
            // 테이블 내부에 직접 embed로 정의합니다.
            drop_items: embed {
                item_id: u32;
                drop_chance: f32; // 0.0 ~ 1.0
            }[]; // 여러 아이템을 드랍할 수 있으므로 배열로 지정
        }
        ```
    *   **예제 3: 테이블에 종속된 명명된 `embed` 정의**

        특정 테이블 내에서만 여러 번 재사용하고 싶은 구조체가 있다면, 테이블 내부에 명명된 `embed`를 정의할 수 있습니다. 이는 해당 타입이 부모 테이블에 강하게 종속됨을 명시하며, 네임스페이스를 깔끔하게 유지합니다.
        ```poly
        table Player {
            // 'Player' 테이블에 종속적인 'EquipmentSlot' 타입을 정의합니다.
            embed EquipmentSlot {
                item_id: u32?;
                enchant_level: u8 default(0);
            }

            // 위에서 정의한 타입을 여러 필드에서 재사용합니다.
            main_hand: EquipmentSlot;
            helmet: EquipmentSlot;
            armor: EquipmentSlot;
        }
        ```

---

#### **관계 정의 (Defining Relationships)**

*   **목적:**
    Polygen에서는 더 이상 `relation` 키워드를 사용하지 않습니다. 대신, `foreign_key` 제약조건을 확장하여 테이블 간의 논리적 관계를 명확하고 직관적으로 정의합니다. 관계는 항상 외래 키가 선언되는 쪽(1:N 관계의 'N'쪽 또는 N:M 관계의 연결 테이블)에서 정의되며, 이를 통해 관계의 "단일 진실 공급원"이 명확해집니다.
    
*   **문법 (Syntax):**
    `field_name: Type foreign_key(TargetTable.target_field) [as <RelationNameOnTarget>];`

*   **핵심 요소:**
    *   **`as <RelationNameOnTarget>` (선택적):** 이 외래 키가 `TargetTable`에 어떤 이름의 관계 필드를 생성할지를 지정합니다. 이 부분이 바로 `TargetTable`에서 연관된 데이터 목록(예: `user.posts`)에 접근할 수 있게 해주는 핵심입니다.

*   **예제 (Example):**

    **1. 1:N 관계 (일대다)**
    
    한 명의 `User`가 여러 개의 `Post`를 작성하는 경우입니다. `Post` 테이블에 `author_id`라는 외래 키를 두어 `User`를 참조합니다.
    
    ```poly
    table User {
        id: u32 primary_key;
        // 'posts: Post[]' 관계는 아래 Post 테이블의
        // foreign_key 선언에 의해 자동으로 생성됩니다.
    }
    
    table Post {
        id: u32 primary_key;
        content: string;
        // User.id를 참조하는 외래 키를 정의합니다.
        // 'as posts'를 통해 User 테이블에 'posts'라는 이름의
        // 1:N 관계가 생성됨을 명시합니다.
    `field_name: Type foreign_key(TargetTable.target_field) [as <RelationNameOnTarget>];`

*   **핵심 요소:**
    *   **`as <RelationNameOnTarget>` (선택적):** 이 외래 키가 `TargetTable`에 어떤 이름의 관계 필드를 생성할지를 지정합니다. 이 부분이 바로 `TargetTable`에서 연관된 데이터 목록(예: `user.posts`)에 접근할 수 있게 해주는 핵심입니다.

*   **예제 (Example):**

    **1. 1:N 관계 (일대다)**
    
    한 명의 `User`가 여러 개의 `Post`를 작성하는 경우입니다. `Post` 테이블에 `author_id`라는 외래 키를 두어 `User`를 참조합니다.
    
    ```poly
    table User {
        id: u32 primary_key;
        // 'posts: Post[]' 관계는 아래 Post 테이블의
        // foreign_key 선언에 의해 자동으로 생성됩니다.
    }
    
    table Post {
        id: u32 primary_key;
        content: string;
        // User.id를 참조하는 외래 키를 정의합니다.
        // 'as posts'를 통해 User 테이블에 'posts'라는 이름의
        // 1:N 관계가 생성됨을 명시합니다.
        author_id: u32 foreign_key(User.id) as posts;
    }
    ```

    **2. N:M 관계 (다대다)**
    
    한 명의 `Player`가 여러 `Skill`을 가질 수 있고, 하나의 `Skill`은 여러 `Player`가 가질 수 있는 경우입니다. 이 관계는 `PlayerSkill`이라는 **연결 테이블(Junction Table)**을 통해 정의됩니다.
    
    ```poly
    table Player {
        id: u32 primary_key;
        name: string;
        // 'skills: Skill[]' 관계는 PlayerSkill 테이블에 의해
        // 자동으로 생성됩니다.
    }
    
    table Skill {
        id: u32 primary_key;
        name: string unique;
        // 'players: Player[]' 관계는 PlayerSkill 테이블에 의해
        // 자동으로 생성됩니다.
    }
    
    // Player와 Skill을 연결하는 Junction Table
    table PlayerSkill {
        // Player 테이블에 'skills' 라는 이름의 N:M 관계를 생성합니다.
        player_id: u32 foreign_key(Player.id) as skills;
        // Skill 테이블에 'players' 라는 이름의 N:M 관계를 생성합니다.
        skill_id: u32 foreign_key(Skill.id) as players;
        // 이 테이블에 추가적인 정보(예: 스킬 습득 레벨)를 넣을 수도 있습니다.
        // acquired_level: u16;
    }
    ```

---

#### **`@` 어노테이션 (메타데이터)**

*   **목적:**
    `@` 기호로 시작하는 어노테이션은 컴파일러에게 추가적인 지시나 힌트를 제공하는 확장 기능입니다. 이를 통해 핵심 문법을 더럽히지 않으면서도, 타겟별 특화 기능이나 부가 정보를 유연하게 추가할 수 있습니다.

*   **`@taggable`**:
    *   테이블이 스키마에 정의되지 않은, 자유로운 "비공식 태그"를 가질 수 있음을 선언합니다.
    *   컴파일러는 데이터 소스에서 `_tags[0]`, `_tags[1]`... 과 같은 특수 컬럼을 찾아 처리하고, 데이터베이스에는 `JSON` 타입 컬럼을 추가하는 등 타겟에 맞는 최적의 구현을 자동으로 수행합니다.

*   **예제 (Example):**
    ```poly
    /// @taggable
    table Monster {
        id: u32 primary_key;
        name: string;
    }
    // 데이터 소스(CSV)에서는 다음과 같이 표현됩니다:
    // id,name,_tags[0],_tags[1]
    //

*   **`@load(type: DataSourceType, path: DataSourcePath)`**:
    *   테이블이 특정 데이터 소스에서 로드될 수 있음을 나타냅니다.
    *   컴파일러는 이 정보를 사용하여 데이터 로딩 기능을 자동으로 생성할 수 있습니다.
        - `type`: 데이터 소스 유형 (예: "DB", "CSV", "Excel", "Packet")
        - `path`: 데이터 소스 경로 또는 식별자 (예: "Players" 테이블, "data/players.csv" 파일)

*   **`@save(type: StorageType, path: StoragePath)`**:
    *   테이블의 데이터를 특정 저장소에 저장할 수 있음을 나타냅니다.
    *   컴파일러는 이 정보를 사용하여 데이터 저장 기능을 자동으로 생성할 수 있습니다.
        - `type`: 저장소 유형 (예: "DB", "CSV", "Excel", "Packet")
        - `path`: 저장소 경로 또는 식별자

*   **예제 (Example):**
    ```poly
    /// 플레이어 캐릭터 정보를 정의하는 테이블입니다.
    @taggable
    @load(type: "DB", path: "Players")
    @save(type: "CSV", path: "exports/players.csv")
    table Player {
        id: u32 primary_key;
        name: string unique max_length(30) = 2;
        level: u16 = 3;
        // 다른 네임스페이스의 타입을 참조할 때는 '네임스페이스.타입' 형식으로 사용합니다.
        stats: game.common.StatBlock = 4;
    }
    }
    // 데이터 소스(CSV)에서는 다음과 같이 표현됩니다:
    // id,name,_tags[0],_tags[1]
    //

*   **`@link_rows(partition_by: P, link_with: L)`**:
    *   복합 키로 구성된 테이블에서, 특정 그룹(`P`) 내의 행들을 다른 키(`L`)를 기준으로 연결 리스트처럼 만듭니다.
    *   **동작:** 컴파일러는 `next_L` 이라는 이름의 새로운 필드를 테이블에 자동으로 추가합니다. 이 필드는 같은 그룹(`P`) 내의 다음 행을 가리키는 포인터 역할을 합니다. 그룹의 마지막 행에서는 이 값이 `null`이 됩니다.
    *   **사용 사례:** 특정 유저의 인벤토리 아이템 목록이나 스킬 트리를 `ORDER BY` 없이 효율적으로 순회하고자 할 때 유용합니다.
    *   **문법:**
        - `partition_by`: 데이터를 그룹화할 필드를 지정합니다.
        - `link_with`: 그룹 내에서 연결 고리로 사용할 필드를 지정합니다.
    *   **예제:**
        ```poly
        /// @link_rows(partition_by: player_id, link_with: skill_id)
        table PlayerSkill {
            player_id: u32 foreign_key(Player.id) as skills;
            skill_id: u32 foreign_key(Skill.id) as users;
        }
        ```

## Data Conversion
- See JSON → Table → CSV conversion spec: [docs/json-to-csv-conversion-spec.md](docs/json-to-csv-conversion-spec.md)

## Run Demo (C#)
- Generate C# from example schema and run the demo app.
- PowerShell: `./rundemo.ps1`
- Options: `./rundemo.ps1 -SchemaPath examples/game_schema.poly -Lang csharp`
- What happens:
  - Runs `cargo run -- --schema-path <schema> --lang csharp` and writes to `output/csharp`.
  - Builds and runs `dist/run-csharp/RunDemo`, which links the generated files.
  - The demo writes small CSVs under `RunDemo/bin/.../data` and reads them using generated mappers.
