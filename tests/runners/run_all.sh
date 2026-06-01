#!/usr/bin/env bash
# PolyGen all integration runner.
# Usage:
#   tests/runners/run_all.sh
#   tests/runners/run_all.sh csharp rust sqlite
#   tests/runners/run_all.sh --verify

set -u
set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DEFAULT_RUNNERS=(
    csharp
    cpp
    rust
    typescript
    go
    sqlite
    mysql
    postgresql
    mermaid
    redis
    python
    messagepack
    protobuf
    kotlin
    swift
    unreal
)

case "${1:-}" in
    --help|-h)
        cat <<'USAGE'
Usage:
  tests/runners/run_all.sh
  tests/runners/run_all.sh sqlite rust
  tests/runners/run_all.sh --list
  tests/runners/run_all.sh --verify
  tests/runners/run_all.sh --help

Runs all integration runners, or only the runner names passed as arguments.
--verify checks runner matrix synchronization and verifier regression tests.
USAGE
        exit 0
        ;;
    --list)
        printf '%s\n' "${DEFAULT_RUNNERS[@]}"
        exit 0
        ;;
    --verify)
        if command -v python3 >/dev/null 2>&1; then
            PYTHON_BIN=python3
        elif command -v python >/dev/null 2>&1; then
            PYTHON_BIN=python
        else
            echo "FAILED (python not found)"
            exit 1
        fi
        echo "=== Verifying runner matrix ==="
        PYTHONDONTWRITEBYTECODE=1 "$PYTHON_BIN" "$SCRIPT_DIR/verify_runner_matrix.py"
        VERIFY_EXIT=$?
        if [ "$VERIFY_EXIT" -ne 0 ]; then
            exit "$VERIFY_EXIT"
        fi
        echo
        echo "=== Verifying runner matrix regression tests ==="
        PYTHONDONTWRITEBYTECODE=1 "$PYTHON_BIN" "$SCRIPT_DIR/test_verify_runner_matrix.py"
        exit $?
        ;;
esac

if [ "$#" -gt 0 ]; then
    RUNNERS=("$@")
else
    RUNNERS=("${DEFAULT_RUNNERS[@]}")
fi

PASSED=0
FAILED=0

echo "=== PolyGen All Integration Runners ==="
echo

for RUNNER in "${RUNNERS[@]}"; do
    RUNNER_SCRIPT="$SCRIPT_DIR/$RUNNER/run_tests.sh"
    echo "=== Runner: $RUNNER ==="

    case "$RUNNER" in
        ''|*[!abcdefghijklmnopqrstuvwxyz0123456789_-]*)
            echo "  FAILED (invalid runner name)"
            FAILED=$((FAILED + 1))
            echo
            continue
            ;;
    esac

    if [ ! -f "$RUNNER_SCRIPT" ]; then
        echo "  FAILED (runner script not found)"
        FAILED=$((FAILED + 1))
    elif (cd "$PROJECT_ROOT" && bash "$RUNNER_SCRIPT"); then
        echo "  PASSED"
        PASSED=$((PASSED + 1))
    else
        echo "  FAILED (runner $RUNNER failed)"
        FAILED=$((FAILED + 1))
    fi

    echo
done

echo "=== All Runner Summary ==="
echo "  Passed:  $PASSED"
echo "  Failed:  $FAILED"
echo

if [ "$FAILED" -gt 0 ]; then
    exit 1
fi

exit 0
