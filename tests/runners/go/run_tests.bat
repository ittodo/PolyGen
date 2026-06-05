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
set TEST_CASES=01_basic_types 02_imports 03_nested_namespaces 04_inline_enums 05_embedded_structs 06_arrays_and_optionals 07_indexes 08_complex_schema 09_sqlite 10_pack_embed 11_relations_indexes

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
        echo   FAILED ^(schema file not found^)
        set /a FAILED+=1
    ) else (
        call :run_case "%%T" "!SCHEMA_PATH!" "!TEST_OUTPUT!"
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

:run_case
set CASE_NAME=%~1
set SCHEMA_PATH=%~2
set TEST_OUTPUT=%~3

echo   Generating Go code...
"%POLYGEN%" --schema-path "%SCHEMA_PATH%" --lang go --output-dir "%TEST_OUTPUT%" --templates-dir "%PROJECT_ROOT%\templates"
if errorlevel 1 (
    echo   FAILED ^(generation error^)
    set /a FAILED+=1
    exit /b 0
)

if not exist "!TEST_OUTPUT!\go" (
    echo   FAILED ^(no go output directory^)
    set /a FAILED+=1
    exit /b 0
)

if not exist "!TEST_OUTPUT!\go\*.go" (
    echo   FAILED ^(no go files generated^)
    set /a FAILED+=1
    exit /b 0
)

cd /d "!TEST_OUTPUT!\go"

if not exist "go.mod" (
    set "MOD_LOG=!TEST_OUTPUT!\go_mod_init.log"
    go mod init generated_!CASE_NAME! > "!MOD_LOG!" 2>&1
    if errorlevel 1 (
        echo   FAILED ^(go mod init error^)
        type "!MOD_LOG!"
        set /a FAILED+=1
        cd /d "!PROJECT_ROOT!"
        exit /b 0
    )
)

if exist "!SCRIPT_DIR!tests\!CASE_NAME!_test.go" (
    copy /Y "!SCRIPT_DIR!tests\!CASE_NAME!_test.go" "polygen_integration_test.go" >nul
    if errorlevel 1 (
        echo   FAILED ^(could not copy smoke test^)
        set /a FAILED+=1
        cd /d "!PROJECT_ROOT!"
        exit /b 0
    )
)

if "%CASE_NAME%"=="09_sqlite" (
    set "GET_LOG=!TEST_OUTPUT!\go_get_sqlite.log"
    go get modernc.org/sqlite@v1.51.0 > "!GET_LOG!" 2>&1
    if errorlevel 1 (
        echo   FAILED ^(go get sqlite driver error^)
        type "!GET_LOG!"
        set /a FAILED+=1
        cd /d "!PROJECT_ROOT!"
        exit /b 0
    )
)

echo   Testing...
set "TEST_LOG=!TEST_OUTPUT!\go_test.log"
go test ./... > "!TEST_LOG!" 2>&1
if errorlevel 1 (
    echo   FAILED ^(go test error^)
    type "!TEST_LOG!"
    set /a FAILED+=1
) else (
    echo   PASSED
    set /a PASSED+=1
)

cd /d "!PROJECT_ROOT!"
exit /b 0
