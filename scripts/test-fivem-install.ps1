# Isolated regression checks for the real FiveM installer. No game/server needed.
# Every run keeps its own fixtures and logs; never removes existing directories.
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$fixtureBase = [IO.Path]::GetFullPath((Join-Path $repo 'build\fivem-install-tests'))
$run = [IO.Path]::GetFullPath((Join-Path $fixtureBase ([guid]::NewGuid().ToString('N'))))
if (-not $run.StartsWith($fixtureBase.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Fixture escaped test root.' }
# Existing ancestors must not redirect the unique fixture outside the repository.
for ($ancestor = $fixtureBase; ; $ancestor = Split-Path -Parent $ancestor) {
    if ((Test-Path -LiteralPath $ancestor) -and ((Get-Item -LiteralPath $ancestor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Linked fixture ancestor: $ancestor" }
    if ([string]::Equals($ancestor, $repo, [StringComparison]::OrdinalIgnoreCase)) { break }
    if (-not $ancestor.StartsWith($repo.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Fixture ancestor escaped repository.' }
}
$installer = Join-Path $PSScriptRoot 'install-fivem.ps1'
$shell = Join-Path $env:WINDIR 'System32\WindowsPowerShell\v1.0\powershell.exe'
$package = Join-Path $repo 'fivem\gtaxskate_rules'
function Assert([bool]$condition, [string]$message) { if (-not $condition) { throw $message } }
function Hash([string]$path) { (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash }
function Install([string]$folder, [string]$log, [int]$expectedExit) {
    # PowerShell 5.1 turns a native stderr line into an ErrorRecord. Capture
    # expected installer failures as process results, not harness exceptions.
    $savedPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        & $shell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $installer -ServerData $folder *> (Join-Path $run $log)
        $code = $LASTEXITCODE
    } finally { $ErrorActionPreference = $savedPreference }
    if ($expectedExit -eq 0) { Assert ($code -eq 0) "Install failed: $log (exit $code)" }
    else { Assert ($code -ne 0) "Invalid destination unexpectedly accepted: $log" }
}
New-Item -ItemType Directory -Path $run | Out-Null
$data = Join-Path $run 'server-data'
$resources = Join-Path $data 'resources'
New-Item -ItemType Directory -Path $resources | Out-Null
$cfg = Join-Path $data 'server.cfg'
# Deliberately use mixed line endings and non-ASCII UTF-8 to catch rewrites.
[IO.File]::WriteAllBytes($cfg, [byte[]](0x23,0x20,0xC3,0xA4,0x0D,0x0A,0x65,0x6E,0x73,0x75,0x72,0x65,0x20,0x74,0x65,0x73,0x74,0x0A))
$cfgHash = Hash $cfg
Install $data 'new-install.log' 0
$target = Join-Path $resources 'gtaxskate_rules'
$files = @('fxmanifest.lua', 'server.js', 'client.js', 'policy.js', 'policy.json')
foreach ($file in $files) { Assert ((Hash (Join-Path $target $file)) -eq (Hash (Join-Path $package $file))) "New install differs: $file" }
Assert ((Hash $cfg) -eq $cfgHash) 'New install changed server.cfg bytes.'
Assert (-not (Test-Path -LiteralPath (Join-Path $data 'gtaxskate-backups'))) 'Fresh install unexpectedly created a backup.'
Write-Host 'PASS new install: package copied; server.cfg byte exact.'

$policy = Join-Path $target 'policy.json'
[IO.File]::WriteAllText($policy, '{"version":1,"mode":"all","defaults":{"Pop":1.1},"rules":{}}', (New-Object Text.UTF8Encoding $false))
$policyHash = Hash $policy
$custom = Join-Path $target 'local\notes.txt'
New-Item -ItemType Directory -Path (Split-Path -Parent $custom) | Out-Null
[IO.File]::WriteAllBytes($custom, [byte[]](0,1,2,255,13,10))
$customHash = Hash $custom
$serverCode = Join-Path $target 'server.js'
[IO.File]::WriteAllText($serverCode, '// old local runtime, expected in backup')
$oldCodeHash = Hash $serverCode
Install $data 'update-install.log' 0
Assert ((Hash $policy) -eq $policyHash) 'Update overwrote server policy.'
Assert ((Hash $custom) -eq $customHash) 'Update overwrote custom nested file.'
Assert ((Hash $serverCode) -eq (Hash (Join-Path $package 'server.js'))) 'Update failed to refresh runtime code.'
Assert ((Hash $cfg) -eq $cfgHash) 'Update changed server.cfg bytes.'
$backups = @(Get-ChildItem -LiteralPath (Join-Path $data 'gtaxskate-backups') -Directory)
Assert ($backups.Count -eq 1) 'Expected exactly one update backup.'
$backup = $backups[0].FullName
Assert (-not $backup.StartsWith($resources.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)) 'Backup is inside resources.'
Assert ((Hash (Join-Path $backup 'server.js')) -eq $oldCodeHash) 'Backup lost old runtime code.'
Assert ((Hash (Join-Path $backup 'policy.json')) -eq $policyHash) 'Backup lost old policy.'
Assert ((Hash (Join-Path $backup 'local\notes.txt')) -eq $customHash) 'Backup lost custom file.'
Assert (@(Get-ChildItem -LiteralPath $resources -Force).Count -eq 1) 'Update left staging/backup or duplicate resource inside resources.'
Write-Host 'PASS update: policy/custom files preserved, old code backed up outside resources, server.cfg byte exact.'

$invalid = Join-Path $run 'missing-config'
New-Item -ItemType Directory -Path (Join-Path $invalid 'resources') | Out-Null
$sentinel = Join-Path $invalid 'keep.bin'
[IO.File]::WriteAllBytes($sentinel, [byte[]](19,7,42))
$sentinelHash = Hash $sentinel
$before = @(Get-ChildItem -LiteralPath $invalid -Force -Recurse | ForEach-Object { $_.FullName })
Install $invalid 'missing-config.log' 1
$after = @(Get-ChildItem -LiteralPath $invalid -Force -Recurse | ForEach-Object { $_.FullName })
Assert (@(Compare-Object $before $after).Count -eq 0) 'Rejected destination created or removed files.'
Assert ((Hash $sentinel) -eq $sentinelHash) 'Rejected destination changed existing file.'
Write-Host 'PASS missing server.cfg: rejected without destination changes.'
Write-Host "All FiveM installer checks passed. Fixtures/logs: $run"
# GitHub's PowerShell wrapper propagates LASTEXITCODE from the final child.
# That child intentionally failed in the invalid-folder test above. Success
# here means every assertion passed; terminating failures never reach this line.
exit 0
