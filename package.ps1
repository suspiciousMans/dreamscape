<#
.SYNOPSIS
    Builds a game in release mode and assembles a distributable folder under dist/<game>/.

.EXAMPLE
    .\package.ps1 -Game sandbox
#>
param(
    [Parameter(Mandatory = $true)][string]$Game
)

$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
$gameDir = Join-Path $root "games\$Game"

if (-not (Test-Path $gameDir)) {
    Write-Error "No such game: games\$Game"
    exit 1
}

Write-Host "Building '$Game' in release mode..." -ForegroundColor Cyan
& cargo build --release -p $Game
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo build failed"
    exit $LASTEXITCODE
}

$exePath = Join-Path $root "target\release\$Game.exe"
if (-not (Test-Path $exePath)) {
    Write-Error "Expected build output not found: $exePath"
    exit 1
}

$distDir = Join-Path $root "dist\$Game"
if (Test-Path $distDir) {
    Remove-Item -Recurse -Force $distDir
}
New-Item -ItemType Directory -Path $distDir | Out-Null

Copy-Item $exePath $distDir

foreach ($folder in @("assets", "profiles")) {
    $src = Join-Path $gameDir $folder
    if (Test-Path $src) {
        Copy-Item $src (Join-Path $distDir $folder) -Recurse
    }
}

Write-Host "Packaged to $distDir" -ForegroundColor Green
Get-ChildItem $distDir -Recurse | Select-Object -ExpandProperty FullName | ForEach-Object {
    Write-Host "  $($_.Substring($distDir.Length + 1))"
}
