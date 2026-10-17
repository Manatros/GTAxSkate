# Refuse packaging an outdated basis instead of silently labelling it current.
param([string]$Fork = (Join-Path (Split-Path -Parent $PSScriptRoot) 'external\sk8v'))
$ErrorActionPreference = 'Stop'
$release = Invoke-RestMethod -Uri 'https://api.github.com/repos/becaring/SK8V/releases/latest' -Headers @{'User-Agent'='GTAxSkate-build'} -TimeoutSec 20
$tag = [string]$release.tag_name
if ($release.draft -or $release.prerelease -or $tag -notmatch '^v?\d+\.\d+\.\d+$') { throw 'Unrecognized official SK8V release; review required.' }
$ErrorActionPreference = 'Continue'
$commit = git -C $Fork rev-parse --verify "refs/tags/${tag}^{commit}" 2>$null
$tagCode = $LASTEXITCODE
$ErrorActionPreference = 'Stop'
if ($tagCode -ne 0) { throw "Latest SK8V $tag tag is missing locally. Fetch upstream tags and integrate it before packaging." }
git -C $Fork merge-base --is-ancestor $commit HEAD
if ($LASTEXITCODE -ne 0) { throw "Latest SK8V $tag is not integrated into this source. Refusing an outdated test package." }
$installer = [IO.File]::ReadAllText((Join-Path $PSScriptRoot 'install.ps1'))
$match = [regex]::Match($installer, '(?m)^\$Sk8vTag = ''([^'']+)''')
if (-not $match.Success -or $match.Groups[1].Value -ne $tag) { throw "Installer basis differs from latest SK8V $tag; update and verify it before packaging." }
[pscustomobject]@{tag=$tag; commit=([string]$commit).Trim(); url=[string]$release.html_url; checkedUtc=[DateTime]::UtcNow.ToString('o')}
