# Skate 3's own board sounds for the GTA V plugin, from your own extracted
# Skate 3 (Xbox 360) game folder: the rolling "grains" for each surface
# (data\audio\grains.big) and the wheels spinning (wheels.big), as WAV files
# in <Mashup>\skate-data\assets\sounds, where SkateGTA.asi finds them.
#
#   powershell -ExecutionPolicy Bypass -File .\get-skate-sounds.ps1 -Game "C:\Games\Skate 3" -Assets "C:\Games\2010-Rust-Rewrite-Mashup\skate-data\assets"
#
# The archives are EA "EB" v3: big-endian, entries at 0x30 (offset / 16, 0,
# size, djb2 of the name, path "data/audio/" sometimes included), names at
# the offset in 0x0C. A sound is EA's audio core header (8 bytes: codec 3 =
# XMA, rate, samples) and SNS blocks, kept in memory by the game; marked as
# streamed, vgmstream reads it as an .snr/.sns pair. A grain file has a
# table of its grains first (its length in its first four bytes).
# audiofiles.big keeps its entries at 64-byte steps (the byte at 0x0A is
# the step's power of two); its sound banks are EA ABK (vgmstream reads
# them) and SPLC patch banks, whose sounds are EA audio core headers and
# SNS blocks one after another. The board's hits (pops, landings per
# material, grinds, slides, steps) go to sounds\hits\<event>\ for the
# plugin. Pops and landings are rendered from Skate's SPLC banks the way
# the game mixes them, picked by Skate's own tables (see below).

param(
    [Parameter(Mandatory)] [string] $Game,
    [Parameter(Mandatory)] [string] $Assets
)
$ErrorActionPreference = 'Stop'
$tools = Join-Path $PSScriptRoot 'tools'
$vgm = Get-ChildItem $tools -Recurse -Filter vgmstream-cli.exe -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName
if (-not $vgm) {
    Write-Host 'vgmstream is missing: run get-skate-music.ps1 once (it fetches it into .\tools).' -ForegroundColor Red
    exit 1
}
if ((Split-Path -Leaf $Game) -ieq 'default.xex') { $Game = Split-Path -Parent $Game }
$audio = Join-Path $Game 'data\audio'
if (-not (Test-Path (Join-Path $audio 'grains.big'))) {
    Write-Host "$audio has no grains.big. Point -Game at your extracted Skate 3 folder." -ForegroundColor Red
    exit 1
}
$out = Join-Path $Assets 'sounds'
$work = Join-Path $env:TEMP 'skate-sounds'
New-Item -ItemType Directory -Force $out, $work | Out-Null

function U32([byte[]]$d, [int]$at) { ([uint32]$d[$at] -shl 24) -bor ([uint32]$d[$at + 1] -shl 16) -bor ([uint32]$d[$at + 2] -shl 8) -bor [uint32]$d[$at + 3] }
function Djb2([string]$s) {
    [uint64]$h = 5381
    foreach ($c in [Text.Encoding]::GetEncoding(28591).GetBytes($s)) { $h = ([uint64]$h * [uint64]33 + [uint64]$c) % [uint64]4294967296 }
    [uint32]$h
}

# The entries of an EB archive: name, offset, size.
function Read-Eb([string]$path) {
    $d = [IO.File]::ReadAllBytes($path)
    if ($d[0] -ne 0x45 -or $d[1] -ne 0x42) { throw "$path is not an EB archive" }
    $count = U32 $d 4
    $step = [int][Math]::Pow(2, $d[0x0A])
    $namesAt = U32 $d 0x0C; $namesLen = U32 $d 0x10
    $byHash = @{}
    $text = [Text.Encoding]::GetEncoding(28591).GetString($d, $namesAt, $namesLen)
    foreach ($n in $text.Split([char]0)) {
        if (-not $n) { continue }
        foreach ($p in '', 'data/audio/') { $byHash[[uint32](Djb2 ($p + $n))] = $n }
    }
    $entries = for ($i = 0; $i -lt $count; $i++) {
        $at = 0x30 + $i * 16
        $hash = [uint32](U32 $d ($at + 12))
        [pscustomobject]@{ Name = $byHash[$hash]; Offset = [int](U32 $d $at) * $step; Size = [int](U32 $d ($at + 8)) }
    }
    [pscustomobject]@{ Data = $d; Entries = $entries }
}

