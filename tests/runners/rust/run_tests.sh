#!/bin/bash
# PolyGen Rust Integration Test Runner
# Generates code from test schemas and compiles/runs the tests

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
INTEGRATION_DIR="$PROJECT_ROOT/tests/integration"
GENERATED_DIR="$SCRIPT_DIR/generated"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== PolyGen Rust Integration Tests ===${NC}"
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
)

# Create generated directory
mkdir -p "$GENERATED_DIR"

PASSED=0
FAILED=0
SKIPPED=0

# Function to safely increment counters
inc_passed() { PASSED=$((PASSED + 1)); }
inc_failed() { FAILED=$((FAILED + 1)); }
inc_skipped() { SKIPPED=$((SKIPPED + 1)); }

for TEST_CASE in "${TEST_CASES[@]}"; do
    echo ""
    echo -e "${BLUE}--- Test Case: $TEST_CASE ---${NC}"

    TEST_DIR="$INTEGRATION_DIR/$TEST_CASE"
    OUTPUT_DIR="$GENERATED_DIR/$TEST_CASE"

    if [ ! -d "$TEST_DIR" ]; then
        echo -e "${YELLOW}  Skipped: Test directory not found${NC}"
        inc_skipped
        continue
    fi

    # Find schema files
    SCHEMA_FILES=$(find "$TEST_DIR" -name "*.poly" | sort)
    if [ -z "$SCHEMA_FILES" ]; then
        echo -e "${YELLOW}  Skipped: No schema files found${NC}"
        inc_skipped
        continue
    fi

    # Clean and create output directory
    rm -rf "$OUTPUT_DIR"
    mkdir -p "$OUTPUT_DIR/rust"

    # Generate code for each schema
    echo "  Generating Rust code..."
    GEN_SUCCESS=true
    for SCHEMA in $SCHEMA_FILES; do
        SCHEMA_NAME=$(basename "$SCHEMA")
        echo "    - $SCHEMA_NAME"
        if ! "$POLYGEN" --schema-path "$SCHEMA" --lang rust --output-dir "$OUTPUT_DIR" --templates-dir "$PROJECT_ROOT/templates" 2>&1 | sed 's/^/      /'; then
            GEN_SUCCESS=false
        fi
    done

    if [ "$GEN_SUCCESS" = false ]; then
        echo -e "${RED}  FAILED (code generation error)${NC}"
        inc_failed
        continue
    fi

    # Check if test file exists
    TEST_FILE="$SCRIPT_DIR/tests/test_${TEST_CASE}.rs"
    if [ ! -f "$TEST_FILE" ]; then
        echo -e "${YELLOW}  Skipped: Test file not found ($TEST_FILE)${NC}"
        inc_skipped
        continue
    fi

    # Create a Cargo project
    TEST_PROJECT_DIR="$OUTPUT_DIR/rust"
    mkdir -p "$TEST_PROJECT_DIR/src"

    # Create Cargo.toml
    cat > "$TEST_PROJECT_DIR/Cargo.toml" << 'CARGO'
[package]
name = "polygen_test"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
byteorder = "1.5"
CARGO

    # Copy generated files to src/
    cp "$OUTPUT_DIR/rust"/*.rs "$TEST_PROJECT_DIR/src/" 2>/dev/null || true

    # Create lib.rs that includes all modules
    cat > "$TEST_PROJECT_DIR/src/lib.rs" << 'LIBRS'
pub mod polygen_support;
pub mod schema;
pub mod schema_loaders;
pub mod schema_container;
LIBRS

    # Copy test file as main.rs
    cp "$TEST_FILE" "$TEST_PROJECT_DIR/src/main.rs"

    # Compile
    echo "  Compiling..."
    cd "$TEST_PROJECT_DIR"
    COMPILE_OUTPUT=$(mktemp)
    if cargo build --release 2>"$COMPILE_OUTPUT"; then
        # Run test
        echo "  Running..."
        RUN_OUTPUT=$(mktemp)
        if cargo run --release 2>&1 >"$RUN_OUTPUT"; then
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
    cd "$PROJECT_ROOT"
done

echo ""
echo -e "${BLUE}=== Test Summary ===${NC}"
echo -e "  ${GREEN}Passed:${NC}  $PASSED"
echo -e "  ${RED}Failed:${NC}  $FAILED"
echo -e "  ${YELLOW}Skipped:${NC} $SKIPPED"
echo ""

if [ $FAILED -gt 0 ]; then
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
else
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
fi
