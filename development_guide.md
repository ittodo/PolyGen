# PolyGen 개발 가이드: 내부 구조 및 구현 방향

 이 문서는 PolyGen 프로젝트의 아키텍처, 주요 구성 요소, 그리고 각 기능이 어떻게 구현되었거나 구현될 예정인지에 대한 기술적인 가이드입니다. PolyGen의 "살아있는 설계 청사진"을 만드는 과정과 그 기반이 되는 원칙들을 설명합니다.

 ---

 ## 1. PolyGen의 아키텍처 개요

 PolyGen은 스키마 정의(`.poly` 파일)를 입력받아 파싱하고, 추상 구문 트리(AST)로 변환합니다. 그 후, AST는 **범용 생성기**를 통해 처리됩니다.

 1.  **범용 코드 생성**: AST를 언어 중립적인 **중간 표현(IR)**으로 변환한 후, **MiniJinja 템플릿 엔진**을 사용하여 C#, TypeScript 등 다양한 언어의 코드를 생성합니다.
 2.  **Mermaid 다이어그램 생성**: AST를 직접 분석하여 **MiniJinja 템플릿 엔진**으로 Mermaid 클래스 다이어그램을 생성합니다. (이 기능은 범용 생성기에 통합되었습니다.)

 ```
 .poly Schema
      ↓
 [ Parser (pest) ]
      ↓
 [ Abstract Syntax Tree (AST) ]
      ↓
 [ IR Builder ]
      ↓
 [ Intermediate Representation (IR) ]
      ↓
 [ Generic Generator (minijinja) ] --+--> C# Code (.cs)
                                     +--> TypeScript Code (.ts)
                                     +--> Mermaid Diagram (.md)
                                     +--> ... Other Languages
 ```

 ### 핵심 원칙

 *   **단일 진실 공급원 (Single Source of Truth)**: `.poly` 스키마 파일이 모든 데이터 모델 정보의 유일한 원천입니다.
 *   **관심사 분리**: AST(구문 구조), IR(템플릿용 데이터), 생성기(로직), 템플릿(표현)을 명확히 분리합니다.
 *   **확장성**: 새로운 타겟 언어는 새로운 `minijinja` 템플릿을 추가하는 것만으로 쉽게 지원할 수 있습니다.
 *   **유연성**: 어노테이션 시스템을 통해 스키마의 핵심 문법을 변경하지 않고도 다양한 메타데이터와 기능을 추가할 수 있습니다.

 ---

 ## 2. 주요 구성 요소 및 역할

 ### `d:\Rust\PolyGen\src\main.rs`
 *   **역할**: PolyGen 애플리케이션의 진입점입니다. 명령줄 인자를 파싱하고, 스키마 파일을 읽어 파싱 및 AST 빌드, 유효성 검사를 수행합니다. 그 후 **IR 빌더**와 **범용 생성기**를 호출하여 코드를 생성하고, **Mermaid 생성기**를 별도로 호출하여 다이어그램을 생성하는 전체 워크플로우를 관리합니다.

 ### `d:\Rust\PolyGen\polygen.pest`
 *   **역할**: PolyGen 스키마 언어의 문법을 정의하는 Pest 문법 파일입니다. `table`, `enum`, `embed`, `field`, `constraint`, `annotation` 등 스키마의 모든 구성 요소를 규칙(Rule)으로 정의합니다.

 ### `d:\Rust\PolyGen\src\ast.rs`
 *   **역할**: 파싱된 `.poly` 스키마의 구조를 표현하는 **추상 구문 트리(AST)**의 정의를 포함합니다. 스키마의 모든 요소를 Rust의 `enum`과 `struct`로 모델링하며, 스키마의 모든 정보를 가장 정확하게 표현합니다.

 ### `d:\Rust\PolyGen\src\validation.rs`
 *   **역할**: 빌드된 AST의 유효성을 검사합니다. 예를 들어, 중복된 이름의 정의가 있는지, 참조하는 타입이 실제로 존재하는지 등을 확인하여 스키마의 논리적 일관성을 보장합니다.

 ### `d:\Rust\PolyGen\src\ir_model.rs` (신규)
 *   **역할**: 템플릿 생성을 위한 **언어 중립적 중간 표현(Intermediate Representation, IR)**을 정의합니다. `SchemaContext`, `NamespaceDef`, `StructDef`, `FieldDef` 등 템플릿에서 사용하기 쉽고 단순화된 데이터 구조를 포함합니다. 이 모델은 `serde::Serialize`를 구현하여 모든 `minijinja` 템플릿에 컨텍스트로 전달됩니다.

 ### `d:\Rust\PolyGen\src\ir_builder.rs` (신규)
 *   **역할**: AST를 순회하며 `ir_model`에 정의된 **IR 객체를 구축**하는 로직을 담당합니다. 복잡한 AST 구조를 템플릿에서 사용하기 쉬운 평탄하고 단순한 IR 구조로 변환하는 핵심적인 역할을 합니다.

 ### `d:\Rust\PolyGen\src\generator.rs`
 *   **역할**: **범용 코드 생성기**입니다. **`minijinja`** 템플릿 엔진을 사용하여 IR 데이터를 템플릿에 주입하고 최종 코드를 렌더링합니다. `lang` 파라미터(예: "csharp")에 따라 `templates/{lang}/{lang}_file.jinja`와 같은 동적인 경로의 템플릿을 로드하여 코드를 생성하며, Mermaid 다이어그램 생성 기능도 통합되어 있습니다. 새로운 언어 지원이 용이합니다.

 ### `d:\Rust\PolyGen\templates`
 *   **역할**: 코드 및 다이어그램 생성을 위한 템플릿 파일들을 포함합니다.
     *   `templates/{lang}/`: **`minijinja`** 템플릿이 위치합니다. (예: `templates/csharp/csharp_file.jinja`). 각 템플릿은 `ir_model`의 데이터를 입력받아 특정 언어의 코드를 생성합니다.
     *   `templates/mermaid/`: **`minijinja`** 템플릿이 위치합니다. `mermaid_model` 데이터를 기반으로 Mermaid 다이어그램 문법을 생성합니다.

 ---

 ## 3. 주요 기능 구현 상세

 ### 3.1. 어노테이션 (`@taggable`, `@link_rows`, `@load`, `@save`)

 *   **AST 파싱**: `d:\Rust\PolyGen\src\ast.rs`에서 어노테이션을 파싱하여 `Annotation` 구조체로 변환합니다. (기존과 동일)
 *   **Mermaid 다이어그램 표시**: `d:\Rust\PolyGen\src\generator.rs`에서 어노테이션을 Mermaid 스테레오타입으로 변환하여 표시합니다. (기존과 동일)
 *   **코드 생성 활용 (IR 기반)**: `d:\Rust\PolyGen\src\ir_builder.rs`가 AST의 어노테이션을 분석하여 IR의 `StructDef`나 `FieldDef`에 특정 플래그나 속성 목록을 추가할 수 있습니다. 그러면 `minijinja` 템플릿은 이 데이터를 활용하여 조건부로 코드를 생성합니다. 예를 들어, `field.attributes`에 `Key`가 포함되어 있으면 C# 템플릿에서 `[Key]` 어트리뷰트를 출력할 수 있습니다.

 ### 3.2. 임베드 타입 (`embed`)

 *   **AST 파싱**: `d:\Rust\PolyGen\src\ast.rs`에서 `embed` 정의를 파싱합니다. (기존과 동일)
 *   **Mermaid 다이어그램 표시**: `d:\Rust\PolyGen\src\generator.rs`에서 명명된 임베드는 별도 클래스로, 인라인 임베드는 부모 클래스의 속성으로 처리합니다. (기존과 동일)
 *   **코드 생성 활용 (IR 기반)**: `d:\Rust\PolyGen\src\ir_builder.rs`는 AST의 `embed`를 `ir_model::StructDef`로 변환합니다. 
     *   **네임스페이스/테이블 레벨 명명된 임베드**: 별도의 `StructDef`로 변환되어 재사용 가능한 타입이 됩니다.
     *   **인라인 임베드**: 부모 타입 내에 중첩된 `StructDef`로 변환되거나, 필드 이름에 기반한 새로운 타입(예: `DropItems`)으로 만들어져 부모 필드의 타입으로 참조됩니다. 템플릿은 이 구조를 보고 중첩 클래스나 별도의 클래스 파일을 생성할 수 있습니다.

 ### 3.3. 외래 키 및 관계 (`foreign_key`)

 *   **AST 파싱**: `d:\Rust\PolyGen\src\ast.rs`에서 `foreign_key` 제약조건을 파싱합니다. (기존과 동일)
 *   **Mermaid 다이어그램 표시**: `d:\Rust\PolyGen\src\generator.rs`에서 `foreign_key`를 분석하여 테이블 간의 관계선과 카디널리티를 생성합니다. (기존과 동일)
 *   **코드 생성 활용 (IR 기반)**: `d:\Rust\PolyGen\src\ir_builder.rs`는 `foreign_key` 제약조건을 분석하여 `ir_model::FieldDef`에 `[ForeignKey("PlayerId")]`와 같은 어트리뷰트 문자열을 추가할 수 있습니다. C# 템플릿은 이 어트리뷰트 문자열을 그대로 렌더링하여 ORM 등이 인식할 수 있는 코드를 생성합니다. `as <RelationName>` 정보는 탐색 속성(Navigation Property)을 생성하는 데 사용될 수 있습니다.

 ---

 ## 4. 개발 워크플로우

 1.  **스키마 정의**: `game_schema.poly` 또는 다른 `.poly` 파일을 수정하여 데이터 모델을 정의합니다.
 2.  **코드 및 다이어그램 생성**: `cargo run -- --schema-path examples/game_schema.poly --lang csharp` 명령을 실행합니다.
     *   PolyGen이 스키마를 파싱하고 AST 빌드 및 유효성 검사를 수행합니다.
     *   AST가 IR로 변환된 후, `output/csharp/` 디렉토리에 C# 코드가 생성됩니다.
     *   동시에 `output/diagram/class_diagram.md`에 Mermaid 다이어그램이 생성됩니다.
 3.  **시각적 확인**: VS Code의 Markdown Preview Mermaid Support 확장 프로그램을 사용하여 `class_diagram.md` 파일을 열어 다이어그램을 시각적으로 확인합니다.

 ---

 ## 5. 향후 확장 방향

 PolyGen의 현재 아키텍처는 다음과 같은 확장을 염두에 두고 설계되었습니다.

 *   **새로운 생성기 추가**: TypeScript 인터페이스를 생성하고 싶다면, `templates/typescript/typescript_file.jinja` 템플릿을 작성하고 `cargo run -- ... --lang typescript`를 실행하기만 하면 됩니다. 별도의 Rust 코드 변경이 거의 필요 없습니다.
 *   **새로운 어노테이션**: `@version`, `@deprecated` 등 스키마에 새로운 메타데이터를 추가하고, `ir_builder.rs`가 이를 해석하여 IR에 반영한 후, 각 언어 템플릿에서 이 정보를 활용하여 코드(예: C#의 `[Obsolete]` 어트리뷰트)를 생성하도록 확장할 수 있습니다.
 *   **스키마 유효성 검사 확장**: 더 복잡한 비즈니스 로직이나 도메인별 제약조건을 `validation.rs`에 추가할 수 있습니다.

 이 가이드가 PolyGen의 개발에 도움이 되기를 바랍니다.