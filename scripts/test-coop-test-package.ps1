$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$qa=Join-Path $root ('build\qa-coop-package\'+[guid]::NewGuid().ToString('N'))
$gta=Join-Path $qa 'GTA'
New-Item -ItemType Directory -Path $gta -Force|Out-Null
$exe=Join-Path $gta 'GTA5.exe'
Add-Type -TypeDefinition '[assembly: System.Reflection.AssemblyFileVersion("1.0.3889.0")] public class CoopGtaFixture { public static void Main() {} }' -OutputAssembly $exe -OutputType ConsoleApplication
[IO.File]::WriteAllText((Join-Path $gta 'ScriptHookV.dll'),'fixture dependency')
$script=Join-Path $PSScriptRoot 'prepare-coop-test.ps1'
$package=Join-Path $qa 'package'
& $script -Mode Prepare -Package $package
function Check([bool]$Ok,[string]$Message){if(-not $Ok){throw $Message};Write-Host "PASS $Message"}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path).Hash}
function Reject([scriptblock]$Action,[string]$Message){$caught=$false;try{& $Action|Out-Null}catch{$caught=$true};Check $caught $Message}
foreach($name in 'SkateVLegacy.asi','SkateVRuntime.dll','SkateVLegacy.ini'){
    [IO.File]::WriteAllText((Join-Path $gta $name),'preserve-'+$name)
}
$before=Hash (Join-Path $gta 'SkateVLegacy.ini')
Reject {& $script -Mode Install -Stage Client -Package $package -Gta $gta} 'Client requires pinned dependencies'
[IO.File]::WriteAllText((Join-Path $gta 'ScriptHookVDotNet.asi'),'old dependency')
$old=Hash (Join-Path $gta 'ScriptHookVDotNet.asi')
& $script -Mode Install -Stage Dependencies -Package $package -Gta $gta
$depBackup=(Get-ChildItem (Join-Path $gta 'GTAxSkate-coop-backups') -Directory|Sort-Object LastWriteTime|Select-Object -Last 1).FullName
Check ((Hash (Join-Path $gta 'SkateVLegacy.ini')) -eq $before) 'Dependencies preserve player settings'
& $script -Mode Install -Stage Client -Package $package -Gta $gta
$clientBackup=(Get-ChildItem (Join-Path $gta 'GTAxSkate-coop-backups') -Directory|Sort-Object LastWriteTime|Select-Object -Last 1).FullName
Check (Test-Path (Join-Path $gta 'scripts\RageCoop\RageCoop.Client.dll')) 'Client files installed in upstream nested folder'
& $script -Mode Rollback -Backup $clientBackup
Check (-not(Test-Path (Join-Path $gta 'scripts\RageCoop\RageCoop.Client.dll'))) 'Client rollback removes newly added files'
$dependency=Join-Path $gta 'ScriptHookVDotNet3.dll'
[IO.File]::AppendAllText($dependency,'changed')
Reject {& $script -Mode Rollback -Backup $depBackup} 'Changed destination blocks rollback before mutation'
Copy-Item -LiteralPath (Join-Path $package 'payload\ScriptHookVDotNet3.dll') -Destination $dependency -Force
& $script -Mode Rollback -Backup $depBackup
Check ((Hash (Join-Path $gta 'ScriptHookVDotNet.asi')) -eq $old) 'Original dependency restored byte for byte'
Check (-not(Test-Path $dependency)) 'Originally missing dependency remains missing after rollback'
Check ((Hash (Join-Path $gta 'SkateVLegacy.ini')) -eq $before) 'All stages preserve player settings'
$global:CoopFixtureInjectPartial=$true
function Copy-Item {
    [CmdletBinding()]param([string]$LiteralPath,[string]$Destination,[switch]$Force)
    if($global:CoopFixtureInjectPartial -and $Destination -like '*ScriptHookVDotNet3.dll' -and $LiteralPath -like '*payload*'){
        $global:CoopFixtureInjectPartial=$false
        [IO.File]::WriteAllText($Destination,'partial-copy')
        throw 'Injected partial copy failure'
    }
    Microsoft.PowerShell.Management\Copy-Item @PSBoundParameters
}
Reject {& $script -Mode Install -Stage Dependencies -Package $package -Gta $gta} 'Partial copy triggers automatic recovery'
Remove-Item Function:\Copy-Item
Remove-Variable CoopFixtureInjectPartial -Scope Global
Check ((Hash (Join-Path $gta 'ScriptHookVDotNet.asi')) -eq $old -and -not(Test-Path $dependency)) 'Partial copy recovery restores entire original state'
$legacy=Join-Path $gta 'scripts\RageCoop.Client.dll'
[IO.File]::WriteAllText($legacy,'old parallel installation')
Reject {& $script -Mode Install -Stage Dependencies -Package $package -Gta $gta} 'Parallel older RAGECOOP refused before dependencies activate it'
Remove-Item -LiteralPath $legacy
$manifestPath=Join-Path $package 'manifest.json'
$manifest=Get-Content $manifestPath -Raw|ConvertFrom-Json
$manifest.stages.Dependencies[0].name='..\escape.dll'
[IO.File]::WriteAllText($manifestPath,($manifest|ConvertTo-Json -Depth 10))
Reject {& $script -Mode Install -Stage Dependencies -Package $package -Gta $gta} 'Traversal manifest refused'
Write-Host '12 Coop package checks passed; no game launched.'
