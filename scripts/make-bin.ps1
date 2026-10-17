# Packs GTAxSkate's builds for players without the build tools:
# build\release\GTAxSkate-bin.zip (the installer's step 3 downloads it from
# the GitHub release). Contents: our SK8V build (SkateVLegacy.asi,
# SkateVRuntime.dll, from external\sk8v branch skatemods), the board tool
# (skate-ped.exe), the soundtrack tool (skate-music.exe), and the part model
# builder (prepare-board-parts.py with SK8V's _common.py), plus the licences.
#   powershell -ExecutionPolicy Bypass -File scripts\make-bin.ps1
param([string] $GplSourceReceipt, [Parameter(Mandatory)] [string] $PrivacyPatternsFile, [string] $ForkPath = '')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path -LiteralPath $PrivacyPatternsFile -PathType Leaf)) { throw 'Supply a local -PrivacyPatternsFile; it is never included in the package.' }
$sk8v = if ($ForkPath) { [IO.Path]::GetFullPath($ForkPath) } else { Join-Path $root 'external\sk8v' }
$out = Join-Path $root 'build\release'
$stage = Join-Path $out 'bin'
if (-not $GplSourceReceipt) { $GplSourceReceipt = Join-Path $root 'build\compliance-audit\GPL-SOURCE-RECEIPT.json' }
if (-not (Test-Path -LiteralPath $GplSourceReceipt -PathType Leaf)) { throw 'Missing GPL corresponding-source receipt; prepare exact source and rebuilt tool hashes before packaging.' }
$gplReceipt = Get-Content -LiteralPath $GplSourceReceipt -Raw | ConvertFrom-Json
if ($gplReceipt.format -ne 'gtaxskate-gpl-source' -or $gplReceipt.version -ne 1 -or $gplReceipt.sourceCommit -notmatch '^[0-9a-f]{40}$') { throw 'Invalid GPL source receipt.' }
foreach ($component in 'gta/skate-ped','gta/skate-music','gta/crates') {
    $expectedTree = $gplReceipt.componentTrees.PSObject.Properties[$component].Value
    $currentTree = git -C $root rev-parse "HEAD:$component"
    if ($LASTEXITCODE -or $expectedTree -notmatch '^[0-9a-f]{40}$' -or $currentTree -ne $expectedTree) { throw "GPL source receipt is stale for $component." }
    $dirty = git -C $root status --porcelain -- $component
    if ($LASTEXITCODE -or $dirty) { throw "Commit component changes and rebuild before packaging: $component" }
}
$gplArchive = Join-Path (Split-Path -Parent ([IO.Path]::GetFullPath($GplSourceReceipt))) 'GPL-CORRESPONDING-SOURCE.zip'
if (-not (Test-Path -LiteralPath $gplArchive -PathType Leaf) -or (Get-FileHash -LiteralPath $gplArchive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $gplReceipt.sourceArchiveSha256) { throw 'Missing or changed GPL corresponding-source archive.' }
foreach ($tool in 'skate-ped','skate-music') {
    $binary = Join-Path $root "gta\$tool\target\release\$tool.exe"
    $expectedHash = $gplReceipt.binarySha256.PSObject.Properties["$tool.exe"].Value
    if ($expectedHash -notmatch '^[0-9a-f]{64}$' -or -not (Test-Path -LiteralPath $binary -PathType Leaf) -or (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expectedHash) { throw "GPL source receipt does not match $tool.exe; rebuild and prepare source again." }
}
foreach ($required in 'NOTICE','build\package\THIRD-PARTY-NOTICES.txt') {
    $requiredPath = Join-Path $sk8v $required
    if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf) -or (Get-Item -LiteralPath $requiredPath).Length -lt 100) { throw "Missing required SK8V notice: $required" }
}
function AssertStage([string]$Path) {
    $resolved = [IO.Path]::GetFullPath($Path)
    if (-not $resolved.StartsWith(([IO.Path]::GetFullPath($out) + '\'), [StringComparison]::OrdinalIgnoreCase)) { throw 'Unexpected package staging path.' }
    if ((Test-Path -LiteralPath $resolved) -and ((Get-Item -LiteralPath $resolved).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Package staging cannot be a junction.' }
}
AssertStage $stage
Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $stage | Out-Null

foreach ($repo in $root,$sk8v) {
    if (git -C $repo status --porcelain --untracked-files=no) { throw 'Commit source changes before packaging.' }
}
$storyReceipt = & (Join-Path $PSScriptRoot 'check-story-build.ps1') -Fork $sk8v
$basis = & (Join-Path $PSScriptRoot 'check-latest-sk8v.ps1') -Fork $sk8v
& (Join-Path $PSScriptRoot 'check-package-overlays.ps1') -Fork $sk8v
$branch = git -C $sk8v rev-parse --abbrev-ref HEAD
# Isolated builds are accepted by exact build receipt and clean source checks.
$files = @(
    (Join-Path $sk8v 'build\package\SkateVLegacy.asi'),
    (Join-Path $sk8v 'build\package\SkateVRuntime.dll'),
    (Join-Path $sk8v 'tools\prepare-board-parts.py'),
    (Join-Path $sk8v 'tools\_common.py'),
    (Join-Path $root 'gta\skate-ped\target\release\skate-ped.exe'),
    (Join-Path $root 'gta\skate-music\target\release\skate-music.exe')
)
foreach ($f in $files) {
    if (-not (Test-Path $f)) { throw "missing: $f (build it first)" }
    Copy-Item $f $stage
}
Copy-Item (Join-Path $sk8v 'LICENSE') (Join-Path $stage 'LICENSE-SK8V.txt')
Copy-Item (Join-Path $sk8v 'NOTICE') (Join-Path $stage 'NOTICE-SK8V.txt')
Copy-Item (Join-Path $sk8v 'build\package\THIRD-PARTY-NOTICES.txt') $stage
Copy-Item (Join-Path $root 'public\COMPONENT-LICENSES.md') $stage
Copy-Item (Join-Path $root 'public\licenses') $stage -Recurse
Copy-Item -LiteralPath $gplArchive -Destination (Join-Path $stage 'GPL-CORRESPONDING-SOURCE.zip')
Copy-Item -LiteralPath $GplSourceReceipt -Destination (Join-Path $stage 'GPL-SOURCE-RECEIPT.json')
Copy-Item -LiteralPath (Join-Path (Split-Path -Parent $GplSourceReceipt) 'GPL-TOOL-NOTICES.txt') -Destination $stage
Copy-Item (Join-Path $root 'public\LICENSE') (Join-Path $stage 'LICENSE.txt')
Copy-Item (Join-Path $root 'public\NOTICE') (Join-Path $stage 'NOTICE.txt')
Copy-Item (Join-Path $sk8v 'build\package\GTAxSkate-build.json') $stage
$commit = git -C $sk8v rev-parse --short HEAD
Set-Content (Join-Path $stage 'VERSION.txt') "SK8V build: Manatros/SK8V skatemods $commit (SK8V $($basis.tag) + GTAxSkate's additions)`r`nPackaged: $(Get-Date -Format s)`r`nHost/runtime build receipt: GTAxSkate-build.json"
$composer = Join-Path $stage 'composer'
New-Item -ItemType Directory -Force $composer | Out-Null
foreach ($name in 'board-composition-worker.py','prepare-native-custom-board.py','setup-board-composer.ps1','restore-board-composer.ps1') {
    Copy-Item (Join-Path $PSScriptRoot $name) $composer
}
Copy-Item (Join-Path $root 'public\BOARD-COMPOSITION.md') $stage
python (Join-Path $PSScriptRoot 'public-privacy.py') --path $stage --forbidden-patterns-file $PrivacyPatternsFile --reviewed-upstream-exceptions (Join-Path $PSScriptRoot 'reviewed-upstream-paths.json')
if ($LASTEXITCODE) { throw 'Binary payload privacy scan failed; rebuild with neutral paths.' }
$zip = Join-Path $out 'GTAxSkate-bin.zip'
Remove-Item $zip -ErrorAction SilentlyContinue
Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zip
Write-Host "$zip ($([math]::Round((Get-Item $zip).Length / 1MB, 1)) MB)" -ForegroundColor Green

# The drop-in replaces the matched host/runtime pair and includes its notices.
$drop = Join-Path $out 'dropin'
AssertStage $drop
Remove-Item -LiteralPath $drop -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $drop | Out-Null
Copy-Item (Join-Path $sk8v 'build\package\SkateVLegacy.asi'), (Join-Path $sk8v 'build\package\SkateVRuntime.dll') $drop
Copy-Item (Join-Path $root 'public\dropin\README.txt') $drop
Copy-Item (Join-Path $stage 'LICENSE-SK8V.txt'), (Join-Path $stage 'NOTICE-SK8V.txt'), (Join-Path $stage 'NOTICE.txt'), (Join-Path $stage 'VERSION.txt'), (Join-Path $stage 'THIRD-PARTY-NOTICES.txt'), (Join-Path $stage 'COMPONENT-LICENSES.md') $drop
Copy-Item (Join-Path $stage 'licenses') $drop -Recurse
Copy-Item (Join-Path $stage 'GTAxSkate-build.json') $drop

$dropZip = Join-Path $out "GTAxSkate-dropin-for-SK8V-$($basis.tag).zip"
python (Join-Path $PSScriptRoot 'public-privacy.py') --path $drop --forbidden-patterns-file $PrivacyPatternsFile --reviewed-upstream-exceptions (Join-Path $PSScriptRoot 'reviewed-upstream-paths.json')
if ($LASTEXITCODE) { throw 'Drop-in payload privacy scan failed.' }
Remove-Item $dropZip -ErrorAction SilentlyContinue
Compress-Archive -Path (Join-Path $drop '*') -DestinationPath $dropZip
Write-Host "$dropZip ($([math]::Round((Get-Item $dropZip).Length / 1MB, 1)) MB)" -ForegroundColor Green

# The complete package: the installer with every GTAxSkate build in its bin
# folder (no second download); SK8V itself and the game data it makes from the
# player's own Skate 3 come at install time.
$setup = Join-Path $out 'setup\GTAxSkate'
AssertStage (Join-Path $out 'setup')
Remove-Item -LiteralPath (Join-Path $out 'setup') -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force (Join-Path $setup 'scripts'), (Join-Path $setup 'bin'), (Join-Path $setup 'custom-assets\decks') | Out-Null
Copy-Item (Join-Path $root 'Install GTAxSkate.cmd') $setup
Copy-Item (Join-Path $root 'scripts\install.ps1'), (Join-Path $root 'scripts\get-skate-music.ps1') (Join-Path $setup 'scripts')
Copy-Item (Join-Path $stage '*') (Join-Path $setup 'bin') -Recurse
Copy-Item (Join-Path $root 'custom-assets\README.md') (Join-Path $setup 'custom-assets')
Copy-Item (Join-Path $root 'public\setup\README.txt') $setup
Copy-Item (Join-Path $root 'public\LICENSE') (Join-Path $setup 'LICENSE.txt')
Copy-Item (Join-Path $root 'public\NOTICE') (Join-Path $setup 'NOTICE.txt')
python (Join-Path $PSScriptRoot 'public-privacy.py') --path $setup --forbidden-patterns-file $PrivacyPatternsFile --reviewed-upstream-exceptions (Join-Path $PSScriptRoot 'reviewed-upstream-paths.json')
if ($LASTEXITCODE) { throw 'Setup privacy scan failed.' }
$setupZip = Join-Path $out 'GTAxSkate-Setup.zip'
Remove-Item $setupZip -ErrorAction SilentlyContinue
Compress-Archive -Path (Join-Path $out 'setup\*') -DestinationPath $setupZip
Write-Host "$setupZip ($([math]::Round((Get-Item $setupZip).Length / 1MB, 1)) MB)" -ForegroundColor Green
