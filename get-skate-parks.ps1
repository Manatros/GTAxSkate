# Exports GTA V's skate spots as Skate collision: the exact collision
# triangles (the game's own .ybn files and the skate ramp props' bounds where
# the map places them) round each spot, into
# <Mashup>\skate-data\assets\parks\<id>.tri for the SkateGTA plugin.
# Reads GTA's archives only (CodeWalker); writes nothing into GTA's folder.
param(
    [string]$Gta,
    [string]$CodeWalker,
    [string]$Out
)
$ErrorActionPreference = 'Stop'
foreach ($dll in 'SharpDX.dll', 'SharpDX.Mathematics.dll', 'CodeWalker.Core.dll') {
    $p = Join-Path $CodeWalker $dll
    try { [void][Reflection.Assembly]::LoadFrom($p) } catch { [void][Reflection.Assembly]::UnsafeLoadFrom($p) }
}
$load = [CodeWalker.GameFiles.GTA5Keys].GetMethod('LoadFromPath')
$a = New-Object 'System.Collections.Generic.List[object]'
foreach ($p in $load.GetParameters()) { switch ($p.Name) { 'path' { $a.Add([string]$Gta) } 'gen9' { $a.Add($false) } 'key' { $a.Add($null) } default { $a.Add($p.DefaultValue) } } }
[void]$load.Invoke($null, $a.ToArray())

# The spots (same ids as the plugin's parks.rs): centre and radius, metres.
$parks = @(
    @{ Id = 'vespucci';   X = -1374; Y = -1396; R = 60 },
    @{ Id = 'hawick';     X = 280;   Y = -206;  R = 30 },
    @{ Id = 'littleseoul'; X = -950; Y = -786;  R = 30 },
    @{ Id = 'strawberry'; X = -266;  Y = -1571; R = 30 },
    @{ Id = 'lamesa';     X = 718;   Y = -1231; R = 30 }
)
foreach ($park in $parks) { $park.Tris = New-Object 'System.Collections.Generic.List[single]'; $park.Files = @() }

function All($r) { $r; if ($r.Children) { foreach ($x in $r.Children) { All $x } } }
function Near($park, $min, $max) {
    $park.X + $park.R -ge $min.X -and $park.X - $park.R -le $max.X -and $park.Y + $park.R -ge $min.Y -and $park.Y - $park.R -le $max.Y
}

# The triangles of bounds `b` (and its children), through `place` (a
# SharpDX matrix), kept when within a park's circle.
function Add-Bounds($b, $place, $label) {
    if ($null -eq $b) { return }
    if ($b -is [CodeWalker.GameFiles.BoundComposite]) {
        foreach ($child in $b.Children.data_items) { if ($child) { Add-Bounds $child ($child.Transform * $place) $label } }
        return
    }
    if (-not ($b -is [CodeWalker.GameFiles.BoundGeometry])) { return }
    foreach ($poly in $b.Polygons) {
        $corners = $null
        if ($poly -is [CodeWalker.GameFiles.BoundPolygonTriangle]) {
            $tris = , @($poly.Vertex1, $poly.Vertex2, $poly.Vertex3)
        } elseif ($poly -is [CodeWalker.GameFiles.BoundPolygonBox]) {
            # Four alternate corners; the other four mirror them through the centre.
            $v = @($poly.Vertex1, $poly.Vertex2, $poly.Vertex3, $poly.Vertex4)
            $c = ($v[0] + $v[1] + $v[2] + $v[3]) / 4
            $o = $v | ForEach-Object { $c * 2 - $_ }
            # Faces: each pair of given corners with the mirrored ones between them.
            $tris = @()
            foreach ($i in 0..3) { foreach ($j in ($i + 1)..3) { if ($j -le 3) {
                $k = @(0..3 | Where-Object { $_ -ne $i -and $_ -ne $j })
                $tris += , @($v[$i], $o[$k[0]], $v[$j]); $tris += , @($v[$i], $v[$j], $o[$k[1]])
            } } }
        } else { continue }
        foreach ($t in $tris) {
            $w = $t | ForEach-Object { [SharpDX.Vector3]::TransformCoordinate($_, $place) }
            # Kept when it reaches into the circle (its box's nearest point),
            # whole: no gaps at the edge.
            $x0 = [Math]::Min([Math]::Min($w[0].X, $w[1].X), $w[2].X); $x1 = [Math]::Max([Math]::Max($w[0].X, $w[1].X), $w[2].X)
            $y0 = [Math]::Min([Math]::Min($w[0].Y, $w[1].Y), $w[2].Y); $y1 = [Math]::Max([Math]::Max($w[0].Y, $w[1].Y), $w[2].Y)
            foreach ($park in $parks) {
                $nx = [Math]::Max($x0, [Math]::Min($park.X, $x1)); $ny = [Math]::Max($y0, [Math]::Min($park.Y, $y1))
                if ([Math]::Sqrt(($nx - $park.X) * ($nx - $park.X) + ($ny - $park.Y) * ($ny - $park.Y)) -le $park.R) {
                    foreach ($p in $w) { $park.Tris.Add($p.X); $park.Tris.Add($p.Y); $park.Tris.Add($p.Z) }
                    if ($park.Files -notcontains $label) { $park.Files += $label }
                }
            }
        }
    }
}

