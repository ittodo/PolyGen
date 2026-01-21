# PolyGen 언어 지원 가이드

이 문서는 PolyGen에서 새로운 언어를 지원할 때 구현해야 하는 기능들을 정리합니다.

---

## 개요

PolyGen의 언어 지원은 **3단계**로 구분됩니다:

| 단계 | 이름 | 설명 | 예상 작업량 |
|------|------|------|------------|
| **Level 1** | Basic | 기본 타입 생성 (struct, enum, module) | ~300 LOC |
| **Level 2** | Loaders | 데이터 로더 (CSV, JSON, Binary) | ~800 LOC |
| **Level 3** | Container | 컨테이너 시스템 (인덱스, 관계) | ~600 LOC |

---

## Level 1: Basic (기본 타입 생성)

### 필수 파일

```
templates/{lang}/
├── {lang}.toml              # 언어 설정
├── {lang}_file.rhai         # 메인 진입점
├── {lang}_mod.rhai          # 모듈/네임스페이스 생성
├── {lang}_struct.rhai       # 구조체/클래스 생성
├── {lang}_enum.rhai         # Enum 생성
└── rhai_utils/
    └── type_mapping.rhai    # 타입 매핑
```

### 구현 체크리스트

- [ ] **타입 매핑**: .poly 타입 → 언어별 타입
  ```
  string   → String (Rust), string (C#), std::string (C++)
  bool     → bool
  bytes    → Vec<u8> (Rust), byte[] (C#), std::vector<uint8_t> (C++)
  u8~u64   → u8~u64 (Rust), byte~ulong (C#), uint8_t~uint64_t (C++)
  i8~i64   → i8~i64 (Rust), sbyte~long (C#), int8_t~int64_t (C++)
  f32, f64 → f32, f64 (Rust), float, double (C#/C++)
  ```

- [ ] **카디널리티**
  ```
  T?   → Option<T> (Rust), T? (C#), std::optional<T> (C++)
  T[]  → Vec<T> (Rust), List<T> (C#), std::vector<T> (C++)
  ```

- [ ] **Struct 생성**
  - 필드명 케이스 변환 (snake_case, camelCase 등)
  - Doc comment 변환
  - 직렬화 속성 (serde, Serializable 등)

- [ ] **Enum 생성**
  - 명시적 값 할당 지원
  - 정수 표현 타입 지정

- [ ] **모듈/네임스페이스**
  - 중첩 namespace 지원
  - 이름 변환 (snake_case 등)

### 예시: Rust Level 1 (완료)

```rust
// 생성 결과
pub mod game_item {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[repr(i32)]
    pub enum ItemType {
        WEAPON = 0,
        ARMOR = 1,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Item {
        pub id: u32,
        pub name: String,
        pub item_type: ItemType,
        pub description: Option<String>,
    }
}
```

---

## Level 2: Loaders (데이터 로더)

외부 데이터 파일(CSV, JSON, Binary)을 읽어서 생성된 타입으로 변환합니다.

### 필수 파일

```
templates/{lang}/
├── {lang}_csv_loader.rhai       # CSV 로더 생성
├── {lang}_json_loader.rhai      # JSON 로더 생성
├── {lang}_binary_reader.rhai    # Binary 리더 생성
├── {lang}_binary_writer.rhai    # Binary 라이터 생성
└── rhai_utils/
    ├── csv_mapping.rhai         # CSV 파싱 로직
    ├── json_mapping.rhai        # JSON 파싱 로직
    └── binary_mapping.rhai      # Binary I/O 로직

static/{lang}/
├── CsvUtils.{ext}               # CSV 파싱 유틸리티
├── JsonUtils.{ext}              # JSON 파싱 유틸리티
└── BinaryUtils.{ext}            # Binary I/O 유틸리티
```

### 2.1 CSV Loader

#### 기능 요구사항

1. **헤더 기반 매핑**: CSV 첫 줄을 헤더로 사용, 필드명과 매핑
2. **타입 변환**: 문자열 → 타입별 파싱
3. **Optional 처리**: 빈 문자열 → None/null
4. **배열 처리**: 구분자로 분리 (예: `"1,2,3"` → `[1, 2, 3]`)
5. **Enum 처리**: 문자열/숫자 → Enum 값

#### 생성 코드 예시 (의사 코드)

