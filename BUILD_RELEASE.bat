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
echo Building Aurafy for Windows...
cargo build --release --locked
if errorlevel 1 goto :failed
echo.
echo Build complete:
echo %CD%\target\release\aurafy.exe
pause
exit /b 0
:failed
echo.
echo Build failed. Review the compiler output above.
pause
exit /b 1
