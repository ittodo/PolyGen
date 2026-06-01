@echo off
setlocal enabledelayedexpansion
REM PolyGen Unreal Integration Test Runner for Windows

set SCRIPT_DIR=%~dp0
set PROJECT_ROOT=%SCRIPT_DIR%..\..\..
set INTEGRATION_DIR=%PROJECT_ROOT%\tests\integration
set OUTPUT_DIR=%SCRIPT_DIR%output

echo === PolyGen Unreal Integration Tests ===
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
        set GENERATION_LOG=!OUTPUT_DIR!\%%T_polygen_generation.log
        set VALIDATION_LOG=!TEST_OUTPUT!\unreal_validation.log
        echo   Generating Unreal headers...
        "%POLYGEN%" --schema-path "!SCHEMA_PATH!" --lang unreal --output-dir "!TEST_OUTPUT!" --templates-dir "%PROJECT_ROOT%\templates" > "!GENERATION_LOG!" 2>&1
        if errorlevel 1 (
            echo   FAILED ^(generation error^)
            type "!GENERATION_LOG!"
            set /a FAILED+=1
        ) else if not exist "!TEST_OUTPUT!\unreal" (
            echo   FAILED ^(no unreal output directory^)
            set /a FAILED+=1
        ) else if not exist "!TEST_OUTPUT!\unreal\*.h" (
            echo   FAILED ^(no unreal headers generated^)
            set /a FAILED+=1
        ) else (
            echo   Validating Unreal header structure...
            set CASE_FAILED=0
            if exist "!VALIDATION_LOG!" del /q "!VALIDATION_LOG!" >nul 2>&1
            for %%F in ("!TEST_OUTPUT!\unreal\*.h") do (
                python "%SCRIPT_DIR%validate_unreal.py" "%%~fF" >> "!VALIDATION_LOG!" 2>&1
                if errorlevel 1 set CASE_FAILED=1
            )

            if !CASE_FAILED! neq 0 (
                echo   FAILED ^(Unreal validation error^)
                type "!VALIDATION_LOG!"
                set /a FAILED+=1
            ) else (
                echo   PASSED
                set /a PASSED+=1
            )
        )
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