$props = @{}
foreach ($n in 'prop_skate_flatramp', 'prop_skate_funbox', 'prop_skate_halfpipe', 'prop_skate_kickers', 'prop_skate_quartpipe', 'prop_skate_rail', 'prop_skate_spiner') {
    $props[[CodeWalker.GameFiles.JenkHash]::GenHash($n)] = $n
}
$propBounds = @{}
$placed = @()
$ybns = 0
foreach ($f in Get-ChildItem $Gta -Filter 'x64*.rpf') {
    $rpf = New-Object CodeWalker.GameFiles.RpfFile($f.FullName, $f.Name)
    $rpf.ScanStructure([Action[string]]{ param($m) }, [Action[string]]{ param($m) })
    foreach ($r in All $rpf) {
        foreach ($e in $r.AllEntries) {
            $name = $e.NameLower
            if ($name.EndsWith('.ybn')) {
                # The high-detail and the pedestrians' (mover) copies duplicate the plain ones.
                if ($name.StartsWith('hi@') -or $name.StartsWith('ma@')) { continue }
                $ybns++
                try { $y = New-Object CodeWalker.GameFiles.YbnFile; $y.Load($e.File.ExtractFile($e), $e) } catch { continue }
                $b = $y.Bounds
                if (-not $b) { continue }
                if (-not ($parks | Where-Object { Near $_ $b.BoxMin $b.BoxMax })) { continue }
                Add-Bounds $b ([SharpDX.Matrix]::Identity) $e.Path
            } elseif ($name.EndsWith('.ydr') -and $name.StartsWith('prop_skate_') -and -not $name.Contains('_cr')) {
                $d = New-Object CodeWalker.GameFiles.YdrFile; $d.Load($e.File.ExtractFile($e), $e)
                $propBounds[[CodeWalker.GameFiles.JenkHash]::GenHash($name.Replace('.ydr', ''))] = $d.Drawable.Bound
            } elseif ($name.EndsWith('.ymap')) {
                try { $m = New-Object CodeWalker.GameFiles.YmapFile; $m.Load($e.File.ExtractFile($e), $e) } catch { continue }
                foreach ($ent in @($m.AllEntities)) {
                    if ($ent -and $props.ContainsKey($ent.CEntityDef.archetypeName.Hash)) { $placed += $ent }
                }
            }
        }
    }
}
foreach ($ent in $placed) {
    $b = $propBounds[$ent.CEntityDef.archetypeName.Hash]
    if (-not $b) { continue }
    $place = [SharpDX.Matrix]::RotationQuaternion($ent.Orientation) * [SharpDX.Matrix]::Translation($ent.Position)
    Add-Bounds $b $place $props[$ent.CEntityDef.archetypeName.Hash]
}

New-Item -ItemType Directory -Force -Path $Out | Out-Null
foreach ($park in $parks) {
    $count = $park.Tris.Count / 9
    $bytes = New-Object byte[] (8 + $park.Tris.Count * 4)
    [Text.Encoding]::ASCII.GetBytes('SKPK').CopyTo($bytes, 0)
    [BitConverter]::GetBytes([uint32]$count).CopyTo($bytes, 4)
    [Buffer]::BlockCopy($park.Tris.ToArray(), 0, $bytes, 8, $park.Tris.Count * 4)
    [IO.File]::WriteAllBytes((Join-Path $Out "$($park.Id).tri"), $bytes)
    Write-Host ("{0,-12} {1,6} triangles from {2}" -f $park.Id, $count, ($park.Files -join ', '))
}
Write-Host "collision files read: $ybns"