```rust
// Rust
impl Item {
    pub fn from_csv_row(row: &CsvRow) -> Result<Self, Error> {
        Ok(Self {
            id: row.get("id")?.parse()?,
            name: row.get("name")?.to_string(),
            item_type: ItemType::from_str(row.get("item_type")?)?,
            description: row.get_optional("description").map(|s| s.to_string()),
        })
    }
}

pub fn load_items_from_csv(path: &str) -> Result<Vec<Item>, Error> {
    let reader = csv::Reader::from_path(path)?;
    reader.records()
        .map(|r| Item::from_csv_row(&r?))
        .collect()
}
```

```csharp
// C#
public static class ItemCsvMapper
{
    public static Item FromCsvRow(CsvRow row)
    {
        return new Item
        {
            Id = row.GetUInt32("id"),
            Name = row.GetString("name"),
            ItemType = row.GetEnum<ItemType>("item_type"),
            Description = row.GetOptionalString("description"),
        };
    }

    public static List<Item> LoadFromCsv(string path)
    {
        return CsvUtils.ReadAll(path, FromCsvRow);
    }
}
```

### 2.2 JSON Loader

#### 기능 요구사항

1. **객체 배열**: `[{...}, {...}]` 형태 파싱
2. **중첩 객체**: embed 타입 처리
3. **Optional 처리**: null → None
4. **타입 검증**: 타입 불일치 시 에러

#### 생성 코드 예시

```rust
// Rust (serde 사용 시 자동 생성됨)
pub fn load_items_from_json(path: &str) -> Result<Vec<Item>, Error> {
    let content = std::fs::read_to_string(path)?;
    let items: Vec<Item> = serde_json::from_str(&content)?;
    Ok(items)
}
```

```csharp
// C#
public static class ItemJsonMapper
{
    public static Item FromJsonElement(JsonElement elem)
    {
        return new Item
        {
            Id = elem.GetProperty("id").GetUInt32(),
            Name = elem.GetProperty("name").GetString()!,
            ItemType = Enum.Parse<ItemType>(elem.GetProperty("item_type").GetString()!),
            Description = elem.TryGetProperty("description", out var desc)
                ? desc.GetString() : null,
        };
    }
}
```

### 2.3 Binary Reader/Writer

#### 기능 요구사항

1. **고정 크기 타입**: 리틀/빅 엔디안 선택
2. **가변 길이 타입**: 길이 prefix + 데이터
3. **문자열**: UTF-8, 길이 prefix (u32)
4. **배열**: 길이 prefix (u32) + 요소들
5. **Optional**: 존재 플래그 (u8: 0/1) + 값
6. **Enum**: 정수 값으로 저장

#### 직렬화 형식

```
[u32: 문자열 길이][bytes: UTF-8 문자열]     // string
[u8: 0 또는 1][T: 값 (존재 시)]            // Option<T>
[u32: 배열 길이][T: 요소1][T: 요소2]...    // Vec<T>
[i32: enum 값]                            // Enum
```

#### 생성 코드 예시

```rust
// Rust
impl Item {
    pub fn read_binary<R: Read>(reader: &mut R) -> Result<Self, Error> {
        Ok(Self {
            id: reader.read_u32::<LittleEndian>()?,
            name: read_string(reader)?,
            item_type: ItemType::try_from(reader.read_i32::<LittleEndian>()?)?,
            description: read_optional_string(reader)?,
        })
    }

    pub fn write_binary<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        writer.write_u32::<LittleEndian>(self.id)?;
        write_string(writer, &self.name)?;
        writer.write_i32::<LittleEndian>(self.item_type as i32)?;
        write_optional_string(writer, &self.description)?;
        Ok(())
    }
}
```

---

## Level 3: Container (컨테이너 시스템)

모든 데이터 테이블을 통합 관리하고, 인덱스와 관계를 제공합니다.

### 필수 파일

```
templates/{lang}/
├── {lang}_container.rhai        # 컨테이너 생성
├── container/
│   ├── {lang}_data_table.rhai   # DataTable 클래스
│   ├── {lang}_indexes.rhai      # 인덱스 생성
│   └── {lang}_relations.rhai    # 관계 생성

static/{lang}/
└── DataContainer.{ext}          # 컨테이너 기반 클래스
```

### 3.1 DataTable

각 테이블(struct)별로 생성되는 컬렉션 래퍼입니다.

#### 기능 요구사항

1. **All**: 모든 행 조회
2. **Count**: 행 수
3. **Iterator**: foreach 지원
4. **AddRow**: 행 추가 (인덱스 자동 업데이트)

