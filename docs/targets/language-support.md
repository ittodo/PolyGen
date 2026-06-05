# PolyGen 언어 지원 가이드

> 최종 업데이트: 2026-06-05

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
├── {lang}_file.ptpl         # 메인 진입점
├── detail/                  # 구조체/Enum/모듈 세부 템플릿
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
├── {lang}_loaders_file.ptpl     # CSV/JSON/Binary 로더 생성
├── detail/                      # loader 세부 템플릿
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
├── {lang}_container_file.ptpl   # 컨테이너 생성
├── container/
│   ├── datatable.ptpl           # DataTable 클래스
│   ├── indexes.ptpl             # 인덱스 생성
│   └── relations.ptpl           # 관계 생성

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
| **C#** | ✅ | ✅ | ✅ | 완전 지원 + Redis key helper |
| **C++** | ✅ | ✅ | ✅ | 헤더/로더/컨테이너 + Redis key helper |
| **Rust** | ✅ | ✅ | ✅ | 완전 지원 + Redis key helper |
| **TypeScript** | ✅ | ✅ | ✅ | 인터페이스/Zod/SQLite + Redis key helper |
| **Go** | ✅ | ✅ | ✅ | Struct/Enum/Container + Redis key helper |
| **Python** | ✅ | ✅ | ✅ | dataclass/Pydantic/SQLAlchemy + Redis key helper |
| **Kotlin** | ✅ | ✅ | ✅ | data class/kotlinx.serialization + Redis key helper |
| **Swift** | ✅ | ✅ | ✅ | Codable/SwiftData + Redis key helper |
| **MySQL** | ✅ | - | - | DDL만 |
| **PostgreSQL** | ✅ | - | - | DDL만 |
| **Unreal** | ✅ | ✅ | ✅ | USTRUCT/UENUM/Loaders + Redis key helper |
| **Redis** | ✅ | - | - | cache key schema descriptor + ttlSeconds + Lua/C#/C++/Rust/TypeScript/Go/Python/Kotlin/Swift/Unreal key helpers |
| **Protocol Buffers** | ✅ | - | - | proto3 `.proto` |
| **MessagePack** | ✅ | - | - | array encoding schema descriptor |
| **Mermaid** | ✅ | - | - | ER 다이어그램 |

---

## C# Parity 확장 기준

Level 1~3는 언어 지원의 최소 단위입니다. C#은 여기에 더해 운영용 데이터 로딩,
인덱스, 검색, SQLite 접근자, BinaryRef 같은 보조 산출물을 가장 넓게 지원하므로,
다른 언어를 확장할 때는 아래 기능 축을 기준으로 parity를 판단합니다.

| 기능 축 | 기준 |
|---------|------|
| 기본 모델 | struct/class/data class, enum, namespace/module/package, doc comment, default 값 |
| 타입/카디널리티 | primitive, timestamp, bytes, enum, embed, optional, array/list |
| 검증 | field constraint, unique/primary key, foreign key, `ValidateAll`/예외 경로 |
| CSV/JSON 로더 | header 기반 CSV, optional/list/enum 파싱 오류 처리, JSON 객체 배열 로딩 |
| Binary I/O | 언어 간 동일한 little-endian 형식, invalid enum discriminant 거부, bytes/optional/list roundtrip |
| Container | DataTable, root/namespace container, unique/group index, FK validation, directory load |
| Navigation | FK navigation, reverse relation navigation, container 기반 relation helper |
| `@pack` | `Pack`, `Unpack`, `TryUnpack` 또는 언어별 동등 API, invalid input 거부 |
| SQLite accessor | recursive namespace table 수집, `@datasource("sqlite")` 상속, generated accessor 타입 |
| Redis helper | `@cache`/`@datasource("redis"|"cache")` key helper와 TTL 반영 |
| BinaryRef | indexed binary package, lazy row reference, shared document lifetime/ownership |
| `@search` | in-memory container postings, BinaryRef postings, `SearchBy<Field>` 계열 조회 API |

### Parity Matrix

