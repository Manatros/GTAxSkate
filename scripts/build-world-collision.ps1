# Generate collision from the player's Story archives, then promote a complete
# verified dataset. Never replace a working world with a partial bake.
#
# The one collision pipeline (installer and developers alike; 2026-10-24 it
# replaced build/v071-rc/run-autoheal-world-bake.ps1 and run-parallel-city-bake.ps1):
#   visual rails -> Story floor map -> tiles (one baker process, threads inside)
#   -> native verification -> completeness and rules gate -> transactional install.
# -StageOnly bakes and verifies without touching the game folder (the game may
# run meanwhile); -InstallFrom <work folder> installs such a staged result.
# Every installed tile comes from one bake: tiles of other bakes are moved into
# the recovery folder, because the runtime silently skips them (2026-10-23).
param(
    [string]$Gta,
    [string]$Keys,
    [string]$Baker,
    [string]$Rails,
    [Parameter(Mandatory)][string]$World,
    # The whole Story map: interiors, tunnels and metro included (Manatros
    # 2026-10-23: "the entire map, open world, interiors, tunnels, metro").
    [int]$MinX = -4096, [int]$MaxX = 4607,
    [int]$MinY = -4096, [int]$MaxY = 8191,
    [switch]$StageOnly,
    [string]$InstallFrom
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'install-asset-transaction.ps1')
# Native tools print progress on stderr. Windows PowerShell 5.1 turns that into
# errors under ErrorAction Stop and aborts a successful run (Codex review
# 2026-10-24): run them with stderr tolerated and judge only the exit code.
function Invoke-Tool([string]$Log, [scriptblock]$Run) {
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { & $Run *> $Log } finally { $ErrorActionPreference = $saved }
    $LASTEXITCODE
}
function Assert-GameClosed {
    if (Get-Process GTA5,GTA5_Enhanced,FiveM,OpenIV,CodeWalker -ErrorAction SilentlyContinue) {
        throw 'Close GTA, FiveM, OpenIV and CodeWalker before installing collision.'
    }
}
if ($MinX -ge $MaxX -or $MinY -ge $MaxY -or
    @($MinX,$MaxX,$MinY,$MaxY | Where-Object { [math]::Abs($_) -gt 20000 }).Count) { throw 'Invalid collision region.' }
$World = [IO.Path]::GetFullPath($World)

if ($InstallFrom) {
    $work = [IO.Path]::GetFullPath($InstallFrom)
    $receiptPath = Join-Path $work 'payload\collision-build.json'
    if (-not (Test-Path -LiteralPath $receiptPath)) { throw "No staged collision in $work (missing payload\collision-build.json)." }
    $staged = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
    $MinX, $MaxX, $MinY, $MaxY = $staged.region
    if (-not $Baker -or -not (Test-Path -LiteralPath $Baker -PathType Leaf)) { throw '-Baker is required to verify the staged collision.' }
} else {
    if (-not $StageOnly) { Assert-GameClosed }
    foreach ($p in 'Gta','Keys','Baker','Rails') { if (-not (Get-Variable $p -ValueOnly)) { throw "-$p is required to bake." } }
    foreach ($tool in $Baker,$Rails) { if (-not (Test-Path -LiteralPath $tool -PathType Leaf)) { throw "Missing collision tool: $tool" } }
    $work = Join-Path (Split-Path -Parent $World) ('world-build-' + [guid]::NewGuid().ToString('N'))
}
$payload = Join-Path $work 'payload'
$tiles = Join-Path $payload 'tiles'

