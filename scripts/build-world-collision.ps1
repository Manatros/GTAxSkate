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
#
# Bake to play (design 2026-11-16-bake-to-play):
#   - every stage is timed into <work>\timings.json and printed as "TIMING ...";
#   - the rails and the Story floor map are reused from build\prep-cache when
#     the producing tool, the keys, svsd/tracks and every archive the tools read
#     are unchanged (-NoCache forces a rebuild);
#   - -InstallPartial installs a snapshot of the finished priority tiles (made by
#     bake-snapshot.ps1) and gates exactly the cells it holds: verify-world and
#     segcheck get the cell list, so a partial set is gated like a complete one.
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
    [string]$InstallFrom,
    [switch]$InstallPartial,
    [string]$ResumeWork,
    [switch]$NoCache,
    [string]$CacheDir
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'install-asset-transaction.ps1')
. (Join-Path $PSScriptRoot 'prep-cache.ps1')
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
# Per-stage timings: <work>\timings.json, merged across runs on the same work
# folder (a resumed bake, the later install). Printed so progress.ps1 and the
# console show every line.
$script:timingsPath = $null
function Set-Timing([string]$Name, $Value, [string]$Note = '') {
    if (-not $script:timingsPath) { return }
    $t = [ordered]@{}
    if (Test-Path -LiteralPath $script:timingsPath) {
        try { (Get-Content -LiteralPath $script:timingsPath -Raw | ConvertFrom-Json).PSObject.Properties | ForEach-Object { $t[$_.Name] = $_.Value } } catch {}
    }
    $t[$Name] = $Value
    $t | ConvertTo-Json | Set-Content -LiteralPath $script:timingsPath -Encoding UTF8
    if ($Value -is [double] -or $Value -is [int]) { Write-Host ('TIMING {0,-20} {1,9:N1} s {2}' -f $Name, $Value, $Note) }
    else { Write-Host ('TIMING {0,-20} {1}' -f $Name, $Value) }
}
function Measure-Stage([string]$Name, [scriptblock]$Run, [string]$Note = '') {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $r = & $Run
    Set-Timing $Name ([math]::Round($sw.Elapsed.TotalSeconds, 1)) $Note
    $r
}
# Park-rule A/B switches (SKATEV_R1_OFF, SKATEV_R2_OFF, SKATEV_R3a_OFF, SKATEV_R3b_OFF)
# make the baker write a trial that is never installable (peer review 2026-11-16).
$trialSwitches = @(Get-ChildItem Env: | Where-Object { $_.Name -match '^SKATEV_R[0-9A-Za-z]+_OFF$' } | ForEach-Object { $_.Name })
if ($trialSwitches.Count) { throw "Rule trial switches are set ($($trialSwitches -join ', ')): a trial bake is never installable. Unset them to bake or install." }
if ($MinX -ge $MaxX -or $MinY -ge $MaxY -or
    @($MinX,$MaxX,$MinY,$MaxY | Where-Object { [math]::Abs($_) -gt 20000 }).Count) { throw 'Invalid collision region.' }
$World = [IO.Path]::GetFullPath($World)
if (-not $CacheDir) { $CacheDir = Join-Path (Split-Path $PSScriptRoot -Parent) 'build\prep-cache' }

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
    $work = if ($ResumeWork) { [IO.Path]::GetFullPath($ResumeWork) } else { Join-Path (Split-Path -Parent $World) ('world-build-' + [guid]::NewGuid().ToString('N')) }
}
$payload = Join-Path $work 'payload'
$tiles = Join-Path $payload 'tiles'
New-Item -ItemType Directory -Force -Path $work | Out-Null
$script:timingsPath = Join-Path $work 'timings.json'

