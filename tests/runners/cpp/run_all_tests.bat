@echo off
setlocal enabledelayedexpansion

set LOG=%~dp0test_results.log
echo === PolyGen C++ Tests === > "%LOG%"
echo. >> "%LOG%"

REM Setup MSVC environment
echo Setting up MSVC environment... >> "%LOG%"
if exist "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" (
    call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" >nul
    echo Using VS2022 Community >> "%LOG%"
) else if exist "C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat" (
    call "C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat" >nul
    echo Using VS18 Community >> "%LOG%"
)

where cl >nul 2>&1
if errorlevel 1 (
    echo ERROR: cl.exe not found after environment setup >> "%LOG%"
    exit /b 1
)

cl 2>&1 | findstr /C:"Microsoft" >> "%LOG%"

set SCRIPT_DIR=%~dp0
set PASSED=0
set FAILED=0

for %%T in (01_basic_types 02_imports 03_nested_namespaces 04_inline_enums 05_embedded_structs 06_arrays_and_optionals 07_indexes 08_complex_schema) do (
    echo. >> "%LOG%"
    echo === Test: %%T === >> "%LOG%"

    set OUTPUT_DIR=%SCRIPT_DIR%generated\%%T
    set TEST_FILE=%SCRIPT_DIR%tests\test_%%T.cpp
    set BINARY=!OUTPUT_DIR!\test_%%T.exe

    if not exist "!TEST_FILE!" (
        echo   SKIP: Test file not found >> "%LOG%"
    ) else (
        echo   Compiling... >> "%LOG%"
        cl /std:c++17 /EHsc /O2 /I"!OUTPUT_DIR!\cpp" "!TEST_FILE!" /Fe:"!BINARY!" /nologo >> "%LOG%" 2>&1
        if errorlevel 1 (
            echo   FAILED: Compilation >> "%LOG%"
            set /a FAILED+=1
        ) else (
            echo   Running... >> "%LOG%"
            "!BINARY!" >> "%LOG%" 2>&1
            if errorlevel 1 (
                echo   FAILED: Runtime >> "%LOG%"
                set /a FAILED+=1
            ) else (
                echo   PASSED >> "%LOG%"
                set /a PASSED+=1
            )
        )
    )
)

echo. >> "%LOG%"
echo === Summary === >> "%LOG%"
echo   Passed: %PASSED% >> "%LOG%"
echo   Failed: %FAILED% >> "%LOG%"

if %FAILED% gtr 0 exit /b 1
exit /b 0
