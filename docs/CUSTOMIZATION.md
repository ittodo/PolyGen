# PolyGen Template Customization Guide

> 최종 업데이트: 2026-01-28

이 문서는 PolyGen의 Rhai 템플릿 시스템을 커스터마이징하는 방법을 설명합니다.

---

## 개요

PolyGen은 Rhai 스크립트 엔진을 사용하여 코드를 생성합니다. 사용자는 템플릿을 수정하여 생성되는 코드의 스타일, 명명 규칙, 기능을 자유롭게 변경할 수 있습니다.

### 템플릿 구조

```
templates/
├── <language>/
│   ├── <lang>.toml           # 언어 설정 파일
│   ├── <lang>_file.rhai      # 메인 템플릿 (엔티티/구조체)
│   ├── <lang>_*_file.rhai    # 추가 템플릿 (로더, 컨테이너 등)
│   └── rhai_utils/
│       └── type_mapping.rhai # 타입 매핑 유틸리티
├── csharp/                   # C# (8개 템플릿)
├── cpp/                      # C++ (3개 템플릿)
├── rust/                     # Rust (4개 템플릿)
├── typescript/               # TypeScript (3개 템플릿)
└── go/                       # Go (2개 템플릿)
```

---

## 빠른 시작: 공통 커스터마이징

### 1. 타입 매핑 변경

각 언어의 `rhai_utils/type_mapping.rhai`에서 타입 매핑을 수정합니다.

```rhai
// 예: Go에서 u64를 BigInt로 변경
fn map_primitive(type_name) {
    switch type_name {
        "u64" => "BigInt",  // 기본값: "uint64"
        // ...
    }
}
```

### 2. 코드 스타일 변경

메인 템플릿(`<lang>_file.rhai`)에서 생성되는 코드의 형식을 변경합니다.

```rhai
// 예: C#에서 필드를 프로퍼티 대신 public 필드로
// Before: public int Id { get; set; }
// After:  public int Id;
output += `    public ${field_type} ${field.name};\n`;
```

### 3. 네이밍 컨벤션 변경

Rhai의 내장 함수를 사용하여 이름 변환:

```rhai
to_pascal_case("player_name")  // "PlayerName"
to_camel_case("player_name")   // "playerName"
to_snake_case("PlayerName")    // "player_name"
to_upper(text)                 // 대문자 변환
to_lower(text)                 // 소문자 변환
```

---

## 언어별 상세 가이드

### C# 커스터마이징

#### 파일 구조
```
templates/csharp/
├── csharp.toml                    # 설정
├── csharp_file.rhai               # 엔티티 클래스
├── csharp_container_file.rhai     # 데이터 컨테이너
├── csharp_csv_mappers_file.rhai   # CSV 파서
├── csharp_csv_columns_file.rhai   # CSV 컬럼 정의
├── csharp_binary_readers_file.rhai # 바이너리 읽기
├── csharp_binary_writers_file.rhai # 바이너리 쓰기
├── csharp_datacontext_file.rhai   # DataContext
├── csharp_sqlite_accessor_file.rhai # SQLite 접근자
└── rhai_utils/
    ├── type_utils.rhai            # 타입 유틸리티
    └── binary_mapping.rhai        # 바이너리 매핑
```

#### 주요 커스터마이징 포인트

**1. 네임스페이스 구조**
```rhai
// csharp_file.rhai
let namespace_name = to_pascal_case(ns.name);
// 커스텀: 접두사 추가
let namespace_name = "MyProject." + to_pascal_case(ns.name);
```

**2. 어트리뷰트 추가**
```rhai
// 클래스에 Serializable 추가
output += `[Serializable]\n`;
output += `public class ${struct.name}\n`;
```

**3. 인터페이스 구현**
```rhai
output += `public class ${struct.name} : IEntity, IValidatable\n`;
```

**4. Validation 커스터마이징** (`csharp_container_file.rhai`)
```rhai
// 커스텀 검증 메시지
output += `        if (item.${field.name}.Length > ${max_len})\n`;
output += `            errors.Add($"Field '${field.name}' exceeds {${max_len}} characters");\n`;
```

#### C# 타입 매핑 (Rust 헬퍼)

C#은 현재 Rust 헬퍼(`src/rhai/csharp/type_mapping.rs`)를 사용합니다:

| IR 타입 | C# 타입 |
|---------|---------|
| `u8` | `byte` |
| `i8` | `sbyte` |
| `u16`/`i16` | `ushort`/`short` |
| `u32`/`i32` | `uint`/`int` |
| `u64`/`i64` | `ulong`/`long` |
| `f32`/`f64` | `float`/`double` |
| `string` | `string` |
| `bytes` | `byte[]` |
| `timestamp` | `DateTime` |
| `Option<T>` | `T?` |
| `List<T>` | `List<T>` |