if (-not $InstallFrom) {
    New-Item -ItemType Directory -Force -Path $payload | Out-Null
    # Material/track sidecars are existing setup inputs, not redistributable assets.
    foreach ($name in 'sk8v.svsd','sk8v.tracks') {
        $inputPath = Join-Path $World $name
        if (Test-Path -LiteralPath $inputPath) { Copy-Item -LiteralPath $inputPath -Destination $payload -Recurse }
    }
    $rules = @(& $Baker --rules)
    if ($LASTEXITCODE -ne 0 -or $rules.Count -lt 1 -or $rules[0] -notmatch '^rules [0-9a-f]{16}$') { throw 'The baker does not report its rules (--rules); use a baker from the same build as the runtime.' }
    Write-Host "Baker $($rules[0]). Work folder: $work"
    Write-Host 'Preparing rails from Story Mode models. Progress logs are retained if preparation stops.'
    if ((Invoke-Tool (Join-Path $work 'rails.log') { & $Rails (Join-Path $payload 'sk8v.visrails') --gta $Gta --keys $Keys }) -ne 0) { throw "Rail export failed; working collision unchanged. See $work" }
    Write-Host 'Preparing the Story Mode floor fallback.'
    if ((Invoke-Tool (Join-Path $work 'map.log') { & $Baker --bake-story-map (Join-Path $payload 'sk8v-map.svwc') $Gta $Keys }) -ne 0) { throw "Floor preparation failed; working collision unchanged. See $work" }
    Write-Host "Preparing collision for x $MinX..$MaxX, y $MinY..$MaxY. This takes long; progress: $(Join-Path $work 'city.log')"
    if ((Invoke-Tool (Join-Path $work 'city.log') { & $Baker --bake-world-box $tiles $MinX $MaxX $MinY $MaxY $Gta $Keys (Join-Path $work 'unused-flat-input') (Join-Path $payload 'sk8v.svwc') }) -ne 0) { throw "Collision preparation failed; working collision unchanged. See $work" }
    $tileRules = [IO.File]::ReadAllText((Join-Path $tiles 'rules.txt')).Trim()
    if ($tileRules -ne $rules[0]) { throw "Baked tiles report '$tileRules' but the baker '$($rules[0])'." }
}

if ((Invoke-Tool (Join-Path $work 'verify.log') { & $Baker --verify-world $payload $MinX $MaxX $MinY $MaxY }) -ne 0) { throw "Prepared collision failed native verification; working data unchanged. See $work" }
# Skate's grind index holds 65,535 segments per 5 x 5 tile area; over it an
# area fails to load and the previous collision stays (2026-10-25).
$segcheck = Join-Path $PSScriptRoot 'world-inspect\segcheck.py'
$py = Get-Command python -ErrorAction SilentlyContinue
if ($py -and (Test-Path -LiteralPath $segcheck)) {
    if ((Invoke-Tool (Join-Path $work 'segcheck.log') { & $py.Source $segcheck $tiles 52000 }) -ne 0) { throw "Too many grind segments in one area (see $work\segcheck.log); working data unchanged." }
} else { Write-Host 'Grind segment check skipped (no python).' }
foreach ($name in 'sk8v.visrails','sk8v.visrails.policy','sk8v.visparks','sk8v.visparks.policy','sk8v-map.svwc','sk8v-map.svwc.policy','tiles/rules.txt','tiles/tile-size.txt','tiles/live-cells.txt') {
    if (-not (Test-Path -LiteralPath (Join-Path $payload $name) -PathType Leaf)) { throw "Incomplete prepared collision: $name. Working data unchanged; see $work" }
}
$size = [float]([IO.File]::ReadAllText((Join-Path $tiles 'tile-size.txt')).Trim())
if ($size -ne 64) { throw 'Unexpected tile size.' }
$policy = @([IO.File]::ReadAllLines((Join-Path $tiles 'live-cells.txt')))
if ($policy.Count -lt 5 -or $policy[0] -ne 'SVWT-LIVE 1' -or $policy[1] -ne ([IO.File]::ReadAllText((Join-Path $tiles 'rules.txt')).Trim()) -or
    $policy[4] -notmatch '^cells (\d+) [0-9a-f]{16}$') { throw 'Invalid prepared map-state policy.' }