상태 값은 `full`, `partial`, `none`, `n/a`를 사용합니다. `partial`은 산출물이 있더라도
C# runner와 같은 수준의 runtime 검증 또는 API 범위가 아직 부족한 상태입니다.

| 언어 | 모델 | 검증 | CSV/JSON | Binary | Container | Navigation | Pack | SQLite | Redis | BinaryRef | Search |
|------|------|------|----------|--------|-----------|------------|------|--------|-------|-----------|--------|
| C# | full | full | full | full | full | full | full | full | full | full | full |
| C++ | full | full | full | full | full | full | full | full | full | full | full |
| Rust | full | full | full | full | full | full | full | full | full | full | full |
| TypeScript | full | full | full | full | full | full | full | full | full | full | full |
| Go | full | full | full | full | full | full | full | full | full | full | full |
| Python | full | full | full | full | full | full | full | full | full | full | full |
| Kotlin | full | full | full | full | full | full | full | full | full | full | full |
| Swift | full | full | full | full | full | full | full | full | full | full | full |
| Unreal | full | partial | full | none | partial | partial | partial | n/a | full | none | partial |

### Partial/None Parity Audit (2026-06-05)

Unreal runner는 기본 실행에서 생성물 구조를 검증하고, 준비된 fixture/엔진 환경에서는
실제 UnrealBuildTool/UnrealHeaderTool smoke gate까지 실행합니다. Kotlin은 로컬 임시 Kotlin 2.4.0 toolchain과 runtime classpath로,
Swift는 swift.org Windows Swift 6.3.2 toolchain과 `Windows.sdk`로 compile/runtime
gate 실행 증거를 확보했습니다. 따라서 아래 항목은 기능 산출물이 존재하고 구조 검증을
통과하더라도 C# runner 수준의 runtime 또는 engine compile 증거가 부족하면 `partial`로
유지합니다.

| 언어 | 현재 증거 | partial/none 유지 사유 | 다음 gate |
|------|-----------|------------------------|-----------|
| Kotlin | `tests\runners\kotlin\run_tests.bat` 11/11 통과. `POLYGEN_KOTLIN_COMPILE=1`으로 01-11 전체 generated `.kt` compile gate 통과. `POLYGEN_KOTLIN_RUNTIME=1`으로 06 CSV/JSON/Binary, 07 Container/Search/BinaryRef, 08 Validation, 09 SQLite, 10 Pack/Binary, 11 composite index/navigation/BinaryRef runtime assertions 통과. `run_all --verify`는 Kotlin runtime helper command/harness regression도 실행 | Kotlin 2.4.0 compiler, kotlinx.serialization 1.11.0, kotlinx.datetime 0.8.0-0.6.x-compat, sqlite-jdbc 3.53.2.0으로 runtime 증거 확보 완료 | 완료 |
| Swift | `tests\runners\swift\run_tests.bat` 11/11 통과. `POLYGEN_SWIFT_COMPILE=1`으로 01-11 전체 generated `.swift` portable core typecheck gate 통과. `POLYGEN_SWIFT_RUNTIME=1`으로 06 CSV/JSON/Binary, 07 Container/Search/BinaryRef, 08 Validation, 09 SQLite fake connection, 10 Pack/Binary, 11 composite index/navigation/BinaryRef runtime assertions 통과. `run_optional_toolchains.py swift`도 Swift readiness 자동 주입으로 통과 | Swift 6.3.2 compiler, `Windows.sdk`, runtime PATH 자동 탐지로 runtime 증거 확보 완료. SwiftData 파일은 portable core typecheck/runtime compile에서 기본 제외하고 `POLYGEN_SWIFT_INCLUDE_SWIFTDATA=1`일 때 별도 포함 | 완료 |
| Unreal | `tests\runners\unreal\run_tests.bat` 11/11 통과. USTRUCT/UENUM, loader/hot reload, Registry index/search/navigation/validation, `@pack`, Redis helper 구조 검증과 `.generated.h` 마지막 include 규칙, generated `Polygen*.h` 로컬 include 해석, regex `Internationalization/Regex.h` include 검증 포함. `POLYGEN_UNREAL_COMPILE=1` + `POLYGEN_UNREAL_FIXTURE_ROOT=target\polygen-unreal-fixture`로 UE 5.7 설치(`D:\EpicGames\UE_5.7`)의 UnrealBuildTool/UnrealHeaderTool smoke gate 01-11 전체 통과. 생성 타입은 UHT engine-name 충돌을 피하기 위해 `FPolygen*`/`EPolygen*` reflected name을 사용하고, explicit 0 값이 없는 enum에는 `PolygenInvalid = 0`을 추가. `prepare_unreal_fixture.py`는 명시 root에 최소 UBT smoke project를 생성하고, readiness checker/compile helper는 준비된 `POLYGEN_UNREAL_FIXTURE_ROOT`와 Epic Launcher manifest에서 env/UBT를 자동 구성할 수 있음. `run_all --verify`는 `compile_unreal.py` env/header copy/engine root/Epic manifest UBT discovery/UBT command helper/fixture/helper/local include regression도 실행 | Unreal은 C# BinaryRef/SQLite를 의도적으로 복제하지 않으므로 Binary/BinaryRef는 `none`, SQLite는 `n/a` 유지. Registry/Search/Validation/Pack은 UBT/UHT compile 증거를 확보했지만 Blueprint runtime behavior assertion은 아직 별도 자동화하지 않았으므로 engine-specific partial 축은 유지 | 필요 시 Editor/AutomationSpec 기반 Blueprint-callable runtime smoke와 DataAsset/DataTable integration gate 추가 |

