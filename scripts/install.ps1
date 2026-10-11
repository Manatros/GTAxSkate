# GTAxSkate installer: SK8V (Skate 3's engine in GTA V) plus GTAxSkate's
# additions (keyboard and mouse, Skate 3's board shop with every part, GTA's
# camera and a cinematic camera, trainer tuning and stamina, GPS while
# skating), all made from your own Skate 3 and GTA V. Started by
# "Install GTAxSkate.cmd" in the repo folder.
#
#   powershell -ExecutionPolicy Bypass -File scripts\install.ps1 [-Gta <folder>] [-Skate3 <ISO or folder>] [-Redo <step>] [-Until <step>] [-NoMusic]
#
# It finds what it can by itself and asks only for the rest (a folder or file
# picker). Every step leaves a receipt in build\install\, so running it again
# goes on after the last finished step; -Redo <step> runs a step again.
#
# Steps:
#   1 tools    the SK8V release (its Python and rage tool), vgmstream
#   2 sk8v     SK8V's own setup: Skate 3's data from your disc, installed into GTA V
#   3 bin      GTAxSkate's builds: SK8V with our additions, the board tool
#              (built here when the source is, else from the GTAxSkate release)
#   4 boards   the board library and every part as a GTA model (a few minutes)
#   5 music    Skate 3's soundtrack on GTA's Self Radio (-NoMusic skips it)
#   6 install  our SK8V build over SK8V's, the settings
#
# ASCII only: Windows PowerShell 5.1 reads a script without BOM as ANSI.
param(
    [string] $Gta = '',
    [string] $Skate3 = '',
    [ValidateSet('', 'tools', 'sk8v', 'bin', 'boards', 'music', 'install')] [string] $Redo = '',
    # Stop after this step (testing, or preparing without touching GTA yet).
    [ValidateSet('', 'tools', 'sk8v', 'bin', 'boards', 'music', 'install')] [string] $Until = '',
    [switch] $NoMusic
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$root = Split-Path -Parent $PSScriptRoot
$build = Join-Path $root 'build'
$receipts = Join-Path $build 'install'
$tools = Join-Path $root 'tools'
$external = Join-Path $root 'external'
$bin = Join-Path $build 'bin'
$settings = Join-Path $receipts 'folders.json'
New-Item -ItemType Directory -Force $receipts, $tools, $external, $bin | Out-Null

# SK8V: pinned to the release our additions are built on.
$Sk8vTag = '1.1.5'
$Sk8vZip = "https://github.com/becaring/SK8V/releases/download/$Sk8vTag/SK8V-$Sk8vTag.zip"
# GTAxSkate's builds for players without the build tools (a release asset).
$BinZip = 'https://github.com/Manatros/GTAxSkate/releases/latest/download/GTAxSkate-bin.zip'
# Before the first tested release: the newest development build.
$BinZipDev = 'https://github.com/Manatros/GTAxSkate/releases/download/dev-build/GTAxSkate-bin.zip'
$GtaBuild = '1.0.3889.0'
$Steps = 'tools', 'sk8v', 'bin', 'boards', 'music', 'install'

Add-Type -AssemblyName System.Windows.Forms

function Say([string]$text, [string]$color = 'Gray') { Write-Host $text -ForegroundColor $color }
function Fail([string]$text) {
    Say ''
    Say "Stopped: $text" 'Red'
    Say 'Run "Install GTAxSkate.cmd" again once it is fixed; it goes on where it stopped.' 'Yellow'
    exit 1
}
function Done([string]$step) { Set-Content -LiteralPath (Join-Path $receipts "$step.done") (Get-Date -Format s) }
function Is-Done([string]$step) {
    if ($Redo -and [array]::IndexOf($Steps, $step) -ge [array]::IndexOf($Steps, $Redo)) { return $false }
    Test-Path (Join-Path $receipts "$step.done")
}
function Step([int]$n, [string]$text) {
    Say ''
    Say "[$n/$($Steps.Count)] $text" 'Cyan'
}
function Run([string]$what, [scriptblock]$block) {
    Say "  $what..." 'DarkGray'
    & $block
    if ($LASTEXITCODE) { Fail "$what failed (exit code $LASTEXITCODE)." }
}
function Download([string]$url, [string]$to) {
    Say "  downloading $(Split-Path -Leaf $url)..." 'DarkGray'
    Invoke-WebRequest -UseBasicParsing $url -OutFile $to
}
function Pick-Folder([string]$prompt) {
    $dialog = New-Object System.Windows.Forms.FolderBrowserDialog
    $dialog.Description = $prompt
    if ($dialog.ShowDialog() -eq 'OK') { $dialog.SelectedPath } else { '' }
}
function Pick-Skate3 {
    $dialog = New-Object System.Windows.Forms.OpenFileDialog
    $dialog.Title = 'Your Skate 3 (Xbox 360) disc image, or Cancel to pick its extracted folder'
    $dialog.Filter = 'Disc image (*.iso)|*.iso|All files|*.*'
    if ($dialog.ShowDialog() -eq 'OK') { return $dialog.FileName }
    Pick-Folder 'Your extracted Skate 3 folder (it holds default.xex)'
}

# --- The folders: given, remembered from the last run, found, or asked ------
$known = if (Test-Path $settings) { Get-Content $settings -Raw | ConvertFrom-Json } else { [pscustomobject]@{} }
function Remember([string]$name, [string]$value) { $known | Add-Member -NotePropertyName $name -NotePropertyValue $value -Force }
function Known([string]$name) { $p = $known.PSObject.Properties[$name]; if ($p) { [string]$p.Value } else { '' } }

function Find-Gta {
    foreach ($key in 'HKLM:\SOFTWARE\WOW6432Node\Rockstar Games\Grand Theft Auto V', 'HKLM:\SOFTWARE\WOW6432Node\Rockstar Games\GTAV') {
        $p = (Get-ItemProperty $key -ErrorAction SilentlyContinue).InstallFolder
        if ($p -and (Test-Path (Join-Path $p 'GTA5.exe'))) { return $p }
    }
    foreach ($drive in Get-PSDrive -PSProvider FileSystem) {
        foreach ($rel in 'Games\Grand Theft Auto V Legacy', 'Program Files\Rockstar Games\Grand Theft Auto V',
                         'Program Files (x86)\Steam\steamapps\common\Grand Theft Auto V', 'SteamLibrary\steamapps\common\Grand Theft Auto V',
                         'Program Files\Epic Games\GTAV') {
            $p = Join-Path $drive.Root $rel
            if (Test-Path (Join-Path $p 'GTA5.exe')) { return $p }
        }
    }
    ''
}

Say 'GTAxSkate installer' 'White'
if (-not $Gta) { $Gta = Known 'Gta' }
if (-not $Gta) { $Gta = Find-Gta }
if (-not $Gta -or -not (Test-Path (Join-Path $Gta 'GTA5.exe'))) { $Gta = Pick-Folder 'Your GTA V Legacy folder (it holds GTA5.exe)' }
if (-not $Gta -or -not (Test-Path (Join-Path $Gta 'GTA5.exe'))) { Fail 'GTA V Legacy (GTA5.exe) was not found.' }
$version = (Get-Item (Join-Path $Gta 'GTA5.exe')).VersionInfo.FileVersion
if ($version -and $version -ne $GtaBuild) { Say "  Note: SK8V is made for GTA V $GtaBuild; yours is $version." 'Yellow' }
if (-not $Skate3) { $Skate3 = Known 'Skate3' }
if (-not (Is-Done 'boards') -and (-not $Skate3 -or -not (Test-Path -LiteralPath $Skate3))) { $Skate3 = Pick-Skate3 }
if (-not (Is-Done 'boards') -and (-not $Skate3 -or -not (Test-Path -LiteralPath $Skate3))) { Fail 'Your Skate 3 disc image or folder was not found.' }
Remember 'Gta' $Gta
Remember 'Skate3' $Skate3
$known | ConvertTo-Json | Set-Content $settings
if (Get-Process GTA5 -ErrorAction SilentlyContinue) { Fail 'GTA V is running: close it first.' }
Say "  GTA V:  $Gta"
Say "  Skate 3: $Skate3"

function Stop-After([string]$step) { if ($Until -eq $step) { Say ''; Say "Stopped after '$step' (-Until)." 'Yellow'; exit 0 } }

# --- 1 tools -----------------------------------------------------------------
$release = Join-Path $external "sk8v-release\SK8V-$Sk8vTag"
$python = Join-Path $release 'python\python.exe'
$rage = Join-Path $release 'build\tools\rage.exe'
Step 1 'Tools'
if (-not (Is-Done 'tools') -or -not (Test-Path $python) -or -not (Test-Path $rage)) {
    if (-not (Test-Path (Join-Path $release 'tools\prepare.py'))) {
        $zip = Join-Path $env:TEMP "SK8V-$Sk8vTag.zip"
        Download $Sk8vZip $zip
        Say '  unpacking SK8V...' 'DarkGray'
        $unpacked = Join-Path $env:TEMP "SK8V-$Sk8vTag-unpacked"
        Remove-Item -LiteralPath $unpacked -Recurse -Force -ErrorAction SilentlyContinue
        Expand-Archive $zip $unpacked -Force
        Remove-Item $zip
        # The release's top folder (the one with tools\prepare.py), wherever the zip puts it.
        $top = Get-ChildItem $unpacked -Filter 'prepare.py' -File -Recurse -Depth 3 | Where-Object { $_.Directory.Name -eq 'tools' } | Select-Object -First 1
        if (-not $top) { Fail "The SK8V $Sk8vTag release has no tools\prepare.py." }
        New-Item -ItemType Directory -Force (Split-Path -Parent $release) | Out-Null
        Remove-Item -LiteralPath $release -Recurse -Force -ErrorAction SilentlyContinue
        Move-Item $top.Directory.Parent.FullName $release
        Remove-Item -LiteralPath $unpacked -Recurse -Force -ErrorAction SilentlyContinue
    }
    if (-not (Test-Path $python)) {
        # SK8V's own Python fetcher: python.org's embeddable Python and the
        # numpy/Pillow wheels, each checked against its SHA-256 (about 31 MB).
        Push-Location $release
        try { Run 'Getting Python' { powershell -NoProfile -ExecutionPolicy Bypass -File tools\get-python.ps1 } }
        finally { Pop-Location }
    }
    if (-not (Test-Path $python)) { Fail "SK8V's Python is missing ($python)." }
    if (-not (Test-Path $rage)) { Fail "SK8V's rage tool is missing ($rage)." }
    if (-not (Get-ChildItem $tools -Recurse -Filter vgmstream-cli.exe -ErrorAction SilentlyContinue | Select-Object -First 1)) {
        $zip = Join-Path $tools 'vgmstream-win64.zip'
        Download 'https://github.com/vgmstream/vgmstream/releases/latest/download/vgmstream-win64.zip' $zip
        Expand-Archive $zip (Join-Path $tools 'vgmstream') -Force
        Remove-Item $zip
    }
    Done 'tools'
}
Say '  ready' 'Green'
Stop-After 'tools'

# --- 2 sk8v ------------------------------------------------------------------
$data = Join-Path $Gta 'GTAxSkate'
$assets = Join-Path $data 'skate-data\assets'
Step 2 'SK8V: Skate 3 data from your disc, installed into GTA V (10-30 minutes)'
$iniKeep = Join-Path $receipts 'SkateVLegacy.ini.kept'
if (-not (Is-Done 'sk8v')) {
    $iniNow = Join-Path $Gta 'SkateVLegacy.ini'
    if (Test-Path $iniNow) { Copy-Item $iniNow $iniKeep -Force }
    # SK8V's own setup does everything SK8V needs (data, Script Hook V
    # check, its DLC pack, its ini). The disc stays extracted: the boards
    # and the music read it.
    Push-Location $release
    try { Run 'Running SK8V''s setup' { & $python tools\prepare.py --skate $Skate3 --gta $Gta --out $data --install } }
    finally { Pop-Location }
    if (-not (Test-Path (Join-Path $assets 'private\skater.glb'))) { Fail "SK8V's setup left no private\skater.glb ($assets)." }
    if (-not (Test-Path (Join-Path $Gta 'SkateVLegacy.asi'))) { Fail 'SK8V was not installed (no SkateVLegacy.asi in the GTA folder).' }
    Done 'sk8v'
}
# The extracted disc (for the boards and the music): the folder given, or SK8V's copy of the ISO.
$disc = if (Test-Path -LiteralPath $Skate3 -PathType Container) { $Skate3 } else {
    Get-ChildItem (Join-Path $data 'disc') -Filter 'default.xex' -File -Recurse -Depth 2 -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty DirectoryName
}
Say '  ready' 'Green'
Stop-After 'sk8v'

# --- 3 bin -------------------------------------------------------------------
Step 3 'GTAxSkate builds'
$sk8vSrc = Join-Path $external 'sk8v'
if (-not (Is-Done 'bin')) {
    $package = Join-Path $sk8vSrc 'build\package'
    $ped = Join-Path $root 'gta\skate-ped\target\release\skate-ped.exe'
    $music = Join-Path $root 'gta\skate-music\target\release\skate-music.exe'
    if ((Test-Path (Join-Path $package 'SkateVLegacy.asi')) -and (Test-Path $ped)) {
        # A developer's checkout: the builds made here.
        Copy-Item (Join-Path $package 'SkateVLegacy.asi'), (Join-Path $package 'SkateVRuntime.dll') $bin -Force
        Copy-Item (Join-Path $sk8vSrc 'tools\prepare-board-parts.py'), (Join-Path $sk8vSrc 'tools\_common.py') $bin -Force
        Copy-Item $ped $bin -Force
        if (Test-Path $music) { Copy-Item $music $bin -Force }
    } else {
        $zip = Join-Path $env:TEMP 'GTAxSkate-bin.zip'
        try { Download $BinZip $zip } catch { Download $BinZipDev $zip }
        Expand-Archive $zip $bin -Force
        Remove-Item $zip
    }
    foreach ($f in 'SkateVLegacy.asi', 'SkateVRuntime.dll', 'skate-ped.exe', 'prepare-board-parts.py') {
        if (-not (Test-Path (Join-Path $bin $f))) { Fail "The GTAxSkate builds have no $f." }
    }
    Done 'bin'
}
Say '  ready' 'Green'
Stop-After 'bin'

# --- 4 boards ----------------------------------------------------------------
$models = Join-Path $build 'board-models'
$ped = Join-Path $bin 'skate-ped.exe'
Step 4 'Boards: the library and every part as a GTA model (a few minutes)'
if (-not (Is-Done 'boards')) {
    if (-not $disc) { Fail 'The extracted Skate 3 disc was not found (SK8V keeps it under GTAxSkate\disc).' }
    if (-not (Test-Path (Join-Path $assets 'private\boards\library.json'))) {
        $engine = Join-Path $release 'upstream\skate-3-rust-engine-donor'
        Run 'Preparing the board library' { & $ped --assets $assets --prepare-boards --game $disc --engine $engine --python $python }
    }
    $customDecks = Join-Path $root 'custom-assets\decks'
    if (Get-ChildItem $customDecks -Recurse -File -Include *.png, *.jpg, *.jpeg, *.bmp, *.tga, *.dds, *.webp -ErrorAction SilentlyContinue) {
        Run 'Adding your own decks' { & $ped --assets $assets --import-decks $customDecks --out $models }
    }
    Run 'Exporting the board parts' { & $ped --assets $assets --out $models --weapon --game $disc }
    $native = Join-Path $assets 'private\board-native\skatev_board.ydr.xml'
    if (-not (Test-Path $native)) { Fail "SK8V's native board ($native) is missing: run with -Redo sk8v." }
    $parts = Join-Path $assets 'private\board-parts'
    Run 'Building the part models' {
        & $python (Join-Path $bin 'prepare-board-parts.py') --parts (Join-Path $models 'parts') --board $native `
            --board-json (Join-Path $assets 'private\board\board-materials.json') --rage $rage --out $parts
    }
    if (-not (Test-Path (Join-Path $parts 'parts.json'))) { Fail 'The part models were not built.' }
    Done 'boards'
}
Say '  ready' 'Green'
Stop-After 'boards'

# --- 5 music -----------------------------------------------------------------
Step 5 'Skate 3 soundtrack on Self Radio'
if ($NoMusic) { Say '  skipped (-NoMusic)' 'DarkGray' }
elseif (-not (Is-Done 'music')) {
    if ($disc) {
        try { & (Join-Path $PSScriptRoot 'get-skate-music.ps1') -Game $disc -Exe (Join-Path $bin 'skate-music.exe'); Done 'music' }
        catch { Say "  the soundtrack was skipped: $($_.Exception.Message)" 'Yellow' }
    } else { Say '  skipped (no extracted disc)' 'Yellow' }
}
Say '  ready' 'Green'
Stop-After 'music'

# --- 6 install ---------------------------------------------------------------
Step 6 'Installing into GTA V'
Copy-Item (Join-Path $bin 'SkateVLegacy.asi'), (Join-Path $bin 'SkateVRuntime.dll') $Gta -Force
# An older GTAxSkate (its own plugin and packs) goes out of the way, kept as *.off.
foreach ($old in (Join-Path $Gta 'SkateGTA.asi'), (Join-Path $Gta 'SkateExtras.asi')) {
    if (Test-Path $old) { Move-Item $old "$old.off" -Force; Say "  set aside $(Split-Path -Leaf $old) (now .off)" 'DarkGray' }
}
foreach ($pack in 'skate3weapon', 'skate3props') {
    $rpf = Join-Path $Gta "mods\update\x64\dlcpacks\$pack\dlc.rpf"
    if (Test-Path $rpf) { Move-Item $rpf "$rpf.off" -Force; Say "  set aside the $pack pack (now .off)" 'DarkGray' }
}
# SK8V's ini (UTF-16, as SK8V writes it): our additions on; the player's own values kept.
$ini = Join-Path $Gta 'SkateVLegacy.ini'
$text = if (Test-Path $ini) { [IO.File]::ReadAllText($ini) } else { "[SkateV]`r`n" }
$text = ($text -replace "`r+`n", "`n") -replace "`n", "`r`n"
if ($text -notmatch '(?m)^\[SkateV\]\r?$') { $text = "[SkateV]`r`n$text" }
function Add-Key([string]$key, [string]$value) {
    if ($script:text -notmatch "(?m)^$key=") { $script:text = $script:text -replace '(?m)^\[SkateV\]\r?$', "[SkateV]`r`n$key=$value" }
}
# The player's own values from before SK8V's setup that its template dropped.
if (Test-Path $iniKeep) {
    $kept = ([IO.File]::ReadAllText($iniKeep) -replace "`r+`n", "`n") -split "`n"
    foreach ($line in $kept) { if ($line -match '^([A-Za-z0-9_]+)=(.*)$') { Add-Key $Matches[1] $Matches[2] } }
}
$defaults = [ordered]@{ Keyboard = '1'; KeyboardHints = '1'; BoardShop = '1'; WheelsFromPaint = '1'; DrawBoard = '1'; DrawSkater = '1';
                        GpsWhileSkating = '1'; SkateSpots = '1'; Stamina = '1'; CameraView = 'Near'; PedsSurvive = '1'; HippieJump = '1.6'; VehicleShapes = '1' }
# With the soundtrack on Self Radio, skating plays it.
if (Is-Done 'music') { $defaults['SkateRadio'] = 'RADIO_19_USER' }
foreach ($key in $defaults.Keys) { Add-Key $key $defaults[$key] }
foreach ($key in 'DrawBoard', 'DrawSkater') { $text = $text -replace "(?m)^$key=0", "$key=1" }
$text = ($text -replace "`r+`n", "`n") -replace "`n", "`r`n"
[IO.File]::WriteAllText($ini, $text, [Text.Encoding]::Unicode)
Done 'install'

Say ''
Say 'GTAxSkate is installed.' 'Green'
Say 'In GTA V (story mode): pick the skateboard in the weapon wheel to skate. On the keyboard: WASD, Shift push, Space ollie, arrows tricks,' 'White'
Say 'E on/off the board, F back to GTA, V camera, R cinematic camera. G in Binco, the Discount Store or Suburban opens the board shop.' 'White'
Say 'Settings: the SK8V menu (/), or SkateVLegacy.ini; keys: SkateVKeys.txt.' 'White'