# One sound (EA audio core header + SNS blocks at $at) to WAV.
function Write-Sound([byte[]]$d, [int]$at, [int]$end, [string]$name) {
    $snr = New-Object byte[] 8
    [Array]::Copy($d, $at, $snr, 0, 8)
    $snr[4] = ($snr[4] -band 0x3F) -bor 0x40   # streamed
    $sns = New-Object byte[] ($end - $at - 8)
    [Array]::Copy($d, $at + 8, $sns, 0, $sns.Length)
    $base = Join-Path $work $name
    [IO.File]::WriteAllBytes("$base.snr", $snr)
    [IO.File]::WriteAllBytes("$base.sns", $sns)
    $wav = Join-Path $out "$name.wav"
    & $vgm -o $wav "$base.snr" *> $null
    if (Test-Path $wav) { Write-Host "  $name" } else { Write-Host "  ${name}: could not decode" -ForegroundColor Yellow }
}

Write-Host 'Rolling sounds (grains.big):' -ForegroundColor Cyan
$eb = Read-Eb (Join-Path $audio 'grains.big')
foreach ($e in $eb.Entries | Where-Object { $_.Name -like '*.grain' }) {
    $table = [int](U32 $eb.Data $e.Offset)
    Write-Sound $eb.Data ($e.Offset + $table) ($e.Offset + $e.Size) ($e.Name -replace '\.grain$', '')
}
Write-Host 'Wheels spinning (wheels.big):' -ForegroundColor Cyan
$eb = Read-Eb (Join-Path $audio 'wheels.big')
foreach ($e in $eb.Entries | Where-Object { $_.Name -like '*.snr' }) {
    Write-Sound $eb.Data $e.Offset ($e.Offset + $e.Size) ($e.Name -replace '\.snr$', '')
}
# The board's hits. Grinds, slides and steps are ABK banks (vgmstream's
# 1-based subsongs). Pops and landings come from Skate's SPLC "splicer"
# banks, played the way Skate plays them (see Add-Type below).
$sets = [ordered]@{
    'grind_metal' = @(89, 118, 76, 115, 88, 86, 116, 61, 49, 94, 87)
    'grind_concrete' = @(3, 113, 32, 23, 45, 105, 80, 120, 27, 92, 108, 24)
    'slide_wood' = @(16, 19, 17, 18, 21, 25, 20, 24)
    'slide_concrete' = @(1, 8, 12, 9, 10, 11, 7, 5)
    'step' = @(1..24 | ForEach-Object { $_ * 8 })
}
$sources = @{
    'grind_metal' = 'GRINDS.abk'; 'grind_concrete' = 'GRINDS.abk'; 'slide_wood' = 'board_scrapes.abk'; 'slide_concrete' = 'board_scrapes.abk'
    'step' = 'fstep_skateshoe1_sm.abk'
}
Write-Host 'Board hits (audiofiles.big):' -ForegroundColor Cyan
$eb = Read-Eb (Join-Path $audio 'audiofiles.big')
$hits = Join-Path $out 'hits'
if (Test-Path $hits) { Remove-Item $hits -Recurse -Force }
$banks = @{}
foreach ($e in $eb.Entries) { if ($e.Name) { $banks[$e.Name] = $e } }
function Get-Bank([string]$name) {
    $bank = $banks[$name]
    if (-not $bank) { return $null }
    $bytes = New-Object byte[] $bank.Size
    [Array]::Copy($eb.Data, $bank.Offset, $bytes, 0, $bank.Size)
    ,$bytes
}
foreach ($event in $sets.Keys) {
    $dir = Join-Path $hits $event
    New-Item -ItemType Directory -Force $dir | Out-Null
    $bytes = Get-Bank $sources[$event]
    if (-not $bytes) { Write-Host "  ${event}: $($sources[$event]) not found" -ForegroundColor Yellow; continue }
    $file = Join-Path $work $sources[$event]
    [IO.File]::WriteAllBytes($file, $bytes)
    foreach ($n in $sets[$event]) { & $vgm -s $n -o (Join-Path $dir ('{0:d4}.wav' -f $n)) $file *> $null }
    Write-Host ("  {0,-26} {1} sounds" -f $event, @(Get-ChildItem $dir -Filter *.wav).Count)
}

