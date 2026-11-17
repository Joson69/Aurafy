@echo off
setlocal
cd /d "%~dp0"
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
where cargo >nul 2>nul
if errorlevel 1 (
  echo Rust is not installed or Cargo is not on PATH.
  echo Install Rust stable 1.90 or later from https://rustup.rs/ and reopen this window.
  pause
  exit /b 1
)
echo Checking for Aurafy updates and building changed source...
cargo build --release --locked
if errorlevel 1 goto :failed
start "" "%~dp0target\release\aurafy.exe"
exit /b 0
:failed
echo.
echo Aurafy build failed. Review the compiler output above.
pause
exit /b 1