### Unreal Parity Policy

Unreal은 C#의 `DataContainer`, BinaryRef, SQLite accessor API를 그대로 복제하지 않습니다.
엔진 런타임과 Editor workflow에 맞춰 다음 기준으로 parity를 해석합니다.

| 축 | Unreal 정책 |
|----|-------------|
| 모델 | `USTRUCT(BlueprintType)`, `UENUM(BlueprintType)`, `UPROPERTY(EditAnywhere, BlueprintReadWrite)`를 기본 산출물로 유지. UHT engine-name 충돌 방지를 위해 C++ reflected type name은 `FPolygen*`/`EPolygen*`를 사용 |
| 타입 | `TArray<T>`, `FString`, `TArray<uint8>` 등 UE 타입을 우선하고, `UPROPERTY` 제약 때문에 일부 unsigned 정수는 엔진 친화 타입으로 매핑 |
| 로더 | JSON은 `FJsonObjectConverter` 기반을 우선 지원하고, CSV는 scalar 및 `@pack` embed 중심으로 유지 |
| Hot reload | `FPolygenHotReloadManager`, load source 기반 delegate, Editor file watching을 Unreal 고유 Container 대체 축으로 유지 |
| Container/index | read-only registry/index subsystem을 생성합니다. 내부 저장은 `TArray<Row>`와 `TMap<Key, int32>`/`TMap<Key, TArray<int32>>`, 외부 API는 Blueprint callable `Set<Table>s`, `Get<Table>By<Field>`, `Get<Table>sBy<Field>` helper입니다. |
| Navigation | C#의 row navigation property 대신 registry가 `Get<Table><FieldAlias>` forward FK helper와 `Find<Table><Alias>` reverse relation helper를 Blueprint callable API로 생성합니다. |
| Validation | C#의 예외 중심 API 대신 `bool`/diagnostic array 또는 UE log-friendly result 타입을 우선합니다. Registry는 field constraint(`MaxLength`, `Range`, `Regex`), unique/primary duplicate, FK diagnostic을 분리 제공하고 `ValidateAll`로 합산합니다. |
| `@pack` | embed USTRUCT에 `Pack`, `Unpack`, `TryUnpack`을 생성하고 field count, finite float, unsigned negative 입력을 방어합니다. |
| Binary/SQLite | near-term parity 대상이 아닙니다. Unreal에서는 DataTable/DataAsset/JSON asset pipeline을 우선합니다. |
| Redis | 기존 key helper를 유지하고 Blueprint 노출은 필요성이 확인된 뒤 추가합니다. |
| `@search` | registry가 `TMap<Key, TArray<int32>>`/`TMap<FString, TArray<int32>>` postings를 만들고 `Search<Table>By<Field>` Blueprint query helper를 생성합니다. BinaryRef/DataAsset 검색은 별도 범위입니다. |