# Skate's SPLC banks (Skate_Collisions.bnk, Skate_Metal.bnk, ...), big-endian:
#   header: 'SPLC', version 3, audio offset (0x08), patches (0x0C),
#           groups (0x10), recordings (0x18), name (0x1C).
#   patches from 0x38, 36 bytes: 8 zero bytes, u16 id, u16 layer count,
#           gain, pitch, ?, ?, duration (ms).
#   groups after them, 72 bytes: type, mode, ..., u16 patch ids from +8
#           (0xFFFF ends): a random choice among patches.
#   layers after 8 more bytes, the patches' in order: u32 0x10000, u8 count,
#           u8 mode (1: all entries at once, 2: one at random, 0: in turn),
#           count entries of 72 bytes and 4 zero bytes. An entry: u16
#           recording, then floats gain, pitch, ?, ?, delay, start in the
#           recording, length (s, as played), fade in, fade out, ...
#   recordings: EA audio core headers (XMA, 48 kHz) and SNS blocks.
# Skate's attribute database (db.big, skatercollections.vlt/.bin) names a
# sound by a number in its bank: below the patch count a patch, above it a
# group (number - patches). Each played sound is a mix of cut recordings.
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
public class SkateSplicer {
    byte[] d;
    public int Patches, Groups, Recordings, Audio;
    List<int> layerCount = new List<int>(), patchLayer = new List<int>();
    List<float> patchGain = new List<float>(), patchPitch = new List<float>();
    List<int[]> groups = new List<int[]>();
    class Entry { public int Rec; public float Gain, Pitch, Delay, Start, Length, FadeIn, FadeOut; }
    class Layer { public int Mode; public List<Entry> Entries = new List<Entry>(); }
    List<Layer> layers = new List<Layer>();
    public List<int> Starts = new List<int>();
    static int U16(byte[] d, int o) { return (d[o] << 8) | d[o + 1]; }
    static int U32(byte[] d, int o) { return (d[o] << 24) | (d[o + 1] << 16) | (d[o + 2] << 8) | d[o + 3]; }
    static float F(byte[] d, int o) { return BitConverter.ToSingle(new byte[] { d[o + 3], d[o + 2], d[o + 1], d[o] }, 0); }
    public SkateSplicer(byte[] bank) {
        d = bank;
        Audio = U32(d, 8); Patches = U32(d, 12); Groups = U32(d, 16); Recordings = U32(d, 24);
        int total = 0;
        for (int i = 0; i < Patches; i++) {
            int o = 0x38 + 36 * i;
            int n = U16(d, o + 10);
            patchLayer.Add(total); layerCount.Add(n); total += n;
            patchGain.Add(F(d, o + 12)); patchPitch.Add(F(d, o + 16));
        }
        int g = 0x38 + 36 * Patches;
        for (int i = 0; i < Groups; i++) {
            var ids = new List<int>();
            for (int k = 0; k < 32; k++) { int v = U16(d, g + 72 * i + 8 + 2 * k); if (v == 0xFFFF) break; ids.Add(v); }
            groups.Add(ids.ToArray());
        }
        int at = g + 72 * Groups + 8;
        for (int i = 0; i < total; i++) {
            var layer = new Layer();
            int count = d[at + 4]; layer.Mode = d[at + 5];
            for (int k = 0; k < count; k++) {
                int e = at + 8 + 72 * k;
                layer.Entries.Add(new Entry { Rec = U16(d, e), Gain = F(d, e + 4), Pitch = F(d, e + 8), Delay = F(d, e + 20),
                    Start = F(d, e + 24), Length = F(d, e + 28), FadeIn = F(d, e + 32), FadeOut = F(d, e + 36) });
            }
            layers.Add(layer);
            at += 12 + 72 * count;
        }
        for (int o = Audio; o + 4 <= d.Length; o++)
            if (d[o] == 3 && d[o + 1] == 0 && d[o + 2] == 0xBB && d[o + 3] == 0x80) Starts.Add(o);
    }
    // The patches a sound number may play.
    public int[] PatchesOf(int sound) {
        if (sound < Patches) return new[] { sound };
        int g = sound - Patches;
        return g < groups.Count ? groups[g] : new int[0];
    }
    // The recordings a patch uses.
    public int[] RecordingsOf(int patch) {
        var all = new List<int>();
        for (int l = 0; l < layerCount[patch]; l++) foreach (var e in layers[patchLayer[patch] + l].Entries) all.Add(e.Rec);
        return all.ToArray();
    }
    // How many takes a patch has (its random layers' choices, at most 4).
    public int Takes(int patch) {
        int n = 1;
        for (int l = 0; l < layerCount[patch]; l++) { var L = layers[patchLayer[patch] + l]; if (L.Mode != 1) n = Math.Max(n, L.Entries.Count); }
        return Math.Min(n, 4);
    }
    // One take of a patch, 48 kHz mono: every layer's entry (or all of
    // them), cut, faded, pitched and delayed, mixed.
    public float[] Render(int patch, int take, Dictionary<int, float[]> recs, float volume) {
        var pieces = new List<Entry>();
        for (int l = 0; l < layerCount[patch]; l++) {
            var L = layers[patchLayer[patch] + l];
            if (L.Entries.Count == 0) continue;
            if (L.Mode == 1) pieces.AddRange(L.Entries);
            else pieces.Add(L.Entries[(take + l) % L.Entries.Count]);
        }
        float pp = patchPitch[patch] > 0.01f ? patchPitch[patch] : 1f;
        float pg = patchGain[patch] > 0f ? patchGain[patch] : 1f;
        int len = 1;
        foreach (var e in pieces) len = Math.Max(len, (int)((e.Delay + e.Length) * 48000f / pp) + 1);
        var mix = new float[len];
        foreach (var e in pieces) {
            float[] src; if (!recs.TryGetValue(e.Rec, out src) || src.Length == 0) continue;
            float step = (e.Pitch > 0.01f ? e.Pitch : 1f) * pp;
            int n = (int)(e.Length * 48000f / pp);
            int at0 = (int)(e.Delay * 48000f / pp);
            double pos = e.Start * 48000.0;
            int fin = (int)(e.FadeIn * 48000f), fout = (int)(e.FadeOut * 48000f);
            for (int i = 0; i < n && at0 + i < len; i++, pos += step) {
                int p = (int)pos; if (p + 1 >= src.Length) break;
                float s = src[p] + (src[p + 1] - src[p]) * (float)(pos - p);
                float env = 1f;
                if (fin > 0 && i < fin) env = (float)i / fin;
                if (fout > 0 && i > n - fout) env = Math.Min(env, (float)(n - i) / fout);
                mix[at0 + i] += s * env * e.Gain * pg * volume;
            }
        }
        return mix;
    }
    public static void WriteWav(string path, float[] s) {
        using (var w = new System.IO.BinaryWriter(System.IO.File.Create(path))) {
            w.Write(new[] { 'R', 'I', 'F', 'F' }); w.Write(36 + s.Length * 2); w.Write(new[] { 'W', 'A', 'V', 'E', 'f', 'm', 't', ' ' });
            w.Write(16); w.Write((short)1); w.Write((short)1); w.Write(48000); w.Write(96000); w.Write((short)2); w.Write((short)16);
            w.Write(new[] { 'd', 'a', 't', 'a' }); w.Write(s.Length * 2);
            foreach (var x in s) w.Write((short)Math.Max(-32768, Math.Min(32767, (int)(x * 32767f))));
        }
    }
    public static float[] ReadWav(string path) {
        var b = System.IO.File.ReadAllBytes(path);
        int o = 12;
        while (o + 8 <= b.Length) {
            string id = System.Text.Encoding.ASCII.GetString(b, o, 4); int size = BitConverter.ToInt32(b, o + 4);
            if (id == "data") { var s = new float[size / 2]; for (int i = 0; i < s.Length; i++) s[i] = BitConverter.ToInt16(b, o + 8 + 2 * i) / 32768f; return s; }
            o += 8 + size + (size & 1);
        }
        return new float[0];
    }
}
'@
# One recording of an SPLC bank to samples (vgmstream, as an .snr/.sns pair).
function Get-Recording([byte[]]$bytes, $splicer, [int]$n, [string]$tag) {
    if ($n -ge $splicer.Starts.Count) { return ,(New-Object float[] 0) }
    $at = $splicer.Starts[$n]; $end = if ($n + 1 -lt $splicer.Starts.Count) { $splicer.Starts[$n + 1] } else { $bytes.Length }
    $want = (U32 $bytes ($at + 4)) -band 0x1FFFFFFF
    $pos = $at + 8; $got = 0; $last = -1
    $sns = New-Object System.Collections.Generic.List[byte]
    while ($pos + 8 -le $end -and $got -lt $want) {
        $size = ((U32 $bytes $pos) -band 0xFFFFFF)
        if ($size -lt 8 -or $pos + $size -gt $end) { break }
        $last = $sns.Count
        $chunk = New-Object byte[] $size; [Array]::Copy($bytes, $pos, $chunk, 0, $size); $sns.AddRange($chunk)
        $got += (U32 $bytes ($pos + 4)); $pos += $size
    }
    if ($last -lt 0) { return ,(New-Object float[] 0) }
    $sns[$last] = 0x80
    $snr = New-Object byte[] 8; [Array]::Copy($bytes, $at, $snr, 0, 8); $snr[4] = ($snr[4] -band 0x3F) -bor 0x40
    $base = Join-Path $work ('{0}{1:d4}' -f $tag, $n)
    [IO.File]::WriteAllBytes("$base.snr", $snr); [IO.File]::WriteAllBytes("$base.sns", $sns.ToArray())
    & $vgm -o "$base.wav" "$base.snr" *> $null
    if (-not (Test-Path "$base.wav")) { return ,(New-Object float[] 0) }
    ,[SkateSplicer]::ReadWav("$base.wav")
}
# Writes every take of the sound numbers in $sounds (a group: each of its
# patches) to hits\<event>, at $volume.
function Write-Spliced([string]$bankName, [string]$event, [int[]]$sounds, [float]$volume) {
    if (-not $script:spliced.ContainsKey($bankName)) {
        $bytes = Get-Bank $bankName
        if (-not $bytes) { Write-Host "  $bankName not found" -ForegroundColor Yellow; $script:spliced[$bankName] = $null; return }
        $script:spliced[$bankName] = @{ Bytes = $bytes; Splicer = [SkateSplicer]::new($bytes); Recs = New-Object 'System.Collections.Generic.Dictionary[int,float[]]' }
    }
    $b = $script:spliced[$bankName]
    if (-not $b) { return }
    $dir = Join-Path $hits $event
    New-Item -ItemType Directory -Force $dir | Out-Null
    $count = 0
    foreach ($sound in $sounds) {
        foreach ($patch in $b.Splicer.PatchesOf($sound)) {
            foreach ($r in $b.Splicer.RecordingsOf($patch)) {
                if (-not $b.Recs.ContainsKey($r)) { $b.Recs[$r] = Get-Recording $b.Bytes $b.Splicer $r ($bankName -replace '\W', '') }
            }
            for ($take = 0; $take -lt $b.Splicer.Takes($patch); $take++) {
                $mix = $b.Splicer.Render($patch, $take, $b.Recs, $volume)
                if ($mix.Length -gt 16) { [SkateSplicer]::WriteWav((Join-Path $dir ('{0:d4}_{1}.wav' -f $patch, $take)), $mix); $count++ }
            }
        }
    }
    Write-Host ("  {0,-26} {1} takes" -f $event, $count)
}
$script:spliced = @{}

