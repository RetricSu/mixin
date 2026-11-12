@echo off
cd /d "%~dp0"
chcp 65001 >nul

if exist "火漆.exe" (
    start "" "火漆.exe"
    exit /b 0
)

if exist "waxseal.exe" (
    start "" "waxseal.exe"
    exit /b 0
)

echo [错误] 未在当前目录下找到「火漆.exe」或「waxseal.exe」。
echo 请确保已将压缩包完整解压后再运行。
pause