```rust
// Rust 예시
pub struct ItemTable {
    rows: Vec<Item>,
    by_id: HashMap<u32, usize>,      // UniqueIndex
    by_type: HashMap<ItemType, Vec<usize>>,  // GroupIndex
}

impl ItemTable {
    pub fn all(&self) -> &[Item] { &self.rows }
    pub fn count(&self) -> usize { self.rows.len() }
    pub fn get_by_id(&self, id: u32) -> Option<&Item> { ... }
    pub fn get_by_type(&self, item_type: ItemType) -> &[Item] { ... }
}
```

### 3.2 Indexes

IR의 `IndexDef`를 기반으로 생성합니다.

#### 인덱스 유형

| 제약조건 | 인덱스 유형 | 반환 타입 |
|----------|------------|-----------|
| `primary_key` | UniqueIndex | `Option<&T>` |
| `unique` | UniqueIndex | `Option<&T>` |
| `index` | GroupIndex | `&[T]` |
| `foreign_key` | GroupIndex | `&[T]` |

#### IR 정보 활용

```rust
// IR의 IndexDef 구조
pub struct IndexDef {
    pub name: String,        // "ById", "ByName" 등
    pub field_name: String,  // "id", "name" 등
    pub field_type: TypeRef, // 키 타입
    pub is_unique: bool,     // true: UniqueIndex, false: GroupIndex
}
```

#### 생성 코드 예시

```rust
// 템플릿에서 s.indexes 순회하여 생성
impl ItemTable {
    // IndexDef { name: "ById", field_name: "id", is_unique: true }
    pub fn get_by_id(&self, id: u32) -> Option<&Item> {
        self.by_id.get(&id).map(|&idx| &self.rows[idx])
    }

    // IndexDef { name: "ByType", field_name: "item_type", is_unique: false }
    pub fn get_by_type(&self, item_type: ItemType) -> Vec<&Item> {
        self.by_type.get(&item_type)
            .map(|indices| indices.iter().map(|&i| &self.rows[i]).collect())
            .unwrap_or_default()
    }
}
```

### 3.3 Relations (관계)

IR의 `RelationDef`를 기반으로 생성합니다.

#### 관계 유형

| 패턴 | 관계 | 설명 |
|------|------|------|
| `foreign_key(Player.id) as Skills` | 1:N | Player가 여러 Skill 보유 |
| `foreign_key(Skill.id) as Users` | N:M | Skill을 가진 여러 Player |

#### IR 정보 활용

```rust
// IR의 RelationDef 구조
pub struct RelationDef {
    pub name: String,              // "Skills", "Users" 등
    pub source_table_fqn: String,  // 참조하는 테이블 FQN
    pub source_table_name: String, // 참조하는 테이블 이름
    pub source_field: String,      // 참조하는 필드
}
```

#### 생성 코드 예시

```rust
// Player 구조체에 관계 메서드 추가
impl Player {
    // RelationDef { name: "Skills", source_table_name: "PlayerSkill", source_field: "player_id" }
    pub fn skills(&self, container: &GameContainer) -> Vec<&PlayerSkill> {
        container.player_skills.get_by_player_id(self.id)
    }
}
```

### 3.4 Root Container

모든 테이블을 통합하는 루트 컨테이너입니다.

```rust
// Rust 예시
pub struct GameContainer {
    pub items: ItemTable,
    pub players: PlayerTable,
    pub skills: SkillTable,
    pub player_skills: PlayerSkillTable,
}

impl GameContainer {
    pub fn new() -> Self { ... }

    pub fn load_csv(&mut self, base_path: &str) -> Result<(), Error> {
        self.items.load_from_csv(&format!("{}/items.csv", base_path))?;
        self.players.load_from_csv(&format!("{}/players.csv", base_path))?;
        // ...
        Ok(())
    }

    pub fn load_json(&mut self, base_path: &str) -> Result<(), Error> { ... }
    pub fn load_binary(&mut self, base_path: &str) -> Result<(), Error> { ... }
}
```

---

## 언어별 구현 현황

| 언어 | Level 1 | Level 2 | Level 3 | 상태 |
|------|---------|---------|---------|------|
| **C#** | ✅ | ✅ | ✅ | 완전 지원 |
| **Rust** | ✅ | ✅ | ✅ | 완전 지원 |
| **MySQL** | ✅ | - | - | DDL만 |
| **TypeScript** | ❌ | ❌ | ❌ | 미구현 |
| **C++** | ❌ | ❌ | ❌ | 미구현 |

---

## 구현 순서 권장

