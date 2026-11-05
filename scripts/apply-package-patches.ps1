# Independent package overlays; originals stay outside the basis repository.
param([string]$Fork = (Join-Path (Split-Path -Parent $PSScriptRoot) 'external\sk8v'))
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$cumulative = Join-Path $root 'patches\sk8v-gtaxskate-cumulative.patch'
if (Test-Path -LiteralPath $cumulative -PathType Leaf) {
    $verify = Join-Path $PSScriptRoot 'check-cumulative-patch.ps1'
    try {
        & $verify -Fork $Fork -Patch $cumulative -WorkingTree | Out-Null
        return
    } catch {
        # A fresh official checkout is expected not to match yet. Git checks
        # every hunk before changing any file; conflicts are never overwritten.
    }
    git -C $Fork apply --check --binary $cumulative
    if ($LASTEXITCODE) { throw 'Cumulative package patch conflicts with this SK8V source; review instead of overwriting changes.' }
    git -C $Fork apply --binary $cumulative
    if ($LASTEXITCODE) { throw 'Could not apply cumulative package patch.' }
    & $verify -Fork $Fork -Patch $cumulative -WorkingTree | Out-Null
    return
}
$check = Join-Path $PSScriptRoot 'check-package-overlays.ps1'
if (& $check -Fork $Fork -ProbePlayPresets) {
    & $check -Fork $Fork
    return
}
foreach ($name in 'sk8v-package-menu.patch','sk8v-save-reload.patch','sk8v-hud-tick.patch','sk8v-runtime-reload.patch','sk8v-camera-clearance.patch','sk8v-developer-versions.patch','sk8v-controller-camera.patch','sk8v-board-pose-diagnostics.patch','sk8v-board-skin-flags.patch','sk8v-board-carry-ownership.patch','sk8v-board-composition.patch','sk8v-wheel-presentation.patch','sk8v-original-camera-option.patch','sk8v-private-build.patch','sk8v-keyboard-presets.patch','sk8v-performance-reporting.patch') {
    $patch = Join-Path $root "patches\$name"
    if (-not (Test-Path -LiteralPath $patch -PathType Leaf)) { throw "Missing package patch: $name" }
    # Expected negative checks write native stderr; inspect exit codes explicitly.
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    git -C $Fork apply --reverse --check $patch 2>$null
    $already = $LASTEXITCODE -eq 0
    $ErrorActionPreference = $saved
    if ($already) { continue }
    git -C $Fork apply --check $patch
    if ($LASTEXITCODE) { throw "Package patch conflicts with this SK8V version: $name. Review instead of overwriting basis changes." }
    git -C $Fork apply $patch
    if ($LASTEXITCODE) { throw "Could not apply package patch: $name" }
}
$latest = Join-Path $root 'patches\sk8v-play-presets.patch'
git -C $Fork apply --check $latest
if ($LASTEXITCODE) { throw 'Play presets overlay conflicts with this SK8V version; review the basis changes.' }
git -C $Fork apply $latest
if ($LASTEXITCODE) { throw 'Could not apply play presets overlay.' }
& $check -Fork $Fork
