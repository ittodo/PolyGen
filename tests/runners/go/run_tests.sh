#!/bin/bash
# Go Integration Test Runner for PolyGen
# Tests that generated Go code compiles correctly

set -e

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

# Clean and create output directory
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# Build PolyGen
echo -e "\n${YELLOW}Building PolyGen...${NC}"
cd "$PROJECT_ROOT"
cargo build --release

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
)

PASSED=0
FAILED=0

for test_name in "${TEST_SCHEMAS[@]}"; do
    echo -e "\n${YELLOW}Testing: $test_name${NC}"

    SCHEMA_PATH="$PROJECT_ROOT/tests/integration/$test_name/schema.poly"
    TEST_OUTPUT="$OUTPUT_DIR/$test_name"

    if [ ! -f "$SCHEMA_PATH" ]; then
        echo -e "${RED}  SKIP: Schema file not found${NC}"
        continue
    fi

    # Generate Go code
    echo "  Generating Go code..."
    GEN_OUTPUT=$("$PROJECT_ROOT/target/release/polygen" \
        --schema-path "$SCHEMA_PATH" \
        --lang go \
        --output-dir "$TEST_OUTPUT" 2>&1) || {
        echo -e "${RED}  FAIL: Code generation failed${NC}"
        echo "$GEN_OUTPUT"
        FAILED=$((FAILED + 1))
        continue
    }

    # Initialize Go module and compile
    echo "  Compiling Go code..."
    cd "$TEST_OUTPUT/go"

    # Initialize module if needed
    if [ ! -f "go.mod" ]; then
        go mod init "generated_$test_name" > /dev/null 2>&1
    fi

    # Try to build
    if go build . 2>&1; then
        echo -e "${GREEN}  PASS: Compilation successful${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}  FAIL: Compilation failed${NC}"
        FAILED=$((FAILED + 1))
    fi

    cd "$PROJECT_ROOT"
done

# Summary
echo -e "\n=========================================="
echo "Test Summary"
echo "=========================================="
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo "Total:  $((PASSED + FAILED))"

if [ $FAILED -eq 0 ]; then
    echo -e "\n${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}Some tests failed!${NC}"
    exit 1
fi
