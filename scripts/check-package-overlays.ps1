# Validate the ordered overlay stack without modifying the checkout or its index.
param(
    [string]$Fork = (Join-Path (Split-Path -Parent $PSScriptRoot) 'external\sk8v'),
    [switch]$ProbePlayPresets
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$cumulative = Join-Path $root 'patches\sk8v-gtaxskate-cumulative.patch'
if (Test-Path -LiteralPath $cumulative -PathType Leaf) {
    try {
        & (Join-Path $PSScriptRoot 'check-cumulative-patch.ps1') -Fork $Fork -Patch $cumulative -WorkingTree | Out-Null
    } catch {
        if ($ProbePlayPresets) { return $false }
        throw
    }
    if ($ProbePlayPresets) { return $true }
    return
}
$latest = Join-Path $root 'patches\sk8v-play-presets.patch'
$prerequisites = @('sk8v-package-menu.patch','sk8v-save-reload.patch','sk8v-hud-tick.patch','sk8v-runtime-reload.patch','sk8v-camera-clearance.patch','sk8v-developer-versions.patch','sk8v-controller-camera.patch','sk8v-board-pose-diagnostics.patch','sk8v-board-skin-flags.patch','sk8v-board-carry-ownership.patch','sk8v-board-composition.patch','sk8v-wheel-presentation.patch','sk8v-original-camera-option.patch','sk8v-private-build.patch','sk8v-keyboard-presets.patch','sk8v-performance-reporting.patch')
if (-not (Test-Path -LiteralPath $latest -PathType Leaf)) { throw 'Missing package patch: sk8v-play-presets.patch' }
$savedIndex = $env:GIT_INDEX_FILE
$savedErrors = $ErrorActionPreference
$temporaryDirectory = Join-Path $root 'build\overlay-checks'
New-Item -ItemType Directory -Path $temporaryDirectory -Force | Out-Null
$temporaryIndex = Join-Path $temporaryDirectory ([Guid]::NewGuid().ToString('N') + '.index')
try {
    $env:GIT_INDEX_FILE = $temporaryIndex
    git -C $Fork read-tree HEAD
    if ($LASTEXITCODE) { throw 'Cannot read the SK8V source tree for overlay verification.' }
    # Include tracked local edits in the audit; never stage unrelated untracked data.
    git -C $Fork -c core.safecrlf=false add -u
    if ($LASTEXITCODE) { throw 'Cannot inspect the tracked SK8V working tree.' }
    # Freshly applied overlays may add files not yet tracked in HEAD. Include
    # only named patch targets, never receipts or unrelated local game data.
    $targets = [Collections.Generic.HashSet[string]]::new()
    foreach ($name in ($prerequisites + @('sk8v-play-presets.patch'))) {
        $patch = Join-Path $root "patches\$name"
        if (-not (Test-Path -LiteralPath $patch -PathType Leaf)) { throw "Missing package patch: $name" }
        $stats = @(git -C $Fork apply --numstat $patch)
        if ($LASTEXITCODE) { throw "Cannot read package patch targets: $name" }
        foreach ($row in $stats) {
            $target = ($row -split "`t",3)[2]
            if (-not $target -or $target.StartsWith('"')) { throw "Unsupported package patch path: $name" }
            if (Test-Path -LiteralPath (Join-Path $Fork $target) -PathType Leaf) {
                [void]$targets.Add($target)
            }
        }
    }
    if ($targets.Count) {
        git -C $Fork -c core.safecrlf=false add -- @($targets)
        if ($LASTEXITCODE) { throw 'Cannot inspect package patch targets.' }
    }
    $ErrorActionPreference = 'Continue'
    git -C $Fork apply --cached --reverse --check $latest 2>$null
    $hasLatest = $LASTEXITCODE -eq 0
    $ErrorActionPreference = 'Stop'
    if ($ProbePlayPresets) { return $hasLatest }
    if (-not $hasLatest) { throw 'Required overlay is not present: sk8v-play-presets.patch' }
    # Later overlays legitimately change earlier hunks. Peel only this layer in
    # the temporary index before checking every prerequisite independently.
    git -C $Fork apply --cached --reverse $latest
    if ($LASTEXITCODE) { throw 'Cannot inspect the prerequisite overlay tree.' }
    foreach ($name in $prerequisites) {
        $patch = Join-Path $root "patches\$name"
        if (-not (Test-Path -LiteralPath $patch -PathType Leaf)) { throw "Missing package patch: $name" }
        $ErrorActionPreference = 'Continue'
        git -C $Fork apply --cached --reverse --check $patch 2>$null
        $code = $LASTEXITCODE
        $ErrorActionPreference = 'Stop'
        if ($code) { throw "Required prerequisite overlay is not present: $name" }
    }
} finally {
    $ErrorActionPreference = $savedErrors
    $env:GIT_INDEX_FILE = $savedIndex
    if (Test-Path -LiteralPath $temporaryIndex) { Remove-Item -LiteralPath $temporaryIndex }
    if (Test-Path -LiteralPath "$temporaryIndex.lock") { Remove-Item -LiteralPath "$temporaryIndex.lock" }
}
