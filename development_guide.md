# PolyGen 개발 가이드

이 문서는 PolyGen 프로젝트의 아키텍처, 개발 워크플로우, 테스트 절차 등을 설명하는 기술 가이드입니다. 프로젝트에 기여하고자 하는 개발자를 위한 핵심 정보를 제공합니다.

---

## 1. 개요 (Overview)

PolyGen은 스키마 정의(`.poly` 파일)를 입력받아 다양한 프로그래밍 언어에 맞는 코드를 생성하는 코드 생성기입니다. 데이터 모델을 한 곳에서 관리(Single Source of Truth)하여 여러 플랫폼과 언어에 걸쳐 일관성을 유지하는 것을 목표로 합니다.

### 1.1. 핵심 아키텍처

PolyGen은 다음과 같은 파이프라인을 통해 코드를 생성합니다.

```
.poly Schema File(s)
     ↓
[ 1. Parser (pest) ]
// .poly 문법에 따라 텍스트를 파싱하여 초기 구문 트리 생성
     ↓
[ 2. AST Builder (ast_parser.rs) ]
// Pest 구문 트리를 Rust 자료구조인 AST(추상 구문 트리)로 변환
     ↓
[ 3. AST Validation (validation.rs) ]
// 정의되지 않은 타입을 참조하거나 이름이 중복되는 등 스키마의 논리적 오류 검증
     ↓
[ 4. IR Builder (ir_builder.rs) ]
// 템플릿에서 사용하기 쉽도록 AST를 단순화된 IR(중간 표현)로 변환
     ↓
[ 5. Code Generator (rhai_generator.rs) ]
// Rhai 템플릿 엔진을 사용하여 IR 데이터를 템플릿에 주입하고 최종 코드 생성
     ↓
Generated Code (.cs, .ts, etc.)
```

### 1.2. 핵심 원칙

*   **단일 진실 공급원 (Single Source of Truth)**: `.poly` 스키마 파일이 모든 데이터 모델 정보의 유일한 원천입니다.
*   **관심사 분리**: 파싱(Pest), 구문 구조(AST), 템플릿용 데이터(IR), 코드 생성 로직(Rhai)을 명확히 분리합니다.
*   **확장성**: 새로운 타겟 언어는 새로운 Rhai 템플릿을 추가하는 것만으로 쉽게 지원할 수 있습니다.

---

## 2. 시작하기 (Getting Started)

### 2.1. 요구사항