---

### C++ 커스터마이징

#### 파일 구조
```
templates/cpp/
├── cpp.toml                # 설정
├── cpp_file.rhai           # 구조체/Enum 정의
├── cpp_loaders_file.rhai   # CSV/JSON/Binary 로더
└── rhai_utils/
    └── type_mapping.rhai   # 타입 매핑
```

#### 주요 커스터마이징 포인트

**1. 헤더 가드 스타일**
```rhai
// 기본: #pragma once
output += "#pragma once\n";

// 대안: include guard
output += `#ifndef ${guard_name}_HPP\n`;
output += `#define ${guard_name}_HPP\n`;
```

**2. STL 컨테이너 선택**
```rhai
// type_mapping.rhai
if type_ref.is_list {
    return "std::vector<" + base_type + ">";  // 기본
    // 대안: return "std::deque<" + base_type + ">";
}
```

**3. 스마트 포인터 사용**
```rhai
if type_ref.is_option {
    return "std::optional<" + base_type + ">";  // 기본
    // 대안: return "std::shared_ptr<" + base_type + ">";
}
```

**4. 직렬화 라이브러리** (`cpp_loaders_file.rhai`)
```rhai
// 기본: nlohmann/json
output += `#include <nlohmann/json.hpp>\n`;

// 대안: RapidJSON으로 변경
output += `#include <rapidjson/document.h>\n`;
```

#### C++ 타입 매핑

| IR 타입 | C++ 타입 |
|---------|----------|
| `u8`/`i8` | `uint8_t`/`int8_t` |
| `u16`/`i16` | `uint16_t`/`int16_t` |
| `u32`/`i32` | `uint32_t`/`int32_t` |
| `u64`/`i64` | `uint64_t`/`int64_t` |
| `f32`/`f64` | `float`/`double` |
| `bool` | `bool` |
| `string` | `std::string` |
| `bytes` | `std::vector<uint8_t>` |
| `timestamp` | `std::chrono::system_clock::time_point` |
| `Option<T>` | `std::optional<T>` |
| `List<T>` | `std::vector<T>` |

---

### Rust 커스터마이징

#### 파일 구조
```
templates/rust/
├── rust.toml                   # 설정
├── rust_file.rhai              # 구조체/Enum 정의
├── rust_loaders_file.rhai      # CSV/JSON/Binary 로더
├── rust_container_file.rhai    # 컨테이너 시스템
├── rust_sqlite_accessor_file.rhai # SQLite 접근자
└── rhai_utils/
    └── type_mapping.rhai       # 타입 매핑
```

#### 주요 커스터마이징 포인트

**1. Derive 매크로 커스터마이징**
```rhai
// rust_file.rhai
let derives = "#[derive(Debug, Clone, Serialize, Deserialize)]";

// 추가 derive
let derives = "#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]";
```

**2. Serde 어트리뷰트**
```rhai
// 필드에 serde 어트리뷰트 추가
output += `    #[serde(rename = "${field.name}", default)]\n`;
output += `    pub ${field_name}: ${field_type},\n`;
```

**3. 가시성 변경**
```rhai
// 기본: pub 필드
output += `    pub ${field_name}: ${field_type},\n`;

// 대안: pub(crate)
output += `    pub(crate) ${field_name}: ${field_type},\n`;
```

**4. 에러 처리** (`rust_loaders_file.rhai`)
```rhai
// thiserror 사용
output += `#[derive(Debug, thiserror::Error)]\n`;
output += `pub enum LoadError {\n`;
output += `    #[error("CSV parse error: {0}")]\n`;
output += `    CsvError(#[from] csv::Error),\n`;
```

#### Rust 타입 매핑

| IR 타입 | Rust 타입 |
|---------|-----------|
| `u8`-`u64` | `u8`-`u64` |
| `i8`-`i64` | `i8`-`i64` |
| `f32`/`f64` | `f32`/`f64` |
| `bool` | `bool` |
| `string` | `String` |
| `bytes` | `Vec<u8>` |
| `timestamp` | `chrono::DateTime<chrono::Utc>` |
| `Option<T>` | `Option<T>` |
| `List<T>` | `Vec<T>` |

---

### TypeScript 커스터마이징

#### 파일 구조
```
templates/typescript/
├── typescript.toml             # 설정
├── typescript_file.rhai        # 인터페이스/Enum
├── typescript_zod_file.rhai    # Zod 스키마
├── typescript_container_file.rhai # 컨테이너
└── rhai_utils/
    └── type_mapping.rhai       # 타입 매핑
