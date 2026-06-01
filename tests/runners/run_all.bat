@echo off
setlocal enabledelayedexpansion

set "SCRIPT_DIR=%~dp0"
set "PROJECT_ROOT=%SCRIPT_DIR%..\.."
set "DEFAULT_RUNNERS=csharp cpp rust typescript go sqlite mysql postgresql mermaid redis python messagepack protobuf kotlin swift unreal"

if /i "%~1"=="--help" goto :usage
if /i "%~1"=="-h" goto :usage
if /i "%~1"=="/?" goto :usage
if /i "%~1"=="--list" goto :list
if /i "%~1"=="--verify" goto :verify

set PASSED=0
set FAILED=0

echo === PolyGen All Integration Runners ===
echo.

if "%~1"=="" (
    set "RUNNERS="
    goto :collect_defaults
) else (
    set "RUNNERS="
    goto :collect_args
)
goto :run_runners

:collect_defaults
for %%R in (%DEFAULT_RUNNERS%) do (
    set "RUNNER_ARG=%%R"
    call :append_runner_arg
)
goto :run_runners

:collect_args
if "%~1"=="" goto :run_runners
set "RUNNER_ARG=%~1"
call :append_runner_arg
shift
goto :collect_args

:append_runner_arg
call :validate_runner_arg
if errorlevel 1 (
    echo === Runner: ^<invalid^> ===
    echo   FAILED ^(invalid runner name^)
    set /a FAILED+=1
    echo.
) else (
    if defined RUNNERS (
        set "RUNNERS=!RUNNERS! !RUNNER_ARG!"
    ) else (
        set "RUNNERS=!RUNNER_ARG!"
    )
)
exit /b 0

:run_runners
for %%R in (%RUNNERS%) do (
    set "RUNNER=%%R"
    set "RUNNER_SCRIPT=%SCRIPT_DIR%!RUNNER!\run_tests.bat"
    echo === Runner: !RUNNER! ===

    if not exist "!RUNNER_SCRIPT!" (
        echo   FAILED ^(runner script not found^)
        set /a FAILED+=1
    ) else (
        pushd "%PROJECT_ROOT%" >nul
        call "!RUNNER_SCRIPT!"
        set "RUNNER_EXIT=!ERRORLEVEL!"
        popd >nul
        if !RUNNER_EXIT! neq 0 (
            echo   FAILED ^(runner !RUNNER! failed^)
            set /a FAILED+=1
        ) else (
            echo   PASSED
            set /a PASSED+=1
        )
    )

    echo.
)

echo === All Runner Summary ===
echo   Passed:  %PASSED%
echo   Failed:  %FAILED%
echo.

if %FAILED% gtr 0 (
    exit /b 1
)

exit /b 0

:validate_runner_arg
if not defined RUNNER_ARG exit /b 1
set "POLYGEN_RUNNER_ARG=!RUNNER_ARG!"
powershell -NoProfile -ExecutionPolicy Bypass -Command "if ($env:POLYGEN_RUNNER_ARG -match '^[a-z0-9_-]+$') { exit 0 } exit 1" >nul 2>nul
set "VALIDATION_EXIT=!ERRORLEVEL!"
set "POLYGEN_RUNNER_ARG="
exit /b !VALIDATION_EXIT!

:usage
echo Usage:
echo   tests\runners\run_all.bat
echo   tests\runners\run_all.bat sqlite rust
echo   tests\runners\run_all.bat --list
echo   tests\runners\run_all.bat --verify
echo   tests\runners\run_all.bat --help
echo.
echo Runs all integration runners, or only the runner names passed as arguments.
echo --verify checks runner matrix synchronization and verifier regression tests.
exit /b 0

:list
echo %DEFAULT_RUNNERS%
exit /b 0

:verify
echo === Verifying runner matrix ===
where python >nul 2>nul
if errorlevel 1 (
    where py >nul 2>nul
    if errorlevel 1 (
        goto :python_not_found
    )
    set "PYTHON_BIN=py -3"
) else (
    set "PYTHON_BIN=python"
)
set "PYTHONDONTWRITEBYTECODE=1"
%PYTHON_BIN% "%SCRIPT_DIR%verify_runner_matrix.py"
set "VERIFY_EXIT=%ERRORLEVEL%"
if %VERIFY_EXIT% neq 0 (
    exit /b %VERIFY_EXIT%
)
echo.
echo === Verifying runner matrix regression tests ===
%PYTHON_BIN% "%SCRIPT_DIR%test_verify_runner_matrix.py"
exit /b %ERRORLEVEL%

:python_not_found
echo FAILED ^(python not found^)
exit /b 1
