# Isolated checks against the unmodified production installer and rollback script.
param([string]$GtaExeSource = $env:GTAXSKATE_GTA_FIXTURE)
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$qa = Join-Path $root ('build\qa-test-build\' + [guid]::NewGuid().ToString('N'))
for ($part = $qa; $part; $part = Split-Path -Parent $part) {
    if ((Test-Path -LiteralPath $part) -and (((Get-Item -LiteralPath $part -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)) { throw "QA-Pfad ist verknuepft: $part" }
    if ((Split-Path -Parent $part) -eq $part) { break }
}
New-Item -ItemType Directory -Path $qa -Force | Out-Null
$installer = Join-Path $PSScriptRoot 'install-test-build.ps1'
$restore = Join-Path $PSScriptRoot 'restore-test-build.ps1'
function Assert([bool]$Value, [string]$Message) { if (-not $Value) { throw $Message } }
function Hash([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash }
function Put([string]$Path, [string]$Value) { [IO.File]::WriteAllText($Path, $Value) }
function State([string]$Path) {
    $map = @{}
    foreach ($file in Get-ChildItem -LiteralPath $Path -File -Force) { $map[$file.Name] = (Hash $file.FullName) + ':' + [int]$file.Attributes }
    return $map
}
function Assert-State($Expected, [string]$Path) {
    $actual = State $Path
    Assert ($Expected.Count -eq $actual.Count) 'Anzahl urspruenglicher Dateien wurde nicht wiederhergestellt.'
    foreach ($key in $Expected.Keys) { Assert ($actual[$key] -eq $Expected[$key]) "Abweichende Datei: $key" }
}
function Expect-Failure([scriptblock]$Action, [string]$Needle) {
    $message = ''
    try { & $Action | Out-Null } catch { $message = $_.Exception.Message }
    Assert ($message -like "*$Needle*") "Erwarteter Fehler '$Needle' fehlt; erhalten: $message"
}
$exe = Join-Path $qa 'version-fixture.exe'
if ($GtaExeSource -and (Test-Path -LiteralPath $GtaExeSource -PathType Leaf)) {
    Assert ((Get-Item -LiteralPath $GtaExeSource).VersionInfo.FileVersion -eq '1.0.3889.0') 'Falsche GTA5.exe-Testquelle.'
    Copy-Item -LiteralPath $GtaExeSource -Destination $exe
} else {
    # Only a disposable version-resource fixture, never a validation bypass.
    $sourceCode = '[assembly: System.Reflection.AssemblyFileVersion("1.0.3889.0")] public class GtaVersionFixture { public static void Main() {} }'
    Add-Type -TypeDefinition $sourceCode -OutputAssembly $exe -OutputType ConsoleApplication
}
function Fixture([string]$CaseName) {
    $case = Join-Path $qa $CaseName
    $gta = Join-Path $case 'GTA'
    $package = Join-Path $case 'package'
    New-Item -ItemType Directory -Path $gta, $package -Force | Out-Null
    Copy-Item -LiteralPath $exe -Destination (Join-Path $gta 'GTA5.exe')
    foreach ($name in 'SkateVLegacy.asi', 'SkateVRuntime.dll') {
        Put (Join-Path $gta $name) "old-$name"
        Put (Join-Path $package $name) "new-$name"
    }
    foreach ($name in 'SkateGTA.asi', 'SkateGTA.asi.off', 'SkateExtras.asi', 'SkateExtras.asi.off', 'SkateVLegacy.ini', 'SkateGTA.ini', 'SkateVKeys.txt', 'SkateVBoards-franklin.txt', 'SkateGTA-boards.txt', 'GTAxSkate-test-version.json') { Put (Join-Path $gta $name) "original-$name" }
    $files = [ordered]@{}
    foreach ($name in 'SkateVLegacy.asi', 'SkateVRuntime.dll') { $files[$name] = Hash (Join-Path $package $name) }
    $manifest = [ordered]@{format = 'gtaxskate-test'; version = 1; id = "fixture-$CaseName"; gta = '1.0.3889.0'; files = $files}
    Put (Join-Path $package 'manifest.json') ($manifest | ConvertTo-Json -Depth 5)
    $installed = @{format = 'sk8v-install'; version = 1; gta = '1.0.3889.0'; files = @{'SkateVLegacy.asi' = 'old-hash'; 'untouched.bin' = 'existing-entry'}}
    Put (Join-Path $gta 'SkateVLegacy.installed.json') ($installed | ConvertTo-Json -Depth 5)
    return [pscustomobject]@{gta = $gta; package = $package; manifest = $manifest}
}
$count = 0
$f = Fixture 'valid'
$before = State $f.gta
& $installer -Gta $f.gta -Package $f.package
foreach ($name in 'SkateVLegacy.asi', 'SkateVRuntime.dll') { Assert ((Hash (Join-Path $f.gta $name)) -eq (Hash (Join-Path $f.package $name))) "Falsches Binary: $name" }
foreach ($name in 'SkateGTA.asi', 'SkateExtras.asi') { Assert (-not (Test-Path -LiteralPath (Join-Path $f.gta $name))) "Legacy-Plugin noch aktiv: $name" }
foreach ($name in 'SkateVLegacy.ini', 'SkateGTA.ini', 'SkateVKeys.txt', 'SkateVBoards-franklin.txt', 'SkateGTA-boards.txt') { Assert (((State $f.gta)[$name]) -eq $before[$name]) "Benutzerdaten veraendert: $name" }
$installed = Get-Content -LiteralPath (Join-Path $f.gta 'SkateVLegacy.installed.json') -Raw | ConvertFrom-Json
Assert ($installed.files.'untouched.bin' -eq 'existing-entry') 'Bestehender Manifest-Eintrag verloren.'
Assert ($installed.files.'SkateVLegacy.asi' -eq (Hash (Join-Path $f.gta 'SkateVLegacy.asi')).ToLowerInvariant()) 'Host-Manifesthash veraltet.'
$backup = (Get-ChildItem -LiteralPath (Join-Path $f.gta 'GTAxSkate-test-backups') -Directory).FullName
Assert ((Hash (Join-Path $backup 'files\SkateGTA.asi.off')) -eq $before['SkateGTA.asi.off'].Split(':')[0]) 'Kollidierende .off-Datei nicht gesichert.'
$count++; Write-Host 'PASS Installation, Manifest, Legacy-Kollision, Benutzerdaten'
& (Join-Path $backup 'restore-test-build.ps1') -Backup $backup
Assert-State $before $f.gta
$count++; Write-Host 'PASS Eigenstaendiger Rollback stellt alle Dateien wieder her'

$f = Fixture 'wrong-hash'; $before = State $f.gta
Put (Join-Path $f.package 'SkateVRuntime.dll') 'corrupt'
Expect-Failure { & $installer -Gta $f.gta -Package $f.package } 'Paketpruefung'
Assert-State $before $f.gta
$count++; Write-Host 'PASS Falscher Pakethash abgelehnt'

$f = Fixture 'wrong-version'
Put (Join-Path $f.gta 'GTA5.exe') 'wrong version fixture'; $before = State $f.gta
Expect-Failure { & $installer -Gta $f.gta -Package $f.package } '1.0.3889.0'
Assert-State $before $f.gta
$count++; Write-Host 'PASS Falsche GTA-Version abgelehnt'

$f = Fixture 'unknown-manifest'
Put (Join-Path $f.gta 'SkateVLegacy.installed.json') '{"format":"unknown","version":9}'; $before = State $f.gta
Expect-Failure { & $installer -Gta $f.gta -Package $f.package } 'Unbekanntes SK8V'
Assert-State $before $f.gta
$count++; Write-Host 'PASS Unbekanntes Installationsmanifest abgelehnt'

$f = Fixture 'transaction-failure'; $before = State $f.gta
$lock = [IO.File]::Open((Join-Path $f.gta 'GTAxSkate-test-version.json'), [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
try { Expect-Failure { & $installer -Gta $f.gta -Package $f.package } 'vorheriger Stand wiederhergestellt' } finally { $lock.Dispose() }
Assert-State $before $f.gta
$count++; Write-Host 'PASS Schreibfehler rollt Binary-/Legacy-/Manifest-Aenderungen zurueck'

$f = Fixture 'bad-backup'
& $installer -Gta $f.gta -Package $f.package
$before = State $f.gta
$backup = (Get-ChildItem -LiteralPath (Join-Path $f.gta 'GTAxSkate-test-backups') -Directory).FullName
Put (Join-Path $backup 'files\SkateVLegacy.asi') 'corrupt-backup'
Expect-Failure { & $restore -Backup $backup } 'Backuppruefung'
Assert-State $before $f.gta
$count++; Write-Host 'PASS Beschaedigtes Backup vor jeder Aenderung abgelehnt'

$f = Fixture 'first-install'
foreach ($name in 'SkateVLegacy.installed.json', 'GTAxSkate-test-version.json', 'SkateExtras.asi', 'SkateExtras.asi.off') { Remove-Item -LiteralPath (Join-Path $f.gta $name) }
$before = State $f.gta
& $installer -Gta $f.gta -Package $f.package
$backup = (Get-ChildItem -LiteralPath (Join-Path $f.gta 'GTAxSkate-test-backups') -Directory).FullName
& $restore -Backup $backup
Assert-State $before $f.gta
$count++; Write-Host 'PASS Fehlende Ursprungsdateien bleiben nach Rollback fehlend'
Write-Host "$count Tests bestanden. Fixtures: $qa" -ForegroundColor Green
