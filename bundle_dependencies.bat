@echo off
title Bundling Dependencies for Spotify Downloader

echo 🎵 Bundling Dependencies for Spotify Downloader
echo ================================================

REM Check if we're in the right directory
if not exist "package.json" (
    echo ❌ Please run this script from the project root directory
    pause
    exit /b 1
)

REM Create directories
echo 🔄 Creating bundle directories...
if not exist "python_packages" mkdir python_packages
if not exist "ffmpeg" mkdir ffmpeg

REM Step 1: Bundle Python dependencies
echo.
echo 🔄 Step 1: Bundling Python dependencies...

REM Check if Python is available
python --version >nul 2>&1
if %errorlevel% neq 0 (
    echo ❌ Python not found! Please install Python first.
    pause
    exit /b 1
)

echo Installing Python packages to bundle...
pip install --target python_packages mutagen requests yt-dlp

if %errorlevel% neq 0 (
    echo ❌ Failed to install Python packages
    pause
    exit /b 1
)

echo ✅ Python packages bundled successfully

REM Step 2: Download and bundle FFmpeg
echo.
echo 🔄 Step 2: Downloading FFmpeg...

REM Download FFmpeg
echo Downloading FFmpeg from GitHub...
powershell -Command "& {[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; Invoke-WebRequest -Uri 'https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip' -OutFile 'ffmpeg.zip'}"

if not exist "ffmpeg.zip" (
    echo ❌ Failed to download FFmpeg
    pause
    exit /b 1
)

echo Extracting FFmpeg...
powershell -Command "Expand-Archive -Path 'ffmpeg.zip' -DestinationPath 'ffmpeg_temp' -Force"

REM Move FFmpeg files to the correct location
if exist "ffmpeg_temp\ffmpeg-master-latest-win64-gpl" (
    xcopy "ffmpeg_temp\ffmpeg-master-latest-win64-gpl\*" "ffmpeg\" /E /I /Y
) else (
    echo ❌ FFmpeg extraction failed
    pause
    exit /b 1
)

REM Clean up
rmdir /s /q ffmpeg_temp
del ffmpeg.zip

echo ✅ FFmpeg bundled successfully

echo.
echo ========================================
echo ✅ Dependencies bundled successfully!
echo.
echo 📁 Bundle contents:
echo    • python_packages\ - Python dependencies
echo    • ffmpeg\ - FFmpeg binaries
echo.
echo 🎉 Ready to create installer!

pause
