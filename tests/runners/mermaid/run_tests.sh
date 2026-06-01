#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
INTEGRATION_DIR="$PROJECT_ROOT/tests/integration"
OUTPUT_DIR="$SCRIPT_DIR/output"

echo "=== PolyGen Mermaid Integration Tests ==="
echo

if command -v python3 >/dev/null 2>&1; then
    PYTHON_BIN=python3
elif command -v python >/dev/null 2>&1; then
    PYTHON_BIN=python
else
    echo "Error: python3 or python is not installed" >&2
    exit 1
fi

echo "Building PolyGen..."
cd "$PROJECT_ROOT"
cargo build --release
POLYGEN="$PROJECT_ROOT/target/release/polygen"

if [[ ! -x "$POLYGEN" ]]; then
    echo "Error: PolyGen binary not found" >&2
    exit 1
fi

TEST_CASES=(
    01_basic_types
    02_imports
    03_nested_namespaces
    04_inline_enums
    05_embedded_structs
    06_arrays_and_optionals
    07_indexes
    08_complex_schema
    09_sqlite
    10_pack_embed
)

rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

passed=0
failed=0

for case_name in "${TEST_CASES[@]}"; do
    echo
    echo "--- Test Case: $case_name ---"

    schema_path="$INTEGRATION_DIR/$case_name/schema.poly"
    test_output="$OUTPUT_DIR/$case_name"

    if [[ ! -f "$schema_path" ]]; then
        echo "  FAILED (schema file not found)"
        failed=$((failed + 1))
        continue
    fi

    echo "  Generating Mermaid diagram..."
    generation_log="$OUTPUT_DIR/${case_name}_polygen_generation.log"
    if ! "$POLYGEN" --schema-path "$schema_path" --lang mermaid --output-dir "$test_output" --templates-dir "$PROJECT_ROOT/templates" >"$generation_log" 2>&1; then
        echo "  FAILED (generation error)"
        cat "$generation_log"
        failed=$((failed + 1))
        continue
    fi

    echo "  Validating Mermaid diagram..."
    diagram_path="$test_output/mermaid/schema.mmd"
    validation_log="$test_output/mermaid_validation.log"
    if [[ ! -f "$diagram_path" ]]; then
        echo "  FAILED (Mermaid diagram file not found)"
        failed=$((failed + 1))
    elif "$PYTHON_BIN" "$SCRIPT_DIR/validate_mermaid.py" "$diagram_path" >"$validation_log" 2>&1; then
        echo "  PASSED"
        passed=$((passed + 1))
    else
        echo "  FAILED (Mermaid validation error)"
        cat "$validation_log"
        failed=$((failed + 1))
    fi
done

echo
echo "=== Test Summary ==="
echo "  Passed:  $passed"
echo "  Failed:  $failed"
echo

if [[ "$failed" -gt 0 ]]; then
    echo "Some tests failed!"
    exit 1
fi

echo "All tests passed!"
