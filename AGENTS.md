# Repository Guidelines

## Project Structure & Modules
- `src/`: Rust sources (`ast_*`, `ir_*`, `rhai_generator.rs`, `validation.rs`, `polygen.pest`, `lib.rs`, `main.rs`).
- `templates/`: Rhai templates per language (`templates/csharp/...`, with `rhai_utils/`).
- `static/`: Language-specific static assets copied to outputs (e.g., `static/csharp/DataSource.cs`).
- `examples/`: Example schemas (`*.poly`).
- `tests/`: Snapshot tests; inputs in `tests/schemas`, snapshots in `tests/snapshots`.
- `output/`: Generated at runtime; deleted and recreated on each run.

## Build, Test, and Dev Commands
- Build: `cargo build` (optimized: `cargo build --release`).
- Run (example):
  - `cargo run -- --schema-path examples/character_types.poly`
- Test: `cargo test` (runs `insta` snapshots).
- Lint: `cargo clippy -- -D warnings`
- Format: `cargo fmt --all`

## Coding Style & Naming
- Rust 2021; 4-space indent; target ~100 cols when reasonable.
- Modules/files: `snake_case`; types/traits: `PascalCase`; functions/vars: `snake_case`.
- Keep public API in `lib.rs`; `main.rs` only parses CLI and delegates to `run`.
- Before pushing: run `cargo fmt` and `cargo clippy` and keep zero warnings.

## Testing Guidelines
- Framework: `insta` snapshots (`tests/snapshot_tests.rs`).
- Add new cases under `tests/schemas/*.poly`; run tests to generate snapshots.
- Update snapshots intentionally: PowerShell `+$env:INSTA_UPDATE='auto'; cargo test+` or `cargo insta review` (if `cargo-insta` installed).
- Snapshot IDs: `{schema}_ast`, `{schema}_ir`.

## Commit & PR Guidelines
- Use Conventional Commits (e.g., `feat(parser): ...`, `fix(ir): ...`, `refactor(generator): ...`).
- Keep commits small and focused; summarize affected modules/files in the body.
- PRs include description, linked issues, repro/run steps, and before/after codegen samples (attach a subset of `output/`).

## Security & Config Tips
- Warning: running the CLI deletes `output/`. Don’t point `--output-dir` to important paths.
- Pipeline: Parse (`polygen.pest`) → Validate → IR Build → Rhai Codegen.
- For C#, `static/csharp/DataSource.cs` is copied to `output/<lang>/Common/`.
