# PolySheet

Git-friendly spreadsheet editor for PolyGen `.poly` schemas and JSON sources.

- Desktop: Tauri 2 + Svelte 5 + Univer Sheets Core 0.25.1
- Core: canonical JSON/TOML, validation, stable row identity, semantic diff,
  field-level 3-way merge, journaled transaction save, Git staging
- Interrupted journals fail closed until the caller explicitly approves the exact
  recovery targets; structural saves journal known sidecar deletions with the
  workbook update and never delete bound source JSON.
- CLI: `fmt`, `validate`, `diff`, `merge`, `git-config`
- Save As and merge never approve source normalization implicitly; review the
  preview and opt in explicitly when normalization is required.
- Draft-first workflow: start with an unsaved calculation sheet, choose the
  project folder on first save, and connect a `.poly` schema later
- Stable formula references: A1 is rendered from sheet/row/column IDs, so
  inserting rows or columns does not retarget existing formulas
- Contextual JSON inspector: field definitions, stable-ID row JSON editing,
  schema-derived row drafts, and read-only multi-row comparison

See [`docs/tools/polysheet.md`](../docs/tools/polysheet.md) for the document
format, behavior, commands, and v1 scope.

```powershell
npm install
npm run tauri:dev

cargo test --manifest-path core/Cargo.toml
npm run test
cargo run --manifest-path cli/Cargo.toml -- validate path\to\game.polysheet
```
