@echo off
setlocal

echo ========================================
echo  PolyGen GUI - Production Build
echo ========================================
echo.

cd /d "%~dp0.."

echo [1/3] Building polygen CLI (release)...
cargo build --release
if %ERRORLEVEL% neq 0 (
    echo ERROR: polygen CLI build failed.
    pause
    exit /b 1
)
echo       OK
echo.

echo [2/3] Installing GUI dependencies...
cd gui
call npm install --silent
if %ERRORLEVEL% neq 0 (
    echo ERROR: npm install failed.
    pause
    exit /b 1
)
echo       OK
echo.

echo [3/3] Building Tauri app (production)...
echo.
call npm run tauri:build
if %ERRORLEVEL% neq 0 (
    echo ERROR: Tauri build failed.
    pause
    exit /b 1
)

echo.
echo ========================================
echo  Build complete!
echo  Output: gui/src-tauri/target/release/bundle/
echo ========================================
pause

endlocal
