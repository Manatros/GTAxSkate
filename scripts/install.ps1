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
#   5 music    Skate 3's soundtrack (the iPod's 46 songs) on GTA's Self Radio
#              (-NoMusic skips it)
#   6 install  our SK8V build over SK8V's (recorded in SK8V's install record,
#              so SK8V's own uninstall and updates treat it as theirs), the
#              settings; then what only the setup needed is removed
#
# The data lives where SK8V keeps it (GTA\SK8V; GTA\GTAxSkate for installs
# made before), so an SK8V already set up is reused.
#
# ASCII only: Windows PowerShell 5.1 reads a script without BOM as ANSI.
param(
    [string] $Gta = '',
    [string] $Skate3 = '',
    [ValidateSet('', 'tools', 'sk8v', 'bin', 'boards', 'music', 'install')] [string] $Redo = '',
    # Stop after this step (testing, or preparing without touching GTA yet).
    [ValidateSet('', 'tools', 'sk8v', 'bin', 'boards', 'music', 'install')] [string] $Until = '',
    [switch] $NoMusic,
    # Which parts to install (names as in the list below, comma separated), or
    # 'all'; without it the installer asks once with a list of tick boxes.
    [string] $Parts = '',
    # Ask again which parts (else the last choice is kept).
    [switch] $Choose,
    # Internal: the elevated continuation appends to the first window's log.
    [switch] $ResumeTranscript
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
# Windows PowerShell 5.1 on older Windows 10 offers only TLS 1.0 to GitHub.
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
$root = Split-Path -Parent $PSScriptRoot
$build = Join-Path $root 'build'
$receipts = Join-Path $build 'install'
$tools = Join-Path $root 'tools'
$external = Join-Path $root 'external'
$bin = Join-Path $build 'bin'
$settings = Join-Path $receipts 'folders.json'
New-Item -ItemType Directory -Force $receipts, $tools, $external, $bin | Out-Null
# Everything this window shows, for a bug report (SK8V's own output also
# goes to sk8v-setup.log).
try {
    if ($ResumeTranscript) { Start-Transcript -LiteralPath (Join-Path $receipts 'install.log') -Append | Out-Null }
    else { Start-Transcript -LiteralPath (Join-Path $receipts 'install.log') -Force | Out-Null }
} catch { }
# SK8V: pinned to the release our additions are built on.
$Sk8vTag = '1.1.10'
$Sk8vZip = "https://github.com/becaring/SK8V/releases/download/$Sk8vTag/SK8V-$Sk8vTag.zip"
# SHA-256 digests verified against the official GitHub release asset metadata.
# These detect unexpected/corrupted downloads; they are not code signatures.
$Sk8vSha256 = '12020c3335d1329007c3988105c76b140bcee5c1dd03dfbed03a168ddd054d74'
$VgmstreamZip = 'https://github.com/vgmstream/vgmstream/releases/download/r2117/vgmstream-win64.zip'
$VgmstreamSha256 = '6c4a8a3813864fefed081bbd337dbc0ad93bf88e0b92f5db98d7ab258b22dc6c'
# GTAxSkate's own builds for a source checkout without a bin folder: the
# game-accepted release. Update tag and digest together with each release.
$GtaxSkateRelease = 'v0.6.0'
$GtaxSkateBinZip = "https://github.com/Manatros/GTAxSkate/releases/download/$GtaxSkateRelease/GTAxSkate-bin.zip"
$GtaxSkateBinSha256 = 'ee2db3fc355eb7c8479bdaeeb486a3584eb388d498573301dfd20b5d6ac647fd'
$GtaBuild = '1.0.3889.0'
$Steps = 'tools', 'sk8v', 'bin', 'boards', 'music', 'install'

Add-Type -AssemblyName System.Windows.Forms

function Say([string]$text, [string]$color = 'Gray') { Write-Host $text -ForegroundColor $color }
function Fail([string]$text) {
    Say ''
    Say "Stopped: $text" 'Red'
    Say 'Run "Install GTAxSkate.cmd" again once it is fixed; it goes on where it stopped.' 'Yellow'
    Say 'For help, copy only the relevant error lines from build\install\install.log or sk8v-setup.log.' 'Yellow'
    Say 'Review them first: remove your user name, personal folder paths, server addresses and any credentials before sharing.' 'Yellow'
    try { Stop-Transcript | Out-Null } catch { }
    exit 1
}
# Anything unexpected: a plain message instead of a PowerShell error page.
trap { Fail "$($_.Exception.Message) (line $($_.InvocationInfo.ScriptLineNumber))" }
function Done([string]$step) { Set-Content -LiteralPath (Join-Path $receipts "$step.done") (Get-Date -Format s) }
function Is-Done([string]$step) {
    if ($Redo -and [array]::IndexOf($Steps, $step) -ge [array]::IndexOf($Steps, $Redo)) { return $false }
    Test-Path -LiteralPath (Join-Path $receipts "$step.done")
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
function Get-VerifiedArchive([string]$url, [string]$to, [string]$sha256) {
    if ($sha256 -notmatch '^[a-fA-F0-9]{64}$') { throw 'Invalid pinned download checksum.' }
    if (Test-Path -LiteralPath $to) {
        if ((Get-FileHash -LiteralPath $to -Algorithm SHA256).Hash -ne $sha256) {
            Remove-Item -LiteralPath $to -Force
            throw 'Cached download checksum mismatch; cache removed. Run the installer again to download a fresh copy.'
        }
        return
    }
    $partial = "$to.download-$([guid]::NewGuid().ToString('N'))"
    try {
        Download $url $partial
        if ((Get-FileHash -LiteralPath $partial -Algorithm SHA256).Hash -ne $sha256) {
            throw 'Download checksum mismatch; installation stopped before extraction or execution.'
        }
        Move-Item -LiteralPath $partial -Destination $to -Force
    } finally {
        if (Test-Path -LiteralPath $partial) { Remove-Item -LiteralPath $partial -Force }
    }
}
# Dialogs open in front: without an owner they can hide behind this window.
function Owner {
    $f = New-Object System.Windows.Forms.Form
    $f.TopMost = $true; $f.ShowInTaskbar = $false; $f.Opacity = 0; $f.StartPosition = 'CenterScreen'
    $f.Show(); $f.Activate(); $f
}
function Pick-Folder([string]$prompt) {
    $dialog = New-Object System.Windows.Forms.FolderBrowserDialog
    $dialog.Description = $prompt
    $owner = Owner
    try { if ($dialog.ShowDialog($owner) -eq 'OK') { $dialog.SelectedPath } else { '' } } finally { $owner.Close() }
}
function Pick-Skate3 {
    $dialog = New-Object System.Windows.Forms.OpenFileDialog
    $dialog.Title = 'Your Skate 3 (Xbox 360) disc image, or Cancel to pick its extracted folder'
    $dialog.Filter = 'Disc image (*.iso)|*.iso|All files|*.*'
    $owner = Owner
    try { if ($dialog.ShowDialog($owner) -eq 'OK') { return $dialog.FileName } } finally { $owner.Close() }
    Pick-Folder 'Your extracted Skate 3 folder (it holds default.xex)'
}
function Free-GB([string]$path) {
    try { [math]::Round((New-Object IO.DriveInfo ([IO.Path]::GetPathRoot((Resolve-Path -LiteralPath $path).Path))).AvailableFreeSpace / 1GB, 1) } catch { 999 }
}
# The player's GTA folder may need administrator rights (Program Files, or
# a repaired install): start again elevated with the same arguments, as SK8V's
# own setup offers.
function Writable([string]$folder) {
    try { $probe = Join-Path $folder ".gtaxskate-write-test-$PID"; [IO.File]::WriteAllBytes($probe, @()); Remove-Item -LiteralPath $probe; $true } catch { $false }
}
function Restart-Elevated {
    $admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    if ($admin) { return }
    Say '  The GTA V folder needs administrator rights: starting again as administrator (confirm the Windows prompt)...' 'Yellow'
    # The same launcher (it pauses at the end), with the answers given so far.
    $argsList = "-Gta `"$Gta`" -Skate3 `"$Skate3`""
    if ($Redo) { $argsList += " -Redo $Redo" }
    if ($Until) { $argsList += " -Until $Until" }
    if ($NoMusic) { $argsList += ' -NoMusic' }
    $argsList += ' -ResumeTranscript'
    try { Start-Process (Join-Path $root 'Install GTAxSkate.cmd') -Verb RunAs -ArgumentList $argsList | Out-Null }
    catch { Fail "No write access to $Gta and administrator rights were declined." }
    Say '  Continued in the administrator window.' 'Yellow'
    try { Stop-Transcript | Out-Null } catch { }
    exit 0
}

# --- The folders: given, remembered from the last run, found, or asked ------
$known = if (Test-Path -LiteralPath $settings) { Get-Content $settings -Raw | ConvertFrom-Json } else { [pscustomobject]@{} }
function Remember([string]$name, [string]$value) { $known | Add-Member -NotePropertyName $name -NotePropertyValue $value -Force }
function Known([string]$name) { $p = $known.PSObject.Properties[$name]; if ($p) { [string]$p.Value } else { '' } }

function Find-Gta {
    foreach ($key in 'HKLM:\SOFTWARE\WOW6432Node\Rockstar Games\Grand Theft Auto V', 'HKLM:\SOFTWARE\WOW6432Node\Rockstar Games\GTAV') {
        $p = (Get-ItemProperty $key -ErrorAction SilentlyContinue).InstallFolder
        if ($p -and (Test-Path -LiteralPath (Join-Path $p 'GTA5.exe'))) { return $p }
    }
    foreach ($drive in Get-PSDrive -PSProvider FileSystem) {
        foreach ($rel in 'Games\Grand Theft Auto V Legacy', 'Program Files\Rockstar Games\Grand Theft Auto V',
                         'Program Files (x86)\Steam\steamapps\common\Grand Theft Auto V', 'SteamLibrary\steamapps\common\Grand Theft Auto V',
                         'Program Files\Epic Games\GTAV') {
            $p = Join-Path $drive.Root $rel
            if (Test-Path -LiteralPath (Join-Path $p 'GTA5.exe')) { return $p }
        }
    }
    ''
}

Say 'GTAxSkate installer' 'White'
Say 'Install logs and remembered folder choices stay locally in build\install; this installer does not upload them.' 'DarkGray'
Say 'Logs can contain personal folder paths. Review and redact excerpts before sharing them publicly.' 'DarkGray'
if (-not $Gta) { $Gta = Known 'Gta' }
if (-not $Gta) { $Gta = Find-Gta }
if (-not $Gta -or -not (Test-Path -LiteralPath (Join-Path $Gta 'GTA5.exe'))) { $Gta = Pick-Folder 'Your GTA V Legacy folder (it holds GTA5.exe)' }
if (-not $Gta -or -not (Test-Path -LiteralPath (Join-Path $Gta 'GTA5.exe'))) { Fail 'GTA V Legacy (GTA5.exe) was not found.' }
$version = (Get-Item -LiteralPath (Join-Path $Gta 'GTA5.exe')).VersionInfo.FileVersion
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

# --- Which parts: ticked in a list (remembered), or -Parts ----------------------
# Name, what it is, on by default, its INI switches (value when on / off).
$catalog = @(
    @{ N = 'shop';    T = 'Skate 3 board shop and every board part (G in clothing stores)'; On = $true;  Ini = @{ BoardShop = '1', '0'; WheelsFromPaint = '1', '0' } },
    @{ N = 'music';   T = 'Skate 3 soundtrack on Self Radio';                                On = $true;  Ini = @{} },
    @{ N = 'keys';    T = 'Keyboard and mouse (keys shown on screen)';                       On = $true;  Ini = @{ Keyboard = '1', '0'; KeyboardHints = '1', '0' } },
    @{ N = 'camera';  T = 'Third person camera like GTA (else Skate 3''s camera)';           On = $true;  Ini = @{ CameraView = 'Near', 'Skate'; CameraRemember = '1', '0' } },
    @{ N = 'cars';    T = 'Ride and hippie jump cars';                                       On = $true;  Ini = @{ VehicleShapes = '1', '0'; HippieJump = '1.6', '1'; VehiclePush = '0.4', '1'; VehicleDamageScale = '0.2', '0.5'; VehicleDamageRadius = '40', '150'; VehicleDamageMin = '600', '200' } },
    @{ N = 'map';     T = 'GPS route while skating, skate parks on the map';                 On = $true;  Ini = @{ GpsWhileSkating = '1', '0'; SkateSpots = '1', '0' } },
    @{ N = 'stamina'; T = 'Stamina: skating trains it, it sets push and pop';                On = $true;  Ini = @{ Stamina = '1', '0' } },
    @{ N = 'peds';    T = 'People knocked over survive';                                     On = $true;  Ini = @{ PedsSurvive = '1', '0' } },
    @{ N = 'hurt';    T = 'A limp after hard bails';                                         On = $true;  Ini = @{ Injuries = '1', '0' } },
    @{ N = 'carry';   T = 'The board from the weapon wheel stays in hand (swing it; E or F gets on)'; On = $false; Ini = @{ CarryBoard = '1', '0' } }
)
# Stance, as SK8V's own setup asks it: the last answer, else the installed SK8V's.
$Stance = Known 'Stance'
if (-not $Stance) {
    $iniNow = Join-Path $Gta 'SkateVLegacy.ini'
    $Stance = if ((Test-Path -LiteralPath $iniNow) -and ([IO.File]::ReadAllText($iniNow) -match '(?m)^Stance=Goofy')) { 'Goofy' } else { 'Regular' }
}
function Choose-Parts {
    $form = New-Object System.Windows.Forms.Form
    $form.Text = 'GTAxSkate: what to install'; $form.Width = 620; $form.Height = 430; $form.StartPosition = 'CenterScreen'; $form.TopMost = $true
    $list = New-Object System.Windows.Forms.CheckedListBox
    $list.Dock = 'Fill'; $list.CheckOnClick = $true
    foreach ($c in $catalog) { [void]$list.Items.Add($c.T, $c.On) }
    $stanceBox = New-Object System.Windows.Forms.FlowLayoutPanel
    $stanceBox.Dock = 'Bottom'; $stanceBox.Height = 34
    $label = New-Object System.Windows.Forms.Label
    $label.Text = 'Stance:'; $label.AutoSize = $true; $label.Padding = '4,6,4,0'
    $regular = New-Object System.Windows.Forms.RadioButton
    $regular.Text = 'Regular (left foot forward)'; $regular.AutoSize = $true; $regular.Checked = $Stance -ne 'Goofy'
    $goofy = New-Object System.Windows.Forms.RadioButton
    $goofy.Text = 'Goofy (right foot forward)'; $goofy.AutoSize = $true; $goofy.Checked = $Stance -eq 'Goofy'
    $stanceBox.Controls.AddRange(@($label, $regular, $goofy))
    $ok = New-Object System.Windows.Forms.Button
    $ok.Text = 'Install'; $ok.Dock = 'Bottom'; $ok.Height = 36; $ok.DialogResult = 'OK'
    $form.Controls.Add($list); $form.Controls.Add($stanceBox); $form.Controls.Add($ok); $form.AcceptButton = $ok
    if ($form.ShowDialog() -ne 'OK') { Fail 'Cancelled.' }
    $script:Stance = if ($goofy.Checked) { 'Goofy' } else { 'Regular' }
    ($catalog | Where-Object { $list.CheckedItems.Contains($_.T) } | ForEach-Object { $_.N }) -join ','
}
if (-not $Parts) { $Parts = Known 'Parts' }
if (-not $Parts -or $Choose) { $Parts = Choose-Parts }
Remember 'Stance' $Stance
if ($Parts -eq 'all') { $Parts = ($catalog | ForEach-Object { $_.N }) -join ',' }
$chosen = $Parts -split ',' | ForEach-Object { $_.Trim() }
Remember 'Parts' $Parts
$known | ConvertTo-Json | Set-Content $settings
function Wants([string]$name) { $chosen -contains $name }
Say "  Parts:  $(($chosen | Where-Object { $_ }) -join ', ')"
Say "  Stance: $Stance"
if (-not (Writable $Gta)) { Restart-Elevated }
if (-not (Writable $Gta)) { Fail "No write access to $Gta, even as administrator (is the drive read-only, or an antivirus blocking it?)." }

function Stop-After([string]$step) { if ($Until -eq $step) { Say ''; Say "Stopped after '$step' (-Until)." 'Yellow'; exit 0 } }

# --- 1 tools -----------------------------------------------------------------
$release = Join-Path $external "sk8v-release\SK8V-$Sk8vTag"
$python = Join-Path $release 'python\python.exe'
$rage = Join-Path $release 'build\tools\rage.exe'
Step 1 'Tools'
if (-not (Is-Done 'tools') -or -not (Test-Path -LiteralPath $python) -or -not (Test-Path -LiteralPath $rage)) {
    if (-not (Test-Path -LiteralPath (Join-Path $release 'tools\prepare.py'))) {
        $zip = Join-Path $env:TEMP "SK8V-$Sk8vTag.zip"
        Get-VerifiedArchive $Sk8vZip $zip $Sk8vSha256
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
    if (-not (Test-Path -LiteralPath $python)) {
        # SK8V's own Python fetcher: python.org's embeddable Python and the
        # numpy/Pillow wheels, each checked against its SHA-256 (about 31 MB).
        Push-Location $release
        try { Run 'Getting Python' { powershell -NoProfile -ExecutionPolicy Bypass -File tools\get-python.ps1 } }
        finally { Pop-Location }
    }
    if (-not (Test-Path -LiteralPath $python)) { Fail "SK8V's Python is missing ($python)." }
    if (-not (Test-Path -LiteralPath $rage)) { Fail "SK8V's rage tool is missing ($rage)." }
    if (-not (Get-ChildItem $tools -Recurse -Filter vgmstream-cli.exe -ErrorAction SilentlyContinue | Select-Object -First 1)) {
        $zip = Join-Path $tools 'vgmstream-win64.zip'
        Get-VerifiedArchive $VgmstreamZip $zip $VgmstreamSha256
        Expand-Archive $zip (Join-Path $tools 'vgmstream') -Force
        Remove-Item $zip
    }
    Done 'tools'
}
# Pin the bundled Python even on resumed installs. A registered Python or
# Conda PYTHONHOME/PYTHONPATH must not replace SK8V's standard library/packages.
# These variables belong only to this installer process and its children.
$env:PYTHONHOME = Split-Path -Parent $python
$env:PYTHONPATH = $null
$env:PYTHONNOUSERSITE = '1'
Say '  ready' 'Green'
Stop-After 'tools'

# --- 2 sk8v ------------------------------------------------------------------
# SK8V's own data folder (GTA\SK8V), so an SK8V already set up is reused
# (its setup resumes in seconds, nothing twice on disk) and SK8V's uninstall
# notes stay right. Installs from before kept theirs in GTA\GTAxSkate.
$data = Join-Path $Gta 'SK8V'
if (Test-Path -LiteralPath (Join-Path $Gta 'GTAxSkate\skate-data\assets')) { $data = Join-Path $Gta 'GTAxSkate' }
$assets = Join-Path $data 'skate-data\assets'
Step 2 'SK8V: Skate 3 data from your disc, installed into GTA V (10-30 minutes)'
$iniKeep = Join-Path $receipts 'SkateVLegacy.ini.kept'
if (-not (Is-Done 'sk8v')) {
    # SK8V's setup checks these too, but its message scrolls away above
    # its other output: say it here, plainly, before it starts.
    if ($version -ne $GtaBuild) {
        Fail "SK8V needs GTA V Legacy $GtaBuild; your GTA5.exe is $(if ($version) { $version } else { 'unknown' }). GTA V Enhanced does not work."
    }
    if (-not (Test-Path -LiteralPath (Join-Path $Gta 'update\update.rpf'))) { Fail 'update\update.rpf is missing: verify GTA V in its launcher first.' }
    if (-not (Test-Path -LiteralPath (Join-Path $Gta 'ScriptHookV.dll'))) {
        Say ''
        Say '  Script Hook V is missing (no ScriptHookV.dll in the GTA folder).' 'Yellow'
        Say '  Download it from http://www.dev-c.com/gtav/scripthookv/ and copy' 'Yellow'
        Say '  ScriptHookV.dll and dinput8.dll from its bin folder into:' 'Yellow'
        Say "  $Gta" 'Yellow'
        Fail 'Script Hook V is not installed.'
    }
    # An ASI loader: Script Hook V's dinput8.dll, or another (Ultimate ASI Loader names).
    if (-not ('dinput8.dll', 'version.dll', 'winmm.dll', 'dsound.dll', 'xinput1_3.dll' | Where-Object { Test-Path -LiteralPath (Join-Path $Gta $_) })) {
        Say '  Note: no ASI loader found (dinput8.dll from Script Hook V''s bin folder): without one GTA loads no mods.' 'Yellow'
    }
    $hookVersion = (Get-Item (Join-Path $Gta 'ScriptHookV.dll')).VersionInfo.FileVersion
    if ($hookVersion -and $hookVersion -notlike '3889.*') {
        Say "  Note: your Script Hook V is $hookVersion; GTA V $GtaBuild needs v3889.0 (1158.13 or newer). Update it if GTA shows a Script Hook V error." 'Yellow'
    }
    # SK8V asks for about 7 GB free (2 GB stay); the board parts add about 1 GB.
    $free = Free-GB $Gta
    $need = if (Test-Path -LiteralPath (Join-Path $data '.stages')) { 3 } else { 8 }
    if ($free -lt $need) { Fail "Only $free GB free on the GTA V drive; the setup needs about $need GB. Free some space and run it again." }
    $iniNow = Join-Path $Gta 'SkateVLegacy.ini'
    if (Test-Path -LiteralPath $iniNow) { Copy-Item $iniNow $iniKeep -Force }
    # SK8V's own setup does everything SK8V needs (data, its DLC pack, its
    # ini). The disc stays extracted: the boards and the music read it. Its
    # output also goes to a log, and on failure its last lines are shown
    # again (the reason is easily scrolled away).
    $sk8vLog = Join-Path $receipts 'sk8v-setup.log'
    $env:PYTHONUNBUFFERED = '1'
    # Piped, Python would write the console's code page and stop at the
    # first character it lacks (a name in a path).
    $env:PYTHONIOENCODING = 'utf-8'
    Push-Location $release
    try {
        Say '  Running SK8V''s setup...' 'DarkGray'
        cmd /s /c " ""$python"" tools\prepare.py --skate ""$Skate3"" --gta ""$Gta"" --out ""$data"" --stance $Stance --install 2>&1 " | Tee-Object -FilePath $sk8vLog
        $code = $LASTEXITCODE
    }
    finally { Pop-Location }
    if ($code) {
        Say ''
        Say "  SK8V's setup said (last lines, all in $sk8vLog):" 'Yellow'
        Get-Content $sk8vLog -Tail 15 | Where-Object { $_ -notmatch '^@progress' } | ForEach-Object { Say "    $_" 'Yellow' }
        Fail "Running SK8V's setup failed (exit code $code)."
    }
    if (-not (Test-Path -LiteralPath (Join-Path $assets 'private\skater.glb'))) { Fail "SK8V's setup left no private\skater.glb ($assets)." }
    if (-not (Test-Path -LiteralPath (Join-Path $Gta 'SkateVLegacy.asi'))) { Fail 'SK8V was not installed (no SkateVLegacy.asi in the GTA folder).' }
    Done 'sk8v'
}
# The extracted disc for the boards and the music: the folder given, or SK8V's
# copy of the ISO (GTA\SK8V\disc). SK8V copies only the files it needs, so
# what we read beyond them comes from the ISO here (SK8V's own reader).
function Disc-With([string[]]$files) {
    if (Test-Path -LiteralPath $Skate3 -PathType Container) { return $Skate3 }
    if (-not (Test-Path -LiteralPath $Skate3)) { throw "Your Skate 3 disc image ($Skate3) is gone: run again and pick it (-Choose is not needed)." }
    $copy = Join-Path $data 'disc'
    $missing = @('default.xex') + $files | Where-Object { -not (Test-Path -LiteralPath (Join-Path $copy $_)) }
    if ($missing) {
        Say "  copying $($missing.Count) file(s) from the disc image..." 'DarkGray'
        $code = "import sys; sys.path.insert(0, sys.argv[1]); import xiso; from pathlib import Path; d = xiso.Disc(sys.argv[2])`n" +
                "for p in sys.argv[4:]: d.extract(p, Path(sys.argv[3]) / p)"
        & $python -c $code (Join-Path $release 'tools') $Skate3 $copy @($missing)
        if ($LASTEXITCODE) { throw "Could not copy $($missing -join ', ') from $Skate3." }
    }
    $copy
}
Say '  ready' 'Green'
Stop-After 'sk8v'

# --- 3 bin -------------------------------------------------------------------
Step 3 'GTAxSkate builds'
$sk8vSrc = Join-Path $external 'sk8v'
$package = Join-Path $sk8vSrc 'build\package'
$shipped = Join-Path $root 'bin'
# A developer's newer build counts as a new 'bin' step.
$newer = (Test-Path -LiteralPath (Join-Path $package 'SkateVLegacy.asi')) -and (Test-Path -LiteralPath (Join-Path $bin 'SkateVLegacy.asi')) -and
         ((Get-Item (Join-Path $package 'SkateVLegacy.asi')).LastWriteTime -gt (Get-Item (Join-Path $bin 'SkateVLegacy.asi')).LastWriteTime)
if (-not (Is-Done 'bin') -or $newer) {
    $ped = Join-Path $root 'gta\skate-ped\target\release\skate-ped.exe'
    $music = Join-Path $root 'gta\skate-music\target\release\skate-music.exe'
    if ((Test-Path -LiteralPath (Join-Path $package 'SkateVLegacy.asi')) -and (Test-Path -LiteralPath $ped)) {
        # A developer's checkout: the builds made here.
        Copy-Item (Join-Path $package 'SkateVLegacy.asi'), (Join-Path $package 'SkateVRuntime.dll') $bin -Force
        Copy-Item (Join-Path $sk8vSrc 'tools\prepare-board-parts.py'), (Join-Path $sk8vSrc 'tools\_common.py') $bin -Force
        Copy-Item $ped $bin -Force
        if (Test-Path -LiteralPath $music) { Copy-Item $music $bin -Force }
    } elseif (Test-Path -LiteralPath (Join-Path $shipped 'SkateVLegacy.asi')) {
        # The complete package: the builds came in its bin folder.
        Copy-Item (Join-Path $shipped '*') $bin -Recurse -Force
    } else {
        # A source checkout: the accepted release's builds, checked against the pinned digest.
        $binZip = Join-Path $build "GTAxSkate-bin-$GtaxSkateRelease.zip"
        Get-VerifiedArchive $GtaxSkateBinZip $binZip $GtaxSkateBinSha256
        Expand-Archive -LiteralPath $binZip -DestinationPath $bin -Force
    }
    foreach ($f in 'SkateVLegacy.asi', 'SkateVRuntime.dll', 'skate-ped.exe', 'prepare-board-parts.py', 'skate-music.exe') {
        if (-not (Test-Path -LiteralPath (Join-Path $bin $f))) { Fail "The GTAxSkate builds have no $f." }
    }
    Done 'bin'
}
Say '  ready' 'Green'
Stop-After 'bin'

# --- 4 boards ----------------------------------------------------------------
# The exported models (about 0.8 GB, only until the part models are built)
# go on the GTA drive, where the free space was checked, not beside the installer.
$models = Join-Path $data 'board-models'
$ped = Join-Path $bin 'skate-ped.exe'
Step 4 'Boards: the library and every part as a GTA model (a few minutes)'
if (-not (Wants 'shop')) { Say '  skipped (board shop not chosen)' 'DarkGray' }
elseif (-not (Is-Done 'boards')) {
    $disc = Disc-With @('data/content/createacharacter.big')
    if (-not (Test-Path -LiteralPath (Join-Path $assets 'private\boards\library.json'))) {
        $engine = Join-Path $release 'upstream\skate-3-rust-engine-donor'
        Run 'Preparing the board library' { & $ped --assets $assets --prepare-boards --game $disc --engine $engine --python $python }
    }
    $customDecks = Join-Path $root 'custom-assets\decks'
    if (Get-ChildItem $customDecks -Recurse -File -Include *.png, *.jpg, *.jpeg, *.bmp, *.tga, *.dds, *.webp -ErrorAction SilentlyContinue) {
        Run 'Adding your own decks' { & $ped --assets $assets --import-decks $customDecks --out $models }
    }
    Run 'Exporting the board parts' { & $ped --assets $assets --out $models --weapon --game $disc }
    $native = Join-Path $assets 'private\board-native\skatev_board.ydr.xml'
    if (-not (Test-Path -LiteralPath $native)) { Fail "SK8V's native board ($native) is missing: run with -Redo sk8v." }
    $parts = Join-Path $assets 'private\board-parts'
    Run 'Building the part models' {
        & $python (Join-Path $bin 'prepare-board-parts.py') --parts (Join-Path $models 'parts') --board $native `
            --board-json (Join-Path $assets 'private\board\board-materials.json') --rage $rage --out $parts
    }
    if (-not (Test-Path -LiteralPath (Join-Path $parts 'parts.json'))) { Fail 'The part models were not built.' }
    Remove-Item -LiteralPath $models -Recurse -Force -ErrorAction SilentlyContinue
    Done 'boards'
}
Say '  ready' 'Green'
Stop-After 'boards'

# --- 5 music -----------------------------------------------------------------
# The in-game iPod's 46 songs (data\audio\music); not part of SK8V, which
# plays GTA's radio while skating.
Step 5 'Skate 3 soundtrack on Self Radio (about a minute)'
if ($NoMusic -or -not (Wants 'music')) { Say '  skipped (not chosen)' 'DarkGray' }
elseif (-not (Is-Done 'music')) {
    # Optional: a failure is reported and left for the next run, the rest goes on.
    try {
        $disc = Disc-With @('data/audio/music/ipod.mpf', 'data/audio/music/Ipod_Stream.mus')
        & (Join-Path $PSScriptRoot 'get-skate-music.ps1') -Game $disc -Exe (Join-Path $bin 'skate-music.exe')
        Done 'music'
    } catch {
        Say "  the soundtrack was skipped: $($_.Exception.Message)" 'Yellow'
        Say '  To retry it, run the installer again with -Skate3 <disc image or folder>.' 'Yellow'
    }
}
Say '  ready' 'Green'
Stop-After 'music'

# --- 6 install ---------------------------------------------------------------
Step 6 'Installing into GTA V'
if (Get-Process GTA5 -ErrorAction SilentlyContinue) { Fail 'GTA V is running: close it first.' }
# SK8V's install record gets our two files' hashes: its uninstall then
# removes them and its next setup replaces them, as with its own (it keeps
# files that changed since it installed them).
$manifest = Join-Path $Gta 'SkateVLegacy.installed.json'
if (Test-Path -LiteralPath $manifest) {
    try { $record = Get-Content -LiteralPath $manifest -Raw | ConvertFrom-Json -ErrorAction Stop }
    catch { Fail "SK8V's install manifest is invalid ($manifest): $($_.Exception.Message). No files were copied." }
    if ($record -isnot [pscustomobject] -or $record.format -ne 'sk8v-install' -or $record.version -ne 1 -or
        $record.gta -ne $GtaBuild -or $record.files -isnot [pscustomobject]) {
        Fail "SK8V's install manifest has an unsupported or invalid structure ($manifest). No files were copied."
    }
    foreach ($entry in $record.files.PSObject.Properties) {
        $segments = $entry.Name -split '[/\\]'
        if ([string]::IsNullOrWhiteSpace($entry.Name) -or [IO.Path]::IsPathRooted($entry.Name) -or
            $entry.Name -match '^[A-Za-z]:' -or @($segments | Where-Object { $_ -eq '.' -or $_ -eq '..' }).Count -gt 0 -or
            ($null -ne $entry.Value -and $entry.Value -isnot [string])) {
            Fail "SK8V's install manifest contains an invalid file entry ('$($entry.Name)'). No files were copied."
        }
    }
} else {
    # Match SK8V's setup_engine.write_manifest schema so its next setup/uninstall
    # can manage these files even when no SK8V setup manifest existed yet.
    $record = [pscustomobject]@{ format = 'sk8v-install'; version = 1; gta = $GtaBuild; files = [pscustomobject]@{} }
}
$transactionId = [guid]::NewGuid().ToString('N')
$manifestBackup = "$manifest.rollback-$transactionId"
$manifestTemp = "$manifest.new-$transactionId"
$copyBackups = @{}
$rollbackFailed = $false
try {
    foreach ($f in 'SkateVLegacy.asi', 'SkateVRuntime.dll') {
        $destination = Join-Path $Gta $f
        if (Test-Path -LiteralPath $destination) {
            $backup = Join-Path $Gta (".$f.install-backup-$transactionId")
            Copy-Item -LiteralPath $destination -Destination $backup -Force
            $copyBackups[$destination] = $backup
        } else { $copyBackups[$destination] = $null }
        Copy-Item -LiteralPath (Join-Path $bin $f) -Destination $destination -Force
        $hash = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()
        $record.files | Add-Member -NotePropertyName $f -NotePropertyValue $hash -Force
    }
    [IO.File]::WriteAllText($manifestTemp, ($record | ConvertTo-Json -Depth 5), (New-Object Text.UTF8Encoding $false))
    if (Test-Path -LiteralPath $manifest) {
        Copy-Item -LiteralPath $manifest -Destination $manifestBackup -Force
        [IO.File]::Replace($manifestTemp, $manifest, $manifestBackup, $true)
    } else {
        [IO.File]::Move($manifestTemp, $manifest)
    }
} catch {
    foreach ($destination in $copyBackups.Keys) {
        $backup = $copyBackups[$destination]
        try {
            if ($backup -and (Test-Path -LiteralPath $backup)) { Copy-Item -LiteralPath $backup -Destination $destination -Force }
            elseif (Test-Path -LiteralPath $destination) { Remove-Item -LiteralPath $destination -Force }
        } catch {
            $rollbackFailed = $true
            Say "  rollback failed for $(Split-Path -Leaf $destination); backup retained at $backup" 'Red'
        }
    }
    if (Test-Path -LiteralPath $manifestBackup) {
        try { Copy-Item -LiteralPath $manifestBackup -Destination $manifest -Force }
        catch {
            $rollbackFailed = $true
            Say "  manifest rollback failed; backup retained at $manifestBackup" 'Red'
        }
    }
    if ($rollbackFailed) {
        Fail "Could not record the SK8V-owned install, and rollback was incomplete. Recovery backups were retained; original error: $($_.Exception.Message)."
    } else {
        Fail "Could not record the SK8V-owned install; copied files were rolled back ($($_.Exception.Message))."
    }
} finally {
    Remove-Item -LiteralPath $manifestTemp -Force -ErrorAction SilentlyContinue
    if (-not $rollbackFailed) {
        Remove-Item -LiteralPath $manifestBackup -Force -ErrorAction SilentlyContinue
        foreach ($backup in $copyBackups.Values) { if ($backup) { Remove-Item -LiteralPath $backup -Force -ErrorAction SilentlyContinue } }
    }
}
# An older GTAxSkate (its own plugin and packs) goes out of the way, kept as *.off.
foreach ($old in (Join-Path $Gta 'SkateGTA.asi'), (Join-Path $Gta 'SkateExtras.asi')) {
    if (Test-Path -LiteralPath $old) { Move-Item $old "$old.off" -Force; Say "  set aside $(Split-Path -Leaf $old) (now .off)" 'DarkGray' }
}
foreach ($pack in 'skate3weapon', 'skate3props') {
    $rpf = Join-Path $Gta "mods\update\x64\dlcpacks\$pack\dlc.rpf"
    if (Test-Path -LiteralPath $rpf) { Move-Item $rpf "$rpf.off" -Force; Say "  set aside the $pack pack (now .off)" 'DarkGray' }
}
# SK8V's ini (UTF-16, as SK8V writes it): our additions on; the player's own values kept.
$ini = Join-Path $Gta 'SkateVLegacy.ini'
$text = if (Test-Path -LiteralPath $ini) { [IO.File]::ReadAllText($ini) } else { "[SkateV]`r`n" }
$text = ($text -replace "`r+`n", "`n") -replace "`n", "`r`n"
if ($text -notmatch '(?m)^\[SkateV\]\r?$') { $text = "[SkateV]`r`n$text" }
function Add-Key([string]$key, [string]$value) {
    if ($script:text -notmatch "(?m)^$key=") { $script:text = $script:text -replace '(?m)^\[SkateV\]\r?$', "[SkateV]`r`n$key=$value" }
}
# The player's own values from before SK8V's setup that its template dropped.
if (Test-Path -LiteralPath $iniKeep) {
    $kept = ([IO.File]::ReadAllText($iniKeep) -replace "`r+`n", "`n") -split "`n"
    foreach ($line in $kept) { if ($line -match '^([A-Za-z0-9_]+)=(.*)$') { Add-Key $Matches[1] $Matches[2] } }
}
function Set-Key([string]$key, [string]$value) {
    if ($script:text -match "(?m)^$key=") { $script:text = $script:text -replace "(?m)^$key=.*$", "$key=$value" } else { Add-Key $key $value }
}
$defaults = [ordered]@{ DrawBoard = '1'; DrawSkater = '1' }
foreach ($key in $defaults.Keys) { Add-Key $key $defaults[$key] }
# The parts chosen: their switches on, the others off.
foreach ($c in $catalog) {
    foreach ($key in $c.Ini.Keys) {
        $on, $off = $c.Ini[$key]
        if (-not (Wants $c.N)) { Set-Key $key $off }
        # Chosen: on, unless the player set a value of their own (not the off one).
        elseif ($script:text -match "(?m)^$key=(.*)$" -and $Matches[1].Trim() -ne $off) { }
        else { Set-Key $key $on }
    }
}
# With the soundtrack on Self Radio, skating plays it.
if (-not $NoMusic -and (Wants 'music') -and (Is-Done 'music')) { Add-Key 'SkateRadio' 'RADIO_19_USER' }
foreach ($key in 'DrawBoard', 'DrawSkater') { $text = $text -replace "(?m)^$key=0", "$key=1" }
$text = ($text -replace "`r+`n", "`n") -replace "`n", "`r`n"
[IO.File]::WriteAllText($ini, $text, [Text.Encoding]::Unicode)
Done 'install'

# What only the setup needed, as SK8V's own --clean removes it (about 1-2 GB):
# its GTA extractions and, from an ISO, the copied disc files. Its receipts
# stay; a later run copies again what it needs.
Remove-Item -LiteralPath (Join-Path $data 'gta') -Recurse -Force -ErrorAction SilentlyContinue
if (-not (Test-Path -LiteralPath $Skate3 -PathType Container)) { Remove-Item -LiteralPath (Join-Path $data 'disc') -Recurse -Force -ErrorAction SilentlyContinue }

Say ''
Say 'GTAxSkate is installed.' 'Green'
Say 'In GTA V (story mode): pick the skateboard in the weapon wheel or press F to skate. WASD, Shift push, Space ollie, arrows tricks,' 'White'
Say 'E on/off the board, F back to GTA, V camera, R cinematic camera. G in Binco, the Discount Store or Suburban opens the board shop.' 'White'
Say 'Settings: the SK8V menu (F10; keys and camera on its pages), or SkateVLegacy.ini; keys: SkateVKeys.txt.' 'White'
if (-not $NoMusic -and (Wants 'music') -and (Is-Done 'music')) {
    Say 'Skate 3 soundtrack: in GTA V once Settings > Audio > Self Radio Mode: Radio, then Perform Full Scan.' 'White'
}
try { Stop-Transcript | Out-Null } catch { }
