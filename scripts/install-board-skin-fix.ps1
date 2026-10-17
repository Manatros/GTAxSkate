# Install a locally staged skin-flag repair. No model assets are distributed.
param([ValidateSet('Install','Rollback')][string]$Mode='Install',
      [string]$Package='', [string]$Backup='')
$ErrorActionPreference='Stop'
function Plain([string]$Path){
    if([string]::IsNullOrWhiteSpace($Path)){throw 'Missing path.'}
    $full=[IO.Path]::GetFullPath($Path)
    for($p=$full;$p;$p=Split-Path -Parent $p){
        if((Test-Path -LiteralPath $p) -and ((Get-Item -LiteralPath $p -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)){throw 'Linked path refused.'}
        if((Split-Path -Parent $p) -eq $p){break}
    };return $full
}
function Stopped {if(Get-Process GTA5,GTA5_Enhanced,FiveM,OpenIV,CodeWalker -ErrorAction SilentlyContinue){throw 'Close GTA, FiveM, OpenIV and CodeWalker first.'}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Validate($Entries){
    if(-not $Entries -or @($Entries).Count -gt 4096){throw 'Invalid repair count.'}
    $seen=@{}
    foreach($e in $Entries){
        if($e.name -isnot [string] -or $e.name -cnotmatch '^private/(board-native/skatev_board|board-parts/[A-Za-z0-9_-]+)\.ydr$' -or
           $seen.ContainsKey($e.name) -or $e.originalSha256 -cnotmatch '^[a-f0-9]{64}$' -or $e.sha256 -cnotmatch '^[a-f0-9]{64}$'){throw 'Invalid repair entry.'}
        $seen[$e.name]=$true
    }
}
function Restore($Receipt,[bool]$Guard){
    Validate $Receipt.files
    foreach($e in $Receipt.files){
        $saved=Plain (Join-Path $Backup ('files/'+$e.name))
        $target=Plain (Join-Path $Receipt.assets $e.name)
        if((Hash $saved) -ne $e.originalSha256){throw 'Backup hash mismatch.'}
        if($Guard -and (Hash $target) -notin @($e.originalSha256,$e.sha256)){throw 'Board changed after repair; rollback refused.'}
    }
    Stopped
    foreach($e in $Receipt.files){
        $target=Join-Path $Receipt.assets $e.name
        Copy-Item -LiteralPath (Join-Path $Backup ('files/'+$e.name)) -Destination $target -Force
        if((Hash $target) -ne $e.originalSha256){throw 'Restore verification failed.'}
        (Get-Item -LiteralPath $target).Attributes=[IO.FileAttributes]$e.attributes
    }
}
Stopped
if($Mode -eq 'Rollback'){
    $Backup=Plain $Backup
    $receipt=Get-Content -LiteralPath (Join-Path $Backup 'backup.json') -Raw|ConvertFrom-Json
    if($receipt.format -ne 'gtaxskate-board-skin-backup' -or $receipt.version -ne 1){throw 'Invalid repair backup.'}
    $receipt.assets=Plain $receipt.assets
    Restore $receipt $true
    Write-Output 'Original board files restored. Settings and binaries unchanged.'
    return
}
$Package=Plain $Package
$manifest=Get-Content -LiteralPath (Plain (Join-Path $Package 'manifest.json')) -Raw|ConvertFrom-Json
if($manifest.format -ne 'gtaxskate-board-skin-fix' -or $manifest.version -ne 1){throw 'Invalid skin repair package.'}
$assets=Plain $manifest.assets
Validate $manifest.files
$entries=@($manifest.files|Where-Object {$_.changedModels -gt 0})
if(-not $entries){Write-Output 'No board files need repair.';return}
$pending=@()
foreach($e in $entries){
    $current=Hash (Plain (Join-Path $assets $e.name))
    if($current -eq $e.sha256){continue}
    if($current -ne $e.originalSha256){throw "Board source changed: $($e.name)"}
    if((Hash (Plain (Join-Path $Package ('payload/'+$e.name)))) -ne $e.sha256){throw 'Repair payload hash mismatch.'}
    $pending+=$e
}
$entries=$pending
if(-not $entries){Write-Output 'Board repair already installed; no changes.';return}
$Backup=Plain (Join-Path $assets ('GTAxSkate-skin-backups/'+[guid]::NewGuid().ToString('N')))
New-Item -ItemType Directory -Path $Backup -Force|Out-Null
$receipt=[ordered]@{format='gtaxskate-board-skin-backup';version=1;assets=$assets;files=@()}
foreach($e in $entries){
    $saved=Join-Path $Backup ('files/'+$e.name)
    New-Item -ItemType Directory -Path (Split-Path -Parent $saved) -Force|Out-Null
    $target=Join-Path $assets $e.name
    Copy-Item -LiteralPath $target -Destination $saved
    if((Hash $saved) -ne $e.originalSha256){throw 'Backup verification failed.'}
    $receipt.files+=@{name=$e.name;originalSha256=$e.originalSha256;sha256=$e.sha256;attributes=[int](Get-Item -LiteralPath $target).Attributes}
}
[IO.File]::WriteAllText((Join-Path $Backup 'backup.json'),($receipt|ConvertTo-Json -Depth 8),[Text.UTF8Encoding]::new($false))
Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $Backup 'install-board-skin-fix.ps1')
try{
    Stopped
    foreach($e in $entries){
        $target=Join-Path $assets $e.name
        Copy-Item -LiteralPath (Join-Path $Package ('payload/'+$e.name)) -Destination $target -Force
        if((Hash $target) -ne $e.sha256){throw 'Installed board hash mismatch.'}
    }
}catch{
    $cause=$_.Exception.Message
    try{Restore $receipt $false}catch{throw "Repair failed ($cause), rollback incomplete: $($_.Exception.Message). Backup: $Backup"}
    throw "Repair failed; original boards restored: $cause"
}
Write-Output "Repaired $($entries.Count) board files; gameplay confirmation pending. Backup: $Backup"
