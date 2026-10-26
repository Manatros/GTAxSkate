# Source-identified Story test snapshot and separate FiveM rules test.
# Never packages game data or resets the player's configuration.
param([string]$Output = '', [Parameter(Mandatory=$true)][string]$PrivacyPatternsFile, [string]$ForkPath = '')
$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $PrivacyPatternsFile -PathType Leaf)) { throw 'Supply an existing local privacy patterns file; it is never included in the package.' }
$root = Split-Path -Parent $PSScriptRoot
$public = Join-Path $root 'public'
if (-not (Test-Path -LiteralPath (Join-Path $public 'LICENSE'))) { $public = $root }
$fork = if ($ForkPath) { [IO.Path]::GetFullPath($ForkPath) } else { Join-Path $root 'external\sk8v' }
$rootCommit = (git -C $root rev-parse HEAD).Trim()
if ($LASTEXITCODE) { throw 'Cannot identify package source revision.' }
$forkCommit = (git -C $fork rev-parse HEAD).Trim()
if ($LASTEXITCODE) { throw 'Cannot identify basis revision.' }
# Exact source commit, clean checkout, required patches and official ancestry also support isolated detached builds.
foreach ($repo in $root,$fork) {
    if (git -C $repo status --porcelain --untracked-files=no) { throw 'Commit tracked source changes before creating a source-identified test snapshot.' }
}
$basis = & (Join-Path $PSScriptRoot 'check-latest-sk8v.ps1') -Fork $fork
$buildReceipt = & (Join-Path $PSScriptRoot 'check-story-build.ps1') -Fork $fork
$patch = Join-Path $root 'patches\sk8v-package-menu.patch'
& (Join-Path $PSScriptRoot 'check-package-overlays.ps1') -Fork $fork
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
    sk8vBasis=$basis
    buildReceipt=$buildReceipt
    cumulativePatchSha256=$(if (Test-Path -LiteralPath (Join-Path $root 'patches\sk8v-gtaxskate-cumulative.patch')) { (Get-FileHash -LiteralPath (Join-Path $root 'patches\sk8v-gtaxskate-cumulative.patch') -Algorithm SHA256).Hash.ToLowerInvariant() } else { $null })
    menuPatchSha256=(Get-FileHash -LiteralPath $patch -Algorithm SHA256).Hash.ToLowerInvariant()
    saveReloadPatchSha256=(Get-FileHash -LiteralPath (Join-Path $root 'patches\sk8v-save-reload.patch') -Algorithm SHA256).Hash.ToLowerInvariant()
    hudTickPatchSha256=(Get-FileHash -LiteralPath (Join-Path $root 'patches\sk8v-hud-tick.patch') -Algorithm SHA256).Hash.ToLowerInvariant()
    runtimeReloadPatchSha256=(Get-FileHash -LiteralPath (Join-Path $root 'patches\sk8v-runtime-reload.patch') -Algorithm SHA256).Hash.ToLowerInvariant()
    cameraPatchSha256=(Get-FileHash -LiteralPath (Join-Path $root 'patches\sk8v-camera-clearance.patch') -Algorithm SHA256).Hash.ToLowerInvariant()
    developerVersionsPatchSha256=(Get-FileHash -LiteralPath (Join-Path $root 'patches\sk8v-developer-versions.patch') -Algorithm SHA256).Hash.ToLowerInvariant()
    controllerCameraPatchSha256=(Get-FileHash -LiteralPath (Join-Path $root 'patches\sk8v-controller-camera.patch') -Algorithm SHA256).Hash.ToLowerInvariant()
    boardPoseDiagnosticsPatchSha256=(Get-FileHash -LiteralPath (Join-Path $root 'patches\sk8v-board-pose-diagnostics.patch') -Algorithm SHA256).Hash.ToLowerInvariant()
    boardSkinFlagsPatchSha256=(Get-FileHash -LiteralPath (Join-Path $root 'patches\sk8v-board-skin-flags.patch') -Algorithm SHA256).Hash.ToLowerInvariant()
    boardCarryOwnershipPatchSha256=(Get-FileHash -LiteralPath (Join-Path $root 'patches\sk8v-board-carry-ownership.patch') -Algorithm SHA256).Hash.ToLowerInvariant()
    boardCompositionPatchSha256=(Get-FileHash -LiteralPath (Join-Path $root 'patches\sk8v-board-composition.patch') -Algorithm SHA256).Hash.ToLowerInvariant()
    wheelPresentationPatchSha256=(Get-FileHash -LiteralPath (Join-Path $root 'patches\sk8v-wheel-presentation.patch') -Algorithm SHA256).Hash.ToLowerInvariant()
    wheelDirectionPatchSha256=(Get-FileHash -LiteralPath (Join-Path $root 'patches\sk8v-wheel-direction.patch') -Algorithm SHA256).Hash.ToLowerInvariant()
    originalCameraOptionPatchSha256=(Get-FileHash -LiteralPath (Join-Path $root 'patches\sk8v-original-camera-option.patch') -Algorithm SHA256).Hash.ToLowerInvariant()
    privateBuildPatchSha256=(Get-FileHash -LiteralPath (Join-Path $root 'patches\sk8v-private-build.patch') -Algorithm SHA256).Hash.ToLowerInvariant()
    keyboardPresetsPatchSha256=(Get-FileHash -LiteralPath (Join-Path $root 'patches\sk8v-keyboard-presets.patch') -Algorithm SHA256).Hash.ToLowerInvariant()
    performanceReportingPatchSha256=(Get-FileHash -LiteralPath (Join-Path $root 'patches\sk8v-performance-reporting.patch') -Algorithm SHA256).Hash.ToLowerInvariant()
    playPresetsPatchSha256=(Get-FileHash -LiteralPath (Join-Path $root 'patches\sk8v-play-presets.patch') -Algorithm SHA256).Hash.ToLowerInvariant()
    builtUtc=[DateTime]::UtcNow.ToString('o'); files=$files
    provenance='Source revision and exact rebuilt host/runtime hashes verified against GTAxSkate-build.json. Gameplay acceptance remains pending.'
    tests='See TESTPLAN.txt and source QA logs. Game confirmation pending.'
    limitations=@('Story Mode Legacy only; gameplay acceptance of this candidate is pending.', 'Metro collision/rendering, high-speed streaming, stairs and placed-prop performance require a new test.', 'No FiveM/Coop skating runtime in this package.', 'Existing player INI/boards remain unchanged.')
}
[IO.File]::WriteAllText((Join-Path $story 'manifest.json'), ($manifest | ConvertTo-Json -Depth 8))
Copy-Item -LiteralPath (Join-Path $root 'scripts\install-test-build.ps1'),(Join-Path $root 'scripts\restore-test-build.ps1') -Destination $story
$composer = Join-Path $story 'composer'
New-Item -ItemType Directory -Path $composer | Out-Null
$helperHashes = [ordered]@{}
foreach ($name in 'board-composition-worker.py','prepare-native-custom-board.py','setup-board-composer.ps1','restore-board-composer.ps1') {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination $composer
    $helperHashes[$name] = (Get-FileHash -LiteralPath (Join-Path $composer $name) -Algorithm SHA256).Hash.ToLowerInvariant()
}
[IO.File]::WriteAllText((Join-Path $composer 'manifest.json'), ([ordered]@{format='gtaxskate-composer-tools';version=1;files=$helperHashes;requirements='Existing Python >=3.10 and pinned RAGE executable; use setup-board-composer.ps1 with -Assets, -Python and -Rage while GTA is closed. Local experimental opt-in; no game assets included.'} | ConvertTo-Json -Depth 5))
$launcher = "@echo off`r`npowershell -NoProfile -ExecutionPolicy Bypass -File `"%~dp0install-test-build.ps1`" -Package `"%~dp0.`" %*`r`npause`r`n"
[IO.File]::WriteAllText((Join-Path $story 'Install Test.cmd'), $launcher)
Copy-Item -LiteralPath (Join-Path $public 'test-build\TESTPLAN.txt') -Destination $story
Copy-Item -LiteralPath (Join-Path $fork 'NOTICE') -Destination (Join-Path $story 'NOTICE-SK8V.txt')
Copy-Item -LiteralPath (Join-Path $fork 'build\package\THIRD-PARTY-NOTICES.txt') -Destination $story
Copy-Item -LiteralPath (Join-Path $public 'PRIVACY.md') -Destination $story
Copy-Item -LiteralPath (Join-Path $public 'TEST-RUN.md'),(Join-Path $public 'BOARD-COMPOSITION.md') -Destination $story
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
python (Join-Path $PSScriptRoot 'public-privacy.py') --path $story --forbidden-patterns-file $PrivacyPatternsFile
if ($LASTEXITCODE) { throw 'Story payload privacy scan failed.' }
python (Join-Path $PSScriptRoot 'public-privacy.py') --path $rules --forbidden-patterns-file $PrivacyPatternsFile
if ($LASTEXITCODE) { throw 'FiveM rules payload privacy scan failed.' }
Compress-Archive -LiteralPath $story -DestinationPath (Join-Path $out 'GTAxSkate-Story-Test.zip')
Compress-Archive -LiteralPath $rules -DestinationPath (Join-Path $out 'GTAxSkate-FiveM-Rules-Test.zip')
$hashes = Get-FileHash -LiteralPath (Join-Path $out 'GTAxSkate-Story-Test.zip'),(Join-Path $out 'GTAxSkate-FiveM-Rules-Test.zip') -Algorithm SHA256
[IO.File]::WriteAllLines((Join-Path $out 'SHA256SUMS.txt'), @($hashes | ForEach-Object { $_.Hash.ToLowerInvariant() + '  ' + [IO.Path]::GetFileName($_.Path) }))
Write-Host "Test packages: $out"