```

#### 주요 커스터마이징 포인트

**1. 인터페이스 vs 타입**
```rhai
// 기본: interface
output += `export interface ${struct.name} {\n`;

// 대안: type alias
output += `export type ${struct.name} = {\n`;
```

**2. 클래스 생성**
```rhai
// interface 대신 class로 변경
output += `export class ${struct.name} {\n`;
for field in struct.fields {
    output += `    public ${field.name}: ${field_type};\n`;
}
output += `    constructor(data: Partial<${struct.name}>) {\n`;
output += `        Object.assign(this, data);\n`;
output += `    }\n`;
output += `}\n`;
```

**3. Zod 스키마 커스터마이징** (`typescript_zod_file.rhai`)
```rhai
// 커스텀 에러 메시지
output += `z.string().max(${max_len}, { message: "이름은 ${max_len}자 이하여야 합니다" })`;
```

**4. 임포트 스타일**
```rhai
// 기본: named imports
output += `import { z } from 'zod';\n`;

// 대안: namespace import
output += `import * as z from 'zod';\n`;
```

#### TypeScript 타입 매핑

| IR 타입 | TypeScript 타입 |
|---------|-----------------|
| `u8`-`u64`, `i8`-`i64` | `number` |
| `f32`/`f64` | `number` |
| `bool` | `boolean` |
| `string` | `string` |
| `bytes` | `Uint8Array` |
| `timestamp` | `Date` |
| `Option<T>` | `T \| null` |
| `List<T>` | `T[]` |

---

### Go 커스터마이징

#### 파일 구조
```
templates/go/
├── go.toml                   # 설정
├── go_file.rhai              # 구조체/Enum
├── go_container_file.rhai    # 컨테이너
└── rhai_utils/
    └── type_mapping.rhai     # 타입 매핑
```

#### 주요 커스터마이징 포인트

**1. JSON 태그 커스터마이징**
```rhai
// 기본: json 태그
output += `    ${field_name} ${field_type} \`json:"${field.name}"\`\n`;

// 확장: json + yaml + db
output += `    ${field_name} ${field_type} \`json:"${field.name}" yaml:"${field.name}" db:"${field.name}"\`\n`;
```

**2. omitempty 제어**
```rhai
if type_ref.is_option {
    output += `    ${field_name} ${field_type} \`json:"${field.name},omitempty"\`\n`;
} else {
    output += `    ${field_name} ${field_type} \`json:"${field.name}"\`\n`;
}
```

**3. 커스텀 메서드 추가**
```rhai
// Stringer 인터페이스 구현
output += `func (s *${struct.name}) String() string {\n`;
output += `    return fmt.Sprintf("${struct.name}{ID: %v}", s.ID)\n`;
output += `}\n`;
```

**4. Validation 추가** (`go_container_file.rhai`)
```rhai
// go-playground/validator 태그
output += `    ${field_name} ${field_type} \`validate:"required,max=${max_len}"\`\n`;
```

#### Go 타입 매핑

| IR 타입 | Go 타입 |
|---------|---------|
| `u8`-`u64` | `uint8`-`uint64` |
| `i8`-`i64` | `int8`-`int64` |
| `f32`/`f64` | `float32`/`float64` |
| `bool` | `bool` |
| `string` | `string` |
| `bytes` | `[]byte` |
| `Option<T>` | `*T` |
| `List<T>` | `[]T` |

---

## Rhai 스크립트 참조

### 사용 가능한 전역 변수

| 변수 | 타입 | 설명 |
|------|------|------|
| `schema` | `SchemaContext` | 전체 스키마 컨텍스트 |
| `output_dir` | `String` | 출력 디렉토리 경로 |

### SchemaContext 구조

```rhai
schema.files              // FileDef[] - 파일 목록
schema.files[0].name      // 파일 이름
schema.files[0].namespaces // NamespaceDef[] - 네임스페이스 목록

namespace.name            // 네임스페이스 이름
namespace.fqn             // 완전한 이름 (예: "game.character")
namespace.structs         // StructDef[] - 구조체/테이블 목록
namespace.enums           // EnumDef[] - Enum 목록

struct.name               // 구조체 이름
struct.items              // StructItem[] - 필드, 주석, Embed 등
struct.annotations        // AnnotationDef[] - @load, @readonly 등

field.name                // 필드 이름
field.field_type          // TypeRef - 타입 정보
field.is_primary_key      // bool
field.is_unique           // bool
field.max_length          // Option<i32>
field.default_value       // Option<String>
field.range               // Option<{min, max}>
field.foreign_key         // Option<String>

