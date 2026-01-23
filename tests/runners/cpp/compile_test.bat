@echo off
setlocal

REM Setup MSVC environment
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
if errorlevel 1 (
    call "C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
)

set TEST_CASE=%1
set SCRIPT_DIR=%~dp0
set OUTPUT_DIR=%SCRIPT_DIR%generated\%TEST_CASE%
set TEST_FILE=%SCRIPT_DIR%tests\test_%TEST_CASE%.cpp
set BINARY=%OUTPUT_DIR%\test_%TEST_CASE%.exe

echo Compiling %TEST_CASE%...
cl /std:c++17 /EHsc /O2 /I"%OUTPUT_DIR%\cpp" "%TEST_FILE%" /Fe:"%BINARY%" /nologo 2>&1
if errorlevel 1 (
    echo FAILED: Compilation error
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
