@echo off
setlocal enabledelayedexpansion

if "%~1"=="" (
    echo Usage: compile_test.bat ^<test_case^>
    exit /b 1
)

set TEST_CASE=%1
set SCRIPT_DIR=%~dp0
set OUTPUT_DIR=%SCRIPT_DIR%generated\%TEST_CASE%
set TEST_FILE=%SCRIPT_DIR%tests\test_%TEST_CASE%.cpp
set BINARY=%OUTPUT_DIR%\test_%TEST_CASE%.exe
set COMPILE_LOG=%OUTPUT_DIR%\cpp_compile.log
set REDIS_SMOKE=

where cl >nul 2>&1
if errorlevel 1 (
    if exist "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" (
        call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" >nul
    ) else if exist "C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat" (
        call "C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat" >nul
    ) else if exist "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat" (
        call "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat" >nul
    ) else if exist "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvars64.bat" (
        call "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvars64.bat" >nul
    ) else if exist "C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\VC\Auxiliary\Build\vcvars64.bat" (
        call "C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\VC\Auxiliary\Build\vcvars64.bat" >nul
    )
)

where cl >nul 2>&1
if errorlevel 1 (
    where g++ >nul 2>&1
    if errorlevel 1 (
        echo Error: Neither cl ^(MSVC^) nor g++ found
        exit /b 1
    )
    set COMPILER=g++
) else (
    set COMPILER=cl
)

if not exist "%OUTPUT_DIR%\cpp" (
    echo FAILED: Generated C++ directory not found
    exit /b 1
)

if not exist "%OUTPUT_DIR%\cpp\*.hpp" (
    echo FAILED: Generated C++ headers not found
    exit /b 1
)

if not exist "%TEST_FILE%" (
    echo FAILED: Test file not found: %TEST_FILE%
    exit /b 1
)

if exist "%OUTPUT_DIR%\cpp\schema_redis_keys.hpp" (
    set REDIS_SMOKE=%OUTPUT_DIR%\redis_keys_smoke.cpp
    > "!REDIS_SMOKE!" echo #include "schema_redis_keys.hpp"
    >> "!REDIS_SMOKE!" echo int polygen_cpp_redis_keys_smoke^(^) { return 0; }
)

echo Compiling %TEST_CASE%...
if "%COMPILER%"=="g++" (
    if "!REDIS_SMOKE!"=="" (
        g++ -std=c++17 -Wall -Wextra -O2 -I"%OUTPUT_DIR%\cpp" "%TEST_FILE%" -o "%BINARY%" > "%COMPILE_LOG%" 2>&1
    ) else (
        g++ -std=c++17 -Wall -Wextra -O2 -I"%OUTPUT_DIR%\cpp" "%TEST_FILE%" "!REDIS_SMOKE!" -o "%BINARY%" > "%COMPILE_LOG%" 2>&1
    )
) else (
    if "!REDIS_SMOKE!"=="" (
        cl /std:c++17 /EHsc /O2 /I"%OUTPUT_DIR%\cpp" "%TEST_FILE%" /Fe:"%BINARY%" /nologo > "%COMPILE_LOG%" 2>&1
    ) else (
        cl /std:c++17 /EHsc /O2 /I"%OUTPUT_DIR%\cpp" "%TEST_FILE%" "!REDIS_SMOKE!" /Fe:"%BINARY%" /nologo > "%COMPILE_LOG%" 2>&1
    )
)
if errorlevel 1 (
    echo FAILED: Compilation error
    type "%COMPILE_LOG%"
    exit /b 1
)

echo Running %TEST_CASE%...
"%BINARY%"
if errorlevel 1 (
    echo FAILED: Runtime error
    exit /b 1
)

echo PASSED
exit /b 0
