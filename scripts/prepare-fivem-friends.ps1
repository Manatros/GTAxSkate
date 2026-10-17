# Stage the pinned base-game-only FiveM friends test; never install or start it.
param([string]$Name = '')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $Name) { $Name = [guid]::NewGuid().ToString('N') }
if ($Name -notmatch '^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$') { throw 'Name must be a simple fresh package name (letters/numbers/_/-).' }
$output = Join-Path $root "build\fivem-friends\$Name"
& node (Join-Path $root 'fivem\gtaxskate_sk8m\prepare.js') $output
if ($LASTEXITCODE -ne 0) { throw 'FiveM friends preparation failed.' }
