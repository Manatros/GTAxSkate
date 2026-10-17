# Puts the Skate 3 soundtrack (the in-game iPod's 46 songs) on GTA V's Self
# Radio, from your own extracted Skate 3 (Xbox 360) game folder: every song
# becomes an MP3 in Documents\Rockstar Games\GTA V\User Music\Skate 3.
#
# Run in PowerShell:
#   powershell -ExecutionPolicy Bypass -File .\get-skate-music.ps1 -Game "D:\Skate 3"
# -Game is the folder with default.xex (or default.xex itself).
#
# Needs skate-music.exe (-Exe; the GTAxSkate release has one) or Rust to
# build it, and vgmstream (reads EA's game audio; fetched into tools\ if
# missing). Afterwards, in GTA V: Settings > Audio > Self Radio Mode: Radio,
# then Perform Full Scan, once.
#
# Errors are thrown (the installer catches them); run directly, PowerShell
# shows them in red.

param(
    [Parameter(Mandatory)] [string] $Game,
    [string] $Out = (Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'Rockstar Games\GTA V\User Music\Skate 3'),
    # A ready skate-music.exe (the GTAxSkate release has one); else it is built with Rust.
    [string] $Exe = ''
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
$here  = Split-Path -Parent $PSScriptRoot
$tools = Join-Path $here 'tools'
if ((Split-Path -Leaf $Game) -ieq 'default.xex') { $Game = Split-Path -Parent $Game }
if (-not (Test-Path -LiteralPath (Join-Path $Game 'default.xex'))) { throw "$Game has no default.xex: point -Game at your extracted Skate 3 folder." }
New-Item -ItemType Directory -Force $tools | Out-Null

# vgmstream: reads the game's EA audio.
$vgm = Get-ChildItem $tools -Recurse -Filter vgmstream-cli.exe -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName
if (-not $vgm) {
    Write-Host '  fetching vgmstream...' -ForegroundColor DarkGray
    $zip = Join-Path $tools 'vgmstream-win64.zip'
    try {
        # Pinned release and hash, the same as install.ps1's.
        Invoke-WebRequest 'https://github.com/vgmstream/vgmstream/releases/download/r2117/vgmstream-win64.zip' -OutFile $zip -UseBasicParsing
        if ((Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash -ne '6c4a8a3813864fefed081bbd337dbc0ad93bf88e0b92f5db98d7ab258b22dc6c') {
            Remove-Item -LiteralPath $zip
            throw 'vgmstream-win64.zip does not match the expected SHA-256.'
        }
        Expand-Archive $zip (Join-Path $tools 'vgmstream') -Force
        Remove-Item $zip
    } catch {
        throw "Could not download vgmstream ($($_.Exception.Message)). Get vgmstream-win64.zip from https://vgmstream.org, unzip it into $tools\vgmstream and run this again."
    }
    $vgm = Get-ChildItem $tools -Recurse -Filter vgmstream-cli.exe | Select-Object -First 1 -ExpandProperty FullName
}

# The converter: the ready one, or built from gta\skate-music with Rust.
if (-not $Exe -or -not (Test-Path $Exe)) {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'skate-music.exe is missing and Rust is not installed (https://rustup.rs).' }
    cargo build --release --manifest-path (Join-Path $here 'gta\skate-music\Cargo.toml')
    if ($LASTEXITCODE) { throw 'Building skate-music failed.' }
    $Exe = Join-Path $here 'gta\skate-music\target\release\skate-music.exe'
}
& $Exe --game $Game --out $Out --vgmstream $vgm
if ($LASTEXITCODE) { throw "skate-music failed (exit code $LASTEXITCODE): see the lines above." }
Write-Host '  In GTA V: Settings > Audio > Self Radio Mode: Radio, then Perform Full Scan (once).' -ForegroundColor Green
