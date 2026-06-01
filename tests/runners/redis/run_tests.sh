#!/usr/bin/env bash
# Redis Integration Test Runner for PolyGen

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
OUTPUT_DIR="$SCRIPT_DIR/output"

echo "=========================================="
echo "PolyGen Redis Integration Tests"
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

run_case() {
    local test_name="$1"
    local schema_path="$2"
    local test_output="$3"
    local generation_log="$OUTPUT_DIR/${test_name}_polygen_generation.log"
    local validation_log="$test_output/redis_validation.log"

    echo -e "\n${YELLOW}Testing: $test_name${NC}"

    if [ ! -f "$schema_path" ]; then
        echo -e "${RED}  FAIL: Schema file not found${NC}"
        FAILED=$((FAILED + 1))
        return
    fi

    echo "  Generating Redis schema..."
    if ! "$POLYGEN" \
        --schema-path "$schema_path" \
        --lang redis \
        --output-dir "$test_output" \
        --templates-dir "$PROJECT_ROOT/templates" >"$generation_log" 2>&1; then
        echo -e "${RED}  FAIL: Code generation failed${NC}"
        cat "$generation_log"
        FAILED=$((FAILED + 1))
        return
    fi

    echo "  Validating Redis descriptor and Lua helper..."
    local descriptor="$test_output/redis/schema.redis.json"
    local lua_helper="$test_output/redis/schema.redis.lua"
    if [ ! -f "$descriptor" ] || [ ! -f "$lua_helper" ]; then
        echo -e "${RED}  FAIL: Redis output files not found${NC}"
        FAILED=$((FAILED + 1))
    elif "$PYTHON_BIN" "$SCRIPT_DIR/validate_redis.py" "$descriptor" "$lua_helper" >"$validation_log" 2>&1; then
        echo -e "${GREEN}  PASS: Redis valid${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}  FAIL: Redis validation failed${NC}"
        cat "$validation_log"
        FAILED=$((FAILED + 1))
    fi
}

for test_name in "${TEST_SCHEMAS[@]}"; do
    SCHEMA_PATH="$PROJECT_ROOT/tests/integration/$test_name/schema.poly"
    TEST_OUTPUT="$OUTPUT_DIR/$test_name"

    run_case "$test_name" "$SCHEMA_PATH" "$TEST_OUTPUT"
done

run_case "redis_cache_schema" "$PROJECT_ROOT/tests/test_data/redis_cache_schema.poly" "$OUTPUT_DIR/redis_cache_schema"

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
