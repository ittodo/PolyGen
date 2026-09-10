# Sources Config

> 최종 업데이트: 2026-09-10

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

## PolySheet editing

PolySheet는 table의 concrete `json` 경로를 데이터 시트의 단일 source of truth로
사용한다. `.polysheet/` 폴더에는 데이터 행을 복제하지 않는다.

- `@readonly` table은 편집할 수 없다.
- wildcard/directory JSON path와 CSV-only source는 v1에서 직접 편집할 수 없다.
- 상대 JSON 경로는 `workbook.toml`의 `data_root`, 명시적 sources 파일의 부모,
  schema 파일의 부모 순서로 기준 디렉터리를 결정한다.
- PolySheet 최초 저장은 기존 JSON의 canonical formatting 차이를 미리 보여주고
  승인된 경우에만 정규화한다.
- primary key가 없는 source의 stable row ID는 JSON이 아니라
  `.polysheet/sheets/<id>/rowids.json`에 저장한다.

자세한 형식과 Git 동작은 [tools/polysheet.md](tools/polysheet.md)를 참조한다.
