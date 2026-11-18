param(
    [Parameter(Mandatory)] [string]$ForkPath,
    [string]$CoopPackPath = 'build\coop-optional-pack',
    [string]$Output = 'build\release-v091',
    [Parameter(Mandatory)] [string]$PrivacyPatternsFile
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$fork = [IO.Path]::GetFullPath((Join-Path $root $ForkPath))
$coop = [IO.Path]::GetFullPath((Join-Path $root $CoopPackPath))
$out = [IO.Path]::GetFullPath((Join-Path $root $Output))
$privacy = [IO.Path]::GetFullPath((Join-Path $root $PrivacyPatternsFile))
foreach ($path in @($fork, $coop, $out, $privacy)) {
    if (-not (Test-Path -LiteralPath $path)) { throw "Required path not found: $path" }
}

# Tutorial assets are never redistributed. The optional archive contains only
# the generation and installation tools; each player builds from their own disc.
$tutorial = Join-Path $out 'tutorial-optional'
Remove-Item -LiteralPath $tutorial -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $tutorial | Out-Null
foreach ($name in @('prepare-tutorial.py', 'prepare-tutorial-dlc.py', '_common.py', 'install-tutorial-addon.ps1')) {
    $source = Join-Path $fork "tools\$name"
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw "Tutorial tool missing: $source" }
    Copy-Item -LiteralPath $source -Destination $tutorial
}
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'Install-TutorialIsland.ps1') -Destination $tutorial
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'OPTIONAL-TUTORIAL-README.txt') -Destination (Join-Path $tutorial 'README.txt')
Copy-Item -LiteralPath (Join-Path $root 'public\LICENSE') -Destination (Join-Path $tutorial 'LICENSE.txt')
Copy-Item -LiteralPath (Join-Path $root 'public\NOTICE') -Destination (Join-Path $tutorial 'NOTICE.txt')
Copy-Item -LiteralPath (Join-Path $fork 'LICENSE') -Destination (Join-Path $tutorial 'LICENSE-SK8V.txt')
Copy-Item -LiteralPath (Join-Path $fork 'NOTICE') -Destination (Join-Path $tutorial 'NOTICE-SK8V.txt')
python (Join-Path $PSScriptRoot 'public-privacy.py') --path $tutorial --forbidden-patterns-file $privacy --reviewed-upstream-exceptions (Join-Path $PSScriptRoot 'reviewed-upstream-paths.json')
if ($LASTEXITCODE) { throw 'Tutorial optional pack privacy scan failed.' }
$tutorialZip = Join-Path $out 'GTAxSkate-Tutorial-Optional.zip'
Remove-Item -LiteralPath $tutorialZip -Force -ErrorAction SilentlyContinue
Compress-Archive -Path (Join-Path $tutorial '*') -DestinationPath $tutorialZip

# Keep the experimental coop client and private server tools separate from the
# base installer. The build-generated host key is deliberately excluded.
$coopStage = Join-Path $out 'coop-optional'
Remove-Item -LiteralPath $coopStage -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $coopStage | Out-Null
foreach ($file in Get-ChildItem -LiteralPath $coop -Recurse -File) {
    $relative = $file.FullName.Substring($coop.TrimEnd('\').Length + 1)
    if ($relative -ieq 'server\FriendsCoopHost.json') { continue }
    $destination = Join-Path $coopStage $relative
    New-Item -ItemType Directory -Force (Split-Path -Parent $destination) | Out-Null
    Copy-Item -LiteralPath $file.FullName -Destination $destination
}
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'OPTIONAL-COOP-README.txt') -Destination (Join-Path $coopStage 'README.txt')
Copy-Item -LiteralPath (Join-Path $root 'public\OPTIONAL-THIRD-PARTY-NOTICES.txt') -Destination (Join-Path $coopStage 'THIRD-PARTY-NOTICES.txt')
Copy-Item -LiteralPath (Join-Path $root 'public\LICENSE') -Destination (Join-Path $coopStage 'LICENSE.txt')
Copy-Item -LiteralPath (Join-Path $root 'public\NOTICE') -Destination (Join-Path $coopStage 'NOTICE.txt')
New-Item -ItemType Directory -Force (Join-Path $coopStage 'licenses') | Out-Null
Copy-Item -LiteralPath (Join-Path $root 'public\licenses\OPTIONAL-MIT.txt') -Destination (Join-Path $coopStage 'licenses\MIT.txt')
Copy-Item -LiteralPath (Join-Path $root 'public\licenses\OPTIONAL-ZLIB.txt') -Destination (Join-Path $coopStage 'licenses\ZLIB.txt')
python (Join-Path $PSScriptRoot 'public-privacy.py') --path $coopStage --forbidden-patterns-file $privacy --reviewed-upstream-exceptions (Join-Path $PSScriptRoot 'reviewed-upstream-paths.json')
if ($LASTEXITCODE) { throw 'Coop optional pack privacy scan failed.' }
$coopZip = Join-Path $out 'GTAxSkate-Coop-Optional.zip'
Remove-Item -LiteralPath $coopZip -Force -ErrorAction SilentlyContinue
Compress-Archive -Path (Join-Path $coopStage '*') -DestinationPath $coopZip

Write-Host "Prepared optional downloads:`n  $tutorialZip`n  $coopZip"
