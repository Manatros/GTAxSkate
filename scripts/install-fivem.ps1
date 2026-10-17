# Experimental FiveM rules resource, separate from the tested Story Mode installer.
# ASCII for Windows PowerShell 5.1. Does not install FXServer or game assets.
param([string]$ServerData = '')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $ServerData) { $ServerData = Read-Host 'FiveM server-data folder (contains server.cfg and resources)' }
$data = (Resolve-Path -LiteralPath $ServerData -ErrorAction Stop).ProviderPath
if (-not (Test-Path -LiteralPath (Join-Path $data 'server.cfg') -PathType Leaf)) { throw 'Select the server-data folder containing server.cfg.' }
$resources = Join-Path $data 'resources'
if (-not (Test-Path -LiteralPath $resources -PathType Container)) { throw 'Missing resources folder; finish FXServer/txAdmin setup first.' }
$target = Join-Path $resources 'gtaxskate_rules'
$source = Join-Path $root 'fivem\gtaxskate_rules'
$files = @('fxmanifest.lua', 'server.js', 'client.js', 'policy.js', 'policy.json')
foreach ($file in $files) {
    if (-not (Test-Path -LiteralPath (Join-Path $source $file) -PathType Leaf)) { throw "Missing package file: $file" }
}
# Refuse updates of a live FXServer; neither config nor code is replaced mid-session.
if (Get-Process FXServer,cfx-server -ErrorAction SilentlyContinue) { throw 'Stop FXServer before installing/updating the resource.' }
# Reparse points can redirect a seemingly safe destination to unrelated files.
foreach ($path in @($data, $resources, $target)) {
    if (Test-Path -LiteralPath $path) {
        if ((Get-Item -LiteralPath $path -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Linked destination not supported: $path" }
    }
}
$stamp = [guid]::NewGuid().ToString('N')
$stage = Join-Path $resources ".gtaxskate-rules-stage-$stamp"
$backupRoot = Join-Path $data 'gtaxskate-backups'
$backup = Join-Path $backupRoot "rules-$stamp"
$moved = $false
try {
    New-Item -ItemType Directory -Path $stage | Out-Null
    foreach ($file in $files) { Copy-Item -LiteralPath (Join-Path $source $file) -Destination (Join-Path $stage $file) }
    if (Test-Path -LiteralPath $target) {
        # Keep server-owned policy, local edits and unknown files. The backup is
        # outside resources so FiveM never loads a second copy of this resource.
        foreach ($item in Get-ChildItem -LiteralPath $target -Force -Recurse) {
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Linked files in existing resource are not supported.' }
        }
        foreach ($item in Get-ChildItem -LiteralPath $target -Force) {
            if ($files -notcontains $item.Name -or $item.Name -eq 'policy.json') {
                Copy-Item -LiteralPath $item.FullName -Destination (Join-Path $stage $item.Name) -Recurse -Force
            }
        }
        New-Item -ItemType Directory -Path $backupRoot -Force | Out-Null
        if ((Get-Item -LiteralPath $backupRoot).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Linked backup folder not supported.' }
        Move-Item -LiteralPath $target -Destination $backup
        $moved = $true
    }
    Move-Item -LiteralPath $stage -Destination $target
} catch {
    if ($moved -and -not (Test-Path -LiteralPath $target)) { Move-Item -LiteralPath $backup -Destination $target }
    throw
} finally {
    # The only recursive cleanup is the verified staging child created above.
    if (Test-Path -LiteralPath $stage) {
        $resolvedStage = (Resolve-Path -LiteralPath $stage).ProviderPath
        if ([IO.Path]::GetDirectoryName($resolvedStage) -ne $resources -or [IO.Path]::GetFileName($resolvedStage) -ne ".gtaxskate-rules-stage-$stamp") { throw 'Unexpected staging path; cleanup stopped.' }
        Remove-Item -LiteralPath $resolvedStage -Recurse -Force
    }
}
Write-Host 'Installed experimental RULES ONLY. No Skate runtime/adapter is included.'
Write-Host "Server policy (preserved on updates): $target\policy.json"
Write-Host 'Add to server.cfg: ensure gtaxskate_rules'
Write-Host 'Keep sv_scriptHookAllowed 0. No ASI or Script Hook V is needed for this resource.'
Write-Host 'Default mode is locked. Edit policy.json and use skaterules_reload in the server console.'
Write-Host 'Client F8: skaterules; skateset Pop 1.1 (only if server policy permits).'
