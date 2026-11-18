[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string]$Skate3,
    [Parameter(Mandatory)] [string]$GtaDir,
    [Parameter(Mandatory)] [string]$DonorDir,
    [Parameter(Mandatory)] [string]$RageExe,
    [string]$OutputRoot = ''
)
$ErrorActionPreference = 'Stop'
$python = Get-Command python -ErrorAction SilentlyContinue
if (-not $python) { throw 'Python 3 is required. Install it, then run this optional installer again.' }
foreach ($pair in @(@($Skate3, 'default.xex'), @($GtaDir, 'GTA5.exe'), @($DonorDir, 'tools\vendor'), @($RageExe, ''))) {
    $target = if ($pair[1]) { Join-Path $pair[0] $pair[1] } else { $pair[0] }
    if (-not (Test-Path -LiteralPath $target)) { throw "Required file or folder not found: $target" }
}
if (Get-Process GTA5, GTA5_Enhanced, FiveM, OpenIV, CodeWalker -ErrorAction SilentlyContinue) { throw 'Close GTA V, OpenIV, and CodeWalker first.' }
$root = $PSScriptRoot
if (-not $OutputRoot) { $OutputRoot = Join-Path $root 'local' }
$m0 = Join-Path $OutputRoot 'm0-tutorial'
$dlc = Join-Path $OutputRoot 'm1-tutorial-dlc'
$prepM0 = Join-Path $root 'prepare-tutorial.py'
$prepDlc = Join-Path $root 'prepare-tutorial-dlc.py'
& $python.Source -B $prepM0 --skate $Skate3 --donor $DonorDir --out $m0
if ($LASTEXITCODE) { throw 'Tutorial source preparation failed; nothing was installed.' }
$manifest = Join-Path $m0 'm0-tutorial-manifest.json'
& $python.Source -B $prepM0 --verify-manifest $manifest
if ($LASTEXITCODE) { throw 'Tutorial manifest verification failed; nothing was installed.' }
$manifestData = Get-Content -LiteralPath $manifest -Raw | ConvertFrom-Json
$districts = @($manifestData.districts | ForEach-Object { $_.district })
foreach ($district in 'DIST_SkateSchool', 'DIST_StartPark') {
    if ($districts -notcontains $district) { throw "Tutorial manifest is missing required district $district; nothing was installed." }
}
& $python.Source -B $prepDlc --disc $Skate3 --manifest $manifest --donor $DonorDir --rage $RageExe --out $dlc
if ($LASTEXITCODE) { throw 'Tutorial DLC build failed; nothing was installed.' }
$rpf = Join-Path $dlc 'dlc.rpf'
if (-not (Test-Path -LiteralPath $rpf -PathType Leaf) -or (Get-Item -LiteralPath $rpf).Length -lt 4096) { throw 'Tutorial DLC output is missing or incomplete; nothing was installed.' }
& (Join-Path $root 'install-tutorial-addon.ps1') -Mode Install -SourceTutorialDir $m0 -SourceDlcRpf $rpf -GtaDir $GtaDir
if ($LASTEXITCODE) { throw 'Tutorial Island installation failed.' }
