# Builds SkateGTA.asi: Skate 3 for GTA V, from the mashup's source with
# Claude's mods (mashup-mods.patch).
#
# Put this script and mashup-mods.patch in the same folder, then run in
# PowerShell:
#   powershell -ExecutionPolicy Bypass -File .\build-skate-gta.ps1
# Same tools as build-mashup-mods.ps1 (Git, Rust, Visual Studio C++ tools).

$ErrorActionPreference = 'Stop'
$here  = $PSScriptRoot
$patch = Join-Path $here 'mashup-mods.patch'
$base  = 'f608f85e407ff1b7689d54a9aafdd16e95711ac4'
$src   = Join-Path $here 'mashup-src'

foreach ($tool in 'git', 'cargo') {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        Write-Host "Missing '$tool'. Install the tools listed in build-mashup-mods.ps1, then open a new PowerShell window." -ForegroundColor Red
        exit 1
    }
}
if (-not (Test-Path $patch)) {
    Write-Host "mashup-mods.patch must be next to this script." -ForegroundColor Red
    exit 1
}
if (-not (Test-Path $src)) {
    git clone -c core.autocrlf=false https://github.com/chasmlol/2010-rust-rewrite-mashup $src
    if ($LASTEXITCODE) { exit 1 }
}

Push-Location $src
try {
    git fetch --quiet origin
    git checkout --quiet -f $base
    if ($LASTEXITCODE) { throw 'Could not check out the base version.' }
    # Files an earlier version of the patch added; build output is kept.
    git clean -fdq -e target -- crates gta skate third_party/bevy_audio
    git apply $patch
    if ($LASTEXITCODE) { throw 'The patch did not apply.' }
    Write-Host 'Building SkateGTA.asi. The first build takes a while.' -ForegroundColor Cyan
    cargo build --release --manifest-path (Join-Path $here 'gta\skate-gta\Cargo.toml')
    if ($LASTEXITCODE) { throw 'The build failed. Send Claude the last lines above.' }
}
finally {
    Pop-Location
}

$dll = Join-Path $here 'gta\skate-gta\target\release\skate_gta.dll'
Copy-Item $dll (Join-Path $here 'SkateGTA.asi') -Force
Write-Host "Done: $here\SkateGTA.asi" -ForegroundColor Green
Write-Host 'Copy it into your GTA V folder (next to ScriptHookV.dll). It writes SkateGTA.ini on first start; set SkateAssets there.'
