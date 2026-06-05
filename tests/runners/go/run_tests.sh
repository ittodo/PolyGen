#!/usr/bin/env bash
# Go Integration Test Runner for PolyGen
# Tests that generated Go code compiles correctly

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
OUTPUT_DIR="$SCRIPT_DIR/output"

echo "=========================================="
echo "PolyGen Go Integration Tests"
echo "=========================================="

if ! command -v go >/dev/null 2>&1; then
    echo -e "${RED}Error: go is not installed${NC}"
    exit 1
fi

# Clean and create output directory
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# Build PolyGen
echo -e "\n${YELLOW}Building PolyGen...${NC}"
cd "$PROJECT_ROOT"
cargo build --release
POLYGEN="$PROJECT_ROOT/target/release/polygen"

if [ ! -x "$POLYGEN" ]; then
    echo -e "${RED}Error: PolyGen binary not found at $POLYGEN${NC}"
    exit 1
fi

# Test cases
TEST_SCHEMAS=(
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

PASSED=0
FAILED=0

shopt -s nullglob
for test_name in "${TEST_SCHEMAS[@]}"; do
    echo -e "\n${YELLOW}Testing: $test_name${NC}"

    SCHEMA_PATH="$PROJECT_ROOT/tests/integration/$test_name/schema.poly"
    TEST_OUTPUT="$OUTPUT_DIR/$test_name"

    if [ ! -f "$SCHEMA_PATH" ]; then
        echo -e "${RED}  FAIL: Schema file not found${NC}"
        FAILED=$((FAILED + 1))
        continue
    fi

    # Generate Go code
    echo "  Generating Go code..."
    GEN_OUTPUT=$("$POLYGEN" \
        --schema-path "$SCHEMA_PATH" \
        --lang go \
        --output-dir "$TEST_OUTPUT" \
        --templates-dir "$PROJECT_ROOT/templates" 2>&1) || {
        echo -e "${RED}  FAIL: Code generation failed${NC}"
        echo "$GEN_OUTPUT"
        FAILED=$((FAILED + 1))
        continue
    }

    # Initialize Go module and run optional smoke tests
    echo "  Testing Go code..."
    if [ ! -d "$TEST_OUTPUT/go" ]; then
        echo -e "${RED}  FAIL: No go output directory${NC}"
        FAILED=$((FAILED + 1))
        continue
    fi

    GO_FILES=("$TEST_OUTPUT"/go/*.go)
    if [ "${#GO_FILES[@]}" -eq 0 ]; then
        echo -e "${RED}  FAIL: No go files generated${NC}"
        FAILED=$((FAILED + 1))
        continue
    fi

    cd "$TEST_OUTPUT/go"

    # Initialize module if needed
    if [ ! -f "go.mod" ]; then
        MOD_OUTPUT="$TEST_OUTPUT/go_mod_init.log"
        if ! go mod init "generated_$test_name" > "$MOD_OUTPUT" 2>&1; then
            echo -e "${RED}  FAIL: go mod init failed${NC}"
            cat "$MOD_OUTPUT" | sed 's/^/    /'
            FAILED=$((FAILED + 1))
            cd "$PROJECT_ROOT"
            continue
        fi
    fi

    if [ -f "$SCRIPT_DIR/tests/${test_name}_test.go" ]; then
        if ! cp "$SCRIPT_DIR/tests/${test_name}_test.go" polygen_integration_test.go; then
            echo -e "${RED}  FAIL: Could not copy smoke test${NC}"
            FAILED=$((FAILED + 1))
            cd "$PROJECT_ROOT"
            continue
        fi
    fi

    if [ "$test_name" = "09_sqlite" ]; then
        GET_LOG="$TEST_OUTPUT/go_get_sqlite.log"
        if ! go get modernc.org/sqlite@v1.51.0 > "$GET_LOG" 2>&1; then
            echo -e "${RED}  FAIL: go get sqlite driver failed${NC}"
            cat "$GET_LOG" | sed 's/^/    /'
            FAILED=$((FAILED + 1))
            cd "$PROJECT_ROOT"
            continue
        fi
    fi

    TEST_LOG="$TEST_OUTPUT/go_test.log"
    if go test ./... > "$TEST_LOG" 2>&1; then
        echo -e "${GREEN}  PASS: Tests successful${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}  FAIL: Tests failed${NC}"
        cat "$TEST_LOG" | sed 's/^/    /'
        FAILED=$((FAILED + 1))
    fi

    cd "$PROJECT_ROOT"
done
shopt -u nullglob

# Summary
echo -e "\n=========================================="
echo "Test Summary"
echo "=========================================="
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo "Total:  $((PASSED + FAILED))"

if [ "$FAILED" -eq 0 ]; then
    echo -e "\n${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}Some tests failed!${NC}"
    exit 1
fi
