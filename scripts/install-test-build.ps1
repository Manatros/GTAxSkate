# Installs an exact Story Mode test pair. User settings and game data stay intact.
param(
    [Parameter(Mandatory = $true)][string]$Gta,
    [Parameter(Mandatory = $true)][string]$Package
)
$ErrorActionPreference = 'Stop'
$names = @('SkateVLegacy.asi', 'SkateVRuntime.dll', 'SkateGTA.asi', 'SkateGTA.asi.off', 'SkateExtras.asi', 'SkateExtras.asi.off', 'SkateVLegacy.installed.json', 'GTAxSkate-test-version.json')
function Assert-PlainPath([string]$Path) {
    $full = [IO.Path]::GetFullPath($Path)
    for ($part = $full; $part; $part = Split-Path -Parent $part) {
        if (Test-Path -LiteralPath $part) {
            if (((Get-Item -LiteralPath $part -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Linked paths are not supported: $part" }
        }
        $parent = Split-Path -Parent $part
        if ($parent -eq $part) { break }
    }
    return $full
}
function Assert-Stopped {
    if (Get-Process GTA5, GTA5_Enhanced, FiveM, OpenIV, CodeWalker -ErrorAction SilentlyContinue) { throw 'Close GTA, FiveM, OpenIV and CodeWalker first.' }
}
function Hash([string]$Path) { return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
function Restore-Snapshot {
    foreach ($entry in $receipt.files) {
        $target = Assert-PlainPath (Join-Path $Gta $entry.name)
        if ($entry.existed) {
            $saved = Join-Path $backup "files\$($entry.name)"
            if (-not (Test-Path -LiteralPath $target -PathType Leaf) -or (Hash $target) -ne $entry.sha256) { Copy-Item -LiteralPath $saved -Destination $target -Force }
            (Get-Item -LiteralPath $target -Force).Attributes = [IO.FileAttributes]$entry.attributes
        } elseif (Test-Path -LiteralPath $target) { Remove-Item -LiteralPath $target -Force }
    }
}
$Gta = Assert-PlainPath $Gta
$Package = Assert-PlainPath $Package
Assert-Stopped
$exe = Assert-PlainPath (Join-Path $Gta 'GTA5.exe')
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw 'The GTA folder does not contain GTA5.exe.' }
if ((Get-Item -LiteralPath $exe).VersionInfo.FileVersion -ne '1.0.3889.0') { throw 'This test build supports GTA V Legacy 1.0.3889.0 only.' }
$manifestPath = Assert-PlainPath (Join-Path $Package 'manifest.json')
$build = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($build.format -ne 'gtaxskate-test' -or $build.version -ne 1 -or $build.gta -ne '1.0.3889.0' -or
    $build.id -isnot [string] -or $build.id -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]{0,95}$' -or $build.files -isnot [pscustomobject]) { throw 'Invalid test package manifest.' }
$pair = @('SkateVLegacy.asi', 'SkateVRuntime.dll')
if (@($build.files.PSObject.Properties).Count -ne 2) { throw 'The test package must describe exactly the host and runtime.' }
foreach ($name in $pair) {
    $property = $build.files.PSObject.Properties[$name]
    $source = Assert-PlainPath (Join-Path $Package $name)
    if (-not $property -or $property.Value -isnot [string] -or $property.Value -notmatch '^[a-fA-F0-9]{64}$' -or
        -not (Test-Path -LiteralPath $source -PathType Leaf) -or (Hash $source) -ne $property.Value.ToLowerInvariant()) { throw "Package verification failed: $name" }
}
foreach ($name in $names) {
    $target = Assert-PlainPath (Join-Path $Gta $name)
    if ((Test-Path -LiteralPath $target) -and -not (Test-Path -LiteralPath $target -PathType Leaf)) { throw "Destination is not a regular file: $target" }
}
$installed = Join-Path $Gta 'SkateVLegacy.installed.json'
if (Test-Path -LiteralPath $installed) {
    $record = Get-Content -LiteralPath $installed -Raw | ConvertFrom-Json
    if ($record -isnot [pscustomobject] -or $record.format -ne 'sk8v-install' -or $record.version -ne 1 -or
        $record.gta -ne $build.gta -or $record.files -isnot [pscustomobject]) { throw 'Unsupported SK8V installation manifest; installation stopped.' }
    foreach ($property in $record.files.PSObject.Properties) {
        $parts = $property.Name -split '[/\\]'
        if ([string]::IsNullOrWhiteSpace($property.Name) -or [IO.Path]::IsPathRooted($property.Name) -or $property.Name -match '^[A-Za-z]:' -or
            @($parts | Where-Object { $_ -eq '.' -or $_ -eq '..' }).Count -gt 0 -or ($null -ne $property.Value -and $property.Value -isnot [string])) { throw 'Invalid entry in the SK8V installation manifest.' }
    }
} else {
    $record = [pscustomobject]@{format = 'sk8v-install'; version = 1; gta = $build.gta; files = [pscustomobject]@{}}
}
$backupRoot = Assert-PlainPath (Join-Path $Gta 'GTAxSkate-test-backups')
$backup = Assert-PlainPath (Join-Path $backupRoot ((Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [guid]::NewGuid().ToString('N').Substring(0, 8)))
New-Item -ItemType Directory -Path (Join-Path $backup 'files') -Force | Out-Null
$receipt = [pscustomobject]@{format = 'gtaxskate-test-backup'; version = 1; gta = $Gta; build = $build.id; files = @()}
foreach ($name in $names) {
    $target = Join-Path $Gta $name
    $exists = Test-Path -LiteralPath $target -PathType Leaf
    $entry = [pscustomobject]@{name = $name; existed = [bool]$exists; sha256 = $null; attributes = 0}
    if ($exists) {
        $entry.sha256 = Hash $target
        $entry.attributes = [int](Get-Item -LiteralPath $target -Force).Attributes
        $saved = Join-Path $backup "files\$name"
        Copy-Item -LiteralPath $target -Destination $saved -Force
        if ((Hash $saved) -ne $entry.sha256) { throw "Backup verification failed: $name" }
    }
    $receipt.files += $entry
}
$utf8 = New-Object Text.UTF8Encoding $false
[IO.File]::WriteAllText((Join-Path $backup 'backup.json'), ($receipt | ConvertTo-Json -Depth 8), $utf8)
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'restore-test-build.ps1') -Destination (Join-Path $backup 'restore-test-build.ps1')
try {
    Assert-Stopped
    foreach ($name in $pair) {
        $target = Join-Path $Gta $name
        Copy-Item -LiteralPath (Join-Path $Package $name) -Destination $target -Force
        if ((Hash $target) -ne $build.files.$name.ToLowerInvariant()) { throw "Copy verification failed: $name" }
        $record.files | Add-Member -NotePropertyName $name -NotePropertyValue (Hash $target) -Force
    }
    foreach ($legacy in 'SkateGTA.asi', 'SkateExtras.asi') {
        $active = Join-Path $Gta $legacy
        if (Test-Path -LiteralPath $active) { Move-Item -LiteralPath $active -Destination "$active.off" -Force }
    }
    [IO.File]::WriteAllText($installed, ($record | ConvertTo-Json -Depth 8), $utf8)
    [IO.File]::WriteAllText((Join-Path $Gta 'GTAxSkate-test-version.json'), ($build | ConvertTo-Json -Depth 12), $utf8)
} catch {
    $cause = $_.Exception.Message
    try { Restore-Snapshot } catch { throw "Installation failed ($cause); rollback incomplete: $($_.Exception.Message). Backup: $backup" }
    throw "Installation failed; previous build restored. Cause: $cause. Backup: $backup"
}
Write-Host "Test build $($build.id) installed. INI and board files were preserved." -ForegroundColor Green
Write-Host "Rollback: powershell -ExecutionPolicy Bypass -File `"$backup\restore-test-build.ps1`" -Backup `"$backup`""
