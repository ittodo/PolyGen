@echo off
setlocal enabledelayedexpansion
REM PolyGen C++ Integration Test Runner for Windows

set SCRIPT_DIR=%~dp0
set PROJECT_ROOT=%SCRIPT_DIR%..\..\..
set INTEGRATION_DIR=%PROJECT_ROOT%\tests\integration
set GENERATED_DIR=%SCRIPT_DIR%generated
set STATIC_DIR=%PROJECT_ROOT%\static\cpp

echo === PolyGen C++ Integration Tests ===
echo.

REM Check for cl (MSVC) first, then g++
where cl >nul 2>&1
if errorlevel 1 (
    REM Try to setup MSVC environment
    if exist "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" (
        echo Setting up MSVC 2022 Community environment...
        call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" >nul
    ) else if exist "C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat" (
        echo Setting up MSVC 18 Community environment...
        call "C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat" >nul
    ) else if exist "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat" (
        echo Setting up MSVC 2022 Professional environment...
        call "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat" >nul
    ) else if exist "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvars64.bat" (
        echo Setting up MSVC 2022 Enterprise environment...
        call "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvars64.bat" >nul
    ) else if exist "C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\VC\Auxiliary\Build\vcvars64.bat" (
        echo Setting up MSVC 2019 Community environment...
        call "C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\VC\Auxiliary\Build\vcvars64.bat" >nul
    )

    where cl >nul 2>&1
    if errorlevel 1 (
        where g++ >nul 2>&1
        if errorlevel 1 (
            echo Error: Neither cl ^(MSVC^) nor g++ found
            echo Please run from Developer Command Prompt or install Visual Studio
            exit /b 1
        )
        set COMPILER=g++
    ) else (
        set COMPILER=cl
    )
) else (
    set COMPILER=cl
)
echo Using compiler: %COMPILER%

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
set SKIPPED=0

for %%T in (%TEST_CASES%) do (
    echo.
    echo --- Test Case: %%T ---

    set TEST_DIR=%INTEGRATION_DIR%\%%T
    set OUTPUT_DIR=%GENERATED_DIR%\%%T

    if not exist "!TEST_DIR!" (
        echo   Skipped: Test directory not found
        set /a SKIPPED+=1
    ) else (
        REM Clean and create output directory
        if exist "!OUTPUT_DIR!" rmdir /s /q "!OUTPUT_DIR!"
        mkdir "!OUTPUT_DIR!\cpp"

        REM Generate code
        echo   Generating C++ code...
        for %%S in ("!TEST_DIR!\*.poly") do (
            echo     - %%~nxS
            "%POLYGEN%" --schema-path "%%S" --lang cpp --output-dir "!OUTPUT_DIR!" --templates-dir "%PROJECT_ROOT%\templates"
        )

        REM Copy polygen_support.hpp
        if exist "%STATIC_DIR%\polygen_support.hpp" (
            copy "%STATIC_DIR%\polygen_support.hpp" "!OUTPUT_DIR!\cpp\" >nul
        )

        REM Check if test file exists
        set TEST_FILE=%SCRIPT_DIR%tests\test_%%T.cpp
        if not exist "!TEST_FILE!" (
            echo   Skipped: Test file not found
            set /a SKIPPED+=1
        ) else (
            REM Compile
            echo   Compiling...
            set BINARY=!OUTPUT_DIR!\test_%%T.exe

            if "%COMPILER%"=="g++" (
                g++ -std=c++17 -Wall -Wextra -O2 -I"!OUTPUT_DIR!\cpp" "!TEST_FILE!" -o "!BINARY!" 2>nul
            ) else (
                cl /std:c++17 /EHsc /O2 /I"!OUTPUT_DIR!\cpp" "!TEST_FILE!" /Fe:"!BINARY!" /nologo 2>nul
            )

            if errorlevel 1 (
                echo   FAILED ^(compilation error^)
                set /a FAILED+=1
            ) else (
                REM Run test
                echo   Running...
                "!BINARY!"
                if errorlevel 1 (
                    echo   FAILED ^(runtime error^)
                    set /a FAILED+=1
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
echo   Skipped: %SKIPPED%
echo.

if %FAILED% gtr 0 (
    echo Some tests failed!
    exit /b 1
) else (
    echo All tests passed!
    exit /b 0
)
