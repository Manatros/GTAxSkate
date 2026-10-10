# Installs GTAxSkate for GTA V Legacy (story mode): SK8V's skating with the
# GTAxSkate extras (board store, HD board parts, paints, griptapes,
# stickers, wear, cameras, keyboard layout). Needs only your own Skate 3 and
# GTA V; no other mod's data (see README.md).
#
#   powershell -ExecutionPolicy Bypass -File install-gta.ps1 `
#       -Gta "<GTA V Legacy folder>" -Skate3 "<extracted Skate 3 folder>" `
#       -Sk8v "<SK8V release folder>" -CodeWalker "<CodeWalker folder>"
#
# Steps (each skipped when already done, so it can be run again):
#   1. SK8V with the two patches in sk8v-patches\ (built from source)
#   2. the board library from your Skate 3 data (Skate 3 Rust Engine tools)
#   3. SkateGTA.asi (the extras plugin)
#   4. the HD board parts, griptapes, stickers and single wheels, packed as
#      the skate3props DLC
#   5. SK8V's own board hidden so the HD parts take its place
param(
    [Parameter(Mandatory)] [string] $Gta,
    [Parameter(Mandatory)] [string] $Skate3,
    [Parameter(Mandatory)] [string] $Sk8v,
    [Parameter(Mandatory)] [string] $CodeWalker,
    [string] $Sk8vVersion = '1.0.9'
)
$ErrorActionPreference = 'Stop'
$here = $PSScriptRoot
function Step([string]$text) { Write-Host "== $text" -ForegroundColor Cyan }
function Fail([string]$text) { Write-Host $text -ForegroundColor Red; exit 1 }

# --- Checks -----------------------------------------------------------------
if (Get-Process GTA5, PlayGTAV -ErrorAction SilentlyContinue) { Fail 'Close GTA V first.' }
if (-not (Test-Path (Join-Path $Gta 'GTA5.exe'))) { Fail "$Gta has no GTA5.exe." }
if (-not (Test-Path (Join-Path $Gta 'ScriptHookV.dll'))) { Fail 'Script Hook V is not installed in the GTA folder (https://www.dev-c.com/gtav/scripthookv/).' }
if (-not (Test-Path (Join-Path $Gta 'OpenIV.asi'))) { Fail 'OpenIV.asi is not installed (OpenIV > Tools > ASI Manager > OpenIV.ASI): the board parts load from the mods folder.' }
if (-not (Test-Path (Join-Path $Skate3 'data\content\createacharacter.big'))) { Fail "$Skate3 is not an extracted Skate 3 folder (no data\content\createacharacter.big)." }
if (-not ((Test-Path (Join-Path $Gta 'SkateVLegacy.asi')) -or (Test-Path (Join-Path $Gta 'SkateVLegacy.asi.off')))) { Fail 'SK8V is not installed: run its "SK8V Setup.cmd" first.' }
$engine = Join-Path $Sk8v 'upstream\skate-3-rust-engine-donor'
$python = Join-Path $Sk8v 'python\python.exe'
if (-not (Test-Path (Join-Path $engine 'tools\asset_pipeline\customisation_catalog.py'))) { Fail "$Sk8v has no upstream\skate-3-rust-engine-donor (the SK8V release folder its setup ran from)." }
if (-not (Test-Path $python)) { Fail "$Sk8v has no python\python.exe (SK8V's setup downloads it)." }
if (-not (Test-Path (Join-Path $CodeWalker 'CodeWalker.Core.dll'))) { Fail "$CodeWalker has no CodeWalker.Core.dll (https://github.com/dexyfex/CodeWalker/releases)." }
foreach ($tool in 'git', 'cargo') { if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { Fail "$tool is missing: install Git and Rust (rustup)." } }

# --- 1. SK8V with the patches ------------------------------------------------
Step '1/5 SK8V with the GTAxSkate patches'
$fork = Join-Path $here 'sk8v'
if (-not (Test-Path $fork)) {
    git clone --quiet --branch $Sk8vVersion https://github.com/becaring/SK8V.git $fork
    if ($LASTEXITCODE) { Fail 'Could not clone SK8V.' }
    Push-Location $fork
    try {
        git -c user.name=GTAxSkate -c user.email=gtaxskate@localhost am --quiet (Get-ChildItem (Join-Path $here 'sk8v-patches') -Filter *.patch | Sort-Object Name | ForEach-Object FullName)
        if ($LASTEXITCODE) { Fail 'The SK8V patches did not apply.' }
    } finally { Pop-Location }
}
if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) {
    $vs = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -property installationPath 2>$null
    $cmake = if ($vs) { Join-Path $vs 'Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin' }
    if (-not $cmake -or -not (Test-Path (Join-Path $cmake 'cmake.exe'))) { Fail 'CMake and the Visual Studio C++ build tools are needed to build SK8V.' }
    $env:PATH = "$cmake;$env:PATH"
}
$built = Join-Path $fork 'build\package\SkateVLegacy.asi'
if (-not (Test-Path $built)) {
    Push-Location $fork
    try {
        & powershell -NoProfile -ExecutionPolicy Bypass -File tools\bootstrap.ps1
        if ($LASTEXITCODE -or -not (Test-Path $built)) { Fail 'Building SK8V failed.' }
    } finally { Pop-Location }
}
foreach ($name in 'SkateVLegacy.asi', 'SkateVRuntime.dll') {
    $target = Join-Path $Gta $name
    if (-not (Test-Path $target) -and (Test-Path "$target.off")) { Move-Item "$target.off" $target }
    Copy-Item (Join-Path $fork "build\package\$name") $target -Force
}

