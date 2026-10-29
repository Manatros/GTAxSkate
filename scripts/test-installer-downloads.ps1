# Offline tests of the production download verifier; no installer/game/network calls.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$installer = Join-Path $PSScriptRoot 'install.ps1'
$tokens = $null; $errors = $null
$tree = [Management.Automation.Language.Parser]::ParseFile($installer, [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw 'Installer syntax errors.' }
$function = $tree.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Get-VerifiedArchive' }, $true)
if (-not $function) { throw 'Production download verifier missing.' }
Invoke-Expression $function.Extent.Text
foreach ($fnName in 'Test-Skate3Source', 'Test-GtaSource') {
    $fnNode = $tree.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $fnName }, $true)
    if (-not $fnNode) { throw "Missing $fnName in installer." }
    Invoke-Expression $fnNode.Extent.Text
}
function Say([string]$text, [string]$color = 'Gray') { }
$GtaBuild = '1.0.3889.0'
function Assert([bool]$value, [string]$message) { if (-not $value) { throw $message } }
function Expect-Failure([scriptblock]$action, [string]$fragment) {
    $message = ''
    try { & $action } catch { $message = $_.Exception.Message }
    Assert ($message -like "*$fragment*") "Expected failure containing '$fragment'."
}
$qa = Join-Path $root ('build\qa-installer-downloads\' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $qa -Force | Out-Null
$fixture = Join-Path $qa 'reviewed-fixture.zip'
[IO.File]::WriteAllText($fixture, 'reviewed fixture bytes')
$expected = (Get-FileHash -LiteralPath $fixture -Algorithm SHA256).Hash.ToLowerInvariant()
$script:downloadCount = 0
$script:downloadMode = 'valid'
function Download([string]$url, [string]$to) {
    $script:downloadCount++
    if ($script:downloadMode -eq 'valid') { Copy-Item -LiteralPath $fixture -Destination $to }
    else {
        [IO.File]::WriteAllText($to, 'tampered or incomplete bytes')
        if ($script:downloadMode -eq 'interrupted') { throw 'Synthetic network interruption.' }
    }
}
$cache = Join-Path $qa 'cache.zip'
Get-VerifiedArchive 'https://example.invalid/fixture.zip' $cache $expected
Assert ($script:downloadCount -eq 1) 'Fresh download did not use the downloader.'
Assert ((Get-FileHash -LiteralPath $cache -Algorithm SHA256).Hash -eq $expected) 'Fresh download was not verified.'
Get-VerifiedArchive 'https://example.invalid/fixture.zip' $cache $expected.ToUpperInvariant()
Assert ($script:downloadCount -eq 1) 'Verified cache was downloaded again.'
[IO.File]::WriteAllText($cache, 'tampered cache')
Expect-Failure { Get-VerifiedArchive 'https://example.invalid/fixture.zip' $cache $expected } 'Cached download checksum mismatch'
Assert (-not (Test-Path -LiteralPath $cache)) 'Tampered cache survived.'
Assert ($script:downloadCount -eq 1) 'Tampered cache silently retried.'
$script:downloadMode = 'tampered'
Expect-Failure { Get-VerifiedArchive 'https://example.invalid/fixture.zip' $cache $expected } 'Download checksum mismatch'
Assert (-not (Test-Path -LiteralPath $cache)) 'Tampered download reached the cache.'
Assert (@(Get-ChildItem -LiteralPath $qa -Filter '*.download-*').Count -eq 0) 'Tampered partial file survived.'
$script:downloadMode = 'interrupted'
Expect-Failure { Get-VerifiedArchive 'https://example.invalid/fixture.zip' $cache $expected } 'Synthetic network interruption'
Assert (@(Get-ChildItem -LiteralPath $qa -Filter '*.download-*').Count -eq 0) 'Interrupted partial file survived.'
Expect-Failure { Get-VerifiedArchive 'https://example.invalid/fixture.zip' $cache 'invalid' } 'Invalid pinned download checksum'
$text = [IO.File]::ReadAllText($installer)
Assert ($text -notmatch 'Unblock-File') 'Installer still unblocks arbitrary files.'
Assert ($text -notmatch 'releases/(latest|download/dev-build)/download/GTAxSkate-bin.zip') 'Moving remote binary fallback remains.'
Assert ($text.Contains('Get-VerifiedArchive $Sk8vZip $zip $Sk8vSha256')) 'SK8V extraction bypasses verification.'
Assert ($text.Contains('Get-VerifiedArchive $VgmstreamZip $zip $VgmstreamSha256')) 'vgmstream extraction bypasses verification.'
# The remote binary fallback may only fetch a fixed, privacy-reviewed release tag.
Assert ($text -match '(?m)^\$GtaxSkateRelease = ''v\d+\.\d+\.\d+''\r?$') 'Remote binaries are not pinned to a release tag.'
Assert ($text.Contains('$GtaxSkateBinZip = "https://github.com/Manatros/GTAxSkate/releases/download/$GtaxSkateRelease/GTAxSkate-bin.zip"')) 'Remote binaries do not come from the pinned release.'
Assert ($text -match '(?m)^\$GtaxSkateBinSha256 = ''[a-f0-9]{64}''\r?$') 'Remote binaries have no pinned SHA-256.'
Assert ($text.Contains('Get-VerifiedArchive $GtaxSkateBinZip $binZip $GtaxSkateBinSha256')) 'Remote binary extraction bypasses verification.'
Assert ($text.Contains("Disc-With @('data/big/db.big', 'data/content/createacharacter.big')")) 'Boards step does not extract db.big from ISO.'

# Verify Skate 3 source checks (ISO magic, archive rejection, missing db.big, subfolder resolution).
$badFolder = Join-Path $qa 'mounted-video-partition'
New-Item -ItemType Directory -Path $badFolder -Force | Out-Null
Assert ((Test-Skate3Source $badFolder) -like '*missing default.xex, data\big\db.big*Do not mount*') 'Mounted or incomplete folder was not caught.'
$partialFolder = Join-Path $qa 'partial-disc'
New-Item -ItemType Directory -Path (Join-Path $partialFolder 'data\big'), (Join-Path $partialFolder 'data\content') -Force | Out-Null
[IO.File]::WriteAllText((Join-Path $partialFolder 'default.xex'), 'xex')
[IO.File]::WriteAllText((Join-Path $partialFolder 'data\content\createacharacter.big'), 'cac')
Assert ((Test-Skate3Source $partialFolder) -like '*missing data\big\db.big*') 'Missing db.big was not caught.'
[IO.File]::WriteAllText((Join-Path $partialFolder 'data\big\db.big'), 'db')
Assert ((Test-Skate3Source $partialFolder) -eq '') 'Complete extracted Skate 3 folder was rejected.'
$script:Skate3 = Join-Path $partialFolder 'data\big'
Assert ((Test-Skate3Source $script:Skate3) -eq '' -and $script:Skate3 -eq $partialFolder) 'Subfolder inside extracted disc did not resolve to parent.'
$script:Skate3 = Join-Path $partialFolder 'default.xex'
Assert ((Test-Skate3Source $script:Skate3) -eq '' -and $script:Skate3 -eq $partialFolder) 'Selecting default.xex did not resolve to its folder.'
$nestedOuter = Join-Path $qa 'nested-outer'
$nestedInner = Join-Path $nestedOuter 'Skate 3'
New-Item -ItemType Directory -Path (Join-Path $nestedInner 'data\big'), (Join-Path $nestedInner 'data\content') -Force | Out-Null
[IO.File]::WriteAllText((Join-Path $nestedInner 'default.xex'), 'xex')
[IO.File]::WriteAllText((Join-Path $nestedInner 'data\big\db.big'), 'db')
[IO.File]::WriteAllText((Join-Path $nestedInner 'data\content\createacharacter.big'), 'cac')
$script:Skate3 = $nestedOuter
Assert ((Test-Skate3Source $script:Skate3) -eq '' -and $script:Skate3 -eq $nestedInner) 'Nested extracted folder did not resolve to inner folder.'
$zipIso = Join-Path $qa 'Skate3.7z'
[IO.File]::WriteAllText($zipIso, '7z')
Assert ((Test-Skate3Source $zipIso) -like '*compressed archive (.7z)*Unzip*') 'Unextracted .7z archive was not caught.'
$zipFolder = Join-Path $qa 'archive-folder'
New-Item -ItemType Directory -Path $zipFolder -Force | Out-Null
[IO.File]::WriteAllText((Join-Path $zipFolder 'Skate3.zip'), 'zip')
Assert ((Test-Skate3Source $zipFolder) -like '*compressed archive (.zip)*Unzip*') 'Folder containing unextracted .zip archive was not caught.'
$badIso = Join-Path $qa 'not-xbox.iso'
[IO.File]::WriteAllBytes($badIso, (New-Object byte[] 70000))
Assert ((Test-Skate3Source $badIso) -like '*no XDVDFS partition*') 'Non-Xbox ISO was not caught.'
$goodIso = Join-Path $qa 'valid-xiso.iso'
$isoBytes = New-Object byte[] (32 * 2048 + 28)
$magic = [Text.Encoding]::ASCII.GetBytes('MICROSOFT*XBOX*MEDIA')
[Array]::Copy($magic, 0, $isoBytes, 32 * 2048, $magic.Length)
[IO.File]::WriteAllBytes($goodIso, $isoBytes)
Assert ((Test-Skate3Source $goodIso) -eq '') 'Valid XDVDFS ISO was rejected.'
$isoFolder = Join-Path $qa 'folder-with-iso'
New-Item -ItemType Directory -Path $isoFolder -Force | Out-Null
Copy-Item -LiteralPath $goodIso -Destination (Join-Path $isoFolder 'Skate3.iso')
$script:Skate3 = $isoFolder
Assert ((Test-Skate3Source $script:Skate3) -eq '' -and $script:Skate3 -eq (Join-Path $isoFolder 'Skate3.iso')) 'Folder with a single .iso did not resolve to the .iso.'

# Verify GTA V source checks (Enhanced edition rejection).
$enhancedGta = Join-Path $qa 'GTA-Enhanced'
New-Item -ItemType Directory -Path $enhancedGta -Force | Out-Null
[IO.File]::WriteAllText((Join-Path $enhancedGta 'GTA5_Enhanced.exe'), 'enhanced')
Assert ((Test-GtaSource $enhancedGta) -like '*GTA V Enhanced*') 'GTA V Enhanced folder was not caught.'
Write-Host 'PASS: fresh download, verified cache, tampered cache/download, interrupted cleanup, invalid pin, source validation and installer integration.'
