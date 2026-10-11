# ==============================================================================
# Prowly Windows Installation Script
# Builds release binary and installs to user local path or Cargo bin.
# ==============================================================================

Write-Host "🐾 Prowly Windows Installer" -ForegroundColor Cyan
Write-Host "==========================="

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "Cargo is not found. Please install Rust from https://rustup.rs"
    exit 1
}

Write-Host "[1/2] Building Prowly in release mode..." -ForegroundColor Yellow
cargo build --release

$targetBin = Join-Path $PSScriptRoot "..\target\release\prowly.exe"
$installDir = Join-Path $env:USERPROFILE ".cargo\bin"

Write-Host "[2/2] Installing prowly.exe to $installDir..." -ForegroundColor Yellow
Copy-Item $targetBin -Destination $installDir -Force

Write-Host "✨ Prowly installed successfully to $installDir\prowly.exe!" -ForegroundColor Green
