@echo off
setlocal enabledelayedexpansion
REM PolyGen Redis Integration Test Runner for Windows

set SCRIPT_DIR=%~dp0
set PROJECT_ROOT=%SCRIPT_DIR%..\..\..
set INTEGRATION_DIR=%PROJECT_ROOT%\tests\integration
set OUTPUT_DIR=%SCRIPT_DIR%output

echo === PolyGen Redis Integration Tests ===
echo.

where python >nul 2>&1
if errorlevel 1 (
    echo Error: python is not installed
    exit /b 1
)

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

set TEST_CASES=01_basic_types 02_imports 03_nested_namespaces 04_inline_enums 05_embedded_structs 06_arrays_and_optionals 07_indexes 08_complex_schema 09_sqlite 10_pack_embed

if exist "%OUTPUT_DIR%" rmdir /s /q "%OUTPUT_DIR%"
mkdir "%OUTPUT_DIR%"

set PASSED=0
set FAILED=0

for %%T in (%TEST_CASES%) do (
    echo.
    echo --- Test Case: %%T ---

    set SCHEMA_PATH=%INTEGRATION_DIR%\%%T\schema.poly
    set TEST_OUTPUT=%OUTPUT_DIR%\%%T

    if not exist "!SCHEMA_PATH!" (
        echo   FAILED ^(schema file not found^)
        set /a FAILED+=1
    ) else (
        call :run_case "%%T" "!SCHEMA_PATH!" "!TEST_OUTPUT!"
    )
)

echo.
echo --- Test Case: redis_cache_schema ---
set REDIS_FIXTURE=%PROJECT_ROOT%\tests\test_data\redis_cache_schema.poly
if not exist "%REDIS_FIXTURE%" (
    echo   FAILED ^(redis cache fixture not found^)
    set /a FAILED+=1
) else (
    call :run_case "redis_cache_schema" "%REDIS_FIXTURE%" "%OUTPUT_DIR%\redis_cache_schema"
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

:run_case
set CASE_NAME=%~1
set SCHEMA_PATH=%~2
set TEST_OUTPUT=%~3
set GENERATION_LOG=!OUTPUT_DIR!\!CASE_NAME!_polygen_generation.log
set VALIDATION_LOG=!TEST_OUTPUT!\redis_validation.log

echo   Generating Redis schema...
"!POLYGEN!" --schema-path "!SCHEMA_PATH!" --lang redis --output-dir "!TEST_OUTPUT!" --templates-dir "!PROJECT_ROOT!\templates" > "!GENERATION_LOG!" 2>&1
if errorlevel 1 (
    echo   FAILED ^(generation error^)
    type "!GENERATION_LOG!"
    set /a FAILED+=1
    exit /b 0
)

if not exist "!TEST_OUTPUT!\redis\schema.redis.json" (
    echo   FAILED ^(Redis descriptor file not found^)
    set /a FAILED+=1
    exit /b 0
)

if not exist "!TEST_OUTPUT!\redis\schema.redis.lua" (
    echo   FAILED ^(Redis Lua helper file not found^)
    set /a FAILED+=1
    exit /b 0
)

echo   Validating Redis descriptor and Lua helper...
python "!SCRIPT_DIR!validate_redis.py" "!TEST_OUTPUT!\redis\schema.redis.json" "!TEST_OUTPUT!\redis\schema.redis.lua" > "!VALIDATION_LOG!" 2>&1
if errorlevel 1 (
    echo   FAILED ^(Redis validation error^)
    type "!VALIDATION_LOG!"
    set /a FAILED+=1
) else (
    echo   PASSED
    set /a PASSED+=1
)
exit /b 0
