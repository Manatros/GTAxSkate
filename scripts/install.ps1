# GTAxSkate installer: makes the mod from your own Skate 3 and GTA V and puts
# it into GTA V Legacy. Started by "Install GTAxSkate.cmd" in the repo folder.
#
#   powershell -ExecutionPolicy Bypass -File scripts\install.ps1 [-Gta <folder>] [-Skate3 <ISO or folder>] [-CodeWalker <folder>] [-Redo <step>] [-Until <step>]
#
# It finds what it can by itself and asks only for the rest (a folder or file
# picker). Every step leaves a receipt in build\install\, so running it again
# goes on after the last finished step; -Redo <step> runs a step again.
#
# Steps:
#   1 tools      Git, Rust, the SK8V release (Python, Skate 3 converters), vgmstream
#   2 data       Skate 3's data converted from your disc (SK8V's converter)
#   3 engine     the Skate engine: the mashup's skate workspace + SK8V's patches + ours
#   4 plugin     SkateGTA.asi built against it
#   5 boards     the board library, the HD board part models, packed as a DLC
#   6 sounds     Skate 3's board sounds
#   7 parks      GTA's skate parks' exact collision
#   8 install    into the GTA folder (with Script Hook V), settings kept
#
# ASCII only: Windows PowerShell 5.1 reads a script without BOM as ANSI.
param(
    [string] $Gta = '',
    [string] $Skate3 = '',
    [string] $CodeWalker = '',
    [ValidateSet('', 'tools', 'data', 'engine', 'plugin', 'boards', 'sounds', 'parks', 'install')] [string] $Redo = '',
    # Stop after this step (testing, or preparing without touching GTA yet).
    [ValidateSet('', 'tools', 'data', 'engine', 'plugin', 'boards', 'sounds', 'parks', 'install')] [string] $Until = ''
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$root = Split-Path -Parent $PSScriptRoot
$build = Join-Path $root 'build'
$receipts = Join-Path $build 'install'
$tools = Join-Path $root 'tools'
$external = Join-Path $root 'external'
$settings = Join-Path $receipts 'folders.json'
New-Item -ItemType Directory -Force $receipts, $tools, $external | Out-Null

# SK8V: its release brings Python and the Skate 3 converters; its source the
# engine patches. Pinned (the tag and the commit our engine patches are for).
$Sk8vTag = '1.1.0'
$Sk8vCommit = '6f431f5184700ee4e27956f12dfb9c12d6bfe2fa'
$Sk8vZip = "https://github.com/becaring/SK8V/releases/download/$Sk8vTag/SK8V-$Sk8vTag.zip"
$Sk8vRepo = 'https://github.com/becaring/SK8V.git'
$GtaBuild = '1.0.3889.0'
$Steps = 'tools', 'data', 'engine', 'plugin', 'boards', 'sounds', 'parks', 'install'

Add-Type -AssemblyName System.Windows.Forms

function Say([string]$text, [string]$color = 'Gray') { Write-Host $text -ForegroundColor $color }
function Fail([string]$text) {
    Say ''
    Say $text 'Red'
    Say 'Nothing is lost: run the installer again after fixing this, it goes on where it stopped.' 'Yellow'
    exit 1
}
function Done([string]$step) { Set-Content -LiteralPath (Join-Path $receipts "$step.done") (Get-Date -Format s) }
function Is-Done([string]$step) {
    if ($Redo -and [array]::IndexOf($Steps, $step) -ge [array]::IndexOf($Steps, $Redo)) { return $false }
    Test-Path -LiteralPath (Join-Path $receipts "$step.done")
}
function Step([int]$n, [string]$text) {
    if ($Until -and $n -gt [array]::IndexOf($Steps, $Until) + 1) { Say ''; Say "Stopped after '$Until' as asked." 'Yellow'; exit 0 }
    Say ''; Say "[$n/8] $text" 'Cyan'
}
function Run([string]$what, [scriptblock]$block) {
    $global:LASTEXITCODE = 0
    & $block
    if ($LASTEXITCODE) { Fail "$what failed (exit code $LASTEXITCODE). The lines above say why." }
}
function Download([string]$url, [string]$to) {
    Say "  downloading $(Split-Path -Leaf $to)..." 'DarkGray'
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest $url -OutFile $to -UseBasicParsing
}
function Pick-Folder([string]$prompt) {
    $d = New-Object Windows.Forms.FolderBrowserDialog -Property @{ Description = $prompt; ShowNewFolderButton = $false }
    if ($d.ShowDialog() -eq 'OK') { return $d.SelectedPath }
    ''
}
function Pick-Skate3 {
    $answer = [Windows.Forms.MessageBox]::Show(
        "Where is your Skate 3 (Xbox 360)?`n`nYes: pick the disc image (ISO)`nNo: pick the extracted disc folder (with default.xex)",
        'GTAxSkate', 'YesNoCancel', 'Question')
    if ($answer -eq 'Yes') {
        $d = New-Object Windows.Forms.OpenFileDialog -Property @{ Filter = 'Xbox 360 disc image (*.iso)|*.iso|All files|*.*'; Title = 'Skate 3 disc image' }
        if ($d.ShowDialog() -eq 'OK') { return $d.FileName }
    } elseif ($answer -eq 'No') {
        return Pick-Folder 'The extracted Skate 3 folder (it holds default.xex and data)'
    }
    ''
}

# --- The folders: given, remembered from the last run, found, or asked ------
$known = if (Test-Path -LiteralPath $settings) { Get-Content -LiteralPath $settings -Raw | ConvertFrom-Json } else { [pscustomobject]@{} }
function Remember([string]$name, [string]$value) { $known | Add-Member -NotePropertyName $name -NotePropertyValue $value -Force }
function Known([string]$name) { $p = $known.PSObject.Properties[$name]; if ($p) { [string]$p.Value } else { '' } }

function Find-Gta {
    foreach ($k in 'HKLM:\SOFTWARE\WOW6432Node\Rockstar Games\Grand Theft Auto V',
                   'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Steam App 271590',
                   'HKLM:\SOFTWARE\WOW6432Node\Rockstar Games\GTAV') {
        $p = Get-ItemProperty $k -ErrorAction SilentlyContinue
        foreach ($v in $p.InstallFolder, $p.InstallLocation) { if ($v -and (Test-Path (Join-Path $v 'GTA5.exe'))) { return $v } }
    }
    foreach ($drive in (Get-PSDrive -PSProvider FileSystem).Root) {
        foreach ($rel in 'Games\Grand Theft Auto V Legacy', 'Games\Grand Theft Auto V', 'Program Files\Rockstar Games\Grand Theft Auto V',
                         'SteamLibrary\steamapps\common\Grand Theft Auto V', 'Program Files (x86)\Steam\steamapps\common\Grand Theft Auto V') {
            $p = Join-Path $drive $rel
            if (Test-Path (Join-Path $p 'GTA5.exe')) { return $p }
        }
    }
    ''
}
function Find-CodeWalker {
    $places = @((Join-Path $tools 'CodeWalker'), [Environment]::GetFolderPath('Desktop'), (Join-Path $env:USERPROFILE 'Downloads'), $env:ProgramFiles)
    foreach ($place in $places) {
        if (-not $place -or -not (Test-Path $place)) { continue }
        $hit = Get-ChildItem -LiteralPath $place -Filter 'CodeWalker.Core.dll' -File -Recurse -Depth 3 -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($hit) { return $hit.DirectoryName }
    }
    ''
}

Say 'GTAxSkate installer' 'White'
Say 'Skate 3 in GTA V Legacy, made from your own copies of the games.' 'DarkGray'

if (-not $Gta) { $Gta = Known 'Gta' }
if (-not $Gta -or -not (Test-Path (Join-Path $Gta 'GTA5.exe'))) { $Gta = Find-Gta }
if (-not $Gta) { $Gta = Pick-Folder 'Your GTA V Legacy folder (it holds GTA5.exe)' }
if (-not $Gta -or -not (Test-Path (Join-Path $Gta 'GTA5.exe'))) { Fail 'GTA V was not found: run the installer again and pick the folder that holds GTA5.exe.' }
$version = (Get-Item (Join-Path $Gta 'GTA5.exe')).VersionInfo.FileVersion
if ($version -and $version -ne $GtaBuild) { Fail "GTA V is $version; the mod needs GTA V Legacy $GtaBuild." }
if (Get-Process GTA5, PlayGTAV -ErrorAction SilentlyContinue) { Fail 'GTA V is running: close it first.' }
Remember 'Gta' $Gta

if (-not $Skate3) { $Skate3 = Known 'Skate3' }
if (-not $Skate3 -or -not (Test-Path -LiteralPath $Skate3)) { $Skate3 = Pick-Skate3 }
if (-not $Skate3 -or -not (Test-Path -LiteralPath $Skate3)) { Fail 'No Skate 3 was picked.' }
Remember 'Skate3' $Skate3

if (-not $CodeWalker) { $CodeWalker = Known 'CodeWalker' }
if (-not $CodeWalker -or -not (Test-Path (Join-Path $CodeWalker 'CodeWalker.Core.dll'))) { $CodeWalker = Find-CodeWalker }
if (-not $CodeWalker) {
    [void][Windows.Forms.MessageBox]::Show(
        "The board models are packed with CodeWalker.`n`nDownload CodeWalker (https://www.gta5-mods.com/tools/codewalker-gtav-interactive-3d-map), unzip it, then pick its folder in the next window.",
        'GTAxSkate', 'OK', 'Information')
    $CodeWalker = Pick-Folder 'The CodeWalker folder (it holds CodeWalker.Core.dll)'
}
if (-not $CodeWalker -or -not (Test-Path (Join-Path $CodeWalker 'CodeWalker.Core.dll'))) { Fail 'CodeWalker was not found (the folder must hold CodeWalker.Core.dll).' }
Remember 'CodeWalker' $CodeWalker
$known | ConvertTo-Json | Set-Content -LiteralPath $settings

Say "  GTA V      $Gta" 'DarkGray'
Say "  Skate 3    $Skate3" 'DarkGray'
Say "  CodeWalker $CodeWalker" 'DarkGray'

# --- 1 tools -----------------------------------------------------------------
$release = Join-Path $external "sk8v-release\SK8V-$Sk8vTag"
$python = Join-Path $release 'python\python.exe'
Step 1 'Tools'
if (-not (Is-Done 'tools')) {
    if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
        Say '  installing Git (winget)...'
        Run 'Installing Git' { winget install --id Git.Git -e --silent --accept-package-agreements --accept-source-agreements }
        $env:PATH = [Environment]::GetEnvironmentVariable('PATH', 'Machine') + ';' + [Environment]::GetEnvironmentVariable('PATH', 'User')
    }
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Say '  installing Rust (rustup)...'
        $rustup = Join-Path $env:TEMP 'rustup-init.exe'
        Download 'https://win.rustup.rs/x86_64' $rustup
        Run 'Installing Rust' { & $rustup -y --default-toolchain stable --profile minimal }
        $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
    }
    if (-not (Test-Path $python)) {
        $zip = Join-Path $env:TEMP "SK8V-$Sk8vTag.zip"
        Download $Sk8vZip $zip
        Say '  unpacking the SK8V tools...' 'DarkGray'
        $unpacked = Join-Path $env:TEMP "SK8V-$Sk8vTag-unpacked"
        Remove-Item -LiteralPath $unpacked -Recurse -Force -ErrorAction SilentlyContinue
        Expand-Archive $zip $unpacked -Force
        Remove-Item $zip
        # The release's top folder (the one with tools\prepare.py), wherever the zip puts it.
        $top = Get-ChildItem $unpacked -Filter 'prepare.py' -File -Recurse -Depth 3 | Where-Object { $_.Directory.Name -eq 'tools' } | Select-Object -First 1
        if (-not $top) { Fail "The SK8V $Sk8vTag release has no tools\prepare.py." }
        New-Item -ItemType Directory -Force (Split-Path -Parent $release) | Out-Null
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
    if (-not (Test-Path $python)) { Fail "Python for the Skate 3 converter is missing ($python)." }
    if (-not (Get-ChildItem $tools -Recurse -Filter vgmstream-cli.exe -ErrorAction SilentlyContinue | Select-Object -First 1)) {
        $zip = Join-Path $tools 'vgmstream-win64.zip'
        Download 'https://github.com/vgmstream/vgmstream/releases/latest/download/vgmstream-win64.zip' $zip
        Expand-Archive $zip (Join-Path $tools 'vgmstream') -Force
        Remove-Item $zip
    }
    Done 'tools'
}
Say '  ready' 'Green'

