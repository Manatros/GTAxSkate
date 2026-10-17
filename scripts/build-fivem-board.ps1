param(
    [string]$CodeWalker = (Join-Path (Split-Path -Parent $PSScriptRoot) 'tools\CodeWalker'),
    [string]$Out = (Join-Path (Split-Path -Parent $PSScriptRoot) 'build\fivem-board'),
    [string]$Python = 'python'
)
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$source = Join-Path $repo 'fivem\gtaxskate_board'
$generated = Join-Path $Out 'source'
$resource = Join-Path $Out 'gtaxskate_board'
$stream = Join-Path $resource 'stream'
foreach ($dll in 'SharpDX.dll','SharpDX.Mathematics.dll','CodeWalker.Core.dll') {
    $path = Join-Path $CodeWalker $dll
    if (-not (Test-Path -LiteralPath $path)) { throw "Missing compiler library: $path" }
    [void][Reflection.Assembly]::LoadFrom($path)
}
& $Python (Join-Path $source 'generate.py') $generated
if ($LASTEXITCODE -ne 0) { throw 'Original board generation failed' }
New-Item -ItemType Directory -Force -Path $stream | Out-Null
$checks = @()
foreach ($name in 'gtaxskate_test_board.ydr','gtaxskate_test_board.ytd','gtaxskate_board.ytyp') {
    $inputPath = Join-Path $generated ($name+'.xml')
    [xml]$doc = [IO.File]::ReadAllText($inputPath)
    $trim = 4
    $format = [CodeWalker.GameFiles.XmlMeta]::GetXMLFormat(($name+'.xml'),[ref]$trim)
    $assets = Join-Path $generated 'gtaxskate_test_board'
    $bytes = [CodeWalker.GameFiles.XmlMeta]::GetData($doc,$format,$assets)
    if (-not $bytes -or $bytes.Length -lt 32) { throw "Compiler failed: $name" }
    $outputPath = Join-Path $stream $name
    [IO.File]::WriteAllBytes($outputPath,$bytes)
    $serializedSize = $bytes.Length
    # Parse the actual compiled binary; fail on lost geometry/textures/archetypes.
    $entry = [CodeWalker.GameFiles.RpfFile]::CreateResourceFileEntry([ref]$bytes,0)
    $entry.Name = $name
    $entry.NameLower = $name
    $expanded = [CodeWalker.GameFiles.ResourceBuilder]::Decompress($bytes)
    $suggested=''
    $roundtrip=[CodeWalker.GameFiles.MetaXml]::GetXml($entry,$expanded,[ref]$suggested,$generated)
    if (-not $roundtrip) { throw "Binary readback failed: $name" }
    [xml]$back=$roundtrip
    if ($name.EndsWith('.ydr') -and @($back.Drawable.DrawableModelsHigh.Item.Geometries.Item).Count -ne 5) { throw 'Geometry readback mismatch' }
    if ($name.EndsWith('.ydr')) {
        $vertexCount = 0
        foreach ($geometry in $back.Drawable.DrawableModelsHigh.Item.Geometries.Item) {
            $vertexCount += @($geometry.VertexBuffer.Data.Trim() -split '[\r\n]+' | Where-Object { $_.Trim() }).Count
        }
        if ($vertexCount -ne 1704) { throw "Vertex readback mismatch: $vertexCount" }
        if ($back.Drawable.Bounds.Children.Item.CompositeFlags1 -notmatch 'MAP_DYNAMIC') { throw 'Collision flags did not survive serialization' }
    }
    if ($name.EndsWith('.ytd') -and @($back.TextureDictionary.Item).Count -ne 5) { throw 'Texture readback mismatch' }
    if ($name.EndsWith('.ytyp') -and @($back.CMapTypes.archetypes.Item).Count -ne 1) { throw 'Archetype readback mismatch' }
    [IO.File]::WriteAllText((Join-Path $generated ($name+'.roundtrip.xml')),$roundtrip)
    $checks += [pscustomobject]@{ File=$name; Bytes=$serializedSize; Sha256=(Get-FileHash -LiteralPath $outputPath -Algorithm SHA256).Hash.ToLowerInvariant(); BinaryReadback=$true }
}
Copy-Item -LiteralPath (Join-Path $source 'fxmanifest.lua') -Destination $resource -Force
Copy-Item -LiteralPath (Join-Path $source 'README.md') -Destination $resource -Force
Copy-Item -LiteralPath (Join-Path $repo 'public\LICENSE') -Destination (Join-Path $resource 'LICENSE') -Force
Copy-Item -LiteralPath (Join-Path $generated 'provenance.json') -Destination $resource -Force
$checks | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $resource 'build-checks.json') -Encoding UTF8
Write-Output "Original board resource compiled and read back: $resource"
