$ErrorActionPreference = 'Stop'
$fixture = Join-Path (Join-Path (Split-Path -Parent $PSScriptRoot) 'build') ('cumulative-fixture-' + [guid]::NewGuid().ToString('N'))
$scripts = Join-Path $fixture 'scripts'
$patches = Join-Path $fixture 'patches'
$repo = Join-Path $fixture 'fork'
New-Item -ItemType Directory -Force $scripts,$patches,$repo | Out-Null
foreach ($name in 'check-cumulative-patch.ps1','check-package-overlays.ps1','apply-package-patches.ps1') {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination $scripts
}
'$Sk8vTag = ''fixture-base''' | Set-Content -LiteralPath (Join-Path $scripts 'install.ps1')
function G {
    $out = & git.exe -C $repo @args 2>&1
    if ($LASTEXITCODE) { throw "Fixture git failed: $out" }
    $out
}
function Assert($condition, $message) { if (-not $condition) { throw $message } }
function Reject([scriptblock]$action, [string]$message) {
    $failed = $false
    try { & $action | Out-Null } catch { $failed = $true }
    Assert $failed $message
}
G init -q | Out-Null
G config user.name Manatros
G config user.email Manatros@users.noreply.github.com
G config core.autocrlf false
'base' | Set-Content -LiteralPath (Join-Path $repo 'main.txt')
G add main.txt
G commit -qm base
G tag fixture-base
'target' | Set-Content -LiteralPath (Join-Path $repo 'main.txt')
'added' | Set-Content -LiteralPath (Join-Path $repo 'added.txt')
G add main.txt added.txt
G commit -qm target
$check = Join-Path $scripts 'check-cumulative-patch.ps1'
$apply = Join-Path $scripts 'apply-package-patches.ps1'
$overlays = Join-Path $scripts 'check-package-overlays.ps1'
$cumulative = Join-Path $patches 'sk8v-gtaxskate-cumulative.patch'
& $check -Fork $repo -Write | Out-Null
& $check -Fork $repo | Out-Null
'dirty' | Set-Content -LiteralPath (Join-Path $repo 'main.txt')
Reject { & $check -Fork $repo -Write } 'Generator accepted dirty tracked source'
G restore main.txt
G checkout -q fixture-base
'private' | Set-Content -LiteralPath (Join-Path $repo 'private-untracked.txt')
$indexBefore = (Get-FileHash -LiteralPath (Join-Path $repo '.git/index')).Hash
& $apply -Fork $repo
& $check -Fork $repo -WorkingTree | Out-Null
& $overlays -Fork $repo
Assert (& $overlays -Fork $repo -ProbePlayPresets) 'Cumulative probe failed'
Assert ((Get-FileHash -LiteralPath (Join-Path $repo '.git/index')).Hash -eq $indexBefore) 'Real index changed'
Assert ((G diff --cached --name-only) -eq $null) 'Real index was staged'
& $apply -Fork $repo
Reject { & $check -Fork $repo } 'HEAD check accepted an uncommitted overlay'
'wrong' | Set-Content -LiteralPath (Join-Path $repo 'added.txt')
Reject { & $check -Fork $repo -WorkingTree } 'Changed newly added target accepted'
Assert (-not (& $overlays -Fork $repo -ProbePlayPresets)) 'Probe accepted changed source'
'added' | Set-Content -LiteralPath (Join-Path $repo 'added.txt')
Remove-Item -LiteralPath (Join-Path $repo 'main.txt')
Reject { & $check -Fork $repo -WorkingTree } 'Missing tracked target accepted'
G restore main.txt
Remove-Item -LiteralPath (Join-Path $repo 'added.txt')
Remove-Item -LiteralPath $cumulative
# Every legacy overlay adds an independent file. This exercises the original
# stack without involving actual game code or any current package patch.
$names = @('sk8v-package-menu.patch','sk8v-save-reload.patch','sk8v-hud-tick.patch','sk8v-runtime-reload.patch','sk8v-camera-clearance.patch','sk8v-developer-versions.patch','sk8v-controller-camera.patch','sk8v-board-pose-diagnostics.patch','sk8v-board-skin-flags.patch','sk8v-board-carry-ownership.patch','sk8v-board-composition.patch','sk8v-wheel-presentation.patch','sk8v-original-camera-option.patch','sk8v-private-build.patch','sk8v-keyboard-presets.patch','sk8v-performance-reporting.patch','sk8v-play-presets.patch')
foreach ($name in $names) {
    $file = $name + '.txt'
    $name | Set-Content -LiteralPath (Join-Path $repo $file)
    G add -- $file
    G diff --cached --binary --full-index HEAD "--output=$(Join-Path $patches $name)" | Out-Null
    G reset -q HEAD
    Remove-Item -LiteralPath (Join-Path $repo $file)
}
Assert (-not (& $overlays -Fork $repo -ProbePlayPresets)) 'Legacy probe accepted unapplied stack'
& $apply -Fork $repo
& $overlays -Fork $repo
Assert (& $overlays -Fork $repo -ProbePlayPresets) 'Legacy stack not detected'
& $apply -Fork $repo
Assert ((G diff --cached --name-only) -eq $null) 'Legacy checks changed real index'
Assert (Test-Path -LiteralPath (Join-Path $repo 'private-untracked.txt')) 'Private untracked file changed'
"PASS cumulative generation, dirty refusal, fresh apply, idempotence, exact working tree, added/deleted target rejection, untracked exclusion, real-index preservation, and legacy fallback"
"Fixture retained: $fixture"
