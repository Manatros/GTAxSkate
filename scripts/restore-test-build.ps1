# Restores only the eight fixed binary/receipt paths saved by install-test-build.
param([Parameter(Mandatory = $true)][string]$Backup)
$ErrorActionPreference = 'Stop'
$names = @('SkateVLegacy.asi', 'SkateVRuntime.dll', 'SkateGTA.asi', 'SkateGTA.asi.off', 'SkateExtras.asi', 'SkateExtras.asi.off', 'SkateVLegacy.installed.json', 'GTAxSkate-test-version.json')
function Plain([string]$Path) {
    $full = [IO.Path]::GetFullPath($Path)
    for ($part = $full; $part; $part = Split-Path -Parent $part) {
        if ((Test-Path -LiteralPath $part) -and (((Get-Item -LiteralPath $part -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) { throw "Linked paths are not allowed: $part" }
        if ((Split-Path -Parent $part) -eq $part) { break }
    }
    return $full
}
function Hash([string]$Path) { return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
if (Get-Process GTA5, GTA5_Enhanced, FiveM, OpenIV, CodeWalker -ErrorAction SilentlyContinue) { throw 'Close GTA, FiveM, OpenIV and CodeWalker first.' }
$Backup = Plain $Backup
$receipt = Get-Content -LiteralPath (Plain (Join-Path $Backup 'backup.json')) -Raw | ConvertFrom-Json
if ($receipt.format -ne 'gtaxskate-test-backup' -or $receipt.version -ne 1 -or $receipt.gta -isnot [string]) { throw 'Invalid backup.' }
$Gta = Plain $receipt.gta
$expectedParent = Join-Path $Gta 'GTAxSkate-test-backups'
if (-not [string]::Equals((Split-Path -Parent $Backup), $expectedParent, [StringComparison]::OrdinalIgnoreCase)) { throw 'Backup is not inside the expected GTA backup directory.' }
if (-not (Test-Path -LiteralPath (Plain (Join-Path $Gta 'GTA5.exe')) -PathType Leaf)) { throw 'GTA5.exe fehlt.' }
if (@($receipt.files).Count -ne $names.Count -or @($receipt.files.name | Select-Object -Unique).Count -ne $names.Count) { throw 'Unvollstaendiges Backup.' }
foreach ($entry in $receipt.files) {
    if ($entry.name -isnot [string] -or $entry.name -cnotin $names -or $entry.existed -isnot [bool]) { throw 'Invalid backup entry.' }
    $target = Plain (Join-Path $Gta $entry.name)
    if ((Test-Path -LiteralPath $target) -and -not (Test-Path -LiteralPath $target -PathType Leaf)) { throw "Invalid destination file: $target" }
    if ($entry.existed) {
        $saved = Plain (Join-Path $Backup "files\$($entry.name)")
        if ($entry.sha256 -isnot [string] -or $entry.sha256 -notmatch '^[a-fA-F0-9]{64}$' -or -not (Test-Path -LiteralPath $saved -PathType Leaf) -or (Hash $saved) -ne $entry.sha256) { throw "Backup verification failed: $($entry.name)" }
        if ($entry.attributes -isnot [int] -or $entry.attributes -lt 0 -or ($entry.attributes -band ([int][IO.FileAttributes]::Directory -bor [int][IO.FileAttributes]::ReparsePoint)) -ne 0) { throw 'Invalid file attributes in backup.' }
    }
}
foreach ($entry in $receipt.files) {
    $target = Plain (Join-Path $Gta $entry.name)
    if ($entry.existed) {
        if (-not (Test-Path -LiteralPath $target -PathType Leaf) -or (Hash $target) -ne $entry.sha256) { Copy-Item -LiteralPath (Join-Path $Backup "files\$($entry.name)") -Destination $target -Force }
        (Get-Item -LiteralPath $target -Force).Attributes = [IO.FileAttributes]$entry.attributes
    } elseif (Test-Path -LiteralPath $target) { Remove-Item -LiteralPath $target -Force }
}
Write-Host 'Previous test build restored. Settings were preserved.' -ForegroundColor Green
