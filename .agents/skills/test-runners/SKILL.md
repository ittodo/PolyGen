---
name: test-runners
description: >
  Run and maintain PolyGen integration test runners (tests/runners/).
  Trigger when: (1) user asks to "test", "run tests", "run runners", or "integration test" for any language,
  (2) user needs to add/update test cases, test files, or runner scripts,
  (3) user asks to verify generated code compiles and runs correctly across languages.
  Supports: csharp, cpp, rust, typescript, go, sqlite runners.
  This skill acts as the test agent in a feedback loop: main agent requests test, this skill runs it,
  on failure returns error logs, main agent fixes, requests test again, repeat until green.
---

# PolyGen Integration Test Runners

## Workflow: Test Execution Loop

When asked to run tests:

1. Run the requested runner(s) via their `.bat` script (Windows)
2. Capture output, extract ONLY build/compilation errors and test failures
3. Return a concise summary: pass/fail counts + error logs (if any)
4. If errors exist, the calling agent fixes the code and requests re-test
5. Repeat until all tests pass

## Runner Commands

All runners live in `tests/runners/<lang>/`. Execute from project root.

| Language   | Command                                  | Prerequisites  |
|------------|------------------------------------------|----------------|
| C#         | `tests\runners\csharp\run_tests.bat`     | dotnet SDK 8.0 |
| C++        | `tests\runners\cpp\run_tests.bat`        | MSVC or g++    |
| Rust       | `tests\runners\rust\run_tests.bat`       | cargo          |
| TypeScript | `tests\runners\typescript\run_tests.bat` | node, npm      |
| Go         | `tests\runners\go\run_tests.bat`         | go             |
| SQLite     | `tests\runners\sqlite\run_tests.bat`     | (none)         |

- No language specified: ask which language(s)
- "test all": run all runners sequentially

## Output Filtering

Return to caller ONLY:
- **Summary**: `Passed: N, Failed: N, Skipped: N`
- **Error details** on failure: compilation errors or runtime assertion failures
- Omit verbose progress lines ("Generating...", "Compiling...") when tests pass
- Success = one-line confirmation

If `cargo build --release` fails (PolyGen itself broken), return cargo error output immediately.

## Test Case Structure

Schemas in `tests/integration/`:
```
01_basic_types    02_imports           03_nested_namespaces
04_inline_enums   05_embedded_structs  06_arrays_and_optionals
07_indexes        08_complex_schema    09_sqlite
10_pack_embed
```

Test files per language:
- C++: `tests/runners/cpp/tests/test_<case>.cpp`
- C#: `tests/runners/csharp/tests/Test_<case>.cs`
- Rust: `tests/runners/rust/tests/test_<case>.rs`
- TS: `tests/runners/typescript/tests/test_<case>.ts`

## Maintenance: Adding a New Test Case

1. Add case name to `TEST_CASES` in each runner's `.bat` and `.sh`
2. Create test file for target language(s) following naming convention above
3. Write assertions exercising generated code (instantiation, serialization, roundtrip)
4. Handle special deps:
   - C# SQLite: needs `Microsoft.Data.Sqlite` in generated `.csproj`
   - Rust SQLite: needs `rusqlite` in generated `Cargo.toml`
   - Rust: add new module to generated `lib.rs` if file set changes
5. Run runner to verify

## Test File Patterns

**C++**: `#include` generated headers, `assert()`, return 0 on success

**C#**: Static Main, custom `Assert(bool, string)` helper, `Environment.Exit(1)` on fail

**Rust**: `use polygen_test::schema::*`, `assert_eq!`/`assert!`, `std::io::Cursor` for binary

**TypeScript**: Type-check only (`tsc --noEmit`), import generated interfaces, `console.assert()`
