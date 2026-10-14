param([string]$Fork = (Join-Path (Split-Path -Parent $PSScriptRoot) 'external\sk8v'))
$ErrorActionPreference = 'Stop'
$patch = Join-Path (Split-Path -Parent $PSScriptRoot) 'patches\sk8v-package-menu.patch'
# A36: package presentation stays separate from the basis mod and contribution branches.
git -C $Fork apply --reverse --check $patch 2>$null
if ($LASTEXITCODE -eq 0) { return }
git -C $Fork apply --check $patch
if ($LASTEXITCODE -ne 0) { throw 'Package menu patch conflicts with this SK8V version; update it before building.' }
git -C $Fork apply $patch
if ($LASTEXITCODE -ne 0) { throw 'Could not apply package menu patch.' }
