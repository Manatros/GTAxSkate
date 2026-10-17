@echo off
rem Experimental FiveM rules resource only; the skating runtime is not included.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\install-fivem.ps1" %*
pause
