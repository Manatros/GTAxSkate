# Configure the experimental helper. Original boards/settings stay byte-exact.
param(
    [Parameter(Mandatory)][string]$Assets,
    [Parameter(Mandatory)][string]$Python,
    [Parameter(Mandatory)][string]$Rage,
    [string]$Scripts = ''
)
$ErrorActionPreference = 'Stop'
if (-not $Scripts) { $Scripts = $PSScriptRoot }
function Plain([string]$Path) {
    $full = [IO.Path]::GetFullPath($Path)
    for ($part = $full; $part; $part = Split-Path -Parent $part) {
        if ((Test-Path -LiteralPath $part) -and ((Get-Item -LiteralPath $part -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Linked paths are not supported: $part" }
        if ((Split-Path -Parent $part) -eq $part) { break }
    }
    return $full
}
function Hash([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
if (Get-Process GTA5, GTA5_Enhanced, FiveM, OpenIV, CodeWalker -ErrorAction SilentlyContinue) { throw 'Close GTA and model tools first.' }
$Assets = Plain $Assets
$Python = Plain $Python
$Rage = Plain $Rage
$Scripts = Plain $Scripts
foreach ($file in $Python,$Rage,(Join-Path $Assets 'private\board-native\skatev_board.ydr'),(Join-Path $Assets 'private\board-native\skatev_board.ytyp')) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "Required file is missing: $file" }
}
if (-not (Test-Path -LiteralPath (Join-Path $Assets 'private\board-parts') -PathType Container)) { throw 'The built shop models are missing.' }
& $Python -c 'import sys; assert sys.version_info >= (3, 10), "Python 3.10 or newer required"'
if ($LASTEXITCODE) { throw 'Python 3.10 or newer is required.' }
$names = @('board-composition-worker.py','prepare-native-custom-board.py','worker.ini','enabled')
foreach ($name in $names[0..1]) {
    if (-not (Test-Path -LiteralPath (Join-Path $Scripts $name) -PathType Leaf)) { throw "Helper script is missing: $name" }
}
if (-not (Test-Path -LiteralPath (Join-Path $Scripts 'restore-board-composer.ps1') -PathType Leaf)) { throw 'The helper rollback script is missing.' }
$cache = Plain (Join-Path $Assets 'private\board-composed')
$backup = Plain (Join-Path $cache ('backups\' + [guid]::NewGuid().ToString('N')))
New-Item -ItemType Directory -Path $backup -Force | Out-Null
$receipt = [ordered]@{format='gtaxskate-composer-backup'; version=1; assets=$Assets; files=@(); pythonSha256=(Hash $Python); rageSha256=(Hash $Rage)}
foreach ($name in $names) {
    $target = Plain (Join-Path $cache $name)
    $exists = Test-Path -LiteralPath $target -PathType Leaf
    if ((Test-Path -LiteralPath $target) -and -not $exists) { throw "Destination is not a file: $target" }
    $entry = [ordered]@{name=$name; existed=[bool]$exists; sha256=$null}
    if ($exists) {
        $entry.sha256 = Hash $target
        Copy-Item -LiteralPath $target -Destination (Join-Path $backup $name)
        if ((Hash (Join-Path $backup $name)) -ne $entry.sha256) { throw "Backup verification failed: $name" }
    }
    $receipt.files += $entry
}
$utf8 = New-Object Text.UTF8Encoding $false
[IO.File]::WriteAllText((Join-Path $backup 'backup.json'), ($receipt | ConvertTo-Json -Depth 5), $utf8)
Copy-Item -LiteralPath (Join-Path $Scripts 'restore-board-composer.ps1') -Destination $backup
try {
    foreach ($name in $names[0..1]) {
        Copy-Item -LiteralPath (Join-Path $Scripts $name) -Destination (Join-Path $cache $name) -Force
        if ((Hash (Join-Path $cache $name)) -ne (Hash (Join-Path $Scripts $name))) { throw "Copy verification failed: $name" }
    }
    # Windows profile APIs consume a Unicode INI, including non-ASCII paths.
    [IO.File]::WriteAllText((Join-Path $cache 'worker.ini'), "[Worker]`r`nPython=$Python`r`nRage=$Rage`r`n", [Text.Encoding]::Unicode)
    [IO.File]::WriteAllText((Join-Path $cache 'enabled'), 'experimental native shop composition', $utf8)
} catch {
    & (Join-Path $backup 'restore-board-composer.ps1') -Backup $backup | Out-Null
    throw
}
[pscustomobject]@{cache=$cache; backup=$backup; mode='experimental; auto-started by composed-board host; gameplay unconfirmed'}
