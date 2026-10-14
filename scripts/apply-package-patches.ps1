# Independent package overlays; originals stay outside the basis repository.
param([string]$Fork = (Join-Path (Split-Path -Parent $PSScriptRoot) 'external\sk8v'))
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
foreach ($name in 'sk8v-package-menu.patch','sk8v-save-reload.patch','sk8v-hud-tick.patch','sk8v-runtime-reload.patch','sk8v-camera-clearance.patch','sk8v-developer-versions.patch','sk8v-controller-camera.patch') {
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
