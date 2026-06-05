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
where npm >nul 2>&1
if errorlevel 1 (
    echo Error: npm is not installed
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
    npm install > "%SCRIPT_DIR%npm_install.log" 2>&1
    if errorlevel 1 (
        echo Error: npm install failed
        type "%SCRIPT_DIR%npm_install.log"
        exit /b 1
    )
)
if not exist "node_modules\.bin\tsx.cmd" (
    echo Updating dependencies...
    npm install > "%SCRIPT_DIR%npm_install.log" 2>&1
    if errorlevel 1 (
        echo Error: npm install failed
        type "%SCRIPT_DIR%npm_install.log"
        exit /b 1
    )
)

REM Test cases
set TEST_CASES=01_basic_types 02_imports 03_nested_namespaces 04_inline_enums 05_embedded_structs 06_arrays_and_optionals 07_indexes 08_complex_schema 09_sqlite 10_pack_embed 11_relations_indexes

set GENERATED=0
set FAILED=0

REM Phase 1: Generate all code
echo.
echo Phase 1: Generating TypeScript code for all test cases...
if not exist "%GENERATED_DIR%" mkdir "%GENERATED_DIR%"
for %%T in (%TEST_CASES%) do (
    set TEST_DIR=%INTEGRATION_DIR%\%%T
    set OUTPUT_DIR=%GENERATED_DIR%\%%T
    set GENERATION_LOG=%GENERATED_DIR%\%%T_polygen_generation.log

    if not exist "!TEST_DIR!" (
        echo   FAILED %%T ^(test directory not found^)
        set /a FAILED+=1
    ) else if not exist "!TEST_DIR!\*.poly" (
        echo   FAILED %%T ^(schema file not found^)
        set /a FAILED+=1
    ) else (
        REM Clean and create output directory
        if exist "!OUTPUT_DIR!" rmdir /s /q "!OUTPUT_DIR!"
        mkdir "!OUTPUT_DIR!\typescript"

        REM Generate code
        echo   Generating %%T...
        cd /d "%PROJECT_ROOT%"
        set CASE_FAILED=0
        for %%S in ("!TEST_DIR!\*.poly") do (
            "%POLYGEN%" --schema-path "%%S" --lang typescript --output-dir "!OUTPUT_DIR!" --templates-dir "%PROJECT_ROOT%\templates" > "!GENERATION_LOG!" 2>&1
            if errorlevel 1 (
                echo   FAILED %%T ^(generation error: %%~nxS^)
                type "!GENERATION_LOG!"
                set /a FAILED+=1
                set CASE_FAILED=1
            )
        )
        if not exist "!OUTPUT_DIR!\typescript\*.ts" (
            echo   FAILED %%T ^(no typescript files generated^)
            set /a FAILED+=1
            set CASE_FAILED=1
        )
        if !CASE_FAILED! equ 0 (
            set /a GENERATED+=1
        )
    )
)
cd /d "%SCRIPT_DIR%"
echo   Generated %GENERATED% test cases.

if %FAILED% gtr 0 (
    echo.
    echo FAILED: Generation phase had %FAILED% error^(s^)
    exit /b 1
)

REM Phase 2: Type check all files at once
echo.
echo Phase 2: Type checking all generated code and tests...
set TYPECHECK_LOG=%GENERATED_DIR%\typescript_typecheck.log
call npx tsc --noEmit > "%TYPECHECK_LOG%" 2>&1
if errorlevel 1 (
    echo.
    echo FAILED: Type check errors found
    type "%TYPECHECK_LOG%"
    exit /b 1
)

REM Phase 3: Execute runtime tests
echo.
echo Phase 3: Running TypeScript runtime tests...
set RUNTIME_LOG=%GENERATED_DIR%\typescript_runtime.log
call npx tsx tests/run_all.ts > "%RUNTIME_LOG%" 2>&1
if errorlevel 1 (
    echo.
    echo FAILED: Runtime test errors found
    type "%RUNTIME_LOG%"
    exit /b 1
)

REM Count test cases
set PASSED=0
for %%T in (%TEST_CASES%) do (
    set TEST_FILE=%SCRIPT_DIR%tests\test_%%T.ts
    if exist "!TEST_FILE!" (
        set /a PASSED+=1
    ) else (
        echo   FAILED %%T ^(test file not found^)
        set /a FAILED+=1
    )
)

echo.
echo === Test Summary ===
echo   Passed:  %PASSED%
echo   Failed:  %FAILED%
echo.
if %FAILED% gtr 0 (
    echo Some tests failed!
    exit /b 1
) else (
    echo All tests passed!
    exit /b 0
)
