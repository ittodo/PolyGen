---
name: test-runners
description: >
  Run and maintain PolyGen integration test runners (tests/runners/).
  Trigger when: (1) user asks to "test", "run tests", "run runners", or "integration test" for any language,
  (2) user needs to add/update test cases, test files, or runner scripts,
  (3) user asks to verify generated code compiles and runs correctly across languages.
  Supports: csharp, cpp, rust, typescript, go, sqlite, mysql, postgresql, mermaid, redis,
  python, messagepack, protobuf, kotlin, swift, unreal, and run_all runners.
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

| Target     | Windows command                             | POSIX command                              | Prerequisites            |
|------------|---------------------------------------------|--------------------------------------------|--------------------------|
| All/subset | `tests\runners\run_all.bat [runner...]`      | `bash tests/runners/run_all.sh [runner...]` | runner-specific tools    |
| C#         | `tests\runners\csharp\run_tests.bat`        | `bash tests/runners/csharp/run_tests.sh`   | dotnet SDK 8.0           |
| C++        | `tests\runners\cpp\run_tests.bat`           | `bash tests/runners/cpp/run_tests.sh`      | MSVC or g++              |
| Rust       | `tests\runners\rust\run_tests.bat`          | `bash tests/runners/rust/run_tests.sh`     | cargo                    |
| TypeScript | `tests\runners\typescript\run_tests.bat`    | `bash tests/runners/typescript/run_tests.sh` | node, npm              |
| Go         | `tests\runners\go\run_tests.bat`            | `bash tests/runners/go/run_tests.sh`       | go                       |
| SQLite     | `tests\runners\sqlite\run_tests.bat`        | `bash tests/runners/sqlite/run_tests.sh`   | python                   |
| MySQL      | `tests\runners\mysql\run_tests.bat`         | `bash tests/runners/mysql/run_tests.sh`    | python                   |
| PostgreSQL | `tests\runners\postgresql\run_tests.bat`    | `bash tests/runners/postgresql/run_tests.sh` | python                 |
| Mermaid    | `tests\runners\mermaid\run_tests.bat`       | `bash tests/runners/mermaid/run_tests.sh`  | python                   |
| Redis      | `tests\runners\redis\run_tests.bat`         | `bash tests/runners/redis/run_tests.sh`    | python                   |
| Python     | `tests\runners\python\run_tests.bat`        | `bash tests/runners/python/run_tests.sh`   | python                   |
| MessagePack | `tests\runners\messagepack\run_tests.bat`  | `bash tests/runners/messagepack/run_tests.sh` | python                |
| Protobuf   | `tests\runners\protobuf\run_tests.bat`      | `bash tests/runners/protobuf/run_tests.sh` | python                   |
| Kotlin     | `tests\runners\kotlin\run_tests.bat`        | `bash tests/runners/kotlin/run_tests.sh`   | python                   |
| Swift      | `tests\runners\swift\run_tests.bat`         | `bash tests/runners/swift/run_tests.sh`    | python                   |
| Unreal     | `tests\runners\unreal\run_tests.bat`        | `bash tests/runners/unreal/run_tests.sh`   | python                   |

- No target specified: ask which runner(s), or use `run_all` when the user asks to test all.
- `run_all` accepts optional runner names, e.g. `tests\runners\run_all.bat sqlite rust`.
- `run_all --list` prints supported runner names.
- `run_all --verify` checks that `run_all.bat`, `run_all.sh`, and runner directories
  stay synchronized and retain ordered Python availability/fallback, selected-Python live/regression invocation, pre-invocation no-bytecode, runtime runner-argument, `--list`, `--help`, and live/regression `--verify` guards, then runs focused verifier regression tests for duplicate,
  empty, malformed, invalid, missing, extra, one-sided runners, and Windows `--list`
  and `--help` output/default/default-validation/subset/failure/unknown-runner/invalid-argument/metachar drift.
  Live matrix verification failures must short-circuit before regression tests run.
  On Windows, fallback from hidden/missing `python` to the `py -3` launcher and the no-Python failure exit code are covered by execution regression tests.

## Output Filtering

Return to caller ONLY:
- **Summary**: `Passed: N, Failed: N`
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
- TypeScript: `tests/runners/typescript/tests/test_<case>.ts` plus `tests/runners/typescript/tests/run_all.ts`
- Go: optional smoke tests at `tests/runners/go/tests/<case>_test.go`
- Descriptor/DDL/Python/Kotlin/Swift/Unreal runners use validator scripts or generated-code compilation instead of per-case test files.

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

**TypeScript**: Type-check (`tsc --noEmit`) plus runtime assertions via `tsx tests/run_all.ts`
