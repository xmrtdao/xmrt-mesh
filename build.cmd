@echo off
call "C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" > nul
set PATH=C:\Users\PureTrek\.cargo\bin;%PATH%
cd /d C:\Users\PureTrek\Desktop\DevGruGold\xmrt-mesh
cargo build %*
if %ERRORLEVEL% NEQ 0 (
    echo Build failed with error %ERRORLEVEL%
    exit /b %ERRORLEVEL%
)
echo Build successful!
