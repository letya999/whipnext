@echo off
setlocal
cd /d "%~dp0.."

cargo build --release
if errorlevel 1 exit /b %errorlevel%

set "APP_DIR=%LOCALAPPDATA%\whipnext"
if not exist "%APP_DIR%" mkdir "%APP_DIR%"
copy /Y "target\release\whipnext.exe" "%APP_DIR%\whipnext.exe" >nul
if errorlevel 1 exit /b %errorlevel%

if exist "%APP_DIR%\assets" rmdir /S /Q "%APP_DIR%\assets"
xcopy "assets" "%APP_DIR%\assets" /E /I /Y /Q >nul
if errorlevel 1 exit /b %errorlevel%

echo installed: %APP_DIR%
