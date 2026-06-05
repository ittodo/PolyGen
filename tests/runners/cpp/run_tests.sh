#!/usr/bin/env bash
# PolyGen C++ Integration Test Runner
# Generates code from test schemas and compiles/runs the tests

# Don't use set -e as we want to continue on test failures
set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
INTEGRATION_DIR="$PROJECT_ROOT/tests/integration"
GENERATED_DIR="$SCRIPT_DIR/generated"
STATIC_DIR="$PROJECT_ROOT/static/cpp"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== PolyGen C++ Integration Tests ===${NC}"
echo ""

# Build PolyGen first
echo -e "${YELLOW}Building PolyGen...${NC}"
cd "$PROJECT_ROOT"
if ! cargo build --release 2>&1 | sed 's/^/    /'; then
    echo -e "${RED}Error: Failed to build PolyGen${NC}"
    exit 1
fi
POLYGEN="$PROJECT_ROOT/target/release/polygen"

if [ ! -f "$POLYGEN" ]; then
    echo -e "${RED}Error: PolyGen binary not found at $POLYGEN${NC}"
    exit 1
fi

# Test cases
TEST_CASES=(
    "01_basic_types"
    "02_imports"
    "03_nested_namespaces"
    "04_inline_enums"
    "05_embedded_structs"
    "06_arrays_and_optionals"
    "07_indexes"
    "08_complex_schema"
    "09_sqlite"
    "10_pack_embed"
    "11_relations_indexes"
)

# Create generated directory
mkdir -p "$GENERATED_DIR"

PASSED=0
FAILED=0

# Function to safely increment counters
inc_passed() { PASSED=$((PASSED + 1)); }
inc_failed() { FAILED=$((FAILED + 1)); }

for TEST_CASE in "${TEST_CASES[@]}"; do
    echo ""
    echo -e "${BLUE}--- Test Case: $TEST_CASE ---${NC}"

    TEST_DIR="$INTEGRATION_DIR/$TEST_CASE"
    OUTPUT_DIR="$GENERATED_DIR/$TEST_CASE"

    if [ ! -d "$TEST_DIR" ]; then
        echo -e "${RED}  FAILED (test directory not found)${NC}"
        inc_failed
        continue
    fi

    # Find schema files
    SCHEMA_FILES=$(find "$TEST_DIR" -name "*.poly" | sort)
    if [ -z "$SCHEMA_FILES" ]; then
        echo -e "${RED}  FAILED (schema file not found)${NC}"
        inc_failed
        continue
    fi

    # Clean and create output directory
    rm -rf "$OUTPUT_DIR"
    mkdir -p "$OUTPUT_DIR/cpp"

    # Generate code for each schema
    echo "  Generating C++ code..."
    GEN_SUCCESS=true
    for SCHEMA in $SCHEMA_FILES; do
        SCHEMA_NAME=$(basename "$SCHEMA")
        echo "    - $SCHEMA_NAME"
        if ! "$POLYGEN" --schema-path "$SCHEMA" --lang cpp --output-dir "$OUTPUT_DIR" --templates-dir "$PROJECT_ROOT/templates" 2>&1 | sed 's/^/      /'; then
            GEN_SUCCESS=false
        fi
    done
    if [ "$GEN_SUCCESS" = false ]; then
        echo -e "${RED}  FAILED (code generation error)${NC}"
        inc_failed
        continue
    fi

    GENERATED_HEADERS=$(find "$OUTPUT_DIR/cpp" -maxdepth 1 -name "*.hpp" | sort)
    if [ -z "$GENERATED_HEADERS" ]; then
        echo -e "${RED}  FAILED (no C++ headers generated)${NC}"
        inc_failed
        continue
    fi

    # Copy polygen_support.hpp
    if [ ! -f "$STATIC_DIR/polygen_support.hpp" ]; then
        echo -e "${RED}  FAILED (polygen_support.hpp not found)${NC}"
        inc_failed
        continue
    fi
    if ! cp "$STATIC_DIR/polygen_support.hpp" "$OUTPUT_DIR/cpp/"; then
        echo -e "${RED}  FAILED (could not copy polygen_support.hpp)${NC}"
        inc_failed
        continue
    fi

    # Check if test file exists
    TEST_FILE="$SCRIPT_DIR/tests/test_${TEST_CASE}.cpp"
    if [ ! -f "$TEST_FILE" ]; then
        echo -e "${RED}  FAILED (test file not found: $TEST_FILE)${NC}"
        inc_failed
        continue
    fi

    # Compile
    echo "  Compiling..."
    BINARY="$OUTPUT_DIR/test_${TEST_CASE}"
    COMPILE_OUTPUT=$(mktemp)
    EXTRA_SOURCES=()
    if [ -f "$OUTPUT_DIR/cpp/schema_redis_keys.hpp" ]; then
        REDIS_SMOKE="$OUTPUT_DIR/redis_keys_smoke.cpp"
        cat > "$REDIS_SMOKE" <<'EOF'
#include "schema_redis_keys.hpp"
int polygen_cpp_redis_keys_smoke() { return 0; }
EOF
        EXTRA_SOURCES+=("$REDIS_SMOKE")
    fi

    if g++ -std=c++17 -Wall -Wextra -O2 \
        -I"$OUTPUT_DIR/cpp" \
        "$TEST_FILE" "${EXTRA_SOURCES[@]}" \
        -o "$BINARY" >"$COMPILE_OUTPUT" 2>&1; then

        # Run test
        echo "  Running..."
        RUN_OUTPUT=$(mktemp)
        if "$BINARY" >"$RUN_OUTPUT" 2>&1; then
            cat "$RUN_OUTPUT" | sed 's/^/    /'
            echo -e "${GREEN}  PASSED${NC}"
            inc_passed
        else
            cat "$RUN_OUTPUT" | sed 's/^/    /'
            echo -e "${RED}  FAILED (runtime error)${NC}"
            inc_failed
        fi
        rm -f "$RUN_OUTPUT"
    else
        echo -e "${RED}  FAILED (compilation error)${NC}"
        cat "$COMPILE_OUTPUT" | sed 's/^/    /'
        inc_failed
    fi
    rm -f "$COMPILE_OUTPUT"
done

echo ""
echo -e "${BLUE}=== Test Summary ===${NC}"
echo -e "  ${GREEN}Passed:${NC}  $PASSED"
echo -e "  ${RED}Failed:${NC}  $FAILED"
echo ""

if [ $FAILED -gt 0 ]; then
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
else
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
fi
