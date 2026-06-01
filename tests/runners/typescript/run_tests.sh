#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
GENERATED_DIR="$SCRIPT_DIR/generated"

echo "=== TypeScript Integration Tests ==="
echo ""

# Check for Node.js
if ! command -v node &> /dev/null; then
    echo "Error: Node.js is not installed"
    exit 1
fi
if ! command -v npm &> /dev/null; then
    echo "Error: npm is not installed"
    exit 1
fi

echo "Building PolyGen..."
cd "$PROJECT_ROOT"
cargo build --release
POLYGEN="$PROJECT_ROOT/target/release/polygen"

if [ ! -x "$POLYGEN" ]; then
    echo "Error: PolyGen binary not found at $POLYGEN"
    exit 1
fi

cd "$SCRIPT_DIR"

# Install dependencies if needed
NPM_INSTALL_LOG="$SCRIPT_DIR/npm_install.log"
if [ ! -d "node_modules" ]; then
    echo "Installing dependencies..."
    if ! npm install >"$NPM_INSTALL_LOG" 2>&1; then
        echo "Error: npm install failed"
        cat "$NPM_INSTALL_LOG"
        exit 1
    fi
fi
if [ ! -x "node_modules/.bin/tsx" ]; then
    echo "Updating dependencies..."
    if ! npm install >"$NPM_INSTALL_LOG" 2>&1; then
        echo "Error: npm install failed"
        cat "$NPM_INSTALL_LOG"
        exit 1
    fi
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

GENERATED=0
FAILED=0

echo ""
echo "Phase 1: Generating TypeScript code for all test cases..."
rm -rf "$GENERATED_DIR"
mkdir -p "$GENERATED_DIR"

shopt -s nullglob
for case in "${TEST_CASES[@]}"; do
    TEST_DIR="$PROJECT_ROOT/tests/integration/$case"
    OUTPUT_DIR="$GENERATED_DIR/$case"
    GENERATION_LOG="$GENERATED_DIR/${case}_polygen_generation.log"

    if [ ! -d "$TEST_DIR" ]; then
        echo "  FAILED $case (test directory not found)"
        FAILED=$((FAILED + 1))
        continue
    fi

    SCHEMA_FILES=("$TEST_DIR"/*.poly)
    if [ "${#SCHEMA_FILES[@]}" -eq 0 ]; then
        echo "  FAILED $case (schema file not found)"
        FAILED=$((FAILED + 1))
        continue
    fi

    mkdir -p "$OUTPUT_DIR/typescript"

    echo "  Generating $case..."
    CASE_FAILED=0
    for schema in "${SCHEMA_FILES[@]}"; do
        if ! "$POLYGEN" \
            --schema-path "$schema" \
            --lang typescript \
            --output-dir "$OUTPUT_DIR" \
            --templates-dir "$PROJECT_ROOT/templates" >"$GENERATION_LOG" 2>&1; then
            echo "  FAILED $case (generation error: $(basename "$schema"))"
            cat "$GENERATION_LOG"
            FAILED=$((FAILED + 1))
            CASE_FAILED=1
        fi
    done
    OUTPUT_FILES=("$OUTPUT_DIR"/typescript/*.ts)
    if [ "${#OUTPUT_FILES[@]}" -eq 0 ]; then
        echo "  FAILED $case (no typescript files generated)"
        FAILED=$((FAILED + 1))
        CASE_FAILED=1
    fi
    if [ "$CASE_FAILED" -eq 0 ]; then
        GENERATED=$((GENERATED + 1))
    fi
done
shopt -u nullglob

echo "  Generated $GENERATED test cases."

if [ "$FAILED" -gt 0 ]; then
    echo ""
    echo "FAILED: Generation phase had $FAILED error(s)"
    exit 1
fi

# Type check
echo ""
echo "Phase 2: Type checking all generated code and tests..."
TYPECHECK_LOG="$GENERATED_DIR/typescript_typecheck.log"
if ! npx tsc --noEmit >"$TYPECHECK_LOG" 2>&1; then
    echo ""
    echo "FAILED: Type check errors found"
    cat "$TYPECHECK_LOG"
    exit 1
fi

# Execute runtime assertions.
echo ""
echo "Phase 3: Running TypeScript runtime tests..."
RUNTIME_LOG="$GENERATED_DIR/typescript_runtime.log"
if ! npx tsx tests/run_all.ts >"$RUNTIME_LOG" 2>&1; then
    echo ""
    echo "FAILED: Runtime test errors found"
    cat "$RUNTIME_LOG"
    exit 1
fi

echo ""
PASSED=0
for case in "${TEST_CASES[@]}"; do
    TEST_FILE="$SCRIPT_DIR/tests/test_${case}.ts"
    if [ -f "$TEST_FILE" ]; then
        PASSED=$((PASSED + 1))
    else
        echo "  FAILED $case (test file not found)"
        FAILED=$((FAILED + 1))
    fi
done

echo "=== Test Summary ==="
echo "  Passed:  $PASSED"
echo "  Failed:  $FAILED"
echo ""
if [ "$FAILED" -gt 0 ]; then
    echo "Some tests failed!"
    exit 1
else
    echo "All tests passed!"
    exit 0
fi
