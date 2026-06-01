@echo off
setlocal enabledelayedexpansion
REM PolyGen Rust Integration Test Runner for Windows

set SCRIPT_DIR=%~dp0
set PROJECT_ROOT=%SCRIPT_DIR%..\..\..
set INTEGRATION_DIR=%PROJECT_ROOT%\tests\integration
set GENERATED_DIR=%SCRIPT_DIR%generated

echo === PolyGen Rust Integration Tests ===
echo.

REM Check for cargo
where cargo >nul 2>&1
if errorlevel 1 (
    echo Error: cargo is not installed
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
set TEST_CASES=01_basic_types 02_imports 03_nested_namespaces 04_inline_enums 05_embedded_structs 06_arrays_and_optionals 07_indexes 08_complex_schema 09_sqlite 10_pack_embed

REM Create generated directory
if not exist "%GENERATED_DIR%" mkdir "%GENERATED_DIR%"

set PASSED=0
set FAILED=0

for %%T in (%TEST_CASES%) do (
    echo.
    echo --- Test Case: %%T ---

    set TEST_DIR=%INTEGRATION_DIR%\%%T
    set OUTPUT_DIR=%GENERATED_DIR%\%%T

    if not exist "!TEST_DIR!" (
        echo   FAILED ^(test directory not found^)
        set /a FAILED+=1
    ) else if not exist "!TEST_DIR!\*.poly" (
        echo   FAILED ^(schema file not found^)
        set /a FAILED+=1
    ) else (
        REM Clean and create output directory
        if exist "!OUTPUT_DIR!" rmdir /s /q "!OUTPUT_DIR!"
        mkdir "!OUTPUT_DIR!\rust"

        REM Generate code
        echo   Generating Rust code...
        set CASE_FAILED=0
        for %%S in ("!TEST_DIR!\*.poly") do (
            echo     - %%~nxS
            "%POLYGEN%" --schema-path "%%S" --lang rust --output-dir "!OUTPUT_DIR!" --templates-dir "%PROJECT_ROOT%\templates"
            if errorlevel 1 set CASE_FAILED=1
        )

        REM Check if test file exists
        set TEST_FILE=%SCRIPT_DIR%tests\test_%%T.rs
        if !CASE_FAILED! neq 0 (
            echo   FAILED ^(generation error^)
            set /a FAILED+=1
        ) else if not exist "!OUTPUT_DIR!\rust\*.rs" (
            echo   FAILED ^(no Rust files generated^)
            set /a FAILED+=1
        ) else if not exist "!TEST_FILE!" (
            echo   FAILED ^(test file not found^)
            set /a FAILED+=1
        ) else (
            REM Create Cargo project
            set TEST_PROJECT_DIR=!OUTPUT_DIR!\rust
            mkdir "!TEST_PROJECT_DIR!\src" 2>nul

            REM Create Cargo.toml - check if SQLite test case
            if "%%T"=="09_sqlite" (
                (
                    echo [package]
                    echo name = "polygen_test"
                    echo version = "0.1.0"
                    echo edition = "2021"
                    echo.
                    echo [dependencies]
                    echo serde = { version = "1.0", features = ["derive"] }
                    echo serde_json = "1.0"
                    echo byteorder = "1.5"
                    echo chrono = "0.4"
                    echo regex = "1"
                    echo rusqlite = { version = "0.31", features = ["bundled"] }
                ) > "!TEST_PROJECT_DIR!\Cargo.toml"
            ) else (
                (
                    echo [package]
                    echo name = "polygen_test"
                    echo version = "0.1.0"
                    echo edition = "2021"
                    echo.
                    echo [dependencies]
                    echo serde = { version = "1.0", features = ["derive"] }
                    echo serde_json = "1.0"
                    echo byteorder = "1.5"
                    echo chrono = "0.4"
                    echo regex = "1"
                ) > "!TEST_PROJECT_DIR!\Cargo.toml"
            )

            REM Copy generated files to src/
            copy "!OUTPUT_DIR!\rust\*.rs" "!TEST_PROJECT_DIR!\src\" >nul 2>&1
            if errorlevel 1 (
                echo   FAILED ^(could not copy generated Rust files^)
                set /a FAILED+=1
            ) else (

                REM Create lib.rs - check if SQLite test case
                if "%%T"=="09_sqlite" (
                    (
                        echo pub mod polygen_support;
                        echo pub mod schema;
                        echo pub mod schema_loaders;
                        echo pub mod schema_container;
                        echo pub mod schema_redis_keys;
                        echo pub mod schema_sqlite_accessor;
                    ) > "!TEST_PROJECT_DIR!\src\lib.rs"
                ) else (
                    (
                        echo pub mod polygen_support;
                        echo pub mod schema;
                        echo pub mod schema_loaders;
                        echo pub mod schema_container;
                        echo pub mod schema_redis_keys;
                    ) > "!TEST_PROJECT_DIR!\src\lib.rs"
                )

                REM Copy test file as main.rs
                copy "!TEST_FILE!" "!TEST_PROJECT_DIR!\src\main.rs" >nul
                if errorlevel 1 (
                    echo   FAILED ^(could not copy test file^)
                    set /a FAILED+=1
                ) else (

                    REM Compile and run
                    echo   Compiling...
                    cd /d "!TEST_PROJECT_DIR!"
                    set BUILD_LOG=!OUTPUT_DIR!\cargo_build.log
                    cargo build --release > "!BUILD_LOG!" 2>&1
                    if errorlevel 1 (
                        echo   FAILED ^(compilation error^)
                        type "!BUILD_LOG!"
                        set /a FAILED+=1
                    ) else (
                        echo   Running...
                        set RUN_LOG=!OUTPUT_DIR!\cargo_run.log
                        cargo run --release > "!RUN_LOG!" 2>&1
                        if errorlevel 1 (
                            type "!RUN_LOG!"
                            echo   FAILED ^(runtime error^)
                            set /a FAILED+=1
                        ) else (
                            type "!RUN_LOG!"
                            echo   PASSED
                            set /a PASSED+=1
                        )
                    )
                    cd /d "%PROJECT_ROOT%"
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
