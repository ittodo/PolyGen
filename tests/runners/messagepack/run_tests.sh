#!/usr/bin/env bash
# MessagePack Integration Test Runner for PolyGen

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
OUTPUT_DIR="$SCRIPT_DIR/output"

echo "=========================================="
echo "PolyGen MessagePack Integration Tests"
echo "=========================================="

if command -v python3 >/dev/null 2>&1; then
    PYTHON_BIN=python3
elif command -v python >/dev/null 2>&1; then
    PYTHON_BIN=python
else
    echo -e "${RED}Error: python3 or python is not installed${NC}"
    exit 1
fi

rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

echo -e "\n${YELLOW}Building PolyGen...${NC}"
cd "$PROJECT_ROOT"
cargo build --release
POLYGEN="$PROJECT_ROOT/target/release/polygen"

if [ ! -x "$POLYGEN" ]; then
    echo -e "${RED}Error: PolyGen binary not found at $POLYGEN${NC}"
    exit 1
fi

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
)

PASSED=0
FAILED=0

shopt -s nullglob
for test_name in "${TEST_SCHEMAS[@]}"; do
    echo -e "\n${YELLOW}Testing: $test_name${NC}"

    SCHEMA_PATH="$PROJECT_ROOT/tests/integration/$test_name/schema.poly"
    TEST_OUTPUT="$OUTPUT_DIR/$test_name"
    GENERATION_LOG="$OUTPUT_DIR/${test_name}_polygen_generation.log"
    VALIDATION_LOG="$TEST_OUTPUT/messagepack_validation.log"

    if [ ! -f "$SCHEMA_PATH" ]; then
        echo -e "${RED}  FAIL: Schema file not found${NC}"
        FAILED=$((FAILED + 1))
        continue
    fi

    echo "  Generating MessagePack descriptor..."
    if ! "$POLYGEN" \
        --schema-path "$SCHEMA_PATH" \
        --lang messagepack \
        --output-dir "$TEST_OUTPUT" \
        --templates-dir "$PROJECT_ROOT/templates" >"$GENERATION_LOG" 2>&1; then
        echo -e "${RED}  FAIL: Code generation failed${NC}"
        cat "$GENERATION_LOG"
        FAILED=$((FAILED + 1))
        continue
    fi

    echo "  Validating descriptor JSON..."
    OUTPUT_FILES=("$TEST_OUTPUT"/messagepack/*.messagepack.json)
    if [ "${#OUTPUT_FILES[@]}" -eq 0 ]; then
        echo -e "${RED}  FAIL: No MessagePack descriptors generated${NC}"
        FAILED=$((FAILED + 1))
    elif "$PYTHON_BIN" "$SCRIPT_DIR/validate_descriptor.py" "${OUTPUT_FILES[@]}" >"$VALIDATION_LOG" 2>&1; then
        echo -e "${GREEN}  PASS: Descriptor valid${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}  FAIL: Descriptor validation failed${NC}"
        cat "$VALIDATION_LOG"
        FAILED=$((FAILED + 1))
    fi
done
shopt -u nullglob

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
