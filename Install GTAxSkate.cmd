@echo off
rem Makes GTAxSkate from your own Skate 3 and GTA V and installs it into GTA V Legacy.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\install.ps1" %*
pause