# --- 2 data ------------------------------------------------------------------
$data = Join-Path $Gta 'GTAxSkate'
$assets = Join-Path $data 'skate-data\assets'
Step 2 'Skate 3 data (a few minutes)'
if (-not (Is-Done 'data')) {
    # SK8V's converter: the disc read and converted, nothing installed.
    Push-Location $release
    # Its intermediates (disc and GTA extractions) go afterwards, unless the
    # disc came as an ISO: the boards and sounds read its extracted copy.
    $clean = if (Test-Path -LiteralPath $Skate3 -PathType Container) { @('--clean') } else { @() }
    try { Run 'Converting Skate 3' { & $python tools\prepare.py --skate $Skate3 --gta $Gta --out $data @clean } }
    finally { Pop-Location }
    if (-not (Test-Path (Join-Path $assets 'private\skater.glb'))) { Fail "The converted data has no private\skater.glb ($assets)." }
    Done 'data'
}
# The extracted disc (for the boards and sounds): the folder given, or the
# converter's copy of the ISO.
$disc = if (Test-Path -LiteralPath $Skate3 -PathType Container) { $Skate3 } else {
    Get-ChildItem (Join-Path $data 'disc') -Filter 'default.xex' -File -Recurse -Depth 2 -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty DirectoryName
}
if (-not $disc) { Fail 'The extracted Skate 3 disc was not found (the converter keeps it under GTAxSkate\disc).' }
Say '  ready' 'Green'

