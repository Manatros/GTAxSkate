# Offline check of the actual installer function, including a real pwsh child.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$tokens = $null; $errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot 'install.ps1'), [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw 'Installer syntax errors.' }
$fn = $ast.Find({ param($n) $n -is [Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq 'Invoke-WorldCollision' }, $true)
if (-not $fn) { throw 'Missing production world invocation.' }
Invoke-Expression $fn.Extent.Text
$fixture = Join-Path $root ('build\qa-installer-world-host\' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $fixture | Out-Null
$helper = Join-Path $fixture 'world helper.ps1'
@'
param($Gta,$Keys,$Baker,$Rails,$World)
if ($PSVersionTable.PSEdition -ne 'Core') { exit 21 }
if ($Gta -ne 'gta with spaces' -or $Keys -ne 'keys with spaces' -or $Baker -ne 'baker with spaces' -or $Rails -ne 'rails with spaces') { exit 22 }
Get-FileHash -LiteralPath $PSCommandPath | Out-Null
if ($World -eq 'failure') { exit 17 }
Set-Content -LiteralPath $World -Value 'PowerShell 7 success'
'@ | Set-Content -LiteralPath $helper -Encoding ASCII
$marker = Join-Path $fixture 'success marker.txt'
Invoke-WorldCollision -Script $helper -Gta 'gta with spaces' -Keys 'keys with spaces' -Baker 'baker with spaces' -Rails 'rails with spaces' -World $marker
if (-not (Test-Path -LiteralPath $marker)) { throw 'World child did not complete.' }
$failed = $false
try { Invoke-WorldCollision -Script $helper -Gta 'gta with spaces' -Keys 'keys with spaces' -Baker 'baker with spaces' -Rails 'rails with spaces' -World failure }
catch { if ($_.Exception.Message -notmatch 'exit code 17') { throw }; $failed = $true }
if (-not $failed) { throw 'A failed world child was accepted.' }
function Get-Command { param($Name,$CommandType,$ErrorAction) return $null }
$failed = $false
try { Invoke-WorldCollision -Script $helper }
catch { if ($_.Exception.Message -notmatch 'requires PowerShell 7') { throw }; $failed = $true }
if (-not $failed) { throw 'Missing PowerShell 7 was accepted.' }
Write-Host 'PASS: PowerShell 7 host, spaced arguments, Get-FileHash, failure exit, missing host (5 checks).'
