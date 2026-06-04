# Sources Config

> 최종 업데이트: 2026-06-03

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

## Validation

PolyGen validates that:

- referenced table FQNs exist
- load settings target tables, not embeds
- `csv` and `json` values are non-empty strings
- unsupported keys are rejected

