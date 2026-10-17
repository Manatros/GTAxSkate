$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$qa=Join-Path $root ('build/qa-board-skin/'+[guid]::NewGuid().ToString('N'))
$assets=Join-Path $qa 'assets'
$package=Join-Path $qa 'package'
$name='private/board-native/skatev_board.ydr'
$target=Join-Path $assets $name
$payload=Join-Path $package ('payload/'+$name)
New-Item -ItemType Directory -Path (Split-Path -Parent $target),(Split-Path -Parent $payload) -Force|Out-Null
[IO.File]::WriteAllText($target,'old-model')
[IO.File]::WriteAllText($payload,'new-model')
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path).Hash.ToLowerInvariant()}
function Check([bool]$Ok,[string]$Message){if(-not $Ok){throw $Message};Write-Host "PASS $Message"}
function Reject([scriptblock]$Action,[string]$Message){$caught=$false;try{& $Action|Out-Null}catch{$caught=$true};Check $caught $Message}
$entry=@{name=$name;changedModels=1;originalSha256=Hash $target;sha256=Hash $payload}
$manifest=@{format='gtaxskate-board-skin-fix';version=1;assets=$assets;files=@($entry)}
$path=Join-Path $package 'manifest.json'
function Save{[IO.File]::WriteAllText($path,($manifest|ConvertTo-Json -Depth 6))}
Save
$install=Join-Path $PSScriptRoot 'install-board-skin-fix.ps1'
[IO.File]::WriteAllText((Join-Path $assets 'settings.ini'),'keep this')
& $install -Package $package
Check ((Hash $target) -eq $entry.sha256) 'Exact repaired bytes installed'
& $install -Package $package
Check ((Hash $target) -eq $entry.sha256) 'Repeated identical repair is a no-op'
$backup=(Get-ChildItem (Join-Path $assets 'GTAxSkate-skin-backups') -Directory|Select-Object -First 1).FullName
[IO.File]::AppendAllText($target,'changed-by-user')
Reject {& $install -Mode Rollback -Backup $backup} 'Changed model blocks normal rollback'
Copy-Item -LiteralPath $payload -Destination $target -Force
& $install -Mode Rollback -Backup $backup
Check ((Hash $target) -eq $entry.originalSha256) 'Original model restored byte for byte'
Check (([IO.File]::ReadAllText((Join-Path $assets 'settings.ini'))) -eq 'keep this') 'Settings untouched'
[IO.File]::AppendAllText($payload,'corrupt')
Reject {& $install -Package $package} 'Corrupt payload rejected before write'
Check ((Hash $target) -eq $entry.originalSha256) 'Corrupt payload leaves originals intact'
$entry.name='../escape.ydr';Save
Reject {& $install -Package $package} 'Traversal name rejected'
Write-Host '8 board skin installer checks passed; synthetic models only.'