# --- 3 engine ----------------------------------------------------------------
$sk8v = Join-Path $external 'sk8v'
$overlay = Join-Path $sk8v 'build\overlay\skate\crates\skate-host'
Step 3 'Skate engine'
if (-not (Is-Done 'engine')) {
    if (-not (Test-Path (Join-Path $sk8v '.git'))) {
        Run 'Downloading SK8V''s source' { git clone --quiet $Sk8vRepo $sk8v }
    }
    # Our own fork (a developer's) keeps its branch; a fresh clone takes the pinned commit.
    $branch = git -C $sk8v rev-parse --abbrev-ref HEAD
    if ($branch -ne 'skatemods') {
        Run 'Checking out SK8V' { git -C $sk8v -c advice.detachedHead=false checkout --quiet $Sk8vCommit }
        Copy-Item (Join-Path $root 'gta\engine\patches\*.patch') (Join-Path $sk8v 'patches\skate') -Force
    }
    Push-Location $sk8v
    try {
        Run 'Downloading the engine sources' { powershell -NoProfile -ExecutionPolicy Bypass -File tools\fetch-upstreams.ps1 }
        Run 'Building the engine sources' { powershell -NoProfile -ExecutionPolicy Bypass -File tools\prepare-skate-overlay.ps1 -Force }
    } finally { Pop-Location }
    if (-not (Test-Path (Join-Path $overlay 'Cargo.toml'))) { Fail 'The engine sources were not prepared.' }
    Done 'engine'
}
Say '  ready' 'Green'

