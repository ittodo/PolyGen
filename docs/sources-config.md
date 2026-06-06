# Sources Config

> 최종 업데이트: 2026-06-06

`.poly` files describe schema shape. `*.sources.toml` files describe runtime input paths such as CSV and JSON load sources.

## Naming

By default PolyGen looks for a sidecar file next to the schema:

```text
game_schema.poly
game_schema.sources.toml
```

You can also pass an explicit file:

```bash
cargo run -- generate --schema-path examples/game_schema.poly --sources examples/game_schema.sources.toml --lang csharp
```

## Format

Use fully qualified table names as quoted TOML keys:

```toml
[tables."game.item.Item".load]
csv = "data/items.csv"
json = "data/items.json"

[tables."game.character.Monster".load]
csv = "data/monsters.csv"
```

`csv` and `json` are optional individually, but at least one must be present.

## Precedence

- `*.sources.toml` overrides legacy `@load` annotations.
- Legacy `@load` remains supported for compatibility.
- New schemas should prefer sources config instead of `@load`.
- C#, C++, Rust, Go, TypeScript, Python, Kotlin, and Swift generated containers can use these paths for root-directory based CSV/JSON loading. Go also generates per-table Binary I/O loaders for binary files.
- Go CSV loaders parse primitive lists from comma-separated cells and embed/embedded-list fields from JSON cells, for example `tags` as `[{"name":"tag","color":"red"}]`.

## Editor Source Refs

C# and Unreal also generate editor/tooling-oriented SourceRefs from these paths:

- CSV/JSON files remain the source of truth.
- Mutable row refs are for editor and tool code, not normal game runtime mutation.
- SourceRefs open a single source root directory and resolve every table path relative to it. The root can be outside the engine project or inside an editor-visible folder such as `Assets/PolyGen/Sources`.
- C# SourceRefs are emitted under `#if UNITY_EDITOR || !UNITY_5_3_OR_NEWER`. `SaveChanges()` writes dirty source files, refreshes Unity's AssetDatabase for saved source/cache files in editor builds, and rebuilds the generated BinaryRef cache when a binary path is provided.
- Unreal SourceRefs are emitted under `#if WITH_EDITOR`. `SaveChanges()` writes dirty CSV/JSON sources through the generated loader/writer helpers.
- Tables with a primary key use that key for source refs. Source-backed tables without a primary key get a session-only `SourceRefId`; this id is not written to CSV/JSON and is only stable while the source document stays open.
- Source ref keys are immutable. Primary key fields are exposed as read-only through SourceRefs, and `SourceRefId` has no setter. To change identity, add a new row and remove the old row.
- Save support expects a concrete file path. Wildcard and directory load patterns can be used for loading in other generated APIs, but source-ref saves should target explicit CSV/JSON files.

## Validation

PolyGen validates that:

- referenced table FQNs exist
- load settings target tables, not embeds
- `csv` and `json` values are non-empty strings
- unsupported keys are rejected
