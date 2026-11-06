@echo off
start "" powershell.exe -NoProfile -STA -ExecutionPolicy Bypass -File "%~dp0LumaLauncher.ps1" %*
