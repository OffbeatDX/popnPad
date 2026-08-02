@echo off
setlocal
cd /d "%~dp0"
set "CARGO_TARGET_DIR=%~dp0target"

where cargo >nul 2>nul
if errorlevel 1 (
  echo [X] Rust/Cargo is not on PATH.
  echo     Install Rust from https://rustup.rs then re-run this script.
  echo.
  pause
  exit /b 1
)

for /f "usebackq delims=" %%V in (`powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\next-version.ps1"`) do set "POPNPAD_VERSION=%%V"
if not defined POPNPAD_VERSION (
  echo [X] Could not determine the build version.
  exit /b 1
)

echo Building popnPad %POPNPAD_VERSION%...
echo.
cargo build --release --locked
if errorlevel 1 goto :build_failed

set "SRC=target\release\popnpad.exe"
set "DST=popnPad.exe"
copy /Y "%SRC%" "%DST%" >nul
if errorlevel 1 (
  echo.
  echo [X] Build succeeded but copying %DST% failed.
  echo     Is it currently running? Close it and re-run.
  echo.
  pause
  exit /b 1
)
echo.
echo [OK] %DST%  %POPNPAD_VERSION%
echo.
goto :eof

:build_failed
echo.
echo [X] Build failed - see the errors above.
echo.
pause
exit /b 1
