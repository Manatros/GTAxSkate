# Check the source/build receipt before publishing a host/runtime pair.
param([Parameter(Mandatory=$true)][string]$Fork)
$ErrorActionPreference = 'Stop'
$commit = (git -C $Fork rev-parse HEAD).Trim()
if ($LASTEXITCODE) { throw 'Cannot resolve Story source revision.' }
$package = Join-Path $Fork 'build\package'
$receiptPath = Join-Path $package 'GTAxSkate-build.json'
if (-not (Test-Path -LiteralPath $receiptPath -PathType Leaf)) { throw 'Missing Story build receipt; rebuild both host and runtime.' }
$receipt = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
if ($receipt.format -ne 'gtaxskate-build' -or $receipt.version -ne 1 -or
    $receipt.forkCommit -cne $commit -or $receipt.files -isnot [pscustomobject]) {
    throw 'Story build receipt does not match the current source revision.'
}
$pair = @('SkateVLegacy.asi','SkateVRuntime.dll')
if (@($receipt.files.PSObject.Properties).Count -ne 2) { throw 'Story receipt must describe exactly host and runtime.' }
foreach ($name in $pair) {
    $entry = $receipt.files.PSObject.Properties[$name]
    $path = Join-Path $package $name
    if (-not $entry -or $entry.Value -isnot [string] -or $entry.Value -notmatch '^[a-fA-F0-9]{64}$' -or
        -not (Test-Path -LiteralPath $path -PathType Leaf) -or
        (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ine $entry.Value) {
        throw "Story build receipt hash mismatch: $name"
    }
}
return $receipt
