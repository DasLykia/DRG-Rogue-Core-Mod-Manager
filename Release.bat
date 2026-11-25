@echo off
setlocal
cd /d "%~dp0"
title DPG Rogue Core Mod Manager - Release
echo.
echo === DPG: Rogue Core Mod Manager - publish a new version ===
echo.

where gh >nul 2>nul
if errorlevel 1 (
  echo GitHub CLI not found - installing it now...
  winget install --id GitHub.cli -e --accept-source-agreements --accept-package-agreements
  set "PATH=%PATH%;%ProgramFiles%\GitHub CLI"
)
where gh >nul 2>nul
if errorlevel 1 (
  echo.
  echo GitHub CLI was installed. Close this window and double-click Release.bat again.
  pause
  exit /b 1
)

gh auth status >nul 2>nul
if errorlevel 1 (
  echo.
  echo You need to log in to GitHub once. A browser will open - log in as DasLykia.
  echo.
  gh auth login --hostname github.com --git-protocol https --web
  if errorlevel 1 ( echo Login failed. & pause & exit /b 1 )
  gh auth setup-git
)

echo.
set "VER="
set /p VER=Version number (e.g. 0.1.0 for the first release): 
if "%VER%"=="" ( echo No version entered. & pause & exit /b 1 )

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0release.ps1" %VER%
if errorlevel 1 (
  echo.
  echo *** Release FAILED - see the messages above. ***
) else (
  echo.
  echo *** Done! v%VER% is online: https://github.com/DasLykia/DPG-Rogue-Core-Mod-Manager/releases ***
)
echo.
pause
