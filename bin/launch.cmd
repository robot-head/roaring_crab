@echo off
rem roaring-crab Windows launcher.
setlocal
if "%~1"=="" exit /b 2

set "EVENT=%~1"
set "BIN=%CLAUDE_PLUGIN_ROOT%\bin\windows-x86_64\roaring-crab.exe"
if not exist "%BIN%" exit /b 0
"%BIN%" --event %EVENT%
