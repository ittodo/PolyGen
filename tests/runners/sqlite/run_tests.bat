@echo off
setlocal enabledelayedexpansion

echo ========================================
echo SQLite Integration Test Runner
echo ========================================
echo.
echo This test validates SQLite DDL generation.
echo For accessor compilation tests, see:
echo   - tests/runners/csharp (C# SQLite Accessor)
echo ========================================

set "PROJECT_ROOT=%~dp0..\..\..\"
set "POLYGEN=%PROJECT_ROOT%target\release\polygen.exe"
set "SCHEMA_DIR=%PROJECT_ROOT%tests\integration\09_sqlite"
set "OUTPUT_DIR=%PROJECT_ROOT%output"
set "TEMPLATES_DIR=%PROJECT_ROOT%templates"
set "TEST_OUTPUT=%~dp0generated"

:: Check if polygen exists
if not exist "%POLYGEN%" (
    echo ERROR: polygen.exe not found at %POLYGEN%
    echo Please build the project first: cargo build --release
    exit /b 1
)

:: Clean previous test output
if exist "%TEST_OUTPUT%" (
    echo Cleaning previous test output...
    rmdir /s /q "%TEST_OUTPUT%"
)
mkdir "%TEST_OUTPUT%"

echo.
echo [Step 1] Generating SQLite DDL...
echo ----------------------------------------

pushd "%PROJECT_ROOT%"
"%POLYGEN%" --schema-path "%SCHEMA_DIR%\schema.poly" --lang sqlite --output-dir "%OUTPUT_DIR%" --templates-dir "%TEMPLATES_DIR%" > "%TEST_OUTPUT%\sqlite_gen.log" 2>&1
set GEN_RESULT=%errorlevel%
popd

if %GEN_RESULT% neq 0 (
    echo FAILED: SQLite DDL generation failed
    type "%TEST_OUTPUT%\sqlite_gen.log"
    exit /b 1
)

echo PASSED: DDL generation successful

set "SQL_FILE=%OUTPUT_DIR%\sqlite\schema.sql"
if not exist "%SQL_FILE%" (
    echo FAILED: SQL file not found at %SQL_FILE%
    exit /b 1
)

:: Copy generated files for inspection
copy "%SQL_FILE%" "%TEST_OUTPUT%\schema.sql" > nul
echo Generated: %TEST_OUTPUT%\schema.sql

echo.
echo [Step 2] Validating DDL content...
echo ----------------------------------------

findstr /i "CREATE TABLE" "%SQL_FILE%" > nul
if %errorlevel% neq 0 (
    echo FAILED: No CREATE TABLE statements found
    exit /b 1
)
echo PASSED: CREATE TABLE statements found

findstr /i "test_sqlite_User" "%SQL_FILE%" > nul
if %errorlevel% neq 0 (
    echo FAILED: User table not found
    exit /b 1
)
echo PASSED: User table defined

findstr /i "test_sqlite_Post" "%SQL_FILE%" > nul
if %errorlevel% neq 0 (
    echo FAILED: Post table not found
    exit /b 1
)
echo PASSED: Post table defined

findstr /i "test_sqlite_Comment" "%SQL_FILE%" > nul
if %errorlevel% neq 0 (
    echo FAILED: Comment table not found
    exit /b 1
)
echo PASSED: Comment table defined

findstr /i "PRAGMA foreign_keys" "%SQL_FILE%" > nul
if %errorlevel% neq 0 (
    echo WARNING: PRAGMA foreign_keys not found
) else (
    echo PASSED: PRAGMA foreign_keys enabled
)

echo.
echo [Step 3] Displaying generated DDL...
echo ----------------------------------------
type "%SQL_FILE%"

echo.
echo ========================================
echo SQLite DDL Test PASSED!
echo ========================================
echo.
echo For C# accessor compilation test, run:
echo   cd tests\runners\csharp
echo   run_tests.bat
echo.

exit /b 0
