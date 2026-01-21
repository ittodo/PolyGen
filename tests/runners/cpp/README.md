# PolyGen C++ Integration Tests

This directory contains integration tests for the C++ code generator.

## Test Cases

| Case | Description |
|------|-------------|
| 01_basic_types | All primitive types and simple structs |
| 02_imports | Cross-file imports and type references |
| 03_nested_namespaces | Deeply nested namespace structures |
| 04_inline_enums | Enum definitions inside tables |
| 05_embedded_structs | Embed definitions and nested embeds |
| 06_arrays_and_optionals | Array and optional field types |
| 07_indexes | primary_key, unique, foreign_key with containers |
| 08_complex_schema | Comprehensive test combining all features |

## Running Tests

```bash
./run_tests.sh
```

This script will:
1. Build PolyGen (if not already built)
2. Generate C++ code for each test case
3. Compile the test files
4. Run the tests and report results

## Structure

```
runners/cpp/
├── run_tests.sh          # Main test runner script
├── CMakeLists.txt        # CMake configuration (alternative)
├── tests/                # Test source files
│   ├── test_01_basic_types.cpp
│   ├── test_02_imports.cpp
│   └── ...
└── generated/            # Generated code (created by run_tests.sh)
    ├── 01_basic_types/
    │   └── cpp/
    ├── 02_imports/
    │   └── cpp/
    └── ...
```

## Requirements

- C++17 compatible compiler (g++ or clang++)
- Rust toolchain (for building PolyGen)