*   [Rust](https://www.rust-lang.org/tools/install) 최신 안정 버전

### 2.2. 빌드

프로젝트 루트 디렉토리에서 다음 명령어를 실행하여 프로젝트를 빌드합니다.

```bash
cargo build --release
```

### 2.3. 실행

PolyGen은 커맨드 라인 인터페이스(CLI)를 통해 실행됩니다.

```bash
cargo run -- --schema-path <SCHEMA_PATH> --lang <LANGUAGE> --output-dir <OUTPUT_PATH>
```

**주요 인자:**

*   `-s`, `--schema-path` (필수): 최상위 `.poly` 스키마 파일의 경로.
*   `-l`, `--lang`: 생성할 코드의 언어 (예: `csharp`, `typescript`). 기본값: `csharp`.
*   `-t`, `--templates-dir`: 템플릿이 위치한 디렉토리. 기본값: `templates`.
*   `-o`, `--output-dir`: 생성된 코드가 저장될 디렉토리. 기본값: `output`.

**실행 예시:**

```bash
cargo run -- --schema-path examples/game_schema.poly --lang csharp
```
위 명령은 `examples/game_schema.poly` 파일을 읽어 `output/csharp/` 디렉토리에 C# 코드를 생성합니다.

---

## 3. 개발 워크플로우

### 3.1. 스키마 문법 수정

`.poly` 파일의 문법을 추가하거나 변경하려면 `polygen.pest` 파일을 수정해야 합니다. 이 파일은 Pest 파서 생성기가 사용하는 문법 정의 파일입니다.

### 3.2. 템플릿 수정

생성되는 코드의 내용을 변경하려면 `templates` 디렉토리의 Rhai 템플릿 (`.rhai`) 파일을 수정해야 합니다.

*   템플릿은 `templates/<언어>/` 디렉토리 구조를 따릅니다.
*   각 언어의 메인 템플릿은 `<언어>_file.rhai` 입니다. (예: `csharp_file.rhai`)
*   템플릿 내부에서는 `ir_model.rs`에 정의된 `SchemaContext` 구조체의 데이터에 접근할 수 있습니다.

### 3.3. 코어 로직 수정

*   **파싱 로직 (`ast_parser.rs`):** `polygen.pest` 변경 후, Pest 트리를 AST로 변환하는 로직을 수정합니다.
*   **AST 유효성 검사 (`validation.rs`):** 새로운 논리적 검증 규칙을 추가합니다.
*   **IR 변환 (`ir_builder.rs`, `ir_model.rs`):** AST의 변경 사항을 템플릿에서 사용할 IR 데이터에 반영합니다.

---

## 4. 테스트 (Testing)

PolyGen은 스냅샷 테스트를 사용하여 코드 변경이 AST 및 IR 생성에 미치는 영향을 확인합니다.

### 4.1. 테스트 실행

프로젝트의 모든 테스트를 실행하려면 다음 명령을 사용합니다.

```bash
cargo test
```

### 4.2. 스냅샷 테스트

`tests/snapshot_tests.rs`는 `tests/schemas` 디렉토리의 모든 `.poly` 파일에 대해 다음을 수행합니다.
1.  AST를 생성하고 스냅샷과 비교합니다.
2.  IR을 생성하고 스냅샷과 비교합니다.

코어 로직 변경으로 AST 또는 IR의 구조가 의도적으로 변경된 경우, 스냅샷 테스트는 실패합니다.

### 4.3. 스냅샷 업데이트

테스트 실패가 의도된 변경의 결과라면, 스냅샷을 업데이트해야 합니다. `insta` CLI 도구를 사용하여 변경 사항을 검토하고 승인할 수 있습니다.

```bash
cargo insta review
```

위 명령을 실행하면 터미널에서 대화형으로 변경 사항을 확인하고 `a` (accept) 키를 눌러 스냅샷을 갱신할 수 있습니다.

---

## 5. 새로운 언어 지원 추가

새로운 프로그래밍 언어 지원을 추가하는 과정은 간단합니다.

1.  **템플릿 디렉토리 생성:** `templates/<new_lang>` 디렉토리를 만듭니다.
2.  **메인 템플릿 작성:** `templates/<new_lang>/<new_lang>_file.rhai` 파일을 작성합니다. 이 템플릿은 `ir_model.rs`의 `SchemaContext`를 루트 데이터로 받습니다.
3.  **(선택) 하위 템플릿 작성:** 코드 재사용을 위해 `struct`, `enum` 등을 위한 하위 템플릿을 만들고 `include()` 함수로 불러올 수 있습니다.
4.  **생성기 실행:** `--lang <new_lang>` 인자를 사용하여 코드를 생성합니다.
    ```bash
    cargo run -- --schema-path examples/game_schema.poly --lang <new_lang>
    ```
5.  **(선택) 정적 파일 복사:** C#의 `DataSource.cs`처럼, 생성된 코드와 함께 배포되어야 할 정적 파일이 있다면 `src/lib.rs`의 `run` 함수에 파일 복사 로직을 추가합니다.

---

## 6. 디버깅 (Debugging)

PolyGen 실행 시 `output/` 디렉토리에 다음과 같은 디버그 파일들이 생성됩니다. 이 파일들은 파싱부터 코드 생성까지 각 단계의 결과를 보여주어 문제 해결에 도움을 줍니다.

*   `output/debug/parse_tree.txt`: Pest가 `.poly` 파일을 파싱한 직후의 원시 구문 트리입니다.
*   `output/ast_debug.txt`: `parse_tree.txt`를 `ast_model.rs`의 Rust 구조체로 변환한 AST의 모습입니다.
*   `output/ir_debug.txt`: AST를 템플릿에서 사용하기 쉽게 변환한 IR의 모습입니다.

문제가 발생했을 때 이 파일들을 순서대로 확인하면 어느 단계에서 문제가 시작되었는지 추적하기 용이합니다.
