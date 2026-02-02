# PolyGen

[![CI](https://github.com/ittodo/PolyGen/actions/workflows/ci.yml/badge.svg)](https://github.com/ittodo/PolyGen/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/ittodo/PolyGen?label=GUI%20Download)](https://github.com/ittodo/PolyGen/releases/latest)

PolyGen은 `.poly` 스키마를 단일 진실 공급원(SSOT)으로 삼아, 여러 프로그래밍 언어의 코드를 생성하는 **폴리글랏 코드 생성기**입니다.

> **[GUI 앱 다운로드 (Windows / macOS / Linux)](https://github.com/ittodo/PolyGen/releases/latest)**

## 지원 언어

| 언어 | 상태 | 생성 결과물 |
|------|------|------------|
| C# | ✅ 완료 | 클래스, Enum, CSV/JSON/Binary 로더, Container |
| C++ | ✅ 완료 | 헤더 전용 구조체, Enum, CSV/JSON 로더 |
| Rust | ✅ 완료 | 모듈, Struct, Enum, Serde 지원 |
| TypeScript | ✅ 완료 | 인터페이스, Enum, Zod 스키마 검증 |
| Go | 🚧 진행중 | 구조체, Enum |

## 빠른 시작

```bash
# 빌드
cargo build --release

# 코드 생성
cargo run -- generate --schema-path examples/game_schema.poly --lang csharp --output-dir output
```

## 스키마 문법 (.poly)

```poly
namespace game.character {
    // 테이블 정의 (클래스/구조체)
    table Player {
        id: u32 primary_key;
        name: string max_length(100);
        level: u16 default(1) range(1, 100);
        email: string? unique;  // optional
        skills: Skill[];        // array
        position: Position;     // embed 참조
    }

    // Enum 정의
    enum PlayerClass {
        Warrior = 1;
        Mage = 2;
        Rogue = 3;
    }

    // Embed 정의 (재사용 가능한 필드 그룹)
    embed Stats {
        hp: u32;
        mp: u32;
    }
}
```

### 지원 타입

- **기본 타입**: `string`, `bool`, `bytes`
- **정수**: `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`
- **부동소수점**: `f32`, `f64`
- **카디널리티**: `?` (optional), `[]` (array)

### 제약조건

```poly
id: u32 primary_key;
name: string unique max_length(100);
level: u16 default(1) range(1, 100);
email: string regex(".*@.*");
user_id: u32 foreign_key(User.id);
```

### 어노테이션

| 어노테이션 | 적용 대상 | 설명 |
|-----------|----------|------|
| `@load(csv: "path")` | table | CSV/JSON 데이터 로드 경로 |
| `@readonly` | table | 읽기 전용 테이블 |
| `@cache("strategy")` | table | 캐시 전략 (full_load, on_demand 등) |
| `@datasource("name")` | namespace/table | 데이터 소스 지정 |
| `@pack` | embed | 필드를 단일 문자열로 직렬화 |

## @pack 어노테이션

`embed` 타입에 `@pack`을 붙이면 여러 필드를 단일 문자열로 직렬화/역직렬화하는 메서드가 생성됩니다.
CSV나 DB에서 여러 값을 하나의 컬럼에 저장할 때 유용합니다.

```poly
// 기본 구분자: ;
@pack
embed Position {
    x: f32;
    y: f32;
}

// 커스텀 구분자
@pack(separator: ",")
embed Color {
    r: u8;
    g: u8;
    b: u8;
}

table Player {
    id: u32 primary_key;
    position: Position;  // CSV: "100.5;200.3"
    color: Color;        // CSV: "255,128,64"
}
```

**생성되는 메서드:**

```csharp
// C#
position.Pack();                         // "100.5;200.3"
Position.Unpack("100.5;200.3");          // Position { x: 100.5, y: 200.3 }
Position.TryUnpack("100.5;200.3", out p); // true/false
```

```cpp
// C++
position.pack();                    // "100.5;200.3"
Position::unpack("100.5;200.3");    // Position { x: 100.5, y: 200.3 }
Position::try_unpack("...", out);   // true/false
```

```rust
// Rust
position.pack();                    // "100.5;200.3"
Position::unpack("100.5;200.3")?;   // Result<Position, String>
```

```typescript
// TypeScript
packPosition(position);             // "100.5;200.3"
unpackPosition("100.5;200.3");      // Position
tryUnpackPosition("...");           // Position | null
```

## CLI 옵션

```bash
polygen generate [OPTIONS]

Options:
  --schema-path <PATH>     스키마 파일 경로 (필수)
  --lang <LANG>            타겟 언어: csharp, cpp, rust, typescript, go
  --output-dir <DIR>       출력 디렉토리 (기본: output)
  --templates-dir <DIR>    커스텀 템플릿 디렉토리
```

## 프로젝트 구조

```
PolyGen/
├── src/                 # Rust 코어 (파서, 검증, IR, 코드 생성)
├── templates/           # 언어별 Rhai 템플릿
│   ├── csharp/
│   ├── cpp/
│   ├── rust/
│   ├── typescript/
│   └── go/
├── static/              # 생성 코드와 함께 복사되는 런타임 유틸리티
├── gui/                 # Tauri 기반 GUI 앱
├── examples/            # 예제 스키마
├── tests/               # 테스트 (스냅샷, 통합 테스트)
└── docs/                # 설계 문서
```

## GUI

PolyGen은 Tauri 기반 GUI를 제공합니다. **[최신 릴리즈 다운로드](https://github.com/ittodo/PolyGen/releases/latest)**

| 플랫폼 | 패키지 |
|--------|--------|
| Windows | `.msi` / `.exe` 인스톨러 |
| macOS | `.dmg` |
| Linux | `.deb` / `.AppImage` |

소스에서 직접 빌드:

```bash
cd gui
npm install
npm run tauri:build
```

주요 기능:
- Monaco 에디터 기반 스키마 편집
- 실시간 문법 검사
- Go to Definition / Find References
- 자동 완성
- 최근 프로젝트 빠른 로딩

## 테스트

```bash
# 모든 테스트 실행
cargo test

# 스냅샷 리뷰
cargo insta review
```

## 문서

- [CLAUDE.md](./CLAUDE.md) - 상세 개발 가이드
- [docs/](./docs/) - 설계 문서

## 라이선스

MIT License
