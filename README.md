# PolyGen

[![CI](https://github.com/ittodo/PolyGen/actions/workflows/ci.yml/badge.svg)](https://github.com/ittodo/PolyGen/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/ittodo/PolyGen?label=GUI%20Download)](https://github.com/ittodo/PolyGen/releases/latest)

PolyGen generates code for multiple languages and data targets from one `.poly` schema.

Use it when the same data model must be shared across client, server, tools, databases, and runtime loaders without hand-maintaining duplicate types.

## Why PolyGen?

- Define tables, enums, constraints, indexes, relations, and data-source metadata once.
- Generate language models, loaders, containers, DB DDL, migration SQL, Redis key helpers, and descriptors from the same schema.
- Keep generated output consistent across C#, C++, Rust, TypeScript, Go, Python, Kotlin, Swift, Unreal, SQL, Redis, Protobuf, MessagePack, and Mermaid.

## Example

```poly
@datasource("sqlite")
namespace game {
    enum Rarity {
        Common = 1;
        Rare = 2;
        Epic = 3;
    }

    @index(rarity)
    table Item {
        id: u32 primary_key;
        name: string max_length(80);
        rarity: Rarity;
        price: u32 default(0);
    }
}
```

```bash
.\polygen.exe --schema-path examples\quickstart.poly --lang csharp --output-dir output\quickstart
```

Runnable quickstart: [docs/examples/quickstart.md](./docs/examples/quickstart.md)

## Quick Start

Download the latest runtime from [Releases](https://github.com/ittodo/PolyGen/releases/latest), then run:

```powershell
.\polygen.exe --schema-path examples\quickstart.poly --lang csharp --output-dir output\quickstart
.\polygen.exe --schema-path examples\quickstart.poly --lang sqlite --output-dir output\quickstart-sqlite
.\polygen.exe watch --schema examples\quickstart.poly --lang csharp
```

On macOS/Linux, use `./polygen` with the same arguments.

For source builds:

```bash
cargo build --release
cargo run -- --schema-path examples/quickstart.poly --lang rust
cargo run -- watch --schema examples/quickstart.poly --lang csharp
```

GUI builds are also available from the latest release.

## What It Generates

- Language models and enums
- CSV/JSON/Binary loaders
- Containers, indexes, relations, and validation helpers
- SQLite/MySQL/PostgreSQL DDL and migration SQL
- Redis schema descriptors and key helpers
- Protobuf, MessagePack, and Mermaid descriptors

## Tools

- Desktop GUI
- Language Server
- VS Code extension
- Browser-based `.poly` viewer

See [docs/tools](./docs/tools/README.md).

## Documentation

- [Documentation index](./docs/README.md)
- [Schema annotations](./docs/schema-annotations.md)
- [PolyTemplate guide](./docs/polytemplate-guide.md)
- [Targets](./docs/targets/README.md)

## License

MIT License
