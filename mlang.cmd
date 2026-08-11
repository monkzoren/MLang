@echo off
rem MLang launcher (Windows). Usage: .\mlang.cmd run examples\edit.ml
setlocal
set "EXE=%~dp0compiler\target\release\mlang.exe"
if not exist "%EXE%" (
  echo mlang is not built yet. Build it once with:
  echo   cargo build --release --manifest-path "%~dp0compiler\Cargo.toml"
  exit /b 2
)
"%EXE%" %*
exit /b %ERRORLEVEL%