# --- 2. The board library ----------------------------------------------------
Step '2/5 The board library from your Skate 3 data'
$assets = Join-Path $here 'skate-data'
$private = Join-Path $assets 'private'
New-Item -ItemType Directory -Force $private | Out-Null
$sk8vData = Join-Path $Gta 'SK8V\skate-data\assets\private'
Copy-Item (Join-Path $sk8vData 'skater.glb') (Join-Path $private 'skater.glb') -Force
foreach ($folder in 'stock', 'default_skater') {
    $link = Join-Path $private $folder
    if (-not (Test-Path $link)) { cmd /c mklink /J "$link" "$(Join-Path $sk8vData $folder)" | Out-Null }
}
cargo build --release --quiet --manifest-path (Join-Path $here 'gta\skate-ped\Cargo.toml')
if ($LASTEXITCODE) { Fail 'Building skate-ped failed.' }
$ped = Join-Path $here 'gta\skate-ped\target\release\skate-ped.exe'
if (-not (Test-Path (Join-Path $private 'boards\library.json'))) {
    & $ped --assets $assets --prepare-boards --game $Skate3 --engine $engine --python $python
    if ($LASTEXITCODE) { Fail 'Preparing the board library failed.' }
}

# --- 3. The extras plugin ----------------------------------------------------
Step '3/5 SkateGTA.asi'
cargo build --release --quiet --manifest-path (Join-Path $here 'gta\skate-gta\Cargo.toml')
if ($LASTEXITCODE) { Fail 'Building SkateGTA.asi failed.' }
Copy-Item (Join-Path $here 'gta\skate-gta\target\release\skate_gta.dll') (Join-Path $Gta 'SkateGTA.asi') -Force
$ini = Join-Path $Gta 'SkateGTA.ini'
$line = "SkateAssets = $assets"
if (Test-Path $ini) {
    $text = [IO.File]::ReadAllText($ini)
    $text = if ($text -match '(?m)^SkateAssets\s*=.*$') { $text -replace '(?m)^SkateAssets\s*=.*$', $line.Replace('$', '$$') } else { "$line`r`n$text" }
    [IO.File]::WriteAllText($ini, $text)
} else {
    [IO.File]::WriteAllText($ini, "$line`r`n")
}

# --- 4. The board parts ------------------------------------------------------
Step '4/5 The HD board parts (a few minutes)'
$mods = Join-Path $Gta 'mods\update'
if (-not (Test-Path (Join-Path $mods 'update.rpf'))) {
    New-Item -ItemType Directory -Force $mods | Out-Null
    Copy-Item (Join-Path $Gta 'update\update.rpf') (Join-Path $mods 'update.rpf')
}
$made = Join-Path $here 'board-models'
& $ped --assets $assets --out $made --weapon --game $Skate3
if ($LASTEXITCODE) { Fail 'Making the board parts failed.' }
& (Join-Path $here 'pack-skate-skater.ps1') -Gta $Gta -CodeWalker $CodeWalker -Weapon $made -Mode weapon -PropsOnly
if ($LASTEXITCODE) { Fail 'Packing the board parts failed.' }

# --- 5. SK8V's board hidden --------------------------------------------------
Step '5/5 SK8V + GTAxSkate on'
$sk8vIni = Join-Path $Gta 'SkateVLegacy.ini'
if (Test-Path $sk8vIni) {
    $text = [IO.File]::ReadAllText($sk8vIni)
    foreach ($key in 'DrawBoard', 'DrawSkater') {
        $text = if ($text -match "(?m)^$key=") { $text -replace "(?m)^$key=\d", "$key=0" } else { $text.TrimEnd() + "`r`n$key=0`r`n" }
    }
    [IO.File]::WriteAllText($sk8vIni, $text)
}
Write-Host 'Done. In GTA V: the skateboard from the weapon wheel skates; G in a clothing store opens the board store.' -ForegroundColor Green
