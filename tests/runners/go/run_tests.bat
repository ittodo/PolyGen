@echo off
setlocal enabledelayedexpansion
REM PolyGen Go Integration Test Runner for Windows

set SCRIPT_DIR=%~dp0
set PROJECT_ROOT=%SCRIPT_DIR%..\..\..
set INTEGRATION_DIR=%PROJECT_ROOT%\tests\integration
set OUTPUT_DIR=%SCRIPT_DIR%output

echo === PolyGen Go Integration Tests ===
echo.

REM Check for go
where go >nul 2>&1
if errorlevel 1 (
    echo Error: go is not installed
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

REM Test cases
set TEST_CASES=01_basic_types 02_imports 03_nested_namespaces 04_inline_enums 05_embedded_structs 06_arrays_and_optionals 07_indexes 08_complex_schema

REM Create output directory
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
        echo   Skipped: Schema file not found
    ) else (
        REM Generate Go code
        echo   Generating Go code...
        "%POLYGEN%" --schema-path "!SCHEMA_PATH!" --lang go --output-dir "!TEST_OUTPUT!" --templates-dir "%PROJECT_ROOT%\templates"

        REM Check if go directory exists
        if exist "!TEST_OUTPUT!\go" (
            cd /d "!TEST_OUTPUT!\go"

            REM Initialize Go module
            if not exist "go.mod" (
                go mod init generated_%%T >nul 2>&1
            )

            REM Try to build
            echo   Compiling...
            go build . 2>nul
            if errorlevel 1 (
                echo   FAILED ^(compilation error^)
                go build . 2>&1
                set /a FAILED+=1
            ) else (
                echo   PASSED
                set /a PASSED+=1
            )

            cd /d "%PROJECT_ROOT%"
        ) else (
            echo   FAILED ^(no go output directory^)
            set /a FAILED+=1
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
