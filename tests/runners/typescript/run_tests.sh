#!/bin/bash
set -e

cd "$(dirname "$0")"

echo "=== TypeScript Integration Tests ==="
echo ""

# Check for Node.js
if ! command -v node &> /dev/null; then
    echo "Error: Node.js is not installed"
    exit 1
fi

# Create generated directory
mkdir -p generated/typescript

# Generate TypeScript code for complex schema
echo "Generating TypeScript code..."
cd ../../..
cargo run --release -- \
    --schema-path tests/integration/08_complex_schema/schema.poly \
    --lang typescript \
    --output-dir tests/runners/typescript/generated 2>&1 | grep -E "(Generating|완료)"
cd tests/runners/typescript

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
    echo "Installing dependencies..."
    npm install 2>&1 | tail -5
fi

# Type check
echo ""
echo "Type checking generated code..."
npx tsc --noEmit

echo ""
echo "=== All TypeScript tests passed! ==="
