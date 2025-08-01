 # PolyGen 개발 가이드: 내부 구조 및 구현 방향

 이 문서는 PolyGen 프로젝트의 아키텍처, 주요 구성 요소, 그리고 각 기능이 어떻게 구현되었거나 구현될 예정인지에 대한 기술적인 가이드입니다. PolyGen의 "살아있는 설계 청사진"을 만드는 과정과 그 기반이 되는 원칙들을 설명합니다.

 ---

 ## 1. PolyGen의 아키텍처 개요

 PolyGen은 스키마 정의(`.poly` 파일)를 입력받아 이를 파싱하고, 추상 구문 트리(AST)로 변환한 다음, 이 AST를 기반으로 다양한 타겟 언어의 코드나 문서를 생성하는 파이프라인 구조를 가집니다.

 ```
 .poly Schema
      ↓
 [ Parser (pest) ]
      ↓
 [ Abstract Syntax Tree (AST) ]
      ↓
 [ Validation ]
      ↓
 [ Generator (C#) ]  [ Generator (Mermaid) ]  [ ... Other Generators (SQL, TS, etc.) ]
      ↓                    ↓                        ↓
 C# Code (.cs)      Mermaid Diagram (.md)    Other Outputs
 ```

 ### 핵심 원칙

 *   **단일 진실 공급원 (Single Source of Truth)**: `.poly` 스키마 파일이 모든 데이터 모델 정보의 유일한 원천입니다.
 *   **언어 중립성**: AST는 특정 타겟 언어에 종속되지 않는 형태로 스키마의 의미를 표현합니다.
 *   **확장성**: 새로운 타겟 언어나 출력 형식을 쉽게 추가할 수 있도록 모듈화되어 있습니다.
 *   **유연성**: 어노테이션 시스템을 통해 스키마의 핵심 문법을 변경하지 않고도 다양한 메타데이터와 기능을 추가할 수 있습니다.

 ---

 ## 2. 주요 구성 요소 및 역할

 ### `d:\Rust\PolyGen\src\main.rs`
 *   **역할**: PolyGen 애플리케이션의 진입점입니다. 명령줄 인자를 파싱하고, 스키마 파일을 읽어 파싱 및 AST 빌드, 유효성 검사, 그리고 각 생성기(C#, Mermaid)를 호출하는 전체 워크플로우를 관리합니다.

 ### `d:\Rust\PolyGen\polygen.pest`
 *   **역할**: PolyGen 스키마 언어의 문법을 정의하는 Pest 문법 파일입니다. `table`, `enum`, `embed`, `field`, `constraint`, `annotation` 등 스키마의 모든 구성 요소를 규칙(Rule)으로 정의합니다.

 ### `d:\Rust\PolyGen\src\ast.rs`
 *   **역할**: 파싱된 `.poly` 스키마의 구조를 표현하는 **추상 구문 트리(AST)**의 정의를 포함합니다. `Definition`, `Namespace`, `Table`, `FieldDefinition`, `Constraint`, `Annotation` 등 스키마의 모든 요소를 Rust의 `enum`과 `struct`로 모델링합니다.
 *   **구현 방향**: `pest::Pair`로부터 AST를 빌드하는 로직(`build_ast_from_pairs` 및 관련 `parse_*` 함수들)을 포함하여, 파싱된 텍스트를 구조화된 데이터로 변환합니다. `Clone`, `Debug`, `PartialEq` 트레이트를 구현하여 AST의 복제, 디버깅, 비교를 용이하게 합니다.

 ### `d:\Rust\PolyGen\src\validation.rs`
 *   **역할**: 빌드된 AST의 유효성을 검사합니다. 예를 들어, 중복된 이름의 정의가 있는지, 참조하는 타입이 실제로 존재하는지, `foreign_key` 제약조건의 경로가 유효한지 등을 확인합니다.
 *   **구현 방향**: AST를 순회하며 스키마의 논리적 일관성을 보장하는 규칙들을 적용합니다.

 ### `d:\Rust\PolyGen\src\csharp_model.rs`
 *   **역할**: C# 코드 생성을 위한 중간 데이터 모델입니다. `CSharpFile`, `NamespaceDef`, `ClassDef`, `PropertyDef` 등 C# 언어의 구조에 특화된 형태로 스키마 정보를 재구성합니다.
 *   **구현 방향**: `serde::Deserialize`를 구현하여, 향후에는 YAML/TOML과 같은 설정 파일로부터 직접 C# 모델을 구성할 수 있는 유연성을 제공합니다.

 ### `d:\Rust\PolyGen\src\csharp_generator.rs`
 *   **역할**: C# 코드를 생성하는 핵심 로직을 담당합니다.
 *   **현재 상태**: 현재는 `build_csharp_model` 함수 내부에 C# 코드 구조가 하드코딩되어 있습니다. 이는 초기 개발 및 Askama 템플릿 테스트를 위한 임시 구현입니다.
 *   **구현 방향**: **가장 중요한 다음 단계**는 `build_csharp_model` 함수가 하드코딩된 데이터 대신, `main.rs`로부터 전달받은 **AST를 입력으로 받아 C# 모델을 동적으로 구축**하도록 리팩토링하는 것입니다. 이를 통해 `.poly` 스키마의 변경이 C# 코드에 직접 반영되도록 합니다.
 *   **Askama 템플릿 사용**: `askama::Template` 트레이트를 사용하여 `csharp_model`의 데이터를 `templates/csharp/` 폴더의 템플릿에 주입하여 최종 C# 코드를 렌더링합니다.

 ### `d:\Rust\PolyGen\src\mermaid_model.rs`
 *   **역할**: Mermaid 다이어그램 생성을 위한 중간 데이터 모델입니다. `ClassDiagram`, `Class`, `Enum`, `Relationship` 등 Mermaid 문법에 필요한 정보만을 담습니다.

 ### `d:\Rust\PolyGen\src\mermaid_generator.rs`
 *   **역할**: Mermaid 클래스 다이어그램 코드를 생성하는 핵심 로직을 담당합니다.
 *   **구현 방향**:
     *   AST를 순회하며 `mermaid_model`의 `ClassDiagram`을 구축합니다.
     *   `process_field` 함수는 필드의 타입과 제약조건(특히 `foreign_key`)을 분석하여 `Relationship`을 생성합니다.
     *   `find_all_named_embed_fqns` 함수를 통해 모든 명명된 임베드(네임스페이스 레벨 및 테이블 내부 명명된 임베드)를 식별하고, 이들을 다이어그램에 별도의 클래스 박스로 렌더링합니다.
     *   인라인 임베드(`drop_items: embed { ... }[]`)는 별도의 클래스 박스 없이 부모 클래스의 속성으로만 표시됩니다.
     *   `askama::Template` 트레이트를 사용하여 `mermaid_model`의 데이터를 `templates/mermaid/class_diagram.mmd.txt` 템플릿에 주입하여 최종 Mermaid 코드를 렌더링합니다.

 ### `d:\Rust\PolyGen\templates\`
 *   **역할**: Askama 템플릿 파일들을 포함합니다. 이 템플릿들은 최종 출력(C# 코드, Mermaid 다이어그램)의 형식을 정의하며, 데이터 모델과 렌더링 로직을 분리하여 유지보수성을 높입니다.
 *   **구현 방향**: 템플릿 내에서 `{% for %}`, `{% if %}`, `{% match %}`, `{% include %}` 등의 Askama 문법을 사용하여 복잡한 코드 구조를 유연하게 생성합니다. 공백 제어(`{%-` `-%}`)를 통해 생성된 코드의 서식을 깔끔하게 유지합니다.

 ---

 ## 3. 주요 기능 구현 상세

 ### 3.1. 어노테이션 (`@taggable`, `@link_rows`, `@load`, `@save`)

 *   **AST 파싱**: `d:\Rust\PolyGen\src\ast.rs`의 `parse_annotation` 함수가 `@`로 시작하는 어노테이션의 이름과 파라미터(키-값 쌍)를 파싱하여 `Annotation` 구조체로 변환합니다. 이 `Annotation`은 `Table` 구조체의 `annotations` 필드에 저장됩니다.
 *   **Mermaid 다이어그램 표시**: `d:\Rust\PolyGen\src\mermaid_generator.rs`에서 `Table`의 `annotations` 필드를 순회하며, 각 어노테이션을 Mermaid의 스테레오타입(`<<annotation_name(params)>>`) 형식으로 변환하여 클래스 박스 내에 표시합니다.
     *   `@taggable` -> `<<taggable>>`
     *   `@link_rows(partition_by: player_id, link_with: skill_id)` -> `<<link_rows(partition_by: player_id, link_with: skill_id)>>`
     *   `@load(type: "DB")` -> `<<load(DB)>>`
     *   `@save(type: "Map")` -> `<<save(Map)>>`
 *   **C# 코드 생성 활용 (미구현)**: 향후 `csharp_generator.rs`가 AST를 기반으로 동작하게 되면, 이 어노테이션 정보를 활용하여 다음과 같은 C# 코드를 자동으로 생성할 수 있습니다.
     *   `@taggable`: `List<string> Tags { get; set; }`와 같은 태그 필드 및 관련 로직.
     *   `@link_rows`: `uint? NextSkillId { get; set; }`와 같은 연결 필드 및 관련 로직.
     *   `@load`, `@save`: 데이터 로딩/저장 메서드 (예: `public static Player LoadFromDB(uint id) { ... }`, `public void SaveToMap(Dictionary<string, object> map) { ... }`).
         *   **`DB`**: `List<T>`와 같은 배열 타입 필드는 별도의 관계형 테이블로 분리되어 1:N 관계를 형성합니다. `PolyGen`은 스키마의 관계를 기반으로 필요한 조인 로직을 생성합니다.
         *   **`Map`**: `List<T>`와 같은 배열 타입 필드는 키-값 쌍의 맵(Dictionary)에서 배열 형태로 처리됩니다. 이는 CSV, JSON, YAML 등 다양한 키-값 기반 데이터 소스를 추상화합니다.
             *   **CSV**: `tags[0]`, `tags[1]`과 같이 여러 컬럼으로 확장될 수 있습니다.
             *   **JSON/YAML**: `tags: ["value1", "value2"]`와 같이 네이티브 배열로 표현되며, `tags[0]`와 같은 인덱스로 접근합니다.
         *   **`Memory`**: 인메모리 데이터 구조(예: `List<Player>`)에서 직접 객체를 로드하거나 저장하는 메서드를 생성합니다.

 ### 3.2. 임베드 타입 (`embed`)

 *   **AST 파싱**: `d:\Rust\PolyGen\src\ast.rs`에서 `embed` 정의를 `Embed` 구조체로 파싱합니다. `TableMember::Embed`는 테이블 내 명명된 임베드를, `FieldDefinition::InlineEmbed`는 인라인 임베드를 나타냅니다.
 *   **Mermaid 다이어그램 표시**: `d:\Rust\PolyGen\src\mermaid_generator.rs`에서 임베드 타입을 다음과 같이 처리합니다.
     *   **네임스페이스 레벨 임베드 (`embed Position { ... }`)**: `Position`, `StatBlock`처럼 재사용 가능한 임베드는 별도의 클래스 박스로 다이어그램에 표시하여 그 내부 필드를 보여줍니다. 이들을 사용하는 클래스(예: `Player`)와는 Composition 관계(`*--`)로 연결됩니다.
     *   **테이블 내부 명명된 임베드 (`table Player { embed EquipmentSlot { ... } ... }`)**: 이들도 별도의 클래스 박스로 다이어그램에 표시하여 내부 구조를 보여줍니다.
     *   **인라인 임베드 (`drop_items: embed { ... }[]`)**: `Monster`의 `drop_items`처럼 특정 필드에만 사용되는 일회성 구조는 별도의 클래스 박스 없이 부모 클래스의 속성으로만 표시됩니다 (예: `+List<DropItems> drop_items`).
 *   **C# 코드 생성 활용 (미구현)**: 향후 `csharp_generator.rs`가 AST를 기반으로 동작하게 되면, 임베드 타입은 C#의 `struct` 또는 중첩 `class`로 생성될 수 있습니다.

 ### 3.3. 외래 키 및 관계 (`foreign_key`)

 *   **AST 파싱**: `d:\Rust\PolyGen\src\ast.rs`의 `parse_constraint` 함수가 `foreign_key` 제약조건을 `Constraint::ForeignKey`로 파싱합니다. 이때 `as <RelationName>` 부분도 `Option<String>`으로 함께 저장됩니다.
 *   **Mermaid 다이어그램 표시**: `d:\Rust\PolyGen\src\mermaid_generator.rs`에서 `process_field` 함수가 `foreign_key` 제약조건을 가진 필드를 발견하면 `Relationship`을 생성합니다.
     *   `as <RelationName>`이 있는 경우, 이 정보를 사용하여 역방향 관계(예: `Player "1" -- "*" PlayerSkill : skills`)를 생성하여 다이어그램에 추가합니다.
     *   카디널리티(`1`, `*`, `0..1`)를 명확하게 표시합니다.
 *   **C# 코드 생성 활용 (미구현)**: 향후 `csharp_generator.rs`가 AST를 기반으로 동작하게 되면, `foreign_key` 제약조건을 가진 필드에 대해 C#의 `[ForeignKey]` 어트리뷰트를 추가하고, `as <RelationName>`을 사용하여 탐색 속성(Navigation Property)을 자동으로 생성할 수 있습니다.

 ---

 ## 4. 개발 워크플로우

 1.  **스키마 정의**: `game_schema.poly` 파일을 수정하여 데이터 모델을 정의합니다.
 2.  **AST 빌드 및 유효성 검사**: `cargo run -- examples/game_schema.poly` 명령을 실행하면, PolyGen이 스키마를 파싱하고 AST를 빌드한 후 유효성을 검사합니다.
 3.  **코드/다이어그램 생성**: 유효성 검사를 통과하면, `output/csharp/GeneratedFromTemplate.cs` (C# 코드)와 `output/diagram/class_diagram.md` (Mermaid 다이어그램) 파일이 생성됩니다.
 4.  **시각적 확인**: VS Code의 Markdown Preview Mermaid Support 확장 프로그램을 사용하여 `class_diagram.md` 파일을 열어 다이어그램을 시각적으로 확인합니다.

 ---

 ## 5. 향후 확장 방향

 PolyGen의 현재 아키텍처는 다음과 같은 확장을 염두에 두고 설계되었습니다.

 *   **새로운 생성기 추가**: SQL DDL, TypeScript 인터페이스, Protobuf 스키마 등 다양한 타겟 언어 및 형식에 대한 새로운 생성기 모듈을 추가할 수 있습니다. 각 생성기는 AST를 입력으로 받아 해당 언어의 데이터 모델을 구축하고 Askama 템플릿을 통해 코드를 렌더링합니다.
 *   **새로운 어노테이션**: `@version`, `@deprecated` 등 스키마에 새로운 메타데이터를 추가하고, 이를 각 생성기에서 활용하여 타겟 코드에 반영할 수 있습니다.
 *   **스키마 유효성 검사 확장**: 더 복잡한 비즈니스 로직이나 도메인별 제약조건을 AST 유효성 검사 단계에 추가할 수 있습니다.
 *   **설정 파일 기반 모델 구축**: `csharp_generator.rs`의 `build_csharp_model`처럼 하드코딩된 모델 대신, YAML/TOML과 같은 외부 설정 파일로부터 모델을 동적으로 로드하는 기능을 추가할 수 있습니다.

 이 가이드가 PolyGen의 개발에 도움이 되기를 바랍니다.