# --- 4 plugin ----------------------------------------------------------------
$asi = Join-Path $build 'SkateGTA.asi'
Step 4 'SkateGTA.asi (the first build takes 10-30 minutes)'
if (-not (Is-Done 'plugin')) {
    Run 'Building SkateGTA.asi' { cargo build --release --manifest-path (Join-Path $root 'gta\skate-gta-sk8v\Cargo.toml') }
    Copy-Item (Join-Path $root 'gta\skate-gta-sk8v\target\release\skate_gta.dll') $asi -Force
    Run 'Building the board tool' { cargo build --release --manifest-path (Join-Path $root 'gta\skate-ped\Cargo.toml') }
    Done 'plugin'
}
$ped = Join-Path $root 'gta\skate-ped\target\release\skate-ped.exe'
Say '  ready' 'Green'

# --- 5 boards ----------------------------------------------------------------
$models = Join-Path $build 'board-models'
Step 5 'Boards: the library and the HD models (a few minutes)'
if (-not (Is-Done 'boards')) {
    if (-not (Test-Path (Join-Path $assets 'private\boards\library.json'))) {
        $engine = Join-Path $release 'upstream\skate-3-rust-engine-donor'
        Run 'Preparing the board library' { & $ped --assets $assets --prepare-boards --game $disc --engine $engine --python $python }
    }
    $customDecks = Join-Path $root 'custom-assets\decks'
    if (Get-ChildItem $customDecks -Recurse -File -Include *.png, *.jpg, *.jpeg, *.bmp, *.tga, *.dds, *.webp -ErrorAction SilentlyContinue) {
        Run 'Adding your decks' { & $ped --assets $assets --import-decks $customDecks --out $models }
    }
    Run 'Making the board models' { & $ped --assets $assets --out $models --weapon --game $disc }
    Run 'Packing the board models' { & (Join-Path $PSScriptRoot 'pack-skate-skater.ps1') -Gta $Gta -CodeWalker $CodeWalker -Weapon $models -Mode weapon }
    Done 'boards'
}
Say '  ready' 'Green'