# Skate's own tables (aud_contacts/default in db.big; see CLAUDE.md, item 35):
# landings by surface kind (wood or not) and hard or soft, 13 sounds each
# at index kind*3 + strength: kind 0 all four wheels down, 1 two wheels
# (one truck), 2 the second truck after it, 3 one wheel; strength 0-2;
# index 12 a lone extra. Volumes per index (Land_*_Volume, 15 each).
$land = [ordered]@{
    'concrete_hard' = @{ Sounds = @(1051, 1052, 1053, 1054, 1055, 1056, 1057, 1058, 1059, 1060, 1061, 1127, 1050); Volume = @(0.84, 1.0, 0.89, 1.0, 0.78, 0.76, 1.0, 0.7, 0.79, 0.82, 0.88, 0.76, 0.26, 0.38, 0.47) }
    'concrete_soft' = @{ Sounds = @(1073, 1074, 1075, 1076, 1077, 1078, 1079, 1080, 1081, 1082, 1128, 1129, 1130); Volume = @(1.0, 1.0, 0.72, 0.77, 0.98, 0.8, 0.9, 0.98, 0.73, 1.0, 0.88, 0.83, 0.88, 0.55, 0.4) }
    'wood_hard' = @{ Sounds = @(1062, 1063, 1064, 1065, 1066, 1067, 1068, 1069, 1070, 1071, 1131, 1132, 1072); Volume = @(0.65, 1.0, 0.94, 0.59, 1.0, 0.84, 0.81, 0.97, 0.73, 0.64, 0.68, 0.63, 0.42, 0.42, 0.01) }
    'wood_soft' = @{ Sounds = @(1084, 1085, 1086, 1087, 1088, 1089, 1090, 1091, 1092, 1093, 1133, 1134, 1094); Volume = @(0.73, 0.99, 0.56, 0.79, 0.99, 0.59, 0.74, 0.91, 0.55, 0.58, 0.61, 0.59, 0.4, 0.4, 0.4) }
}
foreach ($kind in $land.Keys) {
    $t = $land[$kind]
    for ($i = 0; $i -lt 13; $i++) {
        $name = if ($i -eq 12) { "land_${kind}_extra" } else { 'land_{0}_{1}_{2}' -f $kind, [math]::Floor($i / 3), ($i % 3) }
        Write-Spliced 'Skate_Collisions.bnk' $name @($t.Sounds[$i]) $t.Volume[$i]
    }
}
# Pops (ollies) by strength 0-2: concrete with hard landings' set, and the
# other (Ollie_Concrete_Volume / Ollie_Wood_Volume).
$popHard = @(1097, 1098, 1099); $popHardVolume = @(0.5, 0.75, 1.0)
$popOther = @(1103, 1104, 1105); $popOtherVolume = @(0.58, 0.76, 0.99)
for ($i = 0; $i -lt 3; $i++) {
    Write-Spliced 'Skate_Collisions.bnk' "pop_hard_$i" @($popHard[$i]) $popHardVolume[$i]
    Write-Spliced 'Skate_Collisions.bnk' "pop_other_$i" @($popOther[$i]) $popOtherVolume[$i]
}
# Metal adds its own ring to landings (aud_material/metal: Add_To_Land, its
# sounds in Skate_Metal.bnk).
Write-Spliced 'Skate_Metal.bnk' 'land_metal' @(402, 403) 0.63
Remove-Item $work -Recurse -Force
Write-Host "Done: $out" -ForegroundColor Green