따라서 matrix에서 Unreal의 SQLite는 `n/a`, Binary/BinaryRef는 `none`, Container/Navigation/Search는
엔진 전용 read-only registry 범위까지만 `partial`로 둡니다.

### 확장 순서

1. **문서/테스트 기준선 고정**
   - 이 문서의 parity matrix를 기준으로 구현 범위를 관리합니다.
   - `tests/integration/` 공통 schema와 언어별 runner에 C#의 고급 회귀 테스트를 단계적으로 복제합니다.

2. **Rust/C++/TypeScript 우선**
   - 이미 Container, SQLite, Binary 또는 BinaryRef 기반이 있으므로 C# parity까지의 간격이 가장 작습니다.
   - 우선 구현 대상은 `@search` API, BinaryRef parity, loader 오류 처리, navigation helper입니다.

3. **Go 확장**
   - Container와 `@pack` 기반은 유지하면서 loaders, SQLite accessor, BinaryRef/Search를 추가합니다.
   - runner를 smoke test 중심에서 runtime assertion 중심으로 올립니다.

4. **Python/Kotlin/Swift 확장**
   - 먼저 Container/index/FK validation과 `@search`를 추가하고, 다음에 loader와 SQLite 계층을 추가합니다.
   - 각 언어의 생태계에 맞춰 Python은 Pydantic/SQLAlchemy, Kotlin은 kotlinx.serialization, Swift는 Codable/SwiftData와 연동합니다.

5. **Unreal 별도 parity**
   - C# API를 그대로 복제하지 않고 `TArray`, `TMap`, hot reload, Blueprint 노출 정책에 맞춘 별도 parity를 정의합니다.

### 다음 구현 단위