if (-not $InstallFrom) {
    New-Item -ItemType Directory -Force -Path $payload | Out-Null
    # Material/track sidecars are existing setup inputs, not redistributable assets.
    # crackmaps.svgj (A60 ground alignment) is a required bake input beside the cache since rules 213170a43c53a499.
    foreach ($name in 'sk8v.svsd','sk8v.tracks','crackmaps.svgj') {
        $inputPath = Join-Path $World $name
        if (Test-Path -LiteralPath $inputPath) {
            # -ResumeWork: replace the copy from the earlier run (Copy-Item cannot copy a folder over an existing one).
            $staged = Join-Path $payload $name
            if (Test-Path -LiteralPath $staged) { Remove-Item -LiteralPath $staged -Recurse -Force }
            Copy-Item -LiteralPath $inputPath -Destination $payload -Recurse
        }
    }
    $rules = @(& $Baker --rules)
    if ($LASTEXITCODE -ne 0 -or $rules.Count -lt 1 -or $rules[0] -notmatch '^rules [0-9a-f]{16}$') { throw 'The baker does not report its rules (--rules); use a baker from the same build as the runtime.' }
    Write-Host "Baker $($rules[0]). Work folder: $work"
    $prepKey = $null
    if (-not $NoCache) {
        $prepKey = Get-PrepKeyParts -Gta $Gta -Keys $Keys -Payload $payload
    }
    $railFiles = 'sk8v.visrails','sk8v.visrails.policy','sk8v.visparks','sk8v.visparks.policy'
    $mapFiles = 'sk8v-map.svwc','sk8v-map.svwc.policy'
    Write-Host 'Preparing rails from Story Mode models. Progress logs are retained if preparation stops.'
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $hit = (-not $NoCache) -and (Restore-PrepCache -CacheDir $CacheDir -Stage 'rails' -Tool $Rails -Parts $prepKey -Files $railFiles -Payload $payload)
    if (-not $hit) {
        if ((Invoke-Tool (Join-Path $work 'rails.log') { & $Rails (Join-Path $payload 'sk8v.visrails') --gta $Gta --keys $Keys }) -ne 0) { throw "Rail export failed; working collision unchanged. See $work" }
        if (-not $NoCache) { Save-PrepCache -CacheDir $CacheDir -Stage 'rails' -Tool $Rails -Parts $prepKey -Files $railFiles -Payload $payload }
    }
    Set-Timing 'rails' ([math]::Round($sw.Elapsed.TotalSeconds, 1)) $(if ($NoCache) { '(cache off)' } elseif ($hit) { '(cache hit)' } else { '(built, cached)' })
    Set-Timing 'railsCache' $(if ($NoCache) { 'off' } elseif ($hit) { 'hit' } else { 'miss' })
    Write-Host 'Preparing the Story Mode floor fallback.'
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $hit = (-not $NoCache) -and (Restore-PrepCache -CacheDir $CacheDir -Stage 'map' -Tool $Baker -Parts $prepKey -Files $mapFiles -Payload $payload)
    if (-not $hit) {
        if ((Invoke-Tool (Join-Path $work 'map.log') { & $Baker --bake-story-map (Join-Path $payload 'sk8v-map.svwc') $Gta $Keys }) -ne 0) { throw "Floor preparation failed; working collision unchanged. See $work" }
        if (-not $NoCache) { Save-PrepCache -CacheDir $CacheDir -Stage 'map' -Tool $Baker -Parts $prepKey -Files $mapFiles -Payload $payload }
    }
    Set-Timing 'storyMap' ([math]::Round($sw.Elapsed.TotalSeconds, 1)) $(if ($NoCache) { '(cache off)' } elseif ($hit) { '(cache hit)' } else { '(built, cached)' })
    Set-Timing 'storyMapCache' $(if ($NoCache) { 'off' } elseif ($hit) { 'hit' } else { 'miss' })
    Write-Host "Preparing collision for x $MinX..$MaxX, y $MinY..$MaxY. This takes long; progress: $(Join-Path $work 'city.log')"
    $bakeStartUtc = [DateTime]::UtcNow
    $sw = [Diagnostics.Stopwatch]::StartNew()
    if ((Invoke-Tool (Join-Path $work 'city.log') { & $Baker --bake-world-box $tiles $MinX $MaxX $MinY $MaxY $Gta $Keys (Join-Path $work 'unused-flat-input') (Join-Path $payload 'sk8v.svwc') }) -ne 0) { throw "Collision preparation failed; working collision unchanged. See $work" }
    Set-Timing 'tilesAll' ([math]::Round($sw.Elapsed.TotalSeconds, 1)) '(whole region, this run)'
    # First / last priority tile of this run, from the tiles' write times.
    $prio = @()
    if ($env:SKATEV_BAKE_PRIORITY_FILE -and (Test-Path -LiteralPath $env:SKATEV_BAKE_PRIORITY_FILE)) {
        $prio = @(Get-Content -LiteralPath $env:SKATEV_BAKE_PRIORITY_FILE | ForEach-Object { ($_ -split '#')[0].Trim() } | Where-Object { $_ -match '^-?\d+\s+-?\d+$' } | ForEach-Object { ($_ -split '\s+') -join '_' })
    }
    $times = @($prio | ForEach-Object { Join-Path $tiles ($_ + '.svwt') } | Where-Object { Test-Path -LiteralPath $_ } |
        ForEach-Object { (Get-Item -LiteralPath $_).LastWriteTimeUtc } | Where-Object { $_ -ge $bakeStartUtc.AddSeconds(-1) } | Sort-Object)
    if ($times.Count) {
        Set-Timing 'tilesFirstPriority' ([math]::Round(($times[0] - $bakeStartUtc).TotalSeconds, 1)) '(after the baker started)'
        Set-Timing 'tilesAllPriority' ([math]::Round(($times[-1] - $bakeStartUtc).TotalSeconds, 1)) "($($times.Count) priority tiles written this run)"
    }
    $tileRules = [IO.File]::ReadAllText((Join-Path $tiles 'rules.txt')).Trim()
    if ($tileRules -ne $rules[0]) { throw "Baked tiles report '$tileRules' but the baker '$($rules[0])'." }
}

