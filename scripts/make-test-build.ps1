# Source-identified Story test snapshot and separate FiveM rules test.
# Never packages game data or resets the player's configuration.
param([string]$Output = '')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$public = Join-Path $root 'public'
if (-not (Test-Path -LiteralPath (Join-Path $public 'LICENSE'))) { $public = $root }
$fork = Join-Path $root 'external\sk8v'
$rootCommit = (git -C $root rev-parse HEAD).Trim()
if ($LASTEXITCODE) { throw 'Cannot identify package source revision.' }
$forkCommit = (git -C $fork rev-parse HEAD).Trim()
if ($LASTEXITCODE) { throw 'Cannot identify basis revision.' }
if ((git -C $fork branch --show-current) -ne 'skatemods') { throw 'Test package requires the skatemods branch.' }
foreach ($repo in $root,$fork) {
    if (git -C $repo status --porcelain --untracked-files=no) { throw 'Commit tracked source changes before creating a source-identified test snapshot.' }
}
$patch = Join-Path $root 'patches\sk8v-package-menu.patch'
$ErrorActionPreference = 'Continue'
git -C $fork apply --reverse --check $patch 2>$null
$patchCode = $LASTEXITCODE
$ErrorActionPreference = 'Stop'
if ($patchCode) { throw 'The required package menu overlay is not present in this basis.' }
$id = 'test-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + $forkCommit.Substring(0,7)
if (-not $Output) { $Output = Join-Path $root "build\test-releases\$id" }
$out = [IO.Path]::GetFullPath($Output)
if (Test-Path -LiteralPath $out) { throw 'Choose a new output directory; existing test packages are never overwritten.' }
$story = Join-Path $out 'GTAxSkate-Story-Test'
$rules = Join-Path $out 'GTAxSkate-FiveM-Rules-Test'
New-Item -ItemType Directory -Path $story -Force | Out-Null
$files = [ordered]@{}
foreach ($name in 'SkateVLegacy.asi','SkateVRuntime.dll') {
    $source = Join-Path $fork "build\package\$name"
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw "Missing built binary: $name" }
    Copy-Item -LiteralPath $source -Destination (Join-Path $story $name)
    $files[$name] = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant()
}
$manifest = [ordered]@{
    format='gtaxskate-test'; version=1; id=$id; gta='1.0.3889.0'
    rootCommit=$rootCommit; forkCommit=$forkCommit
    menuPatchSha256=(Get-FileHash -LiteralPath $patch -Algorithm SHA256).Hash.ToLowerInvariant()
    builtUtc=[DateTime]::UtcNow.ToString('o'); files=$files
    provenance='Hashes identify the exact artifacts. Host build and retained runtime evidence are documented in docs/TEST-BUILD-2026-10-14.md; this manifest alone does not prove a source rebuild.'
    tests='See TESTPLAN.txt and source QA logs. Game confirmation pending.'
    limitations=@('Story Mode only; GTA preset is WIP/no-op.', 'No FiveM/Coop skating runtime in this package.', 'Existing player INI/boards remain unchanged.')
}
[IO.File]::WriteAllText((Join-Path $story 'manifest.json'), ($manifest | ConvertTo-Json -Depth 8))
Copy-Item -LiteralPath (Join-Path $root 'scripts\install-test-build.ps1'),(Join-Path $root 'scripts\restore-test-build.ps1') -Destination $story
$launcher = "@echo off`r`npowershell -NoProfile -ExecutionPolicy Bypass -File `"%~dp0install-test-build.ps1`" -Package `"%~dp0.`" %*`r`npause`r`n"
[IO.File]::WriteAllText((Join-Path $story 'Install Test.cmd'), $launcher)
Copy-Item -LiteralPath (Join-Path $public 'test-build\TESTPLAN.txt') -Destination $story
Copy-Item -LiteralPath (Join-Path $public 'LICENSE') -Destination (Join-Path $story 'LICENSE.txt')
Copy-Item -LiteralPath (Join-Path $public 'NOTICE') -Destination (Join-Path $story 'NOTICE.txt')
Copy-Item -LiteralPath (Join-Path $fork 'LICENSE') -Destination (Join-Path $story 'LICENSE-SK8V.txt')
New-Item -ItemType Directory -Path (Join-Path $rules 'scripts'),(Join-Path $rules 'fivem') -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $root 'Install GTAxSkate FiveM.cmd') -Destination $rules
Copy-Item -LiteralPath (Join-Path $root 'scripts\install-fivem.ps1') -Destination (Join-Path $rules 'scripts')
Copy-Item -LiteralPath (Join-Path $root 'fivem\gtaxskate_rules') -Destination (Join-Path $rules 'fivem\gtaxskate_rules') -Recurse
Copy-Item -LiteralPath (Join-Path $root 'fivem\README.md') -Destination (Join-Path $rules 'README.md')
Copy-Item -LiteralPath (Join-Path $public 'LICENSE') -Destination (Join-Path $rules 'LICENSE.txt')
Copy-Item -LiteralPath (Join-Path $public 'NOTICE') -Destination (Join-Path $rules 'NOTICE.txt')
[IO.File]::WriteAllText((Join-Path $rules 'VERSION.txt'), "$id`r`nRoot: $rootCommit`r`nRULES ONLY: no skating adapter. No game assets included.`r`n")
Compress-Archive -LiteralPath $story -DestinationPath (Join-Path $out 'GTAxSkate-Story-Test.zip')
Compress-Archive -LiteralPath $rules -DestinationPath (Join-Path $out 'GTAxSkate-FiveM-Rules-Test.zip')
$hashes = Get-FileHash -LiteralPath (Join-Path $out 'GTAxSkate-Story-Test.zip'),(Join-Path $out 'GTAxSkate-FiveM-Rules-Test.zip') -Algorithm SHA256
[IO.File]::WriteAllLines((Join-Path $out 'SHA256SUMS.txt'), @($hashes | ForEach-Object { $_.Hash.ToLowerInvariant() + '  ' + [IO.Path]::GetFileName($_.Path) }))
Write-Host "Test packages: $out"