$expectedLiveCount = [int]$Matches[1]
$live = New-Object 'System.Collections.Generic.HashSet[string]'
foreach ($line in @($policy | Select-Object -Skip 5)) {
    if ($line -notmatch '^(-?\d+) (-?\d+)$' -or -not $live.Add($line)) { throw 'Invalid/duplicate map-state cell.' }
}
if ($live.Count -ne $expectedLiveCount) { throw 'Truncated map-state policy.' }
$bakedCount = 0; $liveCount = 0
for ($x = [int][math]::Floor($MinX / 64.0); $x -le [int][math]::Floor($MaxX / 64.0); $x++) {
    for ($y = [int][math]::Floor($MinY / 64.0); $y -le [int][math]::Floor($MaxY / 64.0); $y++) {
        $tile = Join-Path $tiles ("{0}_{1}.svwt" -f $x,$y)
        if ($live.Contains("$x $y")) {
            if (Test-Path -LiteralPath $tile) { throw 'A state-dependent cell was also baked.' }
            $liveCount++
        } else {
            if (-not (Test-Path -LiteralPath $tile -PathType Leaf)) { throw "Missing prepared cell $x $y" }
            $bakedCount++
        }
    }
}
$names = @('sk8v.visrails','sk8v.visrails.policy','sk8v.visparks','sk8v.visparks.policy','sk8v-map.svwc','sk8v-map.svwc.policy','tiles/rules.txt','tiles/tile-size.txt','tiles/live-cells.txt')
$tileNames = @(Get-ChildItem -LiteralPath $tiles -Filter '*.svwt' -File | ForEach-Object { 'tiles/' + $_.Name })
$names += $tileNames
if (-not $InstallFrom) {
    $receipt = [ordered]@{ format=2; preparedUtc=[DateTime]::UtcNow.ToString('o'); rules=$policy[1]; source=$policy[2]; bakedCells=$bakedCount; liveCells=$liveCount;
        region=@($MinX,$MaxX,$MinY,$MaxY);
        bakerSha256=(Get-FileHash -LiteralPath $Baker -Algorithm SHA256).Hash; railsSha256=(Get-FileHash -LiteralPath $Rails -Algorithm SHA256).Hash }
    $receipt | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $payload 'collision-build.json') -Encoding UTF8
}
$names += 'collision-build.json'
if ($StageOnly) {
    Write-Host "Staged and verified: $bakedCount baked cells, $liveCount active-state cells ($($policy[1])). Install with -InstallFrom `"$work`" while the game is closed."
    return
}
$files = @($names | ForEach-Object { [pscustomobject]@{ name=$_; sha256=(Get-FileHash -LiteralPath (Join-Path $payload $_) -Algorithm SHA256).Hash } })
Assert-GameClosed
$backup = Join-Path (Split-Path -Parent $World) ('world-recovery-' + [guid]::NewGuid().ToString('N'))
Install-AssetTransaction -TargetRoot $World -StageRoot $work -Files $files -BackupRoot $backup | Out-Null
# Whole-set replacement: tiles from another bake must not stay beside this one.
$keep = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)
foreach ($n in $tileNames) { [void]$keep.Add((Split-Path -Leaf $n)) }
$installedTiles = Join-Path $World 'tiles'
$obsolete = @(Get-ChildItem -LiteralPath $installedTiles -Filter '*.svwt' -File | Where-Object { -not $keep.Contains($_.Name) })
# This step follows the transaction: the new set is complete either way, and a
# tile of another bake is only skipped (with a log warning) by the runtime.
$stuck = @()
if ($obsolete.Count) {
    $aside = Join-Path $backup 'obsolete-tiles'
    New-Item -ItemType Directory -Force -Path $aside | Out-Null
    foreach ($f in $obsolete) {
        try { Move-Item -LiteralPath $f.FullName -Destination (Join-Path $aside $f.Name) -ErrorAction Stop } catch { $stuck += $f.Name }
    }
}
if ($stuck.Count) {
    Write-Warning "$($stuck.Count) tiles of an older bake could not be moved aside ($($stuck[0])...). Close the game and run again with -InstallFrom `"$work`"."
}
Write-Host "Collision ready: $bakedCount baked cells and $liveCount active-state cells; $($obsolete.Count - $stuck.Count) obsolete tiles moved aside. Recovery files: $backup"
# Keep staging/logs: interrupted preparation and recovery must remain inspectable.
