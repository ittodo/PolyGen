@echo off
setlocal enabledelayedexpansion
REM PolyGen C# Integration Test Runner for Windows

set SCRIPT_DIR=%~dp0
set PROJECT_ROOT=%SCRIPT_DIR%..\..\..
set INTEGRATION_DIR=%PROJECT_ROOT%\tests\integration
set GENERATED_DIR=%SCRIPT_DIR%generated

echo === PolyGen C# Integration Tests ===
echo.

REM Check for dotnet
where dotnet >nul 2>&1
if errorlevel 1 (
    echo Error: dotnet is not installed
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
        mkdir "!OUTPUT_DIR!\csharp"

        REM Generate code
        echo   Generating C# code...
        for %%S in ("!TEST_DIR!\*.poly") do (
            echo     - %%~nxS
            "%POLYGEN%" --schema-path "%%S" --lang csharp --output-dir "!OUTPUT_DIR!" --templates-dir "%PROJECT_ROOT%\templates"
        )

        REM Check if test file exists
        set TEST_FILE=%SCRIPT_DIR%tests\Test_%%T.cs
        if not exist "!TEST_FILE!" (
            echo   Skipped: Test file not found
            set /a SKIPPED+=1
        ) else (
            REM Create test project
            set TEST_PROJECT_DIR=!OUTPUT_DIR!\TestProject
            mkdir "!TEST_PROJECT_DIR!"

            REM Create csproj
            (
                echo ^<Project Sdk="Microsoft.NET.Sdk"^>
                echo   ^<PropertyGroup^>
                echo     ^<OutputType^>Exe^</OutputType^>
                echo     ^<TargetFramework^>net8.0^</TargetFramework^>
                echo     ^<ImplicitUsings^>enable^</ImplicitUsings^>
                echo     ^<Nullable^>enable^</Nullable^>
                echo     ^<TreatWarningsAsErrors^>false^</TreatWarningsAsErrors^>
                echo   ^</PropertyGroup^>
                echo ^</Project^>
            ) > "!TEST_PROJECT_DIR!\TestProject.csproj"

            REM Copy all generated files including Container.cs
            for %%F in ("!OUTPUT_DIR!\csharp\*.cs") do (
                copy "%%F" "!TEST_PROJECT_DIR!\" >nul 2>&1
            )
            if exist "!OUTPUT_DIR!\csharp\Common" (
                mkdir "!TEST_PROJECT_DIR!\Common" >nul 2>&1
                xcopy "!OUTPUT_DIR!\csharp\Common\*.cs" "!TEST_PROJECT_DIR!\Common\" /q >nul 2>&1
            )
            copy "!TEST_FILE!" "!TEST_PROJECT_DIR!\Program.cs" >nul

            REM Compile and run
            echo   Compiling...
            cd /d "!TEST_PROJECT_DIR!"
            dotnet build -c Release --nologo -v q >nul 2>&1
            if errorlevel 1 (
                echo   FAILED ^(compilation error^)
                set /a FAILED+=1
            ) else (
                echo   Running...
                dotnet run -c Release --no-build
                if errorlevel 1 (
                    echo   FAILED ^(runtime error^)
                    set /a FAILED+=1
                ) else (
                    echo   PASSED
                    set /a PASSED+=1
                )
            )
            cd /d "%PROJECT_ROOT%"
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