enum_def.name             // Enum 이름
enum_def.variants         // EnumVariant[] - 값 목록
```

### 유용한 내장 함수

```rhai
// 문자열 변환
to_pascal_case(s)         // "hello_world" → "HelloWorld"
to_camel_case(s)          // "hello_world" → "helloWorld"
to_snake_case(s)          // "HelloWorld" → "hello_world"
to_upper(s)               // 대문자
to_lower(s)               // 소문자

// 문자열 조작
s.trim()                  // 공백 제거
s.split(delim)            // 분할
s.replace(old, new)       // 치환
s.starts_with(prefix)     // 접두사 확인
s.ends_with(suffix)       // 접미사 확인
s.contains(sub)           // 포함 확인
s.sub_string(start, len)  // 부분 문자열

// 파일 쓰기
write_file(path, content) // 파일 생성
print(msg)                // 디버그 출력
```

### 헬퍼 함수 (registry.rs에서 등록됨)

```rhai
// 필드 조회
get_field(struct, field_name)      // Option<FieldDef>
get_primary_key(struct)            // Option<FieldDef>
find_struct(schema, fqn)           // Option<StructDef>
find_enum(schema, fqn)             // Option<EnumDef>

// 어노테이션 조회
get_annotation(struct, name)       // Option<AnnotationDef>
has_annotation(struct, name)       // bool

// 타입 조회
is_primitive(type_name)            // bool
is_numeric(type_name)              // bool
is_integer(type_name)              // bool
```

---

## 일반적인 커스터마이징 시나리오

### 1. 새 어트리뷰트/데코레이터 추가

```rhai
// 모든 클래스에 커스텀 어트리뷰트 추가
output += `[GeneratedCode("PolyGen", "1.0")]\n`;
output += `public class ${struct.name}\n`;
```

### 2. 로깅 코드 삽입

```rhai
// 로더에 로깅 추가
output += `    Console.WriteLine($"Loading {count} records from ${file_path}");\n`;
```

### 3. 커스텀 Validation 규칙

```rhai
// 비즈니스 로직 검증 추가
if field.name.ends_with("Email") {
    output += `    if (!IsValidEmail(${field.name})) errors.Add("Invalid email format");\n`;
}
```

### 4. 주석 스타일 변경

```rhai
// XML 문서 주석 (C#)
if struct.doc_comment != () {
    output += `/// <summary>\n`;
    output += `/// ${struct.doc_comment}\n`;
    output += `/// </summary>\n`;
}
```

### 5. 기본 생성자 추가

```rhai
// 파라미터 없는 생성자
output += `    public ${struct.name}() { }\n\n`;

// 전체 필드 생성자
output += `    public ${struct.name}(${param_list})\n`;
output += `    {\n`;
for field in fields {
    output += `        this.${field.name} = ${to_camel_case(field.name)};\n`;
}
output += `    }\n`;
```

### 6. 직렬화 옵션 추가

```rhai
// JSON 직렬화 설정
output += `[JsonPropertyName("${to_camel_case(field.name)}")]\n`;
output += `public ${field_type} ${field.name} { get; set; }\n`;
```

### 7. 불변 객체 생성

```rhai
// C#: init-only 프로퍼티
output += `    public ${field_type} ${field.name} { get; init; }\n`;

// Rust: 이미 기본적으로 불변

// TypeScript: readonly
output += `    readonly ${field.name}: ${field_type};\n`;
```

---

## 디버깅 팁

### 1. 변수 값 확인

```rhai
print("Struct name: " + struct.name);
print("Field count: " + struct.items.len);
```

### 2. 조건부 출력

```rhai
if struct.annotations.len > 0 {
    print("Has annotations: " + struct.annotations.len);
}
```

### 3. IR 구조 탐색

```rhai
for file in schema.files {
    print("File: " + file.name);
    for ns in file.namespaces {
        print("  Namespace: " + ns.fqn);
        for s in ns.structs {
            print("    Struct: " + s.name);
        }
    }
}
```

---

## 참고 문서

| 문서 | 설명 |
|------|------|
| [SOURCE_STRUCTURE.md](SOURCE_STRUCTURE.md) | 소스 코드 구조 |
| [CLAUDE.md](../CLAUDE.md) | 프로젝트 가이드 |
| [templates/agent.md](../templates/agent.md) | 템플릿 시스템 상세 |
| [Rhai Book](https://rhai.rs/book/) | Rhai 공식 문서 |

---

*최종 업데이트: 2026-01-28*
