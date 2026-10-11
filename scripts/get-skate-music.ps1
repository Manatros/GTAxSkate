# Puts the Skate 3 soundtrack on GTA V's Self Radio, from your own extracted
# Skate 3 (Xbox 360) game folder: every song is converted to an MP3 in
# Documents\Rockstar Games\GTA V\User Music\Skate 3.
#
# Run in PowerShell:
#   powershell -ExecutionPolicy Bypass -File .\get-skate-music.ps1 -Game "D:\Skate 3"
# -Game is the folder with default.xex (or default.xex itself).
#
# Needs skate-music.exe (-Exe) or Rust to build it. vgmstream (reads EA's game
# audio) and ffmpeg (writes MP3) are fetched into .\tools if missing.
# Afterwards, in GTA V: Settings > Audio > Self Radio > Full Scan, once.

param(
    [Parameter(Mandatory)] [string] $Game,
    [string] $Out = (Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'Rockstar Games\GTA V\User Music\Skate 3'),
    [int] $MinSeconds = 60,
    # A ready skate-music.exe (the GTAxSkate release has one); else it is built with Rust.
    [string] $Exe = ''
)
$ErrorActionPreference = 'Stop'
$here  = Split-Path -Parent $PSScriptRoot
$tools = Join-Path $here 'tools'
if ((Split-Path -Leaf $Game) -ieq 'default.xex') { $Game = Split-Path -Parent $Game }
if (-not (Test-Path (Join-Path $Game 'default.xex'))) {
    Write-Host "$Game has no default.xex. Point -Game at your extracted Skate 3 folder." -ForegroundColor Red
    exit 1
}
New-Item -ItemType Directory -Force $tools | Out-Null

# vgmstream: reads the game's EA audio streams.
$vgm = Get-Command vgmstream-cli -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source
if (-not $vgm) { $vgm = Get-ChildItem $tools -Recurse -Filter vgmstream-cli.exe -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName }
if (-not $vgm) {
    Write-Host 'Fetching vgmstream...' -ForegroundColor Cyan
    $zip = Join-Path $tools 'vgmstream-win64.zip'
    try {
        Invoke-WebRequest 'https://github.com/vgmstream/vgmstream/releases/latest/download/vgmstream-win64.zip' -OutFile $zip -UseBasicParsing
        Expand-Archive $zip (Join-Path $tools 'vgmstream') -Force
    } catch {
        Write-Host 'Could not download vgmstream. Get the Windows command-line build from https://vgmstream.org (vgmstream-win64.zip), unzip it into the tools folder here, and run this again.' -ForegroundColor Red
        exit 1
    }
    $vgm = Get-ChildItem $tools -Recurse -Filter vgmstream-cli.exe | Select-Object -First 1 -ExpandProperty FullName
}

# ffmpeg: writes the MP3s Self Radio plays.
$ffmpeg = Get-Command ffmpeg -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source
if (-not $ffmpeg) { $ffmpeg = Get-ChildItem $tools -Recurse -Filter ffmpeg.exe -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName }
if (-not $ffmpeg) {
    Write-Host 'Installing ffmpeg with winget...' -ForegroundColor Cyan
    winget install --id Gyan.FFmpeg -e --accept-source-agreements --accept-package-agreements | Out-Host
    $ffmpeg = Get-ChildItem (Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet') -Recurse -Filter ffmpeg.exe -ErrorAction SilentlyContinue |
        Select-Object -First 1 -ExpandProperty FullName
    if (-not $ffmpeg) {
        Write-Host 'ffmpeg is not installed. Install it (winget install Gyan.FFmpeg), open a new PowerShell window and run this again.' -ForegroundColor Red
        exit 1
    }
}

# The converter: the ready one, or built from gta\skate-music with Rust.
if (-not $Exe -or -not (Test-Path $Exe)) {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host 'skate-music.exe is missing and Rust is not installed (https://rustup.rs).' -ForegroundColor Red
        exit 1
    }
    cargo build --release --manifest-path (Join-Path $here 'gta\skate-music\Cargo.toml')
    if ($LASTEXITCODE) { throw 'Building skate-music failed.' }
    $Exe = Join-Path $here 'gta\skate-music\target\release\skate-music.exe'
}
$exe = $Exe
& $exe --game $Game --out $Out --vgmstream $vgm --ffmpeg $ffmpeg --min-seconds $MinSeconds
if ($LASTEXITCODE) { exit $LASTEXITCODE }
Write-Host "Done. In GTA V: Settings > Audio > Self Radio > Full Scan, then skate." -ForegroundColor Green
