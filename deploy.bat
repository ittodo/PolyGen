@echo off
set DIST_DIR=dist

if exist %DIST_DIR% rmdir /s /q %DIST_DIR%
mkdir %DIST_DIR%

cargo build --release

copy target\release\polygen.exe %DIST_DIR%\
xcopy templates %DIST_DIR%\templates /s /e /i /h /k /y
xcopy static %DIST_DIR%\static /s /e /i /h /k /y

echo Deployment complete!

call %DIST_DIR%\polygen.exe --schema-path examples/game_schema.poly --lang csharp