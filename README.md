### **개요 (Overview)**

**Polygen**은 현대 소프트웨어 개발의 파편화되고 반복적인 데이터 관리 작업을 근절하기 위해 탄생한 **통합 스키마 및 코드 생성 플랫폼**입니다.

하나의 직관적인 **Polygen Schema (`.poly`)** 파일에 프로젝트의 "단일 진실 공급원(Single Source of Truth)"을 정의하세요. 그러면 Polygen의 지능적인 컴파일러가 그 스키마로부터 데이터베이스, 백엔드, 프론트엔드에 필요한 모든 코드를 **순식간에, 완벽하게, 일관성 있게 생성(Generate)**합니다.

단순한 코드 생성기를 넘어, Polygen은 데이터 모델링부터 구현까지의 전 과정을 자동화하고 최적화하는 **차세대 개발 파이프라인**을 제공합니다. 이제 개발자는 반복적인 작업에서 해방되어, 오직 비즈니스 로직의 창의적인 부분에만 집중할 수 있습니다.

### **정의 (Definitions)**

Polygen의 스키마는 `table`, `field`, `enum`, `embed` 등 몇 가지 핵심적인 구성 요소로 이루어집니다. 각 요소는 데이터 모델을 명확하고 구조적으로 표현하기 위한 고유한 역할을 가집니다.

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

*   **예제 (Example):**
    ```poly
    table Skill {
        id: u32 primary_key = 1;
        name: string unique max_length(50) = 2;
        description: string? = 3; // 설명은 없어도 됨
        elements: Element[] = 4;   // 여러 개의 원소 타입을 가질 수 있음
    }
    ```

---

#### **`enum` & `embed` (재사용 가능한 타입)**

*   **`enum` (열거형):**
    *   **목적:** `FIRE`, `ICE`와 같이 미리 정해진 값의 목록을 정의하여, 타입 안정성을 높이고 오타를 방지합니다.
    *   **문법:** `enum <EnumName> { VALUE_1; VALUE_2; ... }`
    *   **예제:** `enum Element { FIRE; ICE; LIGHTNING; }`

*   **`embed` (임베드 타입):**
    *   **목적:** 여러 필드를 묶어 재사용 가능한 복합 구조체를 만듭니다. `embed` 타입은 다른 `table`이나 `embed` 내부에 물리적으로 포함(중첩)될 수 있습니다.
    *   **문법:** `embed <EmbedName> { ... }`
    *   **예제:**
        ```poly
        embed Position { x: f32; y: f32; }

        table Monster {
            // Position 타입이 Monster 테이블의 일부로 포함됨
            spawn_point: Position;
            // Position 타입의 배열을 포함할 수도 있음
            patrol_points: Position[];
        }
        ```

---

#### **`relation` (관계)**

*   **목적:**
    테이블 간의 논리적 연결을 정의하는 **가상 필드(Virtual Field)**입니다. `relation`은 실제 데이터 컬럼을 생성하지 않지만, 컴파일러가 데이터 조회 코드를 생성하거나 ORM 관계를 설정하는 데 사용하는 중요한 힌트입니다.

*   **문법 (Syntax):**
    `relation <FieldName>: <TargetTable>[] (via <JoinRule>);`

*   **연결 규칙 (`via`):**
    *   **1:N 관계:** `via foreign.other_table_field` - 상대방 테이블의 외래 키 필드를 지정합니다.
    *   **N:M 관계:** `via JunctionTableName` - 두 테이블을 잇는 연결 테이블의 이름을 지정합니다.

*   **예제 (Example):**
    ```poly
    table User {
        id: u32 primary_key;
        // User는 여러 개의 Post를 가질 수 있음 (1:N)
        relation posts: Post[] (via foreign.author_id);
    }

    table Post {
        id: u32 primary_key;
        author_id: u32 foreign_key(User.id);
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