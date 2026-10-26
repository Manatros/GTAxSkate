# The fork's whole source as one patch on the official SK8V release (since
# v0.7.0; the layered sk8v-*.patch files describe v0.6.0 and older). Applied to
# the release tag in a scratch index, the patch must give exactly the fork's
# HEAD tree, so the published patch is the published build's source.
#
#   check-cumulative-patch.ps1 -Fork <SK8V fork checkout> [-Write]
#
# -Write makes the patch anew from the fork's HEAD first. Never touches the
# fork's checkout or index.
param(
    [string]$Fork = (Join-Path (Split-Path -Parent $PSScriptRoot) 'external\sk8v'),
    [string]$Patch = (Join-Path (Split-Path -Parent $PSScriptRoot) 'patches\sk8v-gtaxskate-cumulative.patch'),
    [string]$Tag = '',
    [switch]$Write,
    [switch]$WorkingTree
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$Patch = [IO.Path]::GetFullPath($Patch)
if (-not $Tag) {
    $match = [regex]::Match([IO.File]::ReadAllText((Join-Path $PSScriptRoot 'install.ps1')), '(?m)^\$Sk8vTag = ''([^'']+)''')
    if (-not $match.Success) { throw 'Cannot read $Sk8vTag from install.ps1.' }
    $Tag = $match.Groups[1].Value
}
# (Not named "Git": PowerShell would resolve git inside it to the function itself.)
function Invoke-ForkGit {
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { $out = & git.exe -C $Fork @args 2>&1 } finally { $ErrorActionPreference = $saved }
    if ($LASTEXITCODE) { throw "git $($args -join ' ') failed: $out" }
    $out
}
$base = ([string](Invoke-ForkGit rev-parse --verify "refs/tags/$Tag^{commit}")).Trim()
$expectedTree = ([string](Invoke-ForkGit rev-parse 'HEAD^{tree}')).Trim()
if ($Write) {
    if (Invoke-ForkGit status --porcelain --untracked-files=no) { throw 'Commit the fork''s changes before writing the cumulative patch.' }
    Invoke-ForkGit diff --binary --full-index $base HEAD "--output=$Patch" | Out-Null
}
if (-not (Test-Path -LiteralPath $Patch -PathType Leaf)) { throw "Missing $Patch (run with -Write)." }
$scratch = Join-Path $root 'build\overlay-checks'
New-Item -ItemType Directory -Force $scratch | Out-Null
$index = Join-Path $scratch ([guid]::NewGuid().ToString('N') + '.index')
$savedIndex = $env:GIT_INDEX_FILE
try {
    $env:GIT_INDEX_FILE = $index
    Invoke-ForkGit read-tree $base | Out-Null
    Invoke-ForkGit apply --cached --binary --whitespace=nowarn $Patch | Out-Null
    $tree = ([string](Invoke-ForkGit write-tree)).Trim()
    if ($WorkingTree) {
        Invoke-ForkGit read-tree HEAD | Out-Null
        Invoke-ForkGit -c core.safecrlf=false add -u | Out-Null
        # Add only files named by the patch. Unrelated untracked files, such
        # as private receipts or game data, must never enter this audit tree.
        $targets = @()
        foreach ($row in @(Invoke-ForkGit apply --numstat $Patch)) {
            $target = ([string]$row -split "`t", 3)[2]
            if (-not $target -or $target.StartsWith('"') -or [IO.Path]::IsPathRooted($target) -or $target -match '(^|[/\\])\.\.([/\\]|$)') {
                throw 'Unsupported cumulative patch target path.'
            }
            if (Test-Path -LiteralPath (Join-Path $Fork $target) -PathType Leaf) { $targets += $target }
        }
        if ($targets.Count) { Invoke-ForkGit -c core.safecrlf=false add -- $targets | Out-Null }
        $expectedTree = ([string](Invoke-ForkGit write-tree)).Trim()
    }
} finally {
    $env:GIT_INDEX_FILE = $savedIndex
    Remove-Item -LiteralPath $index -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath "$index.lock" -Force -ErrorAction SilentlyContinue
}
if ($tree -ne $expectedTree) {
    $kind = if ($WorkingTree) { 'working tree' } else { 'HEAD' }
    throw "The cumulative patch on SK8V $Tag gives tree $tree, the fork's $kind is $expectedTree. Review source changes or regenerate from a committed fork with -Write."
}
[pscustomobject]@{ tag = $Tag; base = $base; tree = $tree; patch = $Patch; sha256 = (Get-FileHash -LiteralPath $Patch -Algorithm SHA256).Hash.ToLowerInvariant() }
