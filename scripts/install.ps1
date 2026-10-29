# GTAxSkate installer: SK8V (Skate 3's engine in GTA V) plus GTAxSkate's
# additions (keyboard and mouse, Skate 3's board shop with every part, GTA's
# camera and a cinematic camera, trainer tuning and stamina, GPS while
# skating), all made from your own Skate 3 and GTA V. Started by
# "Install GTAxSkate.cmd" in the repo folder.
#
#   powershell -ExecutionPolicy Bypass -File scripts\install.ps1 [-Gta <folder>] [-Skate3 <ISO or folder>] [-Redo <step>] [-Until <step>] [-Components boards,music|none] [-NoMusic] [-Remove <component>]
#
# It finds what it can by itself and asks only for the rest (a folder or file
# picker). Every step leaves a receipt in build\install\, so running it again
# goes on after the last finished step; -Redo <step> runs a step again.
#
# -Components boards,glow,music,world selects optional stages; none means core.
# Defaults: boards, music, world; glow only when CodeWalker is found.
# Interactive sessions offer a numbered list; GTAXSKATE_NONINTERACTIVE=1 uses
# defaults. Deselection keeps installed data. -Remove hides options only and
# retains files; explicit -Components selection restores their flags.
#
# Steps:
#   1 tools    the SK8V release (its Python and rage tool), vgmstream
#   2 sk8v     SK8V's own setup: Skate 3's data from your disc, installed into GTA V
#   3 bin      GTAxSkate's builds: SK8V with our additions, the board tool
#              (built here when the source is, else from the GTAxSkate release)
#   4 boards   the board library and every part as a GTA model (a few minutes)
#   5 glow     glowing wheel models (only when CodeWalker is found; -CodeWalker)
#   6 music    Skate 3's soundtrack (the iPod's 46 songs) on GTA's Self Radio
#              (-NoMusic skips it)
#   7 world    the prebaked map: GTA's map collision baked once, the fallback
#              where GTA's live collision has holes (about 10 minutes, 2.5 GB
#              while it runs, 0.6 GB kept)
#   8 install  our SK8V build over SK8V's (recorded in SK8V's install record,
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
    [ValidateSet('', 'tools', 'sk8v', 'bin', 'boards', 'glow', 'music', 'world', 'install')] [string] $Redo = '',
    # Stop after this step (testing, or preparing without touching GTA yet).
    [ValidateSet('', 'tools', 'sk8v', 'bin', 'boards', 'glow', 'music', 'world', 'install')] [string] $Until = '',
    [switch] $NoMusic,
    [string] $Components = '',
    [string] $Remove = '',
    # CodeWalker's folder (it holds CodeWalker.Core.dll), for the glowing wheel
    # models; CodeWalker is not bundled. Without it glowing wheels use the light only.
    [string] $CodeWalker = '',
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
$GtaxSkateRelease = 'v0.7.1'
$GtaxSkateBinZip = "https://github.com/Manatros/GTAxSkate/releases/download/$GtaxSkateRelease/GTAxSkate-bin.zip"
$GtaxSkateBinSha256 = '0495d787725a7ca352b922c0223d2fb9d810c9c253594612e665ac8560df3e68'
$GtaBuild = '1.0.3889.0'
$Steps = 'tools', 'sk8v', 'bin', 'boards', 'glow', 'music', 'world', 'install'
. (Join-Path $PSScriptRoot 'install-asset-transaction.ps1')

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
function Invoke-WorldCollision([string]$Script, [string]$Gta, [string]$Keys, [string]$Baker, [string]$Rails, [string]$World) {
    # Windows PowerShell inherited from pwsh can lose its module paths and
    # Get-FileHash. Always run this pipeline in PowerShell 7 itself.
    $pwsh = Get-Command pwsh.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $pwsh) { throw 'World collision preparation requires PowerShell 7 (pwsh.exe). Install it and rerun with -Redo world.' }
    $savedPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        & $pwsh.Source -NoProfile -ExecutionPolicy Bypass -File $Script -Gta $Gta -Keys $Keys -Baker $Baker -Rails $Rails -World $World
        $worldExit = $LASTEXITCODE
    } finally { $ErrorActionPreference = $savedPreference }
    if ($worldExit -ne 0) { throw "build-world-collision.ps1 failed (exit code $worldExit). Previous collision and preparation logs were retained." }
}
function Download([string]$url, [string]$to) {
    Say "  downloading $(Split-Path -Leaf $url)..." 'DarkGray'
    try { Invoke-WebRequest -UseBasicParsing $url -OutFile $to }
    catch { Fail "Could not download $(Split-Path -Leaf $url) ($($_.Exception.Message)). Check your internet connection or firewall/antivirus." }
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
    $dialog.Title = 'Pick your Skate 3 .iso file (do not mount it), or Cancel to pick an extracted folder'
    $dialog.Filter = 'Xbox 360 disc image (*.iso;default.xex)|*.iso;default.xex|All files|*.*'
    $owner = Owner
    try { if ($dialog.ShowDialog($owner) -eq 'OK') { return $dialog.FileName } } finally { $owner.Close() }
    Pick-Folder 'Your extracted Skate 3 folder (holds default.xex and data\)'
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
    $argsList = "-Gta `"$($Gta.TrimEnd('\'))`" -Skate3 `"$($Skate3.TrimEnd('\'))`""
    if ($Redo) { $argsList += " -Redo $Redo" }
    if ($Until) { $argsList += " -Until $Until" }
    if ($NoMusic) { $argsList += ' -NoMusic' }
    $componentArg = if ($selectedComponents.Count) { $selectedComponents -join ',' } else { 'none' }
    $argsList += " -Components $componentArg"
    if ($Remove) { $argsList += " -Remove $Remove" }
    if ($Parts) { $argsList += " -Parts `"$Parts`"" }
    if ($Choose) { $argsList += ' -Choose' }
    if ($CodeWalker) { $argsList += " -CodeWalker `"$($CodeWalker.TrimEnd('\'))`"" }
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
    foreach ($key in 'HKLM:\SOFTWARE\WOW6432Node\Rockstar Games\Grand Theft Auto V',
                     'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Steam App 271590',
                     'HKLM:\SOFTWARE\WOW6432Node\Rockstar Games\GTAV') {
        $p = Get-ItemProperty $key -ErrorAction SilentlyContinue
        foreach ($v in $p.InstallFolder, $p.InstallLocation) {
            if ($v -and (Test-Path -LiteralPath (Join-Path $v 'GTA5.exe'))) { return $v }
        }
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

function Test-GtaSource([string]$path) {
    if (-not $path -or -not (Test-Path -LiteralPath $path)) { return 'GTA V Legacy (GTA5.exe) was not found.' }
    if (Test-Path -LiteralPath $path -PathType Leaf) {
        $script:Gta = Split-Path -Parent $path
        $path = $script:Gta
    }
    $exe = Join-Path $path 'GTA5.exe'
    if (-not (Test-Path -LiteralPath $exe)) {
        if (Test-Path -LiteralPath (Join-Path $path 'GTA5_Enhanced.exe')) {
            return "$path is GTA V Enhanced (GTA5_Enhanced.exe). GTAxSkate and SK8V require GTA V Legacy ($GtaBuild), which has GTA5.exe."
        }
        return "GTA5.exe was not found in $path. Pick your GTA V Legacy folder."
    }
    $ver = (Get-Item -LiteralPath $exe).VersionInfo.FileVersion
    if ($ver -ne $GtaBuild) {
        return "SK8V needs GTA V Legacy $GtaBuild; your GTA5.exe is $(if ($ver) { $ver } else { 'unknown' }). GTA V Enhanced does not work."
    }
    if (-not (Test-Path -LiteralPath (Join-Path $path 'update\update.rpf'))) {
        return "update\update.rpf is missing in ${path}: verify GTA V in its launcher first."
    }
    if (-not (Test-Path -LiteralPath (Join-Path $path 'ScriptHookV.dll'))) {
        Say ''
        Say '  Script Hook V is missing (no ScriptHookV.dll in the GTA folder).' 'Yellow'
        Say '  Download it from http://www.dev-c.com/gtav/scripthookv/ and copy' 'Yellow'
        Say '  ScriptHookV.dll and dinput8.dll from its bin folder into:' 'Yellow'
        Say "  $path" 'Yellow'
        return 'Script Hook V is not installed.'
    }
    ''
}

function Test-Skate3Source([string]$path) {
    if (-not $path -or -not (Test-Path -LiteralPath $path)) { return 'Your Skate 3 disc image or folder was not found.' }
    if (Test-Path -LiteralPath $path -PathType Container) {
        foreach ($parent in (Split-Path -Parent $path), (Split-Path -Parent (Split-Path -Parent $path))) {
            if ($parent -and (Test-Path -LiteralPath (Join-Path $parent 'default.xex'))) {
                $script:Skate3 = $parent
                $path = $parent
                break
            }
        }
        if (-not (Test-Path -LiteralPath (Join-Path $path 'default.xex'))) {
            $subXex = @(Get-ChildItem -LiteralPath $path -Directory -ErrorAction SilentlyContinue | Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName 'default.xex') })
            if ($subXex.Count -eq 1) {
                $script:Skate3 = $subXex[0].FullName
                $path = $script:Skate3
            }
        }
        $isos = @(Get-ChildItem -LiteralPath $path -Filter '*.iso' -File -ErrorAction SilentlyContinue)
        if (-not (Test-Path -LiteralPath (Join-Path $path 'default.xex')) -and $isos.Count -eq 1) {
            $script:Skate3 = $isos[0].FullName
            return Test-Skate3Source $script:Skate3
        }
        $archives = @(Get-ChildItem -LiteralPath $path -File -ErrorAction SilentlyContinue | Where-Object { $_.Extension -match '^\.(zip|7z|rar)$' })
        if (-not (Test-Path -LiteralPath (Join-Path $path 'default.xex')) -and $archives.Count -gt 0) {
            return "$($archives[0].FullName) is still a compressed archive ($($archives[0].Extension)). Unzip it first so you have the .iso file, then pick the .iso file."
        }
        $missing = @('default.xex', 'data\big\db.big', 'data\content\createacharacter.big') | Where-Object { -not (Test-Path -LiteralPath (Join-Path $path $_)) }
        if ($missing) {
            return "$path is missing $($missing -join ', '). Do not mount or double-click the ISO in Windows (Windows only sees the video partition); pick the .iso file itself, or a fully extracted Xbox 360 disc folder."
        }
        return ''
    }
    $item = Get-Item -LiteralPath $path
    if ($item.Name -ieq 'default.xex') {
        $script:Skate3 = $item.DirectoryName
        return Test-Skate3Source $script:Skate3
    }
    if ($item.Extension -match '^\.(zip|7z|rar)$') {
        return "$path is still a compressed archive ($($item.Extension)). Unzip it first so you have the .iso file, then pick the .iso file."
    }
    if ($item.Name -ieq 'PS3_DISC.SFB' -or $item.Extension -ieq '.xex') {
        return "$path is not an Xbox 360 Skate 3 .iso or default.xex. Pick your Xbox 360 .iso file or the extracted folder with default.xex."
    }
    $fs = [IO.File]::OpenRead($item.FullName)
    try {
        $buf = New-Object byte[] 20
        foreach ($base in 0L, 0xFD90000L, 0x2080000L, 0x18300000L) {
            $offset = $base + 32L * 2048L
            if ($item.Length -lt ($offset + 20L)) { continue }
            [void]$fs.Seek($offset, [IO.SeekOrigin]::Begin)
            if ($fs.Read($buf, 0, 20) -eq 20 -and [Text.Encoding]::ASCII.GetString($buf) -eq 'MICROSOFT*XBOX*MEDIA') { return '' }
        }
    } finally { $fs.Dispose() }
    return "$path is not an Xbox 360 Skate 3 disc image (no XDVDFS partition found). Pick your .iso file or extracted Skate 3 folder."
}

function Resolve-Skate3Source {
    if (-not $script:Skate3 -or -not (Test-Path -LiteralPath $script:Skate3)) { $script:Skate3 = Pick-Skate3 }
    $skateProblem = ''
    for ($attempt = 0; $attempt -lt 2; $attempt++) {
        $skateProblem = Test-Skate3Source $script:Skate3
        if (-not $skateProblem) { break }
        Say ''
        Say "  $skateProblem" 'Yellow'
        $script:Skate3 = Pick-Skate3
        if (-not $script:Skate3) { break }
    }
    if ($skateProblem) { Fail $skateProblem }
}

Say 'GTAxSkate installer' 'White'
Say 'Install logs and remembered folder choices stay locally in build\install; this installer does not upload them.' 'DarkGray'
Say 'Logs can contain personal folder paths. Review and redact excerpts before sharing them publicly.' 'DarkGray'
if (Get-Process GTA5, GTA5_Enhanced, OpenIV, CodeWalker -ErrorAction SilentlyContinue) { Fail 'Close GTA V, OpenIV and CodeWalker first.' }
if (-not $Gta) { $Gta = Known 'Gta' }
if (-not $Gta) { $Gta = Find-Gta }
if (-not $Gta -or -not (Test-Path -LiteralPath (Join-Path $Gta 'GTA5.exe'))) { $Gta = Pick-Folder 'Your GTA V Legacy folder (it holds GTA5.exe)' }
$gtaProblem = Test-GtaSource $Gta
if ($gtaProblem -and $Gta -eq (Known 'Gta') -and $gtaProblem -notlike '*Script Hook V*') {
    Say "  Note: $gtaProblem" 'Yellow'
    $Gta = Pick-Folder 'Your GTA V Legacy folder (it holds GTA5.exe)'
    $gtaProblem = Test-GtaSource $Gta
}
if ($gtaProblem) { Fail $gtaProblem }
if (-not ('dinput8.dll', 'version.dll', 'winmm.dll', 'dsound.dll', 'xinput1_3.dll' | Where-Object { Test-Path -LiteralPath (Join-Path $Gta $_) })) {
    Say '  Note: no ASI loader found (dinput8.dll from Script Hook V''s bin folder): without one GTA loads no mods.' 'Yellow'
}
$hookVersion = (Get-Item (Join-Path $Gta 'ScriptHookV.dll')).VersionInfo.FileVersion
if ($hookVersion -and $hookVersion -notlike '3889.*') {
    Say "  Note: your Script Hook V is $hookVersion; GTA V $GtaBuild needs v3889.0 (1158.13 or newer). Update it if GTA shows a Script Hook V error." 'Yellow'
}
$existingData = if (Test-Path -LiteralPath (Join-Path $Gta 'GTAxSkate\skate-data\assets')) { Join-Path $Gta 'GTAxSkate' } else { Join-Path $Gta 'SK8V' }
if (-not (Is-Done 'sk8v')) {
    $free = Free-GB $Gta
    $need = if (Test-Path -LiteralPath (Join-Path $existingData '.stages')) { 3 } else { 8 }
    if ($free -lt $need) { Fail "Only $free GB free on the GTA V drive; the setup needs about $need GB. Free some space and run it again." }
}
if (-not $Skate3) { $Skate3 = Known 'Skate3' }
if (-not (Is-Done 'sk8v')) { Resolve-Skate3Source }

# Resolve CodeWalker before choosing defaults.
if (-not $CodeWalker) { $CodeWalker = Known 'CodeWalker' }
if (-not $CodeWalker) {
    foreach ($c in (Join-Path $root 'tools\CodeWalker'), (Join-Path $env:USERPROFILE 'Downloads\CodeWalker'), (Join-Path $env:ProgramFiles 'CodeWalker')) {
        if (Test-Path -LiteralPath (Join-Path $c 'CodeWalker.Core.dll')) { $CodeWalker = $c; break }
    }
}
$componentNames = @('boards', 'glow', 'music', 'world')
$interactive = [Environment]::UserInteractive -and $env:GTAXSKATE_NONINTERACTIVE -ne '1'
try { $interactive = $interactive -and -not [Console]::IsInputRedirected -and [Console]::WindowHeight -gt 0 } catch { $interactive = $false }
function Parse-Components([string]$value) {
    if ($value -eq 'none' -or [string]::IsNullOrWhiteSpace($value)) { return }
    if ($value -eq 'all') { return $componentNames }
    foreach ($name in ($value -split ',')) {
        $name = $name.Trim().ToLowerInvariant()
        if ($componentNames -notcontains $name) { Fail "Unknown component '$name'. Use boards, glow, music, world, all or none." }
        $name
    }
}
$componentsExplicit = $PSBoundParameters.ContainsKey('Components')
$selectedComponents = @('boards', 'music', 'world')
if ($CodeWalker -and (Test-Path -LiteralPath (Join-Path $CodeWalker 'CodeWalker.Core.dll'))) { $selectedComponents += 'glow' }
if ($componentsExplicit) { $selectedComponents = @(Parse-Components $Components) }
elseif ($interactive) {
    Say 'Optional components (core is always installed):' 'White'
    for ($i = 0; $i -lt $componentNames.Count; $i++) {
        $name = $componentNames[$i]
        $default = if ($selectedComponents -contains $name) { 'ON' } else { 'OFF' }
        Say "  $($i + 1). $name [$default]"
    }
    $answer = Read-Host 'Enter keeps defaults; or enter numbers to install (1,3), or none'
    if ($answer.Trim()) {
        if ($answer.Trim() -eq 'none') { $selectedComponents = @() }
        else {
            $selectedComponents = @(foreach ($number in ($answer -split ',')) {
                if ($number.Trim() -notmatch '^[1-4]$') { Fail 'Choose component numbers 1-4, or none.' }
                $componentNames[[int]$number.Trim() - 1]
            })
        }
    }
}
if ($NoMusic) { $selectedComponents = @($selectedComponents | Where-Object { $_ -ne 'music' }) }
$removedComponents = @(Parse-Components (Known 'RemovedComponents'))
if ($componentsExplicit) { $removedComponents = @($removedComponents | Where-Object { $selectedComponents -notcontains $_ }) }
$removedComponents = @($removedComponents) + @(Parse-Components $Remove)
$selectedComponents = @($selectedComponents | Where-Object { $removedComponents -notcontains $_ })
Remember 'RemovedComponents' ($removedComponents -join ',')
function Component-On([string]$name) { $selectedComponents -contains $name }
if ($Remove) { Say 'Removal only hides component options; installed files are retained.' 'Yellow' }
if ((Is-Done 'sk8v') -and (Component-On 'boards') -and -not (Is-Done 'boards')) { Resolve-Skate3Source }
Remember 'Gta' $Gta
Remember 'Skate3' $Skate3
$known | ConvertTo-Json | Set-Content $settings
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
    @{ N = 'carry';   T = 'The board from the weapon wheel stays in hand in GTA (swing it as a melee weapon)'; On = $false; Ini = @{ CarryBoard = '1', '0' } }
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
if (-not $Parts -and (-not $interactive -or $componentsExplicit)) { $Parts = ($catalog | Where-Object { $_.On } | ForEach-Object { $_.N }) -join ',' }
if ((-not $Parts -or $Choose) -and $interactive) { $Parts = Choose-Parts }
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
if ($Redo) {
    for ($i = [array]::IndexOf($Steps, $Redo); $i -ge 0 -and $i -lt $Steps.Count; $i++) {
        if ($componentNames -contains $Steps[$i] -and -not (Component-On $Steps[$i])) { continue }
        Remove-Item -LiteralPath (Join-Path $receipts "$($Steps[$i]).done") -Force -ErrorAction SilentlyContinue
        if ($Steps[$i] -eq 'boards') { Remove-Item -LiteralPath (Join-Path $receipts 'skinfix.done') -Force -ErrorAction SilentlyContinue }
    }
}

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
# A newer package unpacked over an older one: its bin folder differs from the copy made last time.
if ((Test-Path -LiteralPath (Join-Path $shipped 'SkateVLegacy.asi')) -and (Test-Path -LiteralPath (Join-Path $bin 'SkateVLegacy.asi')) -and
    (Get-FileHash -LiteralPath (Join-Path $shipped 'SkateVLegacy.asi')).Hash -ne (Get-FileHash -LiteralPath (Join-Path $bin 'SkateVLegacy.asi')).Hash) { $newer = $true }
if (Test-Path -LiteralPath $shipped -PathType Container) {
    foreach ($f in Get-ChildItem -LiteralPath $shipped -Recurse -File) {
        $relative = $f.FullName.Substring($shipped.TrimEnd('\').Length + 1)
        $cached = Join-Path $bin $relative
        if (-not (Test-Path -LiteralPath $cached -PathType Leaf) -or
            (Get-FileHash -LiteralPath $f.FullName).Hash -ne (Get-FileHash -LiteralPath $cached).Hash) { $newer = $true; break }
    }
}
if ((Test-Path -LiteralPath (Join-Path $package 'SkateVRuntime.dll')) -and (Test-Path -LiteralPath (Join-Path $bin 'SkateVRuntime.dll')) -and
    ((Get-Item (Join-Path $package 'SkateVRuntime.dll')).LastWriteTime -gt (Get-Item (Join-Path $bin 'SkateVRuntime.dll')).LastWriteTime)) { $newer = $true }
if (-not (Is-Done 'bin') -or $newer) {
    $ped = Join-Path $root 'gta\skate-ped\target\release\skate-ped.exe'
    $music = Join-Path $root 'gta\skate-music\target\release\skate-music.exe'
    if ((Test-Path -LiteralPath (Join-Path $package 'SkateVLegacy.asi')) -and (Test-Path -LiteralPath $ped)) {
        # A developer's checkout: the builds made here.
        Copy-Item (Join-Path $package 'SkateVLegacy.asi'), (Join-Path $package 'SkateVRuntime.dll') $bin -Force
        Copy-Item (Join-Path $sk8vSrc 'tools\prepare-board-parts.py'), (Join-Path $sk8vSrc 'tools\_common.py'), (Join-Path $sk8vSrc 'tools\make-glow-wheels.ps1'), (Join-Path $sk8vSrc 'tools\repair-board-skin-flags.py') $bin -Force
        $baker = Join-Path $sk8vSrc 'rust\target\release\skatev-world-cache.exe'
        if ($env:CARGO_TARGET_DIR) { $baker = Join-Path $env:CARGO_TARGET_DIR 'release\skatev-world-cache.exe' }
        if (Test-Path -LiteralPath $baker) { Copy-Item $baker $bin -Force }
        $railExporter = Join-Path (Split-Path -Parent $baker) 'visual-rails.exe'
        if (Test-Path -LiteralPath $railExporter) { Copy-Item $railExporter $bin -Force }
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
if (-not (Component-On 'boards')) { Say '  skipped (not chosen)' 'DarkGray' }
elseif (-not (Is-Done 'boards')) {
    $disc = Disc-With @('data/big/db.big', 'data/content/createacharacter.big')
    if (-not ((Test-Path -LiteralPath (Join-Path $assets 'private\boards\library.json')) -and
        (Test-Path -LiteralPath (Join-Path $assets 'private\board-parts\parts.json')))) {
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
    Done 'skinfix'  # just built with the skin render bit set
}
# Part models built by an older install can lack the skin render bit (Flags=0
# with HasSkin=1): GTA then draws trucks and wheels rigid. Checked once; only
# that bit changes (SK8V's rage tool rebuilds and compares every other field
# and the embedded textures), and only files still as they were are replaced.
if ((Component-On 'boards') -and (Is-Done 'boards') -and -not (Test-Path -LiteralPath (Join-Path $receipts 'skinfix.done'))) {
    $fixTool = Join-Path $bin 'repair-board-skin-flags.py'
    $fixStage = Join-Path $data ('skin-fix-' + [guid]::NewGuid().ToString('N'))
    try {
        if (Get-Process GTA5 -ErrorAction SilentlyContinue) { throw 'GTA V is running' }
        if (-not (Test-Path -LiteralPath $fixTool)) { throw 'this package has no repair-board-skin-flags.py' }
        Say '  checking the part models for the skin render bit (once, a few minutes)...' 'DarkGray'
        & $python $fixTool --assets $assets --out $fixStage --rage $rage --workers 6
        if ($LASTEXITCODE) { throw "repair-board-skin-flags.py failed (exit code $LASTEXITCODE)" }
        $fix = Get-Content -LiteralPath (Join-Path $fixStage 'manifest.json') -Raw | ConvertFrom-Json
        $repairFiles = @($fix.files | Where-Object { $_.changedModels -gt 0 })
        $recovery = Join-Path $data ('asset-backups\skin-' + [guid]::NewGuid().ToString('N'))
        $transaction = Install-AssetTransaction -TargetRoot $assets -StageRoot $fixStage -Files $repairFiles -BackupRoot $recovery
        Say "  $($fix.checked) part models checked, $($transaction.installed) repaired; backup: $recovery" 'DarkGray'
        Done 'skinfix'
    } catch {
        if ($_.Exception.Message -like '*ROLLBACK INCOMPLETE*') { Fail $_.Exception.Message }
        Say "  the skin render bit check was skipped: $($_.Exception.Message)" 'Yellow'
        Say '  If wheels and trucks look rigid in game, run the installer again (GTA closed).' 'Yellow'
    }
}
Say '  ready' 'Green'
Stop-After 'boards'

# --- 5 glow ------------------------------------------------------------------
# Glowing wheels (F10 > Skating): an emissive copy of every wheel part, made with
# CodeWalker's library, which is not ours to ship. Without it the option still
# lights the wheels, they just do not glow themselves.
Step 5 'Glowing wheel models (needs CodeWalker, optional)'
if (-not (Component-On 'glow')) { Say '  skipped (not chosen)' 'DarkGray' }
elseif (-not (Test-Path -LiteralPath (Join-Path $assets 'private\board-parts\parts.json'))) {
    Say '  skipped: glow needs board part models. Select boards too, or install boards first.' 'Yellow'
}
elseif (-not (Is-Done 'glow')) {
    if (-not $CodeWalker) { $CodeWalker = Known 'CodeWalker' }
    if (-not $CodeWalker) {
        foreach ($c in (Join-Path $root 'tools\CodeWalker'), (Join-Path $env:USERPROFILE 'Downloads\CodeWalker'), (Join-Path $env:ProgramFiles 'CodeWalker')) {
            if (Test-Path -LiteralPath (Join-Path $c 'CodeWalker.Core.dll')) { $CodeWalker = $c; break }
        }
    }
    $glowTool = Join-Path $bin 'make-glow-wheels.ps1'
    if (-not $CodeWalker -or -not (Test-Path -LiteralPath (Join-Path $CodeWalker 'CodeWalker.Core.dll'))) {
        Say '  skipped: CodeWalker not found. Glowing wheels then light the wheels without glowing themselves.' 'Yellow'
        Say '  To add them later: run the installer with -CodeWalker <folder with CodeWalker.Core.dll> -Redo glow.' 'Yellow'
    } elseif (-not (Test-Path -LiteralPath $glowTool)) {
        Say '  skipped: this package has no make-glow-wheels.ps1.' 'Yellow'
    } else {
        Remember 'CodeWalker' $CodeWalker
        $known | ConvertTo-Json | Set-Content $settings
        $parts = Join-Path $assets 'private\board-parts'
        $glowStage = Join-Path $data ('glow-stage-' + [guid]::NewGuid().ToString('N'))
        try {
            Say '  building the glowing wheel models...' 'DarkGray'
            powershell -NoProfile -ExecutionPolicy Bypass -File $glowTool -Parts $parts -Out $glowStage -CodeWalker $CodeWalker -Colours
            if ($LASTEXITCODE) { throw "make-glow-wheels.ps1 failed (exit code $LASTEXITCODE)" }
            $glowFiles = @(Get-ChildItem -LiteralPath $glowStage -File | Where-Object { $_.Name -match '_glow.*\.(ydr|ytyp)$' })
            if (-not $glowFiles.Count) { throw 'No glowing wheel models were generated.' }
            $payload = Join-Path $glowStage 'payload'
            New-Item -ItemType Directory -Path $payload -Force | Out-Null
            $plan = @($glowFiles | ForEach-Object {
                Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $payload $_.Name)
                [pscustomobject]@{ name = $_.Name; sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash }
            })
            if (Get-Process GTA5 -ErrorAction SilentlyContinue) { throw 'Close GTA V before installing glowing wheel models.' }
            $recovery = Join-Path $data ('asset-backups\glow-' + [guid]::NewGuid().ToString('N'))
            Install-AssetTransaction -TargetRoot $parts -StageRoot $glowStage -Files $plan -BackupRoot $recovery | Out-Null
            Done 'glow'
        } catch {
            if ($_.Exception.Message -like '*ROLLBACK INCOMPLETE*') { Fail $_.Exception.Message }
            Say "  glowing wheel models skipped: $($_.Exception.Message)" 'Yellow'
        }
    }
}
Say '  ready' 'Green'
Stop-After 'glow'

# --- 5 music -----------------------------------------------------------------
# The in-game iPod's 46 songs (data\audio\music); not part of SK8V, which
# plays GTA's radio while skating.
Step 6 'Skate 3 soundtrack on Self Radio (about a minute)'
if (-not (Component-On 'music')) { Say '  skipped (not chosen)' 'DarkGray' }
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

# --- 7 world -----------------------------------------------------------------
# The prebaked map (runtime baked.rs reads <data>\world\sk8v-map.svwc): every map
# collision file of your GTA V baked once, used where GTA's live collision has a
# hole while it streams. Optional: without it skating uses the live collision only.
Step 7 'World collision from your Story Mode archives'
if (Component-On 'world') {
$worldBaker = Join-Path $bin 'skatev-world-cache.exe'
$stateSafeWorld = $false
if (Test-Path -LiteralPath $worldBaker) {
    $savedPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $worldFormat = (& $worldBaker --world-format 2>$null | Out-String).Trim()
        $stateSafeWorld = ($LASTEXITCODE -eq 0 -and $worldFormat -eq 'SVWT-LIVE 1')
    } catch { $stateSafeWorld = $false }
    finally { $ErrorActionPreference = $savedPreference }
}
$worldDone = Is-Done 'world'
if ($stateSafeWorld -and $worldDone) {
    try {
        $worldReceipt = Get-Content -LiteralPath (Join-Path $data 'world\collision-build.json') -Raw | ConvertFrom-Json
        $worldDone = ($worldReceipt.bakerSha256 -eq (Get-FileHash -LiteralPath $worldBaker).Hash -and
            $worldReceipt.railsSha256 -eq (Get-FileHash -LiteralPath (Join-Path $bin 'visual-rails.exe')).Hash -and
            (Test-Path -LiteralPath (Join-Path $data 'world\tiles\live-cells.txt')))
        if ($worldDone) {
            $savedPreference = $ErrorActionPreference
            try {
                $ErrorActionPreference = 'Continue'
                # The region the receipt records (whole map since 2026-10-24; older receipts: city box).
                $region = if ($worldReceipt.region) { @($worldReceipt.region) } else { @(-2200, 1500, -3500, 1000) }
                & $worldBaker --verify-world (Join-Path $data 'world') $region[0] $region[1] $region[2] $region[3] *> $null
                $worldDone = ($LASTEXITCODE -eq 0)
            } finally { $ErrorActionPreference = $savedPreference }
        }
    } catch { $worldDone = $false }
}
if (-not $worldDone) {
    try {
        $world = Join-Path $data 'world'
        $keys = Join-Path $data 'gta\gta-keys'
        if (-not (Test-Path -LiteralPath (Join-Path $keys 'gtav_aes_key.dat'))) {
            Say '  reading the archive keys from GTA5.exe...' 'DarkGray'
            & $rage --no-update-check extract-keys --exe $Gta -o $keys
            if ($LASTEXITCODE) { throw "rage extract-keys failed (exit code $LASTEXITCODE)" }
        }
        if ((Free-GB $Gta) -lt $(if ($stateSafeWorld) { 6 } else { 3 })) { throw 'not enough free space on the GTA V drive for collision preparation and recovery' }
        if (-not (Test-Path -LiteralPath (Join-Path $bin 'skatev-world-cache.exe'))) { throw 'this package has no skatev-world-cache.exe' }
        if ($stateSafeWorld) {
            Invoke-WorldCollision -Script (Join-Path $PSScriptRoot 'build-world-collision.ps1') -Gta $Gta -Keys $keys -Baker $worldBaker `
                -Rails (Join-Path $bin 'visual-rails.exe') -World $world
        } else {
            # Accepted older binaries retain their established map format.
            $rpf = Join-Path $release 'tools\bin\sk8v-rpf.exe'
            & (Join-Path $PSScriptRoot 'build-prebaked-map.ps1') -Gta $Gta -Out (Join-Path $data 'gta\collision') -Rpf $rpf -Keys $keys `
                -Baker $worldBaker -World $world
        }
        if (-not (Test-Path -LiteralPath (Join-Path $world 'sk8v-map.svwc'))) { throw 'no sk8v-map.svwc was written' }
        Done 'world'
    } catch {
        if ($stateSafeWorld) { Fail "World collision preparation stopped: $($_.Exception.Message)" }
        Say "  the prebaked map was skipped: $($_.Exception.Message)" 'Yellow'
        Say '  Skating works without it (live collision only). To retry: run the installer with -Redo world.' 'Yellow'
    }
}
} else { Say '  skipped (not chosen; live collision remains available)' 'DarkGray' }
Say '  ready' 'Green'
Stop-After 'world'

# --- 6 install ---------------------------------------------------------------
Step 8 'Installing into GTA V'
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
if ((Component-On 'music') -and (Test-Path -LiteralPath (Join-Path $receipts 'music.done'))) { Add-Key 'SkateRadio' 'RADIO_19_USER' }
foreach ($key in 'DrawBoard', 'DrawSkater') { $text = $text -replace "(?m)^$key=0", "$key=1" }
$text = ($text -replace "`r+`n", "`n") -replace "`n", "`r`n"
[IO.File]::WriteAllText($ini, $text, [Text.Encoding]::Unicode)
# Actual installed state, independent of this selection and -Redo.
$installed = @{
    boards = ((Test-Path -LiteralPath (Join-Path $receipts 'boards.done')) -or
        ((Test-Path -LiteralPath (Join-Path $assets 'private\boards\library.json')) -and
        (Test-Path -LiteralPath (Join-Path $assets 'private\board-parts\parts.json'))))
    glow = ((Test-Path -LiteralPath (Join-Path $receipts 'glow.done')) -or
        @(Get-ChildItem -LiteralPath (Join-Path $assets 'private\board-parts') -Filter '*_glow*.ydr' -File -ErrorAction SilentlyContinue).Count -gt 0)
    music = (Test-Path -LiteralPath (Join-Path $receipts 'music.done'))
    world = ((Test-Path -LiteralPath (Join-Path $receipts 'world.done')) -or
        (Test-Path -LiteralPath (Join-Path $data 'world\sk8v-map.svwc')))
}
$componentLines = foreach ($name in $componentNames) {
    $present = $installed[$name] -and $removedComponents -notcontains $name
    '{0}={1}' -f $name, [int][bool]$present
}
[IO.File]::WriteAllLines((Join-Path $Gta 'SkateVLegacy.components.txt'), [string[]]$componentLines, [Text.Encoding]::ASCII)
Done 'install'

# What only the setup needed, as SK8V's own --clean removes it (about 1-2 GB):
# its GTA extractions and, from an ISO, the copied disc files. Its receipts
# stay; a later run copies again what it needs.
Remove-Item -LiteralPath (Join-Path $data 'gta') -Recurse -Force -ErrorAction SilentlyContinue
if (-not (Test-Path -LiteralPath $Skate3 -PathType Container)) { Remove-Item -LiteralPath (Join-Path $data 'disc') -Recurse -Force -ErrorAction SilentlyContinue }

Say ''
Say 'GTAxSkate is installed.' 'Green'
Say 'In GTA V (story mode): pick the skateboard in the weapon wheel to skate. WASD, Shift push, Space ollie, arrows tricks,' 'White'
Say 'E (or Y) on/off the board, hold E (or Y) back to GTA on foot, V camera, R cinematic camera. G in Binco, the Discount Store or Suburban opens the board shop.' 'White'
Say 'Settings: the SK8V menu (F10; keys and camera on its pages), or SkateVLegacy.ini; keys: SkateVKeys.txt.' 'White'
Say 'Other mods: if a key clashes with another mod (trainers, menus), rebind it in F10 > Keyboard keys. See README: Mod compatibility.' 'White'
if ((Component-On 'music') -and (Test-Path -LiteralPath (Join-Path $receipts 'music.done'))) {
    Say 'Skate 3 soundtrack: in GTA V once Settings > Audio > Self Radio Mode: Radio, then Perform Full Scan.' 'White'
}
try { Stop-Transcript | Out-Null } catch { }