1. **Level 1 먼저 완성** → 기본 타입 생성 확인
2. **JSON Loader** → 가장 간단 (serde 등 라이브러리 활용)
3. **CSV Loader** → 파싱 로직 필요
4. **Binary I/O** → 직렬화 형식 정의 필요
5. **Container** → 인덱스/관계 생성

---

## 템플릿 작성 팁

### IR 데이터 접근

```rhai
// 파일 순회
for file in schema.files {
    for ns in file.namespaces {
        for item in ns.items {
            if item.is_struct() {
                let s = item.as_struct();
                // s.name, s.fqn, s.items, s.indexes, s.relations
            }
        }
    }
}

// 인덱스 순회
for idx in s.indexes {
    // idx.name, idx.field_name, idx.field_type, idx.is_unique
}

// 관계 순회
for rel in s.relations {
    // rel.name, rel.source_table_fqn, rel.source_table_name, rel.source_field
}
```

### 유틸리티 함수

```rhai
// 케이스 변환
let snake = to_snake_case("PlayerSkill");  // "player_skill"
let pascal = to_pascal_case("player_skill");  // "PlayerSkill"
let camel = to_camel_case("PlayerSkill");  // "playerSkill"

// 들여쓰기
import "templates/rhai_utils/indent" as indent_utils;
let indented = indent_utils::indent_text(content, 1);  // 4스페이스 * 1
```

---

## 참고 자료

- C# 템플릿: `templates/csharp/`
- C# 정적 파일: `static/csharp/`
- Rust 템플릿: `templates/rust/`
- Rust 정적 파일: `static/rust/`
- IR 모델: `src/ir_model.rs`
- Rhai 함수 등록: `src/rhai/registry.rs`

---

## Rust 구현 세부사항

### 파일 구조

```
templates/rust/
├── rust.toml                      # 언어 설정
├── rust_file.rhai                 # 메인 진입점
├── rust_mod.rhai                  # 모듈 생성
├── rust_struct.rhai               # 구조체 생성
├── rust_enum.rhai                 # Enum 생성
├── rust_loaders_file.rhai         # JSON/CSV/Binary 로더 통합
├── rust_container_file.rhai       # 컨테이너 및 인덱스
└── rhai_utils/
    └── type_mapping.rhai          # 타입 매핑

static/rust/
└── polygen_support.rs             # 런타임 지원 라이브러리
```

### Binary I/O 형식

PolyGen의 바이너리 형식은 **모든 지원 언어에서 동일**합니다. C#에서 직렬화한 데이터를 Rust에서 읽을 수 있고, 그 반대도 가능합니다.

#### 언어 간 호환성

| 항목 | C# | Rust | 호환 |
|------|-----|------|------|
| 엔디안 | Little-endian | Little-endian | ✅ |
| 정수형 | `BinaryReader.ReadUInt32()` | `read_u32()` (LE) | ✅ |
| 문자열 | `[u32: len][UTF-8]` | `[u32: len][UTF-8]` | ✅ |
| Optional | `[byte: 0/1][T]` | `[u8: 0/1][T]` | ✅ |
| 배열 | `[u32: count][T...]` | `[u32: count][T...]` | ✅ |
| Enum | `[i32: value]` | `[i32: value]` | ✅ |

#### 직렬화 형식 상세

```
타입               | 형식
-------------------|------------------------------------------
u8, i8            | 1 byte
u16, i16          | 2 bytes, little-endian
u32, i32, f32     | 4 bytes, little-endian
u64, i64, f64     | 8 bytes, little-endian
bool              | 1 byte (0 or 1)
String            | [u32: length][bytes: UTF-8 data]
Option<T>         | [u8: flag][T: value if flag == 1]
Vec<T>            | [u32: length][T: element1][T: element2]...
Enum              | [i32: variant value]
```

### 사용 예시

```rust
use polygen_support::{BinaryReadExt, BinaryWriteExt};
use game_schema_loaders::BinaryIO;
use std::io::Cursor;

// 직렬화
let item = Item { id: 1, name: "Sword".to_string(), ... };
let mut buffer: Vec<u8> = Vec::new();
item.write_binary(&mut buffer)?;

// 역직렬화
let mut cursor = Cursor::new(&buffer);
let loaded = Item::read_binary(&mut cursor)?;

// 다중 아이템
buffer.write_u32(items.len() as u32)?;
for item in &items {
    item.write_binary(&mut buffer)?;
}
```

### 의존성

`Cargo.toml`에 필요한 의존성:

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

---

*최종 업데이트: 2026-01-21*
