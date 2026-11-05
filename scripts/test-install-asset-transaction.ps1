$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'install-asset-transaction.ps1')
$root = Join-Path (Split-Path -Parent $PSScriptRoot) ('build\asset-transaction-tests-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$script:copyCount = 0
$script:failAt = @()
function Copy-Item {
    [CmdletBinding()]
    param([string]$LiteralPath, [string]$Destination, [switch]$Force)
    $script:copyCount++
    if ($script:copyCount -in $script:failAt) { throw 'Injected copy failure' }
    Microsoft.PowerShell.Management\Copy-Item -LiteralPath $LiteralPath -Destination $Destination -Force:$Force -ErrorAction Stop
}
function Assert([bool]$Ok, [string]$Message) { if (-not $Ok) { throw $Message } }
function Fixture([string]$Name) {
    $base = Join-Path $root $Name
    $t = Join-Path $base 'target'; $s = Join-Path $base 'stage'; $b = Join-Path $base 'backup'
    New-Item -ItemType Directory -Force -Path $t,(Join-Path $s 'payload') | Out-Null
    [IO.File]::WriteAllText((Join-Path $t 'a'), 'old-a')
    [IO.File]::WriteAllText((Join-Path $s 'payload\a'), 'new-a')
    [IO.File]::WriteAllText((Join-Path $s 'payload\b'), 'new-b')
    $script:copyCount=0; $script:failAt=@()
    @{TargetRoot=$t; StageRoot=$s; BackupRoot=$b; Files=@(
        [pscustomobject]@{name='a'; sha256=(Get-FileHash (Join-Path $s 'payload\a')).Hash; originalSha256=(Get-FileHash (Join-Path $t 'a')).Hash},
        [pscustomobject]@{name='b'; sha256=(Get-FileHash (Join-Path $s 'payload\b')).Hash})}
}
$p=Fixture 'success'; $r=Install-AssetTransaction @p
Assert ($r.installed -eq 2 -and [IO.File]::ReadAllText((Join-Path $p.TargetRoot 'a')) -eq 'new-a') 'Successful install failed'
Assert ([IO.File]::ReadAllText((Join-Path $p.BackupRoot 'a')) -eq 'old-a') 'Original backup absent'
Write-Host 'PASS success and backup'
$p=Fixture 'validate-all'; $p.Files[1].sha256='0'*64
try { Install-AssetTransaction @p; throw 'Expected validation failure' } catch { Assert ($_.Exception.Message -match 'Invalid staged asset') $_.Exception.Message }
Assert ($script:copyCount -eq 0) 'Mutation occurred before all validation'
Write-Host 'PASS full prevalidation'
$p=Fixture 'rollback'
[IO.File]::WriteAllText((Join-Path $p.StageRoot 'payload\c'), 'new-c')
$p.Files += [pscustomobject]@{name='c'; sha256=(Get-FileHash (Join-Path $p.StageRoot 'payload\c')).Hash}
$script:failAt=@(4)
try { Install-AssetTransaction @p; throw 'Expected copy failure' } catch { Assert ($_.Exception.Message -match 'Rollback completed') $_.Exception.Message }
Assert ([IO.File]::ReadAllText((Join-Path $p.TargetRoot 'a')) -eq 'old-a') 'Original not restored'
Assert (-not (Test-Path (Join-Path $p.TargetRoot 'b'))) 'New destination survived rollback'
Write-Host 'PASS partial copy rollback'
$p=Fixture 'rollback-incomplete'; $script:failAt=@(3,4)
try { Install-AssetTransaction @p; throw 'Expected recovery failure' } catch { Assert ($_.Exception.Message -match 'ROLLBACK INCOMPLETE') $_.Exception.Message }
Assert ((Test-Path $p.BackupRoot) -and (Test-Path $p.StageRoot)) 'Recovery material lost'
Write-Host 'PASS incomplete rollback retains recovery'
$p=Fixture 'traversal'; $p.Files[0].name='..\escape'
try { Install-AssetTransaction @p; throw 'Expected traversal failure' } catch { Assert ($_.Exception.Message -match 'Invalid asset name') $_.Exception.Message }
Assert ($script:copyCount -eq 0) 'Traversal mutated files'
Write-Host 'PASS path traversal rejected'
$p=Fixture 'original-changed'; [IO.File]::WriteAllText((Join-Path $p.TargetRoot 'a'), 'user edit')
try { Install-AssetTransaction @p; throw 'Expected changed original failure' } catch { Assert ($_.Exception.Message -match 'Original asset changed') $_.Exception.Message }
Assert ($script:copyCount -eq 0) 'Changed original mutated'
Write-Host 'PASS original hash guard'
Write-Host "All 6 asset transaction checks passed. Fixtures: $root"
