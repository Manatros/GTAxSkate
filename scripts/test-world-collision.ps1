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
if defined GTAXSKATE_FIXTURE_CALLS >>"%GTAXSKATE_FIXTURE_CALLS%" echo rails
>"%~1" echo rails
>"%~1.policy" echo rail-policy
>"%~dpn1.visparks" echo parks
>"%~dpn1.visparks.policy" echo park-policy
exit /b 0
'@ | Set-Content -LiteralPath $rails -Encoding ASCII
@'
if ($args[0] -eq '--rules') { 'rules 0123456789abcdef'; 'floor-policy 0000000000000000'; exit 0 }
if ($args[0] -eq '--verify-world') {
    if ($env:GTAXSKATE_FIXTURE_LOG) { Add-Content $env:GTAXSKATE_FIXTURE_LOG ('verify args=' + $args.Count) }
    if ($env:GTAXSKATE_WORLD_FIXTURE -eq 'bad-verify' -or $env:GTAXSKATE_FIXTURE_VERIFY -eq 'bad') { exit 11 }
    # With a cell list (7th argument): exactly these cells must exist.
    if ($args.Count -ge 7) { foreach ($l in Get-Content $args[6]) { $w = $l.Trim() -split '\s+'; if (-not (Test-Path (Join-Path $args[1] "tiles\$($w[0])_$($w[1]).svwt"))) { exit 12 } } }
    exit 0
}
if ($args[0] -eq '--bake-story-map') {
    if ($env:GTAXSKATE_FIXTURE_CALLS) { Add-Content $env:GTAXSKATE_FIXTURE_CALLS 'map' }
    [IO.File]::WriteAllText($args[1], 'map')
    [IO.File]::WriteAllText($args[1]+'.policy', 'map-policy')
    exit 0
}
$out = $args[1]
New-Item -ItemType Directory -Force -Path $out | Out-Null
$rules = if ($env:GTAXSKATE_WORLD_FIXTURE -eq 'badrules') { 'rules fedcba9876543210' } else { 'rules 0123456789abcdef' }
# A trial bake (SKATEV_R*_OFF): the real baker labels rules.txt and writes trial.txt.
$label = if ($env:GTAXSKATE_WORLD_FIXTURE -eq 'trial') { "$rules trial R2" } else { $rules }
[IO.File]::WriteAllText((Join-Path $out 'rules.txt'), $label)
if ($env:GTAXSKATE_WORLD_FIXTURE -eq 'trial') { [IO.File]::WriteAllText((Join-Path $out 'trial.txt'), 'trial bake: park rules switched off: R2') }
[IO.File]::WriteAllText((Join-Path $out 'tile-size.txt'), '64')
$body = if ($env:GTAXSKATE_WORLD_FIXTURE -eq 'stateful') { "0 0`n" } elseif ($env:GTAXSKATE_WORLD_FIXTURE -eq 'multi') { "3 0`n" } else { '' }
$count = if ($body) { 1 } else { 0 }
[IO.File]::WriteAllText((Join-Path $out 'live-cells.txt'), "SVWT-LIVE 1`n$rules`nsource fixture`nsize 64`ncells $count 0000000000000000`n$body")
if ($env:GTAXSKATE_WORLD_FIXTURE -eq 'multi') {
    # Like the real baker: rules, size and live cells are already on disk; tiles
    # follow, priority cells first, each published by a rename from .svwt.tmp.
    $cells = foreach ($x in ([int][math]::Floor([int]$args[2] / 64))..([int][math]::Floor([int]$args[3] / 64))) { foreach ($y in ([int][math]::Floor([int]$args[4] / 64))..([int][math]::Floor([int]$args[5] / 64))) { if (-not ($x -eq 3 -and $y -eq 0)) { "$x $y" } } }
    $prio = @(Get-Content $env:SKATEV_BAKE_PRIORITY_FILE | ForEach-Object { $_.Trim() })
    $ordered = @($cells | Where-Object { $prio -contains $_ }) + @($cells | Where-Object { $prio -notcontains $_ })
    $firstRest = @($cells | Where-Object { $prio -contains $_ }).Count
    for ($i = 0; $i -lt $ordered.Count; $i++) {
        if ($i -eq $firstRest) { for ($w = 0; $w -lt 240 -and -not (Test-Path $env:GTAXSKATE_FIXTURE_GATE); $w++) { Start-Sleep -Milliseconds 500 } }
        $c = $ordered[$i] -split ' '
        $p = Join-Path $out "$($c[0])_$($c[1]).svwt"
        [IO.File]::WriteAllText("$p.tmp", 'tile'); Move-Item "$p.tmp" $p
    }
    exit 0
}
if ($env:GTAXSKATE_WORLD_FIXTURE -eq 'fail') { exit 9 }
if ($env:GTAXSKATE_WORLD_FIXTURE -notin @('incomplete','stateful')) { [IO.File]::WriteAllText((Join-Path $out '0_0.svwt'), 'tile') }
exit 0
'@ | Set-Content -LiteralPath $baker
$helper = Join-Path $PSScriptRoot 'build-world-collision.ps1'
$fixtureCache = Join-Path $fixture 'prep-cache-default'
$passed = 0
try {
    foreach ($mode in 'fail','incomplete','bad-verify','badrules','trial','static','stateful') {
        $env:GTAXSKATE_WORLD_FIXTURE = $mode
        $world = Join-Path $fixture ($mode + ' world')
        New-Item -ItemType Directory -Force -Path $world | Out-Null
        [IO.File]::WriteAllText((Join-Path $world 'sk8v-map.svwc'), 'old-map')
        [IO.File]::WriteAllText((Join-Path $world 'sk8v.visrails'), 'old-rails')
        New-Item -ItemType Directory -Force -Path (Join-Path $world 'tiles') | Out-Null
        [IO.File]::WriteAllText((Join-Path $world 'tiles\9_9.svwt'), 'tile of an older bake')
        $failed = $false
        try { & $helper -CacheDir $fixtureCache -Gta $fixture -Keys $fixture -Baker $baker -Rails $rails -World $world -MinX 0 -MaxX 63 -MinY 0 -MaxY 63 | Out-Null }
        catch { $failed = $true; $why = $_.Exception.Message }
        if ($mode -in 'fail','incomplete','bad-verify','badrules','trial') {
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
    # Rule trial switches in the environment stop a bake before it starts.
    $env:GTAXSKATE_WORLD_FIXTURE = 'static'
    $env:SKATEV_R2_OFF = '1'
    $world = Join-Path $fixture 'switched world'
    New-Item -ItemType Directory -Force -Path $world | Out-Null
    [IO.File]::WriteAllText((Join-Path $world 'sk8v-map.svwc'), 'old-map')
    $failed = $false
    try { & $helper -CacheDir $fixtureCache -Gta $fixture -Keys $fixture -Baker $baker -Rails $rails -World $world -MinX 0 -MaxX 63 -MinY 0 -MaxY 63 | Out-Null } catch { $failed = $true; $why = $_.Exception.Message }
    Remove-Item Env:SKATEV_R2_OFF
    Assert $failed 'a bake with SKATEV_R2_OFF set was not refused'
    Assert ($why -like '*trial*') "wrong refusal: $why"
    Assert ([IO.File]::ReadAllText((Join-Path $world 'sk8v-map.svwc')) -eq 'old-map') 'the switched bake changed the map'
    $passed++
    # Stage while the game runs, install once it is closed.
    $env:GTAXSKATE_WORLD_FIXTURE = 'static'
    $world = Join-Path $fixture 'staged world'
    New-Item -ItemType Directory -Force -Path $world | Out-Null
    [IO.File]::WriteAllText((Join-Path $world 'sk8v-map.svwc'), 'old-map')
    $global:GtaxskateFixtureGameRunning = $true
    & $helper -CacheDir $fixtureCache -Gta $fixture -Keys $fixture -Baker $baker -Rails $rails -World $world -MinX 0 -MaxX 63 -MinY 0 -MaxY 63 -StageOnly | Out-Null
    Assert ([IO.File]::ReadAllText((Join-Path $world 'sk8v-map.svwc')) -eq 'old-map') 'stage-only changed the game folder'
    $work = @(Get-ChildItem -LiteralPath $fixture -Directory -Filter 'world-build-*' | Sort-Object LastWriteTime | Select-Object -Last 1)[0].FullName
    $failed = $false
    try { & $helper -CacheDir $fixtureCache -World $world -Baker $baker -InstallFrom $work | Out-Null } catch { $failed = $true }
    Assert $failed 'install while the game runs was not blocked'
    $global:GtaxskateFixtureGameRunning = $false
    # A staged trial (tiles\trial.txt) is refused at install time too.
    $trialMarker = Join-Path $work 'payload\tiles\trial.txt'
    [IO.File]::WriteAllText($trialMarker, 'trial bake: park rules switched off: R3b')
    $failed = $false
    try { & $helper -CacheDir $fixtureCache -World $world -Baker $baker -InstallFrom $work | Out-Null } catch { $failed = $true; $why = $_.Exception.Message }
    Assert $failed 'a staged trial bake was installed'
    Assert ($why -like '*trial*') "wrong refusal: $why"
    Assert ([IO.File]::ReadAllText((Join-Path $world 'sk8v-map.svwc')) -eq 'old-map') 'the staged trial changed the game folder'
    Remove-Item -LiteralPath $trialMarker
    & $helper -CacheDir $fixtureCache -World $world -Baker $baker -InstallFrom $work | Out-Null
    Assert ([IO.File]::ReadAllText((Join-Path $world 'sk8v-map.svwc')) -eq 'map') 'staged map not installed'
    $passed++
    $global:GtaxskateFixtureGameRunning = $true
    $failed = $false
    try { & $helper -CacheDir $fixtureCache -Gta $fixture -Keys $fixture -Baker $baker -Rails $rails -World (Join-Path $fixture 'blocked') | Out-Null } catch { $failed = $true }
    Assert $failed 'running game was not blocked'
    Assert (-not (Test-Path -LiteralPath (Join-Path $fixture 'blocked'))) 'running game changed destination'
    $passed++
    # --- bake to play (design 2026-11-16-bake-to-play) ---
    . (Join-Path $PSScriptRoot 'bake-snapshot.ps1')
    $global:GtaxskateFixtureGameRunning = $false
    $keys = Join-Path $fixture 'keys.bin'
    [IO.File]::WriteAllText($keys, 'keys v1')
    [IO.File]::WriteAllText((Join-Path $fixture 'x64a.rpf'), 'archive v1')
    # Reuse of unchanged preparation: same tools + same inputs -> reused; one
    # changed tool or input -> rebuilt; -NoCache always rebuilds.
    $env:GTAXSKATE_WORLD_FIXTURE = 'static'
    $calls = Join-Path $fixture 'calls.txt'
    $env:GTAXSKATE_FIXTURE_CALLS = $calls
    $cacheB = Join-Path $fixture 'prep-cache-b'
    function Invoke-Stage([string]$Name, [hashtable]$Extra = @{}) {
        $w = Join-Path $fixture "cw-$Name world"
        New-Item -ItemType Directory -Force -Path $w | Out-Null
        $d = Join-Path $fixture "cw-$Name"
        & $helper -Gta $fixture -Keys $keys -Baker $baker -Rails $rails -World $w -MinX 0 -MaxX 63 -MinY 0 -MaxY 63 -StageOnly -ResumeWork $d -CacheDir $cacheB @Extra | Out-Null
        Get-Content -LiteralPath (Join-Path $d 'timings.json') -Raw | ConvertFrom-Json
    }
    function Get-Calls([string]$What) { @(Get-Content -LiteralPath $calls -ErrorAction SilentlyContinue | Where-Object { $_ -eq $What }).Count }
    $r = Invoke-Stage 'c1'
    Assert ($r.railsCache -eq 'miss' -and $r.storyMapCache -eq 'miss') 'first run must build rails and map'
    Assert ((Get-Calls 'rails') -eq 1 -and (Get-Calls 'map') -eq 1) 'first run tool calls'
    $r = Invoke-Stage 'c2'
    Assert ($r.railsCache -eq 'hit' -and $r.storyMapCache -eq 'hit') 'unchanged inputs must reuse rails and map'
    Assert ((Get-Calls 'rails') -eq 1 -and (Get-Calls 'map') -eq 1) 'a cache hit must not run the tools'
    Assert ([IO.File]::ReadAllText((Join-Path $fixture 'cw-c2\payload\sk8v-map.svwc.policy')) -match 'map-policy') 'the map policy sidecar must come with the cached map'
    Assert ([IO.File]::ReadAllText((Join-Path $fixture 'cw-c2\payload\sk8v.visparks.policy')) -match 'park-policy') 'the parks policy sidecar must come with the cached rails'
    Add-Content -LiteralPath $baker -Value '# changed baker'
    $r = Invoke-Stage 'c3'
    Assert ($r.railsCache -eq 'hit' -and $r.storyMapCache -eq 'miss') 'a changed baker must rebuild the map (and only the map)'
    Assert ((Get-Calls 'rails') -eq 1 -and (Get-Calls 'map') -eq 2) 'map rebuilt, rails reused'
    [IO.File]::WriteAllText((Join-Path $fixture 'x64a.rpf'), 'archive v2 (changed)')
    $r = Invoke-Stage 'c4'
    Assert ($r.railsCache -eq 'miss' -and $r.storyMapCache -eq 'miss') 'a changed game archive must rebuild both'
    Assert ((Get-Calls 'rails') -eq 2 -and (Get-Calls 'map') -eq 3) 'both rebuilt after an archive change'
    $r = Invoke-Stage 'c5' @{ NoCache = $true }
    Assert ($r.railsCache -eq 'off' -and (Get-Calls 'rails') -eq 3 -and (Get-Calls 'map') -eq 4) '-NoCache must rebuild'
    [IO.File]::WriteAllText($keys, 'keys v2')
    $r = Invoke-Stage 'c6'
    Assert ($r.railsCache -eq 'miss' -and $r.storyMapCache -eq 'miss') 'changed keys must rebuild both'
    Assert ($r.rails -ge 0 -and $r.storyMap -ge 0 -and $r.tilesAll -ge 0 -and $r.verify -ge 0) 'timings.json must carry the stage timings'
    Remove-Item Env:GTAXSKATE_FIXTURE_CALLS
    $passed++
    # Mid-bake snapshot: gate and partially install the finished priority tiles
    # while the bake keeps running; the live bake folder is never touched.
    $env:GTAXSKATE_WORLD_FIXTURE = 'multi'
    $vlog = Join-Path $fixture 'verify.log'
    $gate = Join-Path $fixture 'gate.flag'
    $env:GTAXSKATE_FIXTURE_LOG = $vlog
    $env:GTAXSKATE_FIXTURE_GATE = $gate
    $prioFile = Join-Path $fixture 'priority.txt'
    Set-Content -LiteralPath $prioFile -Value @('0 0', '1 0', '3 0') -Encoding ASCII
    $env:SKATEV_BAKE_PRIORITY_FILE = $prioFile
    $world = Join-Path $fixture 'snapshot world'
    New-Item -ItemType Directory -Force -Path (Join-Path $world 'tiles') | Out-Null
    [IO.File]::WriteAllText((Join-Path $world 'tiles\9_9.svwt'), 'tile of an older bake')
    [IO.File]::WriteAllText((Join-Path $world 'sk8v-map.svwc'), 'old-map')
    $bakeWork = Join-Path $fixture 'bake-work'
    $argList = @('-NoProfile', '-File', $helper, '-Gta', $fixture, '-Keys', $keys, '-Baker', $baker, '-Rails', $rails, '-World', $world,
        '-MinX', '0', '-MaxX', '255', '-MinY', '0', '-MaxY', '63', '-StageOnly', '-ResumeWork', $bakeWork, '-CacheDir', $cacheB) | ForEach-Object { '"' + $_ + '"' }
    $bake = Start-Process pwsh -ArgumentList $argList -PassThru -WindowStyle Hidden -RedirectStandardOutput (Join-Path $fixture 'bake.out') -RedirectStandardError (Join-Path $fixture 'bake.err')
    $null = $bake.Handle
    $tilesDir = Join-Path $bakeWork 'payload\tiles'
    for ($i = 0; $i -lt 180 -and -not ((Test-Path (Join-Path $tilesDir '0_0.svwt')) -and (Test-Path (Join-Path $tilesDir '1_0.svwt'))); $i++) { Start-Sleep -Milliseconds 500 }
    Assert (-not $bake.HasExited) 'the bake must still be running (waiting at the gate)'
    Assert ((Test-Path (Join-Path $tilesDir '0_0.svwt')) -and (Test-Path (Join-Path $tilesDir '1_0.svwt'))) 'priority tiles were not written'
    Assert (Test-Path (Join-Path $tilesDir 'rules.txt')) 'rules.txt must exist before the bake ends'
    Assert (Test-Path (Join-Path $tilesDir 'tile-size.txt')) 'tile-size.txt must exist before the bake ends'
    Assert (-not (Test-Path (Join-Path $tilesDir '2_0.svwt'))) 'the bake must be mid-run (cell 2 0 is not baked yet)'
    Assert (-not (Get-ChildItem -LiteralPath $tilesDir -Filter '*.tmp')) 'no half-written tile may be visible'
    $cells = @(Get-PriorityWork -PriorityFile $prioFile -Work $bakeWork -MinX 0 -MaxX 255 -MinY 0 -MaxY 63)
    Assert (($cells -join ',') -eq '0 0,1 0') "priority cells (the live-only cell 3 0 is not waited for): $($cells -join ',')"
    Assert (Test-PriorityDone -Work $bakeWork -Cells $cells) 'priority tiles are done'
    # Trial marker in the bake: the partial install is refused, the world stays.
    [IO.File]::WriteAllText((Join-Path $tilesDir 'trial.txt'), 'trial bake: park rules switched off: R2')
    $snapTrial = Join-Path $fixture 'snap-trial'
    $null = New-BakeSnapshot -Work $bakeWork -Cells $cells -Dest $snapTrial -Baker $baker -MinX 0 -MaxX 255 -MinY 0 -MaxY 63
    $failed = $false
    try { & $helper -CacheDir $cacheB -Baker $baker -World $world -InstallFrom $snapTrial -InstallPartial | Out-Null } catch { $failed = $true; $why = $_.Exception.Message }
    Remove-Item -LiteralPath (Join-Path $tilesDir 'trial.txt')
    Assert ($failed -and $why -like '*trial*') "a trial snapshot was not refused: $why"
    Assert ((Test-Path (Join-Path $world 'tiles\9_9.svwt')) -and [IO.File]::ReadAllText((Join-Path $world 'sk8v-map.svwc')) -eq 'old-map') 'the refused trial changed the world'
    # The partial set is gated: a failing verify must stop it (it used to be skipped).
    $snap = Join-Path $fixture 'snap1'
    $n = New-BakeSnapshot -Work $bakeWork -Cells $cells -Dest $snap -Baker $baker -Rails $rails -MinX 0 -MaxX 255 -MinY 0 -MaxY 63
    Assert ($n -eq 2) "snapshot holds $n tiles"
    $env:GTAXSKATE_FIXTURE_VERIFY = 'bad'
    $failed = $false
    try { & $helper -CacheDir $cacheB -Baker $baker -World $world -InstallFrom $snap -InstallPartial | Out-Null } catch { $failed = $true }
    Remove-Item Env:GTAXSKATE_FIXTURE_VERIFY
    Assert $failed 'a partial install skipped the native gate'
    Assert ((Test-Path (Join-Path $world 'tiles\9_9.svwt')) -and [IO.File]::ReadAllText((Join-Path $world 'sk8v-map.svwc')) -eq 'old-map') 'the failed gate changed the world'
    # Game running: no install.
    $global:GtaxskateFixtureGameRunning = $true
    $failed = $false
    try { & $helper -CacheDir $cacheB -Baker $baker -World $world -InstallFrom $snap -InstallPartial | Out-Null } catch { $failed = $true }
    $global:GtaxskateFixtureGameRunning = $false
    Assert $failed 'a partial install ran while the game was open'
    & $helper -CacheDir $cacheB -Baker $baker -World $world -InstallFrom $snap -InstallPartial | Out-Null
    $installed = @(Get-ChildItem -LiteralPath (Join-Path $world 'tiles') -Filter '*.svwt' | ForEach-Object Name | Sort-Object)
    Assert (($installed -join ',') -eq '0_0.svwt,1_0.svwt') "partial install holds exactly the priority tiles, not: $($installed -join ',')"
    Assert (-not (Test-Path (Join-Path $world 'tiles\9_9.svwt'))) 'a tile of an older bake stayed beside the partial set'
    $marker = Get-Content -LiteralPath (Join-Path $world 'tiles\partial.json') -Raw | ConvertFrom-Json
    Assert ($marker.baked -eq 2 -and $marker.total -eq 3 -and $marker.rules -eq 'rules 0123456789abcdef') 'partial.json content'
    Assert ([IO.File]::ReadAllText((Join-Path $world 'tiles\rules.txt')).Trim() -eq 'rules 0123456789abcdef') 'one rules hash'
    Assert ([IO.File]::ReadAllText((Join-Path $world 'sk8v-map.svwc')) -eq 'map') 'map not installed with the partial set'
    Assert ((Get-Content -LiteralPath $vlog) -contains 'verify args=7') 'the gate did not get the cell list'
    Assert (-not $bake.HasExited -and -not (Test-Path (Join-Path $tilesDir '2_0.svwt'))) 'the install disturbed the running bake'
    $partialTimings = Get-Content -LiteralPath (Join-Path $snap 'timings.json') -Raw | ConvertFrom-Json
    Assert ($partialTimings.install -ge 0 -and $partialTimings.installVerify -ge 0) 'install timings'
    # The bake finishes; the whole set replaces the partial one and partial.json goes.
    New-Item -ItemType File -Path $gate | Out-Null
    Assert ($bake.WaitForExit(120000)) 'the bake did not finish'
    Assert ($bake.ExitCode -eq 0) "the bake exited with $($bake.ExitCode): $(Get-Content (Join-Path $fixture 'bake.out') -Raw)"
    & $helper -CacheDir $cacheB -Baker $baker -World $world -InstallFrom $bakeWork | Out-Null
    $installed = @(Get-ChildItem -LiteralPath (Join-Path $world 'tiles') -Filter '*.svwt' | ForEach-Object Name | Sort-Object)
    Assert (($installed -join ',') -eq '0_0.svwt,1_0.svwt,2_0.svwt') "whole install: $($installed -join ',')"
    Assert (-not (Test-Path (Join-Path $world 'tiles\partial.json'))) 'the whole install left partial.json'
    $whole = Get-Content -LiteralPath (Join-Path $bakeWork 'timings.json') -Raw | ConvertFrom-Json
    Assert ($whole.tilesFirstPriority -ge 0 -and $whole.tilesAllPriority -ge $whole.tilesFirstPriority -and $whole.tilesAll -ge 0 -and $whole.install -ge 0) 'bake timings'
    Remove-Item Env:SKATEV_BAKE_PRIORITY_FILE, Env:GTAXSKATE_FIXTURE_LOG, Env:GTAXSKATE_FIXTURE_GATE
    $passed += 2
    Write-Host "World collision installer: $passed cases pass."
    Remove-Item -LiteralPath $fixture -Recurse -Force
} finally { foreach ($n in 'GTAXSKATE_FIXTURE_CALLS','GTAXSKATE_FIXTURE_LOG','GTAXSKATE_FIXTURE_GATE','GTAXSKATE_FIXTURE_VERIFY','SKATEV_BAKE_PRIORITY_FILE') { Remove-Item "Env:$n" -ErrorAction SilentlyContinue }; Remove-Item Env:GTAXSKATE_WORLD_FIXTURE -ErrorAction SilentlyContinue; Remove-Variable GtaxskateFixtureGameRunning -Scope Global -ErrorAction SilentlyContinue }
