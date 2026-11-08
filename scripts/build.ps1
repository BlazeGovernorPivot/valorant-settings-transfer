# Builds ValSync in release mode and packages it as dist\ValSync-v1.0.0-win-x64.zip.
# Usage: powershell -ExecutionPolicy Bypass -File scripts\build.ps1 [-NoInstaller]
param([switch]$NoInstaller)

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Tauri = Join-Path $Root 'src-tauri'
$Version = (Get-Content (Join-Path $Tauri 'tauri.conf.json') -Raw | ConvertFrom-Json).version
$Release = "ValSync-v$Version-win-x64.zip"
$Dist = Join-Path $Root 'dist'

function Step($msg) { Write-Host "==> $msg" -ForegroundColor Magenta }

Step "Running unit tests"
Push-Location $Tauri
try {
    cargo test --quiet
    if ($LASTEXITCODE -ne 0) { throw 'cargo test failed' }

    $hasTauriCli = $null -ne (cargo --list | Select-String -SimpleMatch 'tauri')
    if ($hasTauriCli -and -not $NoInstaller) {
        Step "Building with cargo tauri build (NSIS installer + exe)"
        cargo tauri build
    } else {
        Step "Building with cargo build --release (exe only)"
        cargo build --release
    }
    if ($LASTEXITCODE -ne 0) { throw 'build failed' }
} finally {
    Pop-Location
}

$TargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Tauri 'target' }
$Exe = Join-Path $TargetDir 'release\ValSync.exe'
if (-not (Test-Path $Exe)) { $Exe = Join-Path $TargetDir 'release\valorant-settings-transfer.exe' }
if (-not (Test-Path $Exe)) { throw "executable not found in $TargetDir\release" }

Step "Packaging $Release"
$Stage = Join-Path $Dist 'stage'
Remove-Item $Stage -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $Stage | Out-Null
Copy-Item $Exe (Join-Path $Stage 'ValSync.exe')
Copy-Item (Join-Path $Root 'LICENSE') $Stage
$Installer = Get-ChildItem (Join-Path $TargetDir 'release\bundle\nsis') -Filter '*.exe' -ErrorAction SilentlyContinue | Select-Object -First 1
if ($Installer) { Copy-Item $Installer.FullName (Join-Path $Stage "ValSync-$Version-setup.exe") }

$Zip = Join-Path $Dist $Release
Remove-Item $Zip -Force -ErrorAction SilentlyContinue
Compress-Archive -Path (Join-Path $Stage '*') -DestinationPath $Zip
Remove-Item $Stage -Recurse -Force
$Hash = (Get-FileHash $Zip -Algorithm SHA256).Hash
Step "Done: $Zip"
Write-Host "SHA256 $Hash"
