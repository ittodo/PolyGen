PolyGen Rhai Template & C# Code Style

Scope
- This document captures the conventions we use when writing Rhai templates and for the generated C# code (Csv mappers, structs, etc.). It helps keep output readable and diffs stable.

Rhai Template Conventions
- Backtick strings: Use backtick-delimited strings for template literals and `${...}` interpolation. Example: `code += `obj.${field}.X` + "\n";`.
- Newlines per line: Emit exactly one logical source line per append: always terminate with `+ "\n"`.
- Prefer interpolation: Use `${expr}` inside backticks rather than `"..." + var + "..."` concatenation.
- No mixed quoting: Inside backticks use plain `"` for C# string quotes; avoid `\"` unless actually needed in C# output.
- Minimal side effects: Keep expressions in `${...}` simple (identifiers, property access, cheap computations). Pre-compute complex parts in Rhai variables first.

Generated C# Style
- Braces: Allman style. Opening `{` on the next line for `namespace`, `class`, methods, `if/else`, `for/foreach`, etc. Never place statements on the same line as `{` or `}`.
- Indentation: 4 spaces. Increase indent after `{`, decrease before `}`.
- One statement per line: Avoid `if (...) return;` or multiple statements on one line. Always use blocks.
- Spacing:
  - `for (int i = 0; i < n; i++)` with spaces around operators and after commas.
  - Method calls: `Foo(bar, baz)` with single spaces after commas.
- Early returns: Allowed, but keep each on its own line and inside blocks per brace rule.
- Readability over brevity: Prefer expanded loops/conditions in generated code to aid users reading emitted sources.

Template Structure Guidelines
- Keep code building in small composable chunks (e.g., `AppendRowWithHeader`, `BuildWriteHeaderFromItems`, `CollectWriteHeaderNames`).
- Separate non-list and list handling clearly in output ordering to keep headers predictable.
- Helper methods first: Define small helpers (e.g., `HeaderColumnCount`) before high-level APIs.

Example Snippets
- Method header and body:
  - Rhai: `code += `        private static int HeaderColumnCount(Polygen.Common.CsvIndexHeader h)` + "\n";`
  - Rhai: `code += `        {` + "\n";`
  - Rhai: `code += `            if (h.Index >= 0)` + "\n";`

Review & Testing
- After template edits: run `cargo test` to validate snapshot outputs.
- Keep diffs focused on formatting/codegen logic; avoid unrelated changes.

CSV Mapper Refactor (2025-09-06)

Overview
- Root namespace: `Csv` under which all generated CSV mappers live.
- Class naming: remove `Csv` suffix; class names match source types.
- Embedding: only embedded structs are generated as nested static classes inside their parent.
- Referenced (non-embedded) structs remain top-level under `Csv.<namespace>`.
- Call paths: parents only call direct children; deeper nesting is handled by each child.

Examples
- Embedded: `global::Csv.game.character.Monster.DropItem.Enchantment.AppendRowWithHeader(...)`.
- Referenced: `global::Csv.game.common.StatBlock.AppendRowWithHeader(...)`.

Template Changes
- `templates/csharp/csharp_csv_mappers_namespace.rhai`
  - Generate `namespace Csv.<ns>`; do not emit embedded structs at top level.
- `templates/csharp/struct/csharp_csv_mappers_struct.rhai`
  - Class declaration: `public static class <Type>` (suffix removed).
  - Stack-based nested rendering; only embedded children are rendered inside parents.
  - Skip top-level generation if the struct appears embedded anywhere.
  - Define `let full_t = "global::" + s.fqn;` for each class body before eval of writer/reader.
- `templates/csharp/struct/csharp_csv_mappers_struct_writer.rhai`
  - Switch all paths to `global::Csv.<ns>.<Type>` and remove `Csv` suffix in class names.
  - Parents call only direct children (no skipping intermediate nested types).
  - Option<T> unwrapping before type branching.
  - Enum fields: no helper calls; use `ToString()` or ConvertValue in reader.
  - BuildWriteHeaderFromItems: for non-list structs, pick the single candidate with the most columns; for list structs, use max count; use parent-based path for embedded types.
  - CollectWriteHeaderNames/BuildHeader updated to new paths and rules.
  - Remove unnecessary `__lists` when no list fields exist.
- `templates/csharp/struct/csharp_csv_mappers_struct_reader.rhai`
  - ConvertValue generic uses full FQN for enums.
  - Embedded struct calls use parent-based path; non-embedded use top-level path.

IR Changes
- `src/ir_builder.rs`
  - Add `resolve_type_kinds` to set `TypeRef.is_enum`/`is_struct` after collecting all declared enums.
  - Fallbacks: exact FQN match; same-namespace+name; and unique name across all enums.
  - Recurse into `inner_type` to fix lists/options.

Behavioral Notes
- No alias/wrapper classes are generated (no transitional shims).
- Remove remaining `...Csv.` suffix references in templates.
- Always use `global::Csv.` for mapper paths; use model FQNs (`global::<model fqn>`) for generic type arguments.

Testing & Migration
- Update imports in consumers: use `using Csv.game.character;` where applicable.
- Update call sites to new paths (no `Csv` suffix on classes; paths under `Csv.` root).
- Validate with `cargo test`; review insta snapshots for path/name changes.
