# TODO (Session Summary)

Date: 2025-08-28

## What We Did
- C# codegen extended with external readers/writers
  - Templates: `templates/csharp/csharp_binary_readers_*`, `templates/csharp/csharp_binary_writers_*`
  - Static utils: `static/csharp/BinaryUtils.cs` (read/write primitives, strings UTF-8+u32, enums i32, lists, options)
- JSON mappers (System.Text.Json)
  - Templates: `templates/csharp/csharp_json_mappers_*`
  - Static utils: `static/csharp/JsonUtils.cs`
  - Fixed property quoting and object close issues in generated writers
- CSV columns generator
  - Template: `templates/csharp/csharp_csv_columns_file.rhai`
  - Rules: embed flatten with dots, lists as `[0]` element, Option transparent, enums included
  - External struct flatten support (e.g., `game.common.Position` → `x,y`)
  - Cycle detection + depth guard (max depth 10)
- All-languages mode
  - `--lang` is now optional; runs all languages under `--templates-dir` when omitted

## Decisions
- Endianness: Little-endian only (no BE support)
- Enum base: i32
- String binary format: UTF-8 (no BOM) with u32 (LE) byte-length prefix
- CSV naming: keep original field names
- CSV lists: use `[0]` to represent a single element, then flatten
- CSV embed: use dot notation
- Option in CSV: no special marker; empty if null

## In Progress / Known Gaps
- CSV Mappers generation (C#) is partially implemented but currently disabled in runner due to Rhai fn-in-eval restrictions.
  - Introduced global helpers: `templates/csharp/rhai_utils/csv_helpers.rhai`.
  - Need to finish refactor so struct templates only call global helpers and do not define functions inside eval.
- Some placeholder logic in CSV append code for external struct column counts needs final wiring.
- No end-to-end example for JSON → objects → CSV yet.

## Next Actions
1) Finalize CSV Mappers
   - Complete `struct/csharp_csv_mappers_struct_body.rhai` to fully rely on `csv_helpers` (no fn definitions inside eval blocks).
   - Re-enable CSV Mappers pass in `src/lib.rs` and verify generation.
2) Add a small example utility for JSON → CSV conversion using generated `<Type>Csv` + `CsvUtils`.
3) Optional CSV annotations (future): `@csv(name: ...)`, `@csv(ignore)`.
4) Harden error handling
   - CSV depth/cycle logs, graceful defaults.
   - JSON mappers: stricter/lenient modes.
5) Add quick tests or sample outputs for new generators.

## How to Run
- All languages:
  - `cargo run -- --schema-path examples/game_schema.poly --templates-dir templates --output-dir output`
- Single language (e.g., C#):
  - `cargo run -- --schema-path examples/game_schema.poly --templates-dir templates --output-dir output --lang csharp`

## Notable Paths
- Readers/Writers templates: `templates/csharp/csharp_binary_{readers,writers}_*`
- JSON mappers templates: `templates/csharp/csharp_json_mappers_*`
- CSV columns: `templates/csharp/csharp_csv_columns_file.rhai`
- CSV helpers (Rhai): `templates/csharp/rhai_utils/csv_helpers.rhai`
- Utils (C#): `static/csharp/BinaryUtils.cs`, `static/csharp/JsonUtils.cs`, `static/csharp/CsvUtils.cs`

## New Work Completed (2025-08-28)
- CSV mappers: full flattening implemented with Rust-backed helpers to avoid Rhai recursion.
- Header-indexed CSV reading added:
  - `<Type>Csv.FromRow(string[] header, string[] row, CsvUtils.GapMode gap)`
  - `<Type>Csv.FromRowWithPrefixAndHeader(...)`
- Dynamic CSV writing (2-pass) added:
  - `<Type>Csv.ComputeListMaxes(...)` → list max per field
  - `<Type>Csv.GetDynamicHeader(maxes)` → dynamic header
  - `<Type>Csv.AppendRowDynamic(obj, cols, maxes)`
  - `<Type>Csv.WriteCsvDynamic(items, path, ...)`

## Follow-ups (TODO)
- CsvReader robustness: support quoted fields, escaped separators, CRLF handling.
- Docs: add README section for dynamic CSV APIs (read/write), examples, and GapMode.
- Tests: add snapshot/roundtrip tests for FromRowWithHeader and WriteCsvDynamic.
- Enum/namespace edge cases: expand coverage for nested namespaces and inline enums in lists.
- Performance: consider caching column tail lists per type to reduce per-row overhead.
- Clippy cleanup: remove unused imports/variables and address warnings.
