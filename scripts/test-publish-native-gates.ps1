# Windows PowerShell 5.1 regression: production Cargo gate must allow stderr
# warnings with exit 0 and stop on an actual nonzero native exit.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$source = [IO.File]::ReadAllText((Join-Path $PSScriptRoot 'publish-github.ps1'))
$start = $source.IndexOf("    foreach (`$m in 'gta\skate-gta', 'gta\skate-ped', 'gta\skate-music') {")
$end = $source.IndexOf('    Invoke-PublishGit add -A', $start)
if ($start -lt 0 -or $end -lt 0) { throw 'Production Cargo gate not found.' }
$production = [scriptblock]::Create($source.Substring($start, $end - $start))
$qa = Join-Path $root 'build/publish-native-gates'
New-Item -ItemType Directory -Force -Path $qa | Out-Null
$script:nativePath = Join-Path $qa 'warning.cmd'
[IO.File]::WriteAllText($script:nativePath, "@echo off`r`necho warning: synthetic compiler warning 1>&2`r`nexit /b %PUBLISH_FIXTURE_EXIT%`r`n")
$here = $root
$script:attempts = 0
function cargo { $script:attempts++; & $env:ComSpec /d /c $script:nativePath }
function Fail([string]$message) { throw "Fixture gate stopped: $message" }
$savedTarget = $env:CARGO_TARGET_DIR
$savedExit = $env:PUBLISH_FIXTURE_EXIT
try {
    $env:PUBLISH_FIXTURE_EXIT = '0'
    & $production
    if ($script:attempts -ne 3) { throw 'Successful stderr warnings did not allow all three gates.' }
    if ($ErrorActionPreference -ne 'Stop') { throw 'Successful gate changed the caller error preference.' }
    $script:attempts = 0
    $env:PUBLISH_FIXTURE_EXIT = '17'
    $rejected = $false
    try { & $production } catch { $rejected = $_.Exception.Message -like 'Fixture gate stopped:*' }
    if (-not $rejected -or $script:attempts -ne 1) { throw 'A failing native test was not rejected immediately.' }
    if ($ErrorActionPreference -ne 'Stop') { throw 'Failed gate changed the caller error preference.' }
    'PASS: production Cargo gates accept stderr/exit 0 and stop on stderr/exit 17; caller error handling preserved.'
} finally {
    $env:CARGO_TARGET_DIR = $savedTarget
    $env:PUBLISH_FIXTURE_EXIT = $savedExit
    Remove-Item Function:cargo
    Remove-Item Function:Fail
}
