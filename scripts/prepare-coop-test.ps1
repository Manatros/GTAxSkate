# Pinned, reversible Story co-op baseline. Default: prepare locally only.
[CmdletBinding()]
param(
    [ValidateSet('Prepare','Install','Rollback')][string]$Mode='Prepare',
    [ValidateSet('Dependencies','Client')][string]$Stage='Dependencies',
    [string]$Package='', [string]$Gta='', [string]$Backup='',
    [string]$Lab=''
)
$ErrorActionPreference='Stop'
if(-not $Lab){$Lab=Join-Path (Split-Path -Parent $PSScriptRoot) 'build\coop-lab'}
$sets=@{
    Dependencies=@('ScriptHookVDotNet.asi','ScriptHookVDotNet2.dll','ScriptHookVDotNet3.dll')
    Client=@('scripts\RageCoop\ICSharpCode.SharpZipLib.dll','scripts\RageCoop\Microsoft.Extensions.ObjectPool.dll','scripts\RageCoop\RageCoop.Client.dll','scripts\RageCoop\RageCoop.Core.dll','scripts\RageCoop\System.Buffers.dll')
}
function Plain([string]$Path) {
    if([string]::IsNullOrWhiteSpace($Path)){throw 'An explicit path is required.'}
    $full=[IO.Path]::GetFullPath($Path)
    for($p=$full;$p;$p=Split-Path -Parent $p){
        if((Test-Path -LiteralPath $p) -and ((Get-Item -LiteralPath $p -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)){throw "Linked path refused: $p"}
        if((Split-Path -Parent $p) -eq $p){break}
    }
    return $full
}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function SaveJson($Value,[string]$Path){[IO.File]::WriteAllText($Path,($Value|ConvertTo-Json -Depth 10),[Text.UTF8Encoding]::new($false))}
function Stopped {
    if(Get-Process GTA5,GTA5_Enhanced,FiveM,OpenIV,CodeWalker -ErrorAction SilentlyContinue){throw 'Close GTA, FiveM, OpenIV and CodeWalker first.'}
}
function Game([string]$Path){
    Stopped
    $exe=Plain (Join-Path $Path 'GTA5.exe')
    if(-not(Test-Path -LiteralPath $exe -PathType Leaf) -or (Get-Item -LiteralPath $exe).VersionInfo.FileVersion -ne '1.0.3889.0'){throw 'Requires GTA V Legacy 1.0.3889.0.'}
    if(-not(Test-Path -LiteralPath (Plain (Join-Path $Path 'ScriptHookV.dll')) -PathType Leaf)){throw 'ScriptHookV.dll is required before installing co-op dependencies.'}
    $framework=Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\NET Framework Setup\NDP\v4\Full' -Name Release -ErrorAction SilentlyContinue
    if(-not $framework -or $framework.Release -lt 528040){throw '.NET Framework 4.8 or newer is required.'}
}
function Entries($Value,[string]$Which){
    $expected=@($sets[$Which])
    if($Value -isnot [array] -or $Value.Count -ne $expected.Count){throw 'Invalid allowlist length.'}
    $seen=@{}
    foreach($e in $Value){
        if($e.name -isnot [string] -or $expected -cnotcontains $e.name -or $seen.ContainsKey($e.name) -or $e.sha256 -notmatch '^[a-f0-9]{64}$'){throw 'Invalid allowlist entry.'}
        $seen[$e.name]=$true
    }
}
if($Mode -eq 'Prepare'){
    if(-not $Package){$Package=Join-Path (Split-Path -Parent $PSScriptRoot) ('build\coop-test-'+[guid]::NewGuid().ToString('N').Substring(0,8))}
    $Package=Plain $Package; $Lab=Plain $Lab
    if(Test-Path -LiteralPath $Package){throw 'Use a new empty package path.'}
    $pins=@(
        @{stage='Dependencies';archive='ScriptHookVDotNet-v3.7.0-nightly.193.zip';sha256='e9b8029f9ef08ba0d5fe2c7aff4d54c6a9189826cf83e09becb6cc1611e292ef'},
        @{stage='Client';archive='RageCoop.Client.zip';sha256='0b02e5915ce22358a2e4b13a63d384b3baabe32652e05053b4c25c52342d47b4'}
    )
    foreach($pin in $pins){$archive=Plain (Join-Path $Lab $pin.archive); if((Hash $archive) -ne $pin.sha256){throw 'Pinned archive hash mismatch.'}}
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    New-Item -ItemType Directory -Path $Package | Out-Null
    $manifest=[ordered]@{format='gtaxskate-coop-test';version=1;createdUtc=[DateTime]::UtcNow.ToString('o');gta='1.0.3889.0';candidateOnly=$true;archives=$pins;stages=[ordered]@{}}
    foreach($pin in $pins){
        $zip=[IO.Compression.ZipFile]::OpenRead((Join-Path $Lab $pin.archive))
        try{
            $entries=@()
            foreach($name in $sets[$pin.stage]){
                $entryName=if($pin.stage -eq 'Client'){$name.Substring(8).Replace('\','/')}else{$name}
                $matches=@($zip.Entries|Where-Object {$_.FullName -ceq $entryName})
                if($matches.Count -ne 1){throw "Archive entry missing or duplicated: $entryName"}
                $target=Join-Path $Package ('payload\'+$name)
                New-Item -ItemType Directory -Path (Split-Path -Parent $target) -Force | Out-Null
                [IO.Compression.ZipFileExtensions]::ExtractToFile($matches[0],$target)
                $entries+=@{name=$name;sha256=Hash $target}
            }
            $manifest.stages[$pin.stage]=$entries
        }finally{$zip.Dispose()}
    }
    SaveJson $manifest (Join-Path $Package 'manifest.json')
    Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $Package 'prepare-coop-test.ps1')
    Write-Output "Prepared only: $Package"
    return
}
if($Mode -eq 'Install'){
    $Gta=Plain $Gta; $Package=Plain $Package; Game $Gta
    $scripts=Join-Path $Gta 'scripts'
    if(Test-Path -LiteralPath $scripts){
        foreach($dll in Get-ChildItem -LiteralPath $scripts -Filter 'RageCoop*.dll' -File -Recurse){
            $relative=$dll.FullName.Substring($Gta.TrimEnd('\').Length+1)
            if($sets.Client -notcontains $relative){throw "An older parallel RAGECOOP installation exists: $relative. Preserve and resolve it before this test."}
        }
    }
    $manifest=Get-Content -LiteralPath (Plain (Join-Path $Package 'manifest.json')) -Raw|ConvertFrom-Json
    if($manifest.format -ne 'gtaxskate-coop-test' -or $manifest.version -ne 1 -or $manifest.gta -ne '1.0.3889.0'){throw 'Invalid package manifest.'}
    $entries=@($manifest.stages.$Stage); Entries $entries $Stage
    foreach($e in $entries){
        if((Hash (Plain (Join-Path $Package ('payload\'+$e.name)))) -ne $e.sha256){throw 'Payload hash mismatch.'}
        $target=Plain (Join-Path $Gta $e.name)
        if((Test-Path -LiteralPath $target) -and -not(Test-Path -LiteralPath $target -PathType Leaf)){throw 'Destination is not a file.'}
    }
    if($Stage -eq 'Client'){
        foreach($name in $sets.Dependencies){if((Hash (Plain (Join-Path $Gta $name))) -ne (@($manifest.stages.Dependencies|Where-Object {$_.name -eq $name})[0].sha256)){throw 'Install and test pinned dependencies first.'}}
    }
    $Backup=Plain (Join-Path $Gta ('GTAxSkate-coop-backups\'+[guid]::NewGuid().ToString('N')))
    New-Item -ItemType Directory -Path (Join-Path $Backup 'files') -Force|Out-Null
    $receipt=[ordered]@{format='gtaxskate-coop-backup';version=1;gta=$Gta;stage=$Stage;createdUtc=[DateTime]::UtcNow.ToString('o');files=@()}
    foreach($e in $entries){
        $target=Join-Path $Gta $e.name; $exists=Test-Path -LiteralPath $target -PathType Leaf
        $oldHash=$null; $attributes=0
        if($exists){
            $oldHash=Hash $target; $attributes=[int](Get-Item -LiteralPath $target -Force).Attributes
            $saved=Join-Path $Backup ('files\'+$e.name)
            New-Item -ItemType Directory -Path (Split-Path -Parent $saved) -Force|Out-Null
            Copy-Item -LiteralPath $target -Destination $saved
            if((Hash $saved) -ne $oldHash){throw 'Backup verification failed.'}
        }
        $receipt.files+=@{name=$e.name;sha256=$e.sha256;existed=[bool]$exists;originalSha256=$oldHash;attributes=$attributes}
    }
    SaveJson $receipt (Join-Path $Backup 'backup.json')
    Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $Backup 'prepare-coop-test.ps1')
    try{
        Stopped
        foreach($e in $entries){
            $target=Join-Path $Gta $e.name
            New-Item -ItemType Directory -Path (Split-Path -Parent $target) -Force|Out-Null
            Copy-Item -LiteralPath (Join-Path $Package ('payload\'+$e.name)) -Destination $target -Force
            if((Hash $target) -ne $e.sha256){throw 'Installed hash mismatch.'}
        }
    }catch{
        $cause=$_.Exception.Message
        # This transaction owns these exact destinations. A failed copy can
        # leave a partial file, which the normal user-facing rollback correctly
        # rejects as changed. Restore verified originals directly in this catch.
        try {
            foreach($old in $receipt.files){
                if($old.existed -and (Hash (Plain (Join-Path $Backup ('files\'+$old.name)))) -ne $old.originalSha256){throw 'Recovery backup hash mismatch.'}
                $null=Plain (Join-Path $Gta $old.name)
            }
            Stopped
            foreach($old in $receipt.files){
                $target=Join-Path $Gta $old.name
                if($old.existed){
                    Copy-Item -LiteralPath (Join-Path $Backup ('files\'+$old.name)) -Destination $target -Force
                    if((Hash $target) -ne $old.originalSha256){throw 'Recovery verification failed.'}
                    (Get-Item -LiteralPath $target).Attributes=[IO.FileAttributes]$old.attributes
                }elseif(Test-Path -LiteralPath $target){Remove-Item -LiteralPath $target -Force}
            }
        }
        catch { throw "Installation failed ($cause); rollback incomplete: $($_.Exception.Message). Backup: $Backup" }
        throw "Installation failed; previous files restored. Cause: $cause. Backup: $Backup"
    }
    Write-Output "Installed $Stage only. Rollback backup: $Backup"
    return
}
$Backup=Plain $Backup
$receipt=Get-Content -LiteralPath (Plain (Join-Path $Backup 'backup.json')) -Raw|ConvertFrom-Json
if($receipt.format -ne 'gtaxskate-coop-backup' -or $receipt.version -ne 1 -or $sets.Keys -notcontains $receipt.stage){throw 'Invalid rollback receipt.'}
$Gta=Plain $receipt.gta; Game $Gta
$entries=@($receipt.files); Entries $entries $receipt.stage
# Validate every backup and destination before changing anything. Changed user files stop rollback.
foreach($e in $entries){
    $target=Plain (Join-Path $Gta $e.name)
    if((Test-Path -LiteralPath $target) -and ((Hash $target) -ne $e.sha256) -and (-not $e.existed -or (Hash $target) -ne $e.originalSha256)){throw "Destination changed since install: $($e.name)"}
    if($e.existed -isnot [bool]){throw 'Invalid backup existence marker.'}
    if($e.existed -and ($e.originalSha256 -notmatch '^[a-f0-9]{64}$' -or (Hash (Plain (Join-Path $Backup ('files\'+$e.name)))) -ne $e.originalSha256)){throw 'Backup hash mismatch.'}
}
Stopped
foreach($e in $entries){
    $target=Join-Path $Gta $e.name
    if($e.existed){Copy-Item -LiteralPath (Join-Path $Backup ('files\'+$e.name)) -Destination $target -Force; (Get-Item -LiteralPath $target).Attributes=[IO.FileAttributes]$e.attributes}
    elseif(Test-Path -LiteralPath $target){Remove-Item -LiteralPath $target -Force}
}
Write-Output 'Rollback complete; settings, SK8V pair and server untouched.'
