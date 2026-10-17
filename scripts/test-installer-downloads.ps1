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
Assert ($text.Contains('Remote GTAxSkate binaries are temporarily disabled')) 'Remote binary fallback is not closed.'
Write-Host 'PASS: fresh download, verified cache, tampered cache/download, interrupted cleanup, invalid pin and installer integration.'