# --- 6 sounds ----------------------------------------------------------------
Step 6 'Board sounds'
if (-not (Is-Done 'sounds')) {
    Run 'Converting the board sounds' { & (Join-Path $PSScriptRoot 'get-skate-sounds.ps1') -Game $disc -Assets $assets }
    Done 'sounds'
}
Say '  ready' 'Green'

# --- 7 parks -----------------------------------------------------------------
Step 7 'Skate parks'
if (-not (Is-Done 'parks')) {
    Run 'Reading the skate parks' { & (Join-Path $PSScriptRoot 'get-skate-parks.ps1') -Gta $Gta -CodeWalker $CodeWalker -Out (Join-Path $assets 'parks') }
    Done 'parks'
}
Say '  ready' 'Green'

# --- 8 install ---------------------------------------------------------------
Step 8 'Installing into GTA V'
if (-not (Test-Path (Join-Path $Gta 'ScriptHookV.dll'))) {
    Say '  Script Hook V is missing: fetching it (SK8V''s fetcher)...'
    Push-Location $sk8v
    try { Run 'Getting Script Hook V' { powershell -NoProfile -ExecutionPolicy Bypass -File tools\fetch-scripthookv.ps1 } }
    finally { Pop-Location }
    foreach ($dll in Get-ChildItem (Join-Path $sk8v 'third_party\scripthookv\runtime') -Filter '*.dll' -File -ErrorAction SilentlyContinue) {
        Copy-Item $dll.FullName (Join-Path $Gta $dll.Name)
    }
}
if (-not (Test-Path (Join-Path $Gta 'ScriptHookV.dll'))) { Fail 'Script Hook V is missing: get it from https://www.dev-c.com/gtav/scripthookv/ and put ScriptHookV.dll and dinput8.dll into the GTA folder.' }
if (-not (Test-Path (Join-Path $Gta 'OpenIV.asi'))) {
    Fail 'OpenIV.asi is missing (the board models load from the mods folder): install OpenIV (https://openiv.com), then Tools > ASI Manager > OpenIV.ASI.'
}
Copy-Item $asi (Join-Path $Gta 'SkateGTA.asi') -Force
# The settings: the player's own kept, the data folder set.
$ini = Join-Path $Gta 'SkateGTA.ini'
$line = "SkateAssets = $assets"
if (Test-Path $ini) {
    $text = [IO.File]::ReadAllText($ini)
    $text = if ($text -match '(?m)^SkateAssets\s*=.*$') { $text -replace '(?m)^SkateAssets\s*=.*$', $line.Replace('$', '$$') } else { "$line`r`n$text" }
    [IO.File]::WriteAllText($ini, $text)
} else {
    [IO.File]::WriteAllText($ini, "$line`r`n")
}
Done 'install'

Say ''
Say 'GTAxSkate is installed.' 'Green'
Say 'In GTA V (story mode): pick the skateboard in the weapon wheel, then F to skate. E gets on and off, G opens the board shop in clothing stores, F10 the options.' 'White'
