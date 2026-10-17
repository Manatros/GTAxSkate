param([Parameter(Mandatory)][string]$Backup)
$ErrorActionPreference = 'Stop'
function Plain([string]$Path) {
    $full = [IO.Path]::GetFullPath($Path)
    for ($part = $full; $part; $part = Split-Path -Parent $part) {
        if ((Test-Path -LiteralPath $part) -and ((Get-Item -LiteralPath $part -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Linked paths are not allowed: $part" }
        if ((Split-Path -Parent $part) -eq $part) { break }
    }
    return $full
}
if (Get-Process GTA5, GTA5_Enhanced, FiveM, OpenIV, CodeWalker -ErrorAction SilentlyContinue) { throw 'Close GTA and model tools first.' }
$Backup = Plain $Backup
$receipt = Get-Content -LiteralPath (Plain (Join-Path $Backup 'backup.json')) -Raw | ConvertFrom-Json
if ($receipt.format -ne 'gtaxskate-composer-backup' -or $receipt.version -ne 1) { throw 'Unbekanntes Helper-Backup.' }
$cache = Plain (Join-Path $receipt.assets 'private\board-composed')
if ((Split-Path -Parent $Backup) -ine (Join-Path $cache 'backups')) { throw 'Backup liegt ausserhalb des vorgesehenen Ordners.' }
$names = @('board-composition-worker.py','prepare-native-custom-board.py','worker.ini','enabled')
if (@($receipt.files).Count -ne 4 -or @($receipt.files.name | Select-Object -Unique).Count -ne 4) { throw 'Unvollstaendiges Backup.' }
foreach ($entry in $receipt.files) {
    if ($entry.name -cnotin $names -or $entry.existed -isnot [bool]) { throw 'Invalid backup entry.' }
    $null = Plain (Join-Path $cache $entry.name)
    if ($entry.existed) {
        $saved = Plain (Join-Path $Backup $entry.name)
        if ($entry.sha256 -notmatch '^[a-f0-9]{64}$' -or (Get-FileHash -LiteralPath $saved -Algorithm SHA256).Hash -ine $entry.sha256) { throw 'Backup hash mismatch.' }
    }
}
foreach ($entry in $receipt.files) {
    $target = Join-Path $cache $entry.name
    if ($entry.existed) { Copy-Item -LiteralPath (Join-Path $Backup $entry.name) -Destination $target -Force }
    elseif (Test-Path -LiteralPath $target) { Remove-Item -LiteralPath $target -Force }
}
Write-Host 'Previous helper configuration restored. Original models and settings were preserved.'