foreach ($name in 'sk8v.visrails','sk8v.visrails.policy','sk8v.visparks','sk8v.visparks.policy','sk8v-map.svwc','sk8v-map.svwc.policy','tiles/rules.txt','tiles/tile-size.txt','tiles/live-cells.txt') {
    if (-not (Test-Path -LiteralPath (Join-Path $payload $name) -PathType Leaf)) { throw "Incomplete prepared collision: $name. Working data unchanged; see $work" }
}
if (Test-Path -LiteralPath (Join-Path $tiles 'trial.txt')) { throw "Prepared collision is a rule trial (tiles\trial.txt): never installable. Working data unchanged; see $work" }
if (([IO.File]::ReadAllText((Join-Path $tiles 'rules.txt')).Trim()) -notmatch '^rules [0-9a-f]{16}$') { throw "Prepared collision reports trial rules ($([IO.File]::ReadAllText((Join-Path $tiles 'rules.txt')).Trim())): never installable. Working data unchanged; see $work" }
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
$bakedCount = 0; $liveCount = 0; $missingCount = 0
$presentCells = New-Object 'System.Collections.Generic.List[string]'
for ($x = [int][math]::Floor($MinX / 64.0); $x -le [int][math]::Floor($MaxX / 64.0); $x++) {
    for ($y = [int][math]::Floor($MinY / 64.0); $y -le [int][math]::Floor($MaxY / 64.0); $y++) {
        $tile = Join-Path $tiles ("{0}_{1}.svwt" -f $x,$y)
        if ($live.Contains("$x $y")) {
            if (Test-Path -LiteralPath $tile) { throw 'A state-dependent cell was also baked.' }
            $liveCount++
        } elseif (Test-Path -LiteralPath $tile -PathType Leaf) {
            $bakedCount++
            $presentCells.Add("$x $y")
        } elseif ($InstallPartial) {
            $missingCount++
        } else {
            throw "Missing prepared cell $x $y"
        }
    }
}
# A partial set is gated like a complete one, on exactly the cells it holds
# (the cell list reaches verify-world and segcheck). It used to skip both gates.
$cellArgs = @(); $segCells = @()
if ($InstallPartial -and ($missingCount -gt 0)) {
    if ($bakedCount -eq 0) { throw "A partial install needs at least one baked tile. See $work" }
    $cellFile = Join-Path $work 'gate-cells.txt'
    Set-Content -LiteralPath $cellFile -Value $presentCells -Encoding ASCII
    $cellArgs = @($cellFile); $segCells = @('--cells', $cellFile)
}
$verifyKey = if ($InstallFrom) { 'installVerify' } else { 'verify' }
$segKey = if ($InstallFrom) { 'installSegcheck' } else { 'segcheck' }
$sw = [Diagnostics.Stopwatch]::StartNew()
if ((Invoke-Tool (Join-Path $work 'verify.log') { & $Baker --verify-world $payload $MinX $MaxX $MinY $MaxY @cellArgs }) -ne 0) { throw "Prepared collision failed native verification; working data unchanged. See $work" }
Set-Timing $verifyKey ([math]::Round($sw.Elapsed.TotalSeconds, 1)) $(if ($cellArgs.Count) { "($bakedCount listed cells)" } else { '' })
# Skate's grind index holds 65,535 segments per 5 x 5 tile area; over it an
# area fails to load and the previous collision stays (2026-10-25).
$segcheck = Join-Path $PSScriptRoot 'world-inspect\segcheck.py'
$py = Get-Command python -ErrorAction SilentlyContinue
$sw = [Diagnostics.Stopwatch]::StartNew()
if ($py -and (Test-Path -LiteralPath $segcheck)) {
    if ((Invoke-Tool (Join-Path $work 'segcheck.log') { & $py.Source $segcheck $tiles 52000 @segCells }) -ne 0) { throw "Too many grind segments in one area (see $work\segcheck.log); working data unchanged." }
    Set-Timing $segKey ([math]::Round($sw.Elapsed.TotalSeconds, 1))
} else { Write-Host 'Grind segment check skipped (no python).' }
$names = @('sk8v.visrails','sk8v.visrails.policy','sk8v.visparks','sk8v.visparks.policy','sk8v-map.svwc','sk8v-map.svwc.policy','tiles/rules.txt','tiles/tile-size.txt','tiles/live-cells.txt')
$tileNames = @(Get-ChildItem -LiteralPath $tiles -File | Where-Object { $_.Name -match '^-?\d+_-?\d+\.svwt$' } | ForEach-Object { 'tiles/' + $_.Name })
$names += $tileNames
$partialMarker = Join-Path $tiles 'partial.json'
if ($InstallPartial -and ($missingCount -gt 0)) {
    [ordered]@{ rules=$policy[1]; baked=$bakedCount; total=($bakedCount + $missingCount) } | ConvertTo-Json | Set-Content -LiteralPath $partialMarker -Encoding UTF8
    $names += 'tiles/partial.json'
} elseif (Test-Path -LiteralPath $partialMarker) {
    Remove-Item -LiteralPath $partialMarker -Force -ErrorAction SilentlyContinue
}
if (-not $InstallFrom -and -not $InstallPartial) {
    $receipt = [ordered]@{ format=2; preparedUtc=[DateTime]::UtcNow.ToString('o'); rules=$policy[1]; source=$policy[2]; bakedCells=$bakedCount; liveCells=$liveCount;
        region=@($MinX,$MaxX,$MinY,$MaxY);
        bakerSha256=(Get-FileHash -LiteralPath $Baker -Algorithm SHA256).Hash; railsSha256=(Get-FileHash -LiteralPath $Rails -Algorithm SHA256).Hash }
    $receipt | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $payload 'collision-build.json') -Encoding UTF8
}
if (Test-Path -LiteralPath (Join-Path $payload 'collision-build.json')) {
    $names += 'collision-build.json'
}
if ($StageOnly) {
    Write-Host "Staged and verified: $bakedCount baked cells, $liveCount active-state cells ($($policy[1])). Install with -InstallFrom `"$work`" while the game is closed."
    return
}
$files = @($names | ForEach-Object { [pscustomobject]@{ name=$_; sha256=(Get-FileHash -LiteralPath (Join-Path $payload $_) -Algorithm SHA256).Hash } })
Assert-GameClosed
$installSw = [Diagnostics.Stopwatch]::StartNew()
$backup = Join-Path (Split-Path -Parent $World) ('world-recovery-' + [guid]::NewGuid().ToString('N'))
Install-AssetTransaction -TargetRoot $World -StageRoot $work -Files $files -BackupRoot $backup | Out-Null
if ((-not $InstallPartial) -or ($missingCount -eq 0)) {
    $installedPartial = Join-Path $World 'tiles\partial.json'
    if (Test-Path -LiteralPath $installedPartial -PathType Leaf) {
        Remove-Item -LiteralPath $installedPartial -Force -ErrorAction SilentlyContinue
    }
}
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
    # Not ready while tiles of another bake stay beside the new set (Copilot review 2026-11-16).
    Write-Host "NOT READY: $($stuck.Count) tiles of an older bake could not be moved aside ($($stuck[0])...). Close the game and run again with -InstallFrom `"$work`"." -ForegroundColor Red
    exit 3
}
Set-Timing 'install' ([math]::Round($installSw.Elapsed.TotalSeconds, 1)) $(if ($InstallPartial -and $missingCount -gt 0) { '(partial)' } else { '(whole set)' })
Write-Host "Collision ready: $bakedCount baked cells and $liveCount active-state cells; $($obsolete.Count - $stuck.Count) obsolete tiles moved aside. Recovery files: $backup"
# Keep staging/logs: interrupted preparation and recovery must remain inspectable.
