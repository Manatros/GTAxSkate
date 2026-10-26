$ErrorActionPreference = 'Stop'
$root = Join-Path (Split-Path -Parent $PSScriptRoot) ('build\map-install-tests-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
$builder = Join-Path $PSScriptRoot 'build-prebaked-map.ps1'
$gta = Join-Path $root 'fake-game'
$world = Join-Path $root 'world'
New-Item -ItemType Directory -Force -Path (Join-Path $gta 'update\x64\dlcpacks'), $world | Out-Null
[IO.File]::WriteAllText((Join-Path $gta 'update\update.rpf'), 'fixture archive; never opened')
$rpf = Join-Path $root 'extract.ps1'
$baker = Join-Path $root 'bake.ps1'
$map = Join-Path $world 'sk8v-map.svwc'
$extract = @'
$i = [array]::IndexOf($args, '-o')
$dest = $args[$i+1]
New-Item -ItemType Directory -Force -Path $dest | Out-Null
[IO.File]::WriteAllText((Join-Path $dest 'fixture.ybn'), 'fixture geometry')
exit 0
'@
[IO.File]::WriteAllText($rpf, $extract)
$validBaker = @'
$f=[IO.File]::Create($args[0]); $w=New-Object IO.BinaryWriter $f
try {
 $w.Write([Text.Encoding]::ASCII.GetBytes('SVWC')); $w.Write([uint32]3); $w.Write([single]64)
 $w.Write([uint32]1); $w.Write([uint32]1)
 $w.Write([int32]0); $w.Write([int32]0); $w.Write([uint32]0); $w.Write([uint32]1)
 $w.Write([uint64]0); $w.Write([uint32]0)
 # One raw triangle: three f32 vertices, then material/flags (40 bytes).
 foreach ($v in @([single]0,[single]0,[single]0,[single]1,[single]0,[single]0,[single]0,[single]1,[single]0)) { $w.Write($v) }
 $w.Write([uint16]0); $w.Write([uint16]0)
} finally { $w.Dispose() }
exit 0
'@
function Check([string]$name, [string]$extractCode, [string]$bakeCode, [bool]$success) {
    [IO.File]::WriteAllText($map, 'previous good map')
    [IO.File]::WriteAllText($rpf, $extractCode)
    [IO.File]::WriteAllText($baker, $bakeCode)
    $failed = $false
    try { & $builder -Gta $gta -Out (Join-Path $root 'extract') -Rpf $rpf -Keys $root -Baker $baker -World $world | Out-Null }
    catch { $failed = $true }
    if ($failed -eq $success) { throw "Unexpected outcome: $name" }
    if (-not $success -and [IO.File]::ReadAllText($map) -ne 'previous good map') { throw "Good map replaced on failure: $name" }
    if ($success -and @(Get-ChildItem -LiteralPath $world -Filter '*.backup-*').Count -eq 0) { throw 'Previous map backup missing' }
    Write-Host "PASS $name"
}
Check 'extraction failure preserves map' 'exit 7' $validBaker $false
Check 'empty output preserves map' $extract '[IO.File]::WriteAllText($args[0], "bad"); exit 0' $false
Check 'truncated index preserves map' $extract ($validBaker.Replace('exit 0', '$f=[IO.File]::OpenWrite($args[0]); $f.SetLength(40); $f.Dispose(); exit 0')) $false
Check 'truncated raw body preserves map' $extract ($validBaker.Replace('exit 0', '$f=[IO.File]::OpenWrite($args[0]); $f.SetLength(87); $f.Dispose(); exit 0')) $false
Check 'invalid tile range preserves map' $extract ($validBaker.Replace('$w.Write([uint32]0); $w.Write([uint32]1)', '$w.Write([uint32]1); $w.Write([uint32]1)')) $false
Check 'out of range block offset preserves map' $extract ($validBaker.Replace('$w.Write([uint64]0)', '$w.Write([uint64]::MaxValue)')) $false
Check 'truncated packed body preserves map' $extract ($validBaker.Replace('$w.Write([uint64]0); $w.Write([uint32]0)', '$w.Write([uint64]0); $w.Write([uint32]41)')) $false
Check 'valid header promotes with backup' $extract $validBaker $true
Write-Host 'Header/index/raw-record promotion fixtures only; real archive decoding and zstd decoding are not exercised.'
