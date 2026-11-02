@echo off
cd /d "%~dp0"
chcp 65001 >nul

if exist "密信.exe" (
    start "" "密信.exe"
    exit /b 0
)

if exist "mixin.exe" (
    start "" "mixin.exe"
    exit /b 0
)

echo [错误] 未在当前目录下找到「密信.exe」或「mixin.exe」。
echo 请确保已将压缩包完整解压后再运行。
pause

