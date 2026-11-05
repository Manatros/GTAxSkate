$ErrorActionPreference = 'Stop'
$fixture = Join-Path (Split-Path -Parent $PSScriptRoot) ('build/world-install-fixture-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $fixture | Out-Null
function Assert($condition, $message) { if (-not $condition) { throw $message } }
function Get-Process { [CmdletBinding()]param([string[]]$Name) if ($global:GtaxskateFixtureGameRunning) { [pscustomobject]@{ Name='GTA5' } } }
# A real native tool that reports progress on stderr (Windows PowerShell 5.1
# must not abort on it).
$rails = Join-Path $fixture 'rails.cmd'
$baker = Join-Path $fixture 'baker.ps1'
@'
@echo off
echo rails: progress 1>&2
>"%~1" echo rails
>"%~1.policy" echo rail-policy
>"%~dpn1.visparks" echo parks
>"%~dpn1.visparks.policy" echo park-policy
exit /b 0
'@ | Set-Content -LiteralPath $rails -Encoding ASCII
@'
if ($args[0] -eq '--rules') { 'rules 0123456789abcdef'; 'floor-policy 0000000000000000'; exit 0 }
if ($args[0] -eq '--verify-world') {
    if ($env:GTAXSKATE_WORLD_FIXTURE -eq 'bad-verify') { exit 11 }
    exit 0
}
if ($args[0] -eq '--bake-story-map') {
    [IO.File]::WriteAllText($args[1], 'map')
    [IO.File]::WriteAllText($args[1]+'.policy', 'map-policy')
    exit 0
}
$out = $args[1]
New-Item -ItemType Directory -Force -Path $out | Out-Null
$rules = if ($env:GTAXSKATE_WORLD_FIXTURE -eq 'badrules') { 'rules fedcba9876543210' } else { 'rules 0123456789abcdef' }
[IO.File]::WriteAllText((Join-Path $out 'rules.txt'), $rules)
[IO.File]::WriteAllText((Join-Path $out 'tile-size.txt'), '64')
$body = if ($env:GTAXSKATE_WORLD_FIXTURE -eq 'stateful') { "0 0`n" } else { '' }
$count = if ($body) { 1 } else { 0 }
[IO.File]::WriteAllText((Join-Path $out 'live-cells.txt'), "SVWT-LIVE 1`n$rules`nsource fixture`nsize 64`ncells $count 0000000000000000`n$body")
if ($env:GTAXSKATE_WORLD_FIXTURE -eq 'fail') { exit 9 }
if ($env:GTAXSKATE_WORLD_FIXTURE -notin @('incomplete','stateful')) { [IO.File]::WriteAllText((Join-Path $out '0_0.svwt'), 'tile') }
exit 0
'@ | Set-Content -LiteralPath $baker
$helper = Join-Path $PSScriptRoot 'build-world-collision.ps1'
$passed = 0
try {
    foreach ($mode in 'fail','incomplete','bad-verify','badrules','static','stateful') {
        $env:GTAXSKATE_WORLD_FIXTURE = $mode
        $world = Join-Path $fixture ($mode + ' world')
        New-Item -ItemType Directory -Force -Path $world | Out-Null
        [IO.File]::WriteAllText((Join-Path $world 'sk8v-map.svwc'), 'old-map')
        [IO.File]::WriteAllText((Join-Path $world 'sk8v.visrails'), 'old-rails')
        New-Item -ItemType Directory -Force -Path (Join-Path $world 'tiles') | Out-Null
        [IO.File]::WriteAllText((Join-Path $world 'tiles\9_9.svwt'), 'tile of an older bake')
        $failed = $false
        try { & $helper -Gta $fixture -Keys $fixture -Baker $baker -Rails $rails -World $world -MinX 0 -MaxX 63 -MinY 0 -MaxY 63 | Out-Null }
        catch { $failed = $true; $why = $_.Exception.Message }
        if ($mode -in 'fail','incomplete','bad-verify','badrules') {
            Assert $failed "$mode must stop"
            Assert ([IO.File]::ReadAllText((Join-Path $world 'sk8v-map.svwc')) -eq 'old-map') "$mode changed the map"
            Assert ([IO.File]::ReadAllText((Join-Path $world 'sk8v.visrails')) -eq 'old-rails') "$mode changed the rails"
        } else {
            Assert (-not $failed) "$mode failed: $why"
            Assert ([IO.File]::ReadAllText((Join-Path $world 'sk8v-map.svwc')) -eq 'map') "$mode map not promoted"
            $receipt = Get-Content -LiteralPath (Join-Path $world 'collision-build.json') -Raw | ConvertFrom-Json
            Assert (($receipt.bakedCells + $receipt.liveCells) -eq 1) 'coverage must include intentional live cells'
            Assert ($receipt.liveCells -eq [int]($mode -eq 'stateful')) 'wrong state coverage'
            Assert (-not (Test-Path -LiteralPath (Join-Path $world 'tiles\9_9.svwt'))) "$mode left a tile of an older bake installed"
        }
        $passed++
    }
    # Stage while the game runs, install once it is closed.
    $env:GTAXSKATE_WORLD_FIXTURE = 'static'
    $world = Join-Path $fixture 'staged world'
    New-Item -ItemType Directory -Force -Path $world | Out-Null
    [IO.File]::WriteAllText((Join-Path $world 'sk8v-map.svwc'), 'old-map')
    $global:GtaxskateFixtureGameRunning = $true
    & $helper -Gta $fixture -Keys $fixture -Baker $baker -Rails $rails -World $world -MinX 0 -MaxX 63 -MinY 0 -MaxY 63 -StageOnly | Out-Null
    Assert ([IO.File]::ReadAllText((Join-Path $world 'sk8v-map.svwc')) -eq 'old-map') 'stage-only changed the game folder'
    $work = @(Get-ChildItem -LiteralPath $fixture -Directory -Filter 'world-build-*' | Sort-Object LastWriteTime | Select-Object -Last 1)[0].FullName
    $failed = $false
    try { & $helper -World $world -Baker $baker -InstallFrom $work | Out-Null } catch { $failed = $true }
    Assert $failed 'install while the game runs was not blocked'
    $global:GtaxskateFixtureGameRunning = $false
    & $helper -World $world -Baker $baker -InstallFrom $work | Out-Null
    Assert ([IO.File]::ReadAllText((Join-Path $world 'sk8v-map.svwc')) -eq 'map') 'staged map not installed'
    $passed++
    $global:GtaxskateFixtureGameRunning = $true
    $failed = $false
    try { & $helper -Gta $fixture -Keys $fixture -Baker $baker -Rails $rails -World (Join-Path $fixture 'blocked') | Out-Null } catch { $failed = $true }
    Assert $failed 'running game was not blocked'
    Assert (-not (Test-Path -LiteralPath (Join-Path $fixture 'blocked'))) 'running game changed destination'
    $passed++
    Write-Host "World collision installer: $passed cases pass."
    Remove-Item -LiteralPath $fixture -Recurse -Force
} finally { Remove-Item Env:GTAXSKATE_WORLD_FIXTURE -ErrorAction SilentlyContinue; Remove-Variable GtaxskateFixtureGameRunning -Scope Global -ErrorAction SilentlyContinue }
