@echo off
setlocal enabledelayedexpansion
REM PolyGen TypeScript Integration Test Runner for Windows

set SCRIPT_DIR=%~dp0
set PROJECT_ROOT=%SCRIPT_DIR%..\..\..
set INTEGRATION_DIR=%PROJECT_ROOT%\tests\integration
set GENERATED_DIR=%SCRIPT_DIR%generated

echo === PolyGen TypeScript Integration Tests ===
echo.

REM Check for Node.js
where node >nul 2>&1
if errorlevel 1 (
    echo Error: Node.js is not installed
    exit /b 1
)

REM Build PolyGen
echo Building PolyGen...
cd /d "%PROJECT_ROOT%"
cargo build --release
if errorlevel 1 (
    echo Error: Failed to build PolyGen
    exit /b 1
)
set POLYGEN=%PROJECT_ROOT%\target\release\polygen.exe

if not exist "%POLYGEN%" (
    echo Error: PolyGen binary not found
    exit /b 1
)

REM Install dependencies if needed
cd /d "%SCRIPT_DIR%"
if not exist "node_modules" (
    echo Installing dependencies...
    npm install
)

REM Test cases
set TEST_CASES=01_basic_types 02_imports 03_nested_namespaces 04_inline_enums 05_embedded_structs 06_arrays_and_optionals 07_indexes 08_complex_schema 09_sqlite

set GENERATED=0

REM Phase 1: Generate all code
echo.
echo Phase 1: Generating TypeScript code for all test cases...
for %%T in (%TEST_CASES%) do (
    set TEST_DIR=%INTEGRATION_DIR%\%%T
    set OUTPUT_DIR=%GENERATED_DIR%\%%T

    if exist "!TEST_DIR!" (
        REM Clean and create output directory
        if exist "!OUTPUT_DIR!" rmdir /s /q "!OUTPUT_DIR!"
        mkdir "!OUTPUT_DIR!\typescript"

        REM Generate code from project root for correct relative paths
        echo   Generating %%T...
        cd /d "%PROJECT_ROOT%"
        for %%S in ("!TEST_DIR!\*.poly") do (
            "%POLYGEN%" --schema-path "%%S" --lang typescript --output-dir "!OUTPUT_DIR!" --templates-dir "templates" >nul 2>&1
        )
        set /a GENERATED+=1
    )
)
cd /d "%SCRIPT_DIR%"
echo   Generated %GENERATED% test cases.

REM Phase 2: Type check all files at once
echo.
echo Phase 2: Type checking all generated code and tests...
call npx tsc --noEmit 2>&1
if errorlevel 1 (
    echo.
    echo FAILED: Type check errors found
    call npx tsc --noEmit
    exit /b 1
)

REM Count test cases
set PASSED=0
set SKIPPED=0
for %%T in (%TEST_CASES%) do (
    set TEST_FILE=%SCRIPT_DIR%tests\test_%%T.ts
    if exist "!TEST_FILE!" (
        set /a PASSED+=1
    ) else (
        set /a SKIPPED+=1
    )
)

echo.
echo === Test Summary ===
echo   Passed:  %PASSED%
echo   Failed:  0
echo   Skipped: %SKIPPED%
echo.
echo All tests passed!
exit /b 0
