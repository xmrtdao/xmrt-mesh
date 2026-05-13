# Build script for xmrt-mesh on Windows
# Sets up MSVC environment variables then runs cargo

$vcvars = "C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

if (Test-Path $vcvars) {
    Write-Host "Setting up MSVC environment..."
    # vcvars sets INCLUDE, LIB, LIBPATH, PATH for MSVC tools
    cmd /c "`"$vcvars`" > nul 2>&1 && set" | ForEach-Object {
        if ($_ -match '^([^=]+)=(.*)$') {
            [Environment]::SetEnvironmentVariable($matches[1], $matches[2])
        }
    }
}

# Remove Git Bash's link.exe from PATH
$env:PATH = ($env:PATH -split ';' | Where-Object { $_ -notmatch 'Git\\usr\\bin' }) -join ';'

# Add MSVC bin directories
$msvcBin = "C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64"
if (Test-Path $msvcBin) {
    $env:PATH = "$msvcBin;$env:PATH"
}

Write-Host "Building xmrt-mesh..."
cargo build $args

if ($LASTEXITCODE -eq 0) {
    Write-Host "Build successful!"
} else {
    Write-Host "Build failed with exit code $LASTEXITCODE"
    exit $LASTEXITCODE
}