완료된 구현 단위는 **C# table-level composite tuple-key Container index, BinaryRef composite tuple-key lookup, reverse relation navigation runtime 검증**, **Rust `@search` in-memory Container API**, **C++
`@search` in-memory Container API**, **C++ `@search` BinaryRef reader API**,
**C++ Container FK/reverse navigation helper API**,
**C++ Container composite tuple-key index API**,
**C++ CSV/JSON generated loader runtime path**,
**C++ sources config 기반 Container CSV/JSON load API와 root CSV/JSON runtime 회귀 테스트**,
**C++ CSV/JSON enum name/numeric parser hardening과 BinaryRef invalid enum lazy/search-key read 거부**,
**C++ BinaryRef composite tuple-key lookup API**, **C++ BinaryRef container save/open API**,
**Rust BinaryRef shared document/ref table/index/search API와 invalid enum discriminant read 거부**, **Rust BinaryRef composite tuple-key lookup API**,
**Rust sources config 기반 Container CSV/JSON load API와 root CSV/JSON runtime 회귀 테스트**,
**Rust CSV JSON-cell embed/list parser hardening**,
**Rust CSV enum name/numeric parser hardening**,
**Rust Container field/unique validation API와 runner runtime 회귀 테스트**,
**TypeScript `@search` in-memory Container API**, **TypeScript `@search` BinaryRef
read/write API와 invalid enum discriminant 거부**, **TypeScript BinaryRef composite index lookup API**, **TypeScript CSV/JSON generated loader와 sources config 기반 Container CSV/JSON root load runtime 검증**, **TypeScript Binary I/O row-level read/write API와 invalid enum discriminant 거부**, **TypeScript Container FK validation API**, **TypeScript Container field/unique validation API**, **TypeScript Container reverse navigation/composite index API**, **Go `@search` in-memory Container API**, **Go SQLite accessor
fake-driver와 modernc in-memory SQLite runtime-tested API**, **Go BinaryRef read/write, invalid enum discriminant 거부 및 `@search` API**, **Go BinaryRef composite key lookup API**, **Go CSV/JSON
generated loader와 sources config 기반 Container CSV/JSON root load runtime 검증**, **Go Binary I/O generated
loader와 invalid enum discriminant 거부**, **Go CSV/JSON enum name/numeric parser hardening**, **Go embed/list CSV JSON-cell parser hardening**, **Rust Container FK/reverse navigation
helper API**, **Rust Container composite tuple-key index API**, **Go Container FK/reverse navigation
helper API**, **Go Container composite key index API**, **Go Container field/unique
validation API**, **Python Container/index/FK
validation API**, **Python Container field/unique validation API**, **Python Container FK/reverse navigation helper API**, **Python Container composite
key index API**, **Python `@search` in-memory Container API**, **Python sources config 기반 Container CSV/JSON root load runtime 검증**, **TypeScript Container FK navigation helper API**, **Kotlin/Swift Container FK/reverse navigation helper API**, **Kotlin/Swift Container composite
key index API**, **Kotlin Container/index/FK
validation API**, **Python SQLite accessor API**, **Kotlin Container field/unique validation API**, **Swift Container
field/unique validation API**, **Python CSV/JSON generated loader**, **Kotlin CSV/JSON generated loader와 enum name/numeric parser hardening**,
**Python Binary I/O row-level read/write API와 invalid enum discriminant 거부**,
**Python BinaryRef lazy document/ref table/index/search lookup API, composite key lookup과 invalid enum discriminant 거부**,
**Kotlin Binary I/O row-level read/write API와 invalid enum discriminant 거부**,
**Kotlin BinaryRef lazy document/ref table/index/search lookup API, composite key lookup과 invalid enum discriminant lazy read**,
**Kotlin sources config 기반 Container CSV/JSON root load API 구조 검증**, **Swift CSV/JSON generated loader와 enum name/numeric parser hardening**,
**Swift Binary I/O row-level read/write API와 invalid enum discriminant 거부**,
**Swift BinaryRef lazy document/ref table/index/search lookup API, composite key lookup과 invalid enum discriminant lazy read**,
**Swift sources config 기반 Container CSV/JSON root load API 구조 검증**, **Kotlin `@search` in-memory Container API**, **Kotlin Container `@search` 지역 변수 충돌 방지, non-null search guard 정리와 validator 중복 local 선언 검사**, **Kotlin SQLite accessor API**, **Kotlin `@pack`
pack/unpack/tryUnpack API**, **Swift Container/index/FK
validation API**, **Swift `@search` in-memory Container API**, **Swift `@pack`
pack/unpack/tryUnpack API**, **Swift Container `@search` 지역 변수 충돌 방지와 validator 중복 local 선언 검사**, **Swift SQLite accessor API**, **Unreal 엔진 친화 parity 정책 정의**, **Unreal read-only
registry/index subsystem과 Blueprint query API**, **Unreal `@search` postings 기반
Blueprint query API**, **Unreal registry FK/reverse navigation Blueprint helper API**,
**Kotlin/Swift Container regex validation 메시지 quote-safe 생성**, **C#/C++/Rust/Go/TypeScript Container field constraint(`MaxLength`, `Range`, `Regex`) runtime 회귀 검증**, **Unreal Registry field(`MaxLength`, `Range`, `Regex`)/unique/primary/FK validation API와 combined validation 구조 검증**,
**Python `@pack` Pack/Unpack/TryUnpack API**, **Unreal `@pack`
Pack/Unpack/TryUnpack USTRUCT API**입니다. 다음 구현
단위는 남은 `partial`/`none` parity 축을 재감사해 언어별 runtime 검증과 API 확장
우선순위를 다시 고정하는 것입니다.

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
- 타겟 인덱스: `README.md`
- 도구 문서: `../tools/README.md`

---

## Rust 구현 세부사항

### 파일 구조

```
templates/rust/
├── rust.toml                      # 언어 설정
├── rust_file.ptpl                 # 메인 진입점
├── rust_loaders_file.ptpl         # CSV/Binary 로더 통합
├── rust_container_file.ptpl       # 컨테이너 및 인덱스
├── detail/                        # 구조체/Enum/loader 세부 템플릿
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

*최종 업데이트: 2026-06-05*
