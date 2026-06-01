#!/usr/bin/env bash
# PolyGen C# Integration Test Runner
# Generates code from test schemas and compiles/runs the tests

set -o pipefail

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

echo -e "${BLUE}=== PolyGen C# Integration Tests ===${NC}"
echo ""

# Check for dotnet
if ! command -v dotnet &> /dev/null; then
    echo -e "${RED}Error: dotnet is not installed${NC}"
    exit 1
fi

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
    mkdir -p "$OUTPUT_DIR/csharp"

    # Generate code for each schema
    echo "  Generating C# code..."
    GEN_SUCCESS=true
    for SCHEMA in $SCHEMA_FILES; do
        SCHEMA_NAME=$(basename "$SCHEMA")
        echo "    - $SCHEMA_NAME"
        if ! "$POLYGEN" --schema-path "$SCHEMA" --lang csharp --output-dir "$OUTPUT_DIR" --templates-dir "$PROJECT_ROOT/templates" 2>&1 | sed 's/^/      /'; then
            GEN_SUCCESS=false
        fi
    done

    if [ "$GEN_SUCCESS" = false ]; then
        echo -e "${RED}  FAILED (code generation error)${NC}"
        inc_failed
        continue
    fi

    GENERATED_CS_FILES=$(find "$OUTPUT_DIR/csharp" -name "*.cs" | sort)
    if [ -z "$GENERATED_CS_FILES" ]; then
        echo -e "${RED}  FAILED (no C# files generated)${NC}"
        inc_failed
        continue
    fi

    # Check if test file exists
    TEST_FILE="$SCRIPT_DIR/tests/Test_${TEST_CASE}.cs"
    if [ ! -f "$TEST_FILE" ]; then
        echo -e "${RED}  FAILED (test file not found: $TEST_FILE)${NC}"
        inc_failed
        continue
    fi

    # Create a test project
    TEST_PROJECT_DIR="$OUTPUT_DIR/TestProject"
    mkdir -p "$TEST_PROJECT_DIR"

    # Create csproj file
    cat > "$TEST_PROJECT_DIR/TestProject.csproj" << 'CSPROJ'
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
    <TreatWarningsAsErrors>false</TreatWarningsAsErrors>
  </PropertyGroup>
</Project>
CSPROJ

    # Copy generated files and test file
    if ! cp "$OUTPUT_DIR/csharp"/*.cs "$TEST_PROJECT_DIR/"; then
        echo -e "${RED}  FAILED (could not copy generated C# files)${NC}"
        inc_failed
        continue
    fi

    # Copy subdirectories (Common, Data, etc.)
    COPY_FAILED=false
    shopt -s nullglob
    for subdir in "$OUTPUT_DIR/csharp"/*/; do
        if [ -d "$subdir" ]; then
            dirname=$(basename "$subdir")
            subdir_files=("$subdir"*.cs)
            if ! mkdir -p "$TEST_PROJECT_DIR/$dirname"; then
                COPY_FAILED=true
                break
            fi
            if [ "${#subdir_files[@]}" -gt 0 ] && ! cp "${subdir_files[@]}" "$TEST_PROJECT_DIR/$dirname/"; then
                COPY_FAILED=true
                break
            fi
        fi
    done
    shopt -u nullglob
    if [ "$COPY_FAILED" = true ]; then
        echo -e "${RED}  FAILED (could not copy generated C# subdirectory files)${NC}"
        inc_failed
        continue
    fi

    if ! cp "$TEST_FILE" "$TEST_PROJECT_DIR/Program.cs"; then
        echo -e "${RED}  FAILED (could not copy test file)${NC}"
        inc_failed
        continue
    fi

    # Compile
    echo "  Compiling..."
    COMPILE_OUTPUT=$(mktemp)
    cd "$TEST_PROJECT_DIR"
    if dotnet build -c Release --nologo -v q >"$COMPILE_OUTPUT" 2>&1; then
        # Run test
        echo "  Running..."
        RUN_OUTPUT=$(mktemp)
        if dotnet run -c Release --no-build >"$RUN_OUTPUT" 2>&1; then
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
echo ""

if [ $FAILED -gt 0 ]; then
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
else
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
fi
