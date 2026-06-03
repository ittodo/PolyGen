# PolyGen

[![CI](https://github.com/ittodo/PolyGen/actions/workflows/ci.yml/badge.svg)](https://github.com/ittodo/PolyGen/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/ittodo/PolyGen?label=GUI%20Download)](https://github.com/ittodo/PolyGen/releases/latest)

PolyGen은 `.poly` 스키마를 단일 진실 공급원으로 사용해 여러 언어와 데이터 타겟의 코드를 생성하는 폴리글랏 코드 생성기입니다.

## Quick Start

```bash
cargo build --release
cargo run -- generate --schema-path examples/game_schema.poly --lang csharp --output-dir output
```

GUI 앱은 [최신 릴리즈](https://github.com/ittodo/PolyGen/releases/latest)에서 받을 수 있습니다.

## Targets

- Languages: C#, C++, Rust, TypeScript, Go, Python, Kotlin, Swift, Unreal
- Databases: SQLite, MySQL/MariaDB, PostgreSQL, Redis
- Descriptors: Protocol Buffers, MessagePack, Mermaid

## Common Commands

```bash
cargo run -- generate --schema-path examples/game_schema.poly --lang rust
cargo run -- watch --schema examples/game_schema.poly --lang csharp
cargo run -- migrate --baseline old.poly --schema-path new.poly
cargo run -- lint --schema-path examples/game_schema.poly
cargo test
```

## Project Layout

```text
src/        Rust core: parser, validation, IR, generation
templates/  PolyTemplate (.ptpl) target templates
static/     Runtime support files copied into generated output
examples/   Example .poly schemas
tests/      Snapshot and integration tests
docs/       Specs, guides, and status documents
```

## Docs

- [Documentation index](./docs/README.md)
- [Schema annotations](./docs/schema-annotations.md)
- [PolyTemplate guide](./docs/polytemplate-guide.md)
- [Targets](./docs/targets/README.md)
- [Tools](./docs/tools/README.md)
- [Agent guide](./AGENTS.md)

## License

MIT License
