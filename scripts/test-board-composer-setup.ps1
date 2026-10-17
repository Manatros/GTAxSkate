$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$qa = Join-Path $root ('build\composer-setup-tests\' + [guid]::NewGuid().ToString('N'))
$assets = Join-Path $qa 'asset folder with spaces'
$native = Join-Path $assets 'private\board-native'
$cache = Join-Path $assets 'private\board-composed'
New-Item -ItemType Directory -Path $native,(Join-Path $assets 'private\board-parts'),$cache -Force | Out-Null
[IO.File]::WriteAllText((Join-Path $native 'skatev_board.ydr'), 'original native drawable')
[IO.File]::WriteAllText((Join-Path $native 'skatev_board.ytyp'), 'original native archetype')
[IO.File]::WriteAllText((Join-Path $cache 'worker.ini'), ('original Unicode setting: ' + [char]0x00C4 + [char]0x00D6 + [char]0x00DC), [Text.Encoding]::Unicode)
[IO.File]::WriteAllText((Join-Path $cache 'board-composition-worker.py'), 'original helper')
$before = @{}
foreach ($file in Get-ChildItem -LiteralPath $cache -File) { $before[$file.Name] = (Get-FileHash -LiteralPath $file.FullName).Hash }
$nativeHash = (Get-FileHash -LiteralPath (Join-Path $native 'skatev_board.ydr')).Hash
$python = (Get-Command python.exe).Source
$rage = Join-Path $root 'external\sk8v\build\tools\rage.exe'
$result = & (Join-Path $PSScriptRoot 'setup-board-composer.ps1') -Assets $assets -Python $python -Rage $rage
if (-not (Test-Path -LiteralPath (Join-Path $cache 'enabled'))) { throw 'Helper not enabled.' }
if ((Get-FileHash -LiteralPath (Join-Path $cache 'board-composition-worker.py')).Hash -ne (Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'board-composition-worker.py')).Hash) { throw 'Installed worker differs.' }
$bytes = [IO.File]::ReadAllBytes((Join-Path $cache 'worker.ini'))
if ($bytes[0] -ne 255 -or $bytes[1] -ne 254) { throw 'Worker INI is not Unicode.' }
& (Join-Path $PSScriptRoot 'restore-board-composer.ps1') -Backup $result.backup | Out-Null
$after = @(Get-ChildItem -LiteralPath $cache -File)
if ($after.Count -ne $before.Count) { throw 'Rollback file count differs.' }
foreach ($file in $after) { if ((Get-FileHash -LiteralPath $file.FullName).Hash -ne $before[$file.Name]) { throw 'Rollback bytes differ.' } }
if ((Get-FileHash -LiteralPath (Join-Path $native 'skatev_board.ydr')).Hash -ne $nativeHash) { throw 'Original native model changed.' }
# A corrupt backup must fail before modifying any installed file.
$result = & (Join-Path $PSScriptRoot 'setup-board-composer.ps1') -Assets $assets -Python $python -Rage $rage
[IO.File]::WriteAllText((Join-Path $result.backup 'worker.ini'), 'corrupt backup')
$enabledHash = (Get-FileHash -LiteralPath (Join-Path $cache 'enabled')).Hash
$rejected = $false
try { & (Join-Path $PSScriptRoot 'restore-board-composer.ps1') -Backup $result.backup | Out-Null } catch { $rejected = $_.Exception.Message -like '*Backuphash*' }
if (-not $rejected -or (Get-FileHash -LiteralPath (Join-Path $cache 'enabled')).Hash -ne $enabledHash) { throw 'Corrupt backup was not rejected before writes.' }
Write-Host 'PASS: helper setup, Unicode paths/INI, exact rollback, unchanged originals and corrupt-backup rejection'
