# Builds SK8V's prebaked map, the fallback for holes in GTA's live collision
# (runtime baked.rs): every map collision file (.ybn) from GTA V's archives,
# merged in GTA's load order (base x64*.rpf, then update\update.rpf replacing
# same-named files, then the DLC packs), baked by skatev-world-cache into
# <World>\sk8v-map.svwc (about 0.5 GB; a few minutes). Reads the game only.
# The installer (step 'world') passes every path. By hand, with the data in <GTA>\GTAxSkate:
#   build-prebaked-map.ps1 -Gta <GTA> -Out <GTA>\GTAxSkate\gta\collision -Rpf <SK8V release>\tools\bin\sk8v-rpf.exe `
#       -Keys <GTA>\GTAxSkate\gta\gta-keys -Baker <bin>\skatev-world-cache.exe -World <GTA>\GTAxSkate\world
param(
    [Parameter(Mandatory)][string]$Gta,      # GTA V Legacy folder (GTA5.exe)
    [Parameter(Mandatory)][string]$Out,      # temporary folder for the extracted .ybn (about 2 GB)
    [Parameter(Mandatory)][string]$Rpf,      # sk8v-rpf.exe (SK8V release tools\bin)
    [Parameter(Mandatory)][string]$Keys,     # GTA archive keys (rage extract-keys)
    [Parameter(Mandatory)][string]$Baker,    # skatev-world-cache.exe (GTAxSkate bin)
    [Parameter(Mandatory)][string]$World     # SK8V's world folder (holds sk8v.svwc)
)
$ErrorActionPreference = 'Stop'
$Out = [IO.Path]::GetFullPath($Out)
# A fresh extraction never reuses stale files from a previous failed archive.
$run = Join-Path $Out ('map-' + [guid]::NewGuid().ToString('N'))
$merged = Join-Path $run 'merged'
$stage = Join-Path $run 'stage'
function Clear-Stage {
    $resolved = [IO.Path]::GetFullPath($stage)
    if (-not $resolved.StartsWith($Out.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase) -or
        ((Test-Path -LiteralPath $resolved) -and ((Get-Item -LiteralPath $resolved).Attributes -band [IO.FileAttributes]::ReparsePoint))) { throw 'Unsafe extraction staging folder.' }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
New-Item -ItemType Directory -Force -Path $merged | Out-Null
$archives = @()
$archives += Get-ChildItem -LiteralPath $Gta -Filter 'x64*.rpf' | Sort-Object Name | ForEach-Object FullName
$archives += Join-Path $Gta 'update\update.rpf'
$archives += Get-ChildItem -LiteralPath (Join-Path $Gta 'update\x64\dlcpacks') -Directory | Sort-Object Name |
    ForEach-Object { Join-Path $_.FullName 'dlc.rpf' } | Where-Object { Test-Path -LiteralPath $_ }
$n = 0
foreach ($a in $archives) {
    $n++
    Clear-Stage
    $log = & $Rpf --no-update-check --keys $Keys extract $a '*.ybn' -o $stage --recursive 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) { throw "sk8v-rpf extract failed on $a ($LASTEXITCODE): $log" }
    $files = if (Test-Path -LiteralPath $stage) { @(Get-ChildItem -LiteralPath $stage -Recurse -File -Filter *.ybn) } else { @() }
    # Later archives replace same-named files, as the game's mount order does.
    foreach ($f in $files) { Copy-Item -LiteralPath $f.FullName -Destination (Join-Path $merged $f.Name) -Force }
    "[$n/$($archives.Count)] $(Split-Path -Leaf (Split-Path -Parent $a))\$(Split-Path -Leaf $a): $($files.Count) ybn"
}
Clear-Stage
$all = @(Get-ChildItem -LiteralPath $merged -File)
if ($all.Count -eq 0) { throw "No .ybn files were extracted from $Gta" }
"merged: $($all.Count) ybn, $([math]::Round(($all | Measure-Object Length -Sum).Sum / 1MB)) MB"
# Bake (SK8V's skatev-world-cache: ped-facing bounds only, hi@/ma@ copies left
# out) beside the world cache, where the runtime looks for `<cache>-map.svwc`.
$map = Join-Path $World 'sk8v-map.svwc'
New-Item -ItemType Directory -Path $World -Force | Out-Null
$candidate = Join-Path $World ('sk8v-map.' + [guid]::NewGuid().ToString('N') + '.new')
& $Baker $candidate $merged
if ($LASTEXITCODE -ne 0) { throw "skatev-world-cache failed ($LASTEXITCODE)" }
# Reject truncated/empty output before replacing the previous map. This checks
# the SVWC header/index bounds; the baker performs its own geometry decoding.
$stream = [IO.File]::OpenRead($candidate)
$reader = New-Object IO.BinaryReader $stream
try {
    if ($stream.Length -lt 20 -or [Text.Encoding]::ASCII.GetString($reader.ReadBytes(4)) -ne 'SVWC') { throw 'Invalid baked map header.' }
    $version = $reader.ReadUInt32(); $tileSize = $reader.ReadSingle()
    $tiles = $reader.ReadUInt32(); $records = $reader.ReadUInt32()
    if ($version -ne 3 -or $tileSize -le 0 -or [single]::IsNaN($tileSize) -or [single]::IsInfinity($tileSize) -or
        $tiles -eq 0 -or $records -eq 0 -or $stream.Length -lt (20L + 28L * $tiles)) { throw 'Empty, truncated or unsupported baked map.' }
    # SVWC3: 16-byte tile table entries, then 12-byte block index entries.
    # Each block offset is relative to the byte immediately after the index.
    # packed=0 is the runtime's supported raw-record form (40 bytes per tri).
    $counts = New-Object 'System.Collections.Generic.List[uint32]'
    for ($i = 0L; $i -lt $tiles; $i++) {
        $null = $reader.ReadInt32(); $null = $reader.ReadInt32()
        $first = $reader.ReadUInt32(); $count = $reader.ReadUInt32()
        if ($count -eq 0 -or ([uint64]$first + [uint64]$count) -gt [uint64]$records) { throw 'Baked map tile range exceeds record count.' }
        $counts.Add($count)
    }
    $available = [uint64]($stream.Length - (20L + 28L * $tiles))
    for ($i = 0L; $i -lt $tiles; $i++) {
        $offset = $reader.ReadUInt64(); $packed = $reader.ReadUInt32()
        $bytes = if ($packed) { [uint64]$packed } else { [uint64]$counts[[int]$i] * 40L }
        # Subtraction after the offset check avoids overflowing uint64 on a
        # malformed index; no allocation or decompression is needed here.
        if ($offset -gt $available -or $bytes -gt ($available - $offset)) { throw 'Baked map block extends beyond the file.' }
    }
} finally { $reader.Dispose() }
if (Get-Process GTA5 -ErrorAction SilentlyContinue) { throw "Close GTA first; the new map is $candidate" }
if (Test-Path -LiteralPath $map) {
    [IO.File]::Replace($candidate, $map, ($map + '.backup-' + [guid]::NewGuid().ToString('N')), $true)
} else { [IO.File]::Move($candidate, $map) }
"prebaked map: $map ($([math]::Round((Get-Item -LiteralPath $map).Length / 1MB)) MB)"
