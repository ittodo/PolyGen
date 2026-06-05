@echo off
setlocal enabledelayedexpansion
REM PolyGen Kotlin Integration Test Runner for Windows

set SCRIPT_DIR=%~dp0
set PROJECT_ROOT=%SCRIPT_DIR%..\..\..
set INTEGRATION_DIR=%PROJECT_ROOT%\tests\integration
set OUTPUT_DIR=%SCRIPT_DIR%output

echo === PolyGen Kotlin Integration Tests ===
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

set TEST_CASES=01_basic_types 02_imports 03_nested_namespaces 04_inline_enums 05_embedded_structs 06_arrays_and_optionals 07_indexes 08_complex_schema 09_sqlite 10_pack_embed 11_relations_indexes

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
        set VALIDATION_LOG=!TEST_OUTPUT!\kotlin_validation.log
        echo   Generating Kotlin code...
        "%POLYGEN%" --schema-path "!SCHEMA_PATH!" --lang kotlin --output-dir "!TEST_OUTPUT!" --templates-dir "%PROJECT_ROOT%\templates" > "!GENERATION_LOG!" 2>&1
        if errorlevel 1 (
            echo   FAILED ^(generation error^)
            type "!GENERATION_LOG!"
            set /a FAILED+=1
        ) else if not exist "!TEST_OUTPUT!\kotlin" (
            echo   FAILED ^(no kotlin output directory^)
            set /a FAILED+=1
        ) else if not exist "!TEST_OUTPUT!\kotlin\*.kt" (
            echo   FAILED ^(no kotlin files generated^)
            set /a FAILED+=1
        ) else (
            echo   Validating Kotlin structure...
            set CASE_FAILED=0
            if exist "!VALIDATION_LOG!" del /q "!VALIDATION_LOG!" >nul 2>&1
            for %%F in ("!TEST_OUTPUT!\kotlin\*.kt") do (
                python "%SCRIPT_DIR%validate_kotlin.py" "%%~fF" >> "!VALIDATION_LOG!" 2>&1
                if errorlevel 1 set CASE_FAILED=1
            )

            if !CASE_FAILED! neq 0 (
                echo   FAILED ^(Kotlin validation error^)
                type "!VALIDATION_LOG!"
                set /a FAILED+=1
            ) else (
                if "%POLYGEN_KOTLIN_RUNTIME%"=="1" (
                    set RUNTIME_LOG=!TEST_OUTPUT!\kotlin_runtime.log
                    echo   Running Kotlin runtime assertions...
                    python "%SCRIPT_DIR%run_kotlin_runtime.py" "%%T" "!TEST_OUTPUT!\kotlin\*.kt" >> "!RUNTIME_LOG!" 2>&1
                    if errorlevel 1 (
                        echo   FAILED ^(Kotlin runtime assertion error^)
                        type "!RUNTIME_LOG!"
                        set /a FAILED+=1
                    ) else (
                        echo   PASSED
                        set /a PASSED+=1
                    )
                ) else if "%POLYGEN_KOTLIN_COMPILE%"=="1" (
                    set COMPILE_LOG=!TEST_OUTPUT!\kotlin_compile.log
                    echo   Compiling Kotlin...
                    python "%SCRIPT_DIR%compile_kotlin.py" "!TEST_OUTPUT!\kotlin\*.kt" >> "!COMPILE_LOG!" 2>&1
                    if errorlevel 1 (
                        echo   FAILED ^(Kotlin compile error^)
                        type "!COMPILE_LOG!"
                        set /a FAILED+=1
                    ) else (
                        echo   PASSED
                        set /a PASSED+=1
                    )
                ) else (
                    echo   PASSED
                    set /a PASSED+=1
                )
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
