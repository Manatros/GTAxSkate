# Packs your Skate 3 skater (made by make-skate-skater.ps1) into GTA V as an
# add-on ped, as IMPORT.txt describes, with CodeWalker's own library, and
# the drawn board's textures as a pack of their own (so the board looks
# right even with the skater pack off):
#
#   mods\update\x64\dlcpacks\skate3\dlc.rpf
#     setup2.xml, content.xml, common\data\peds.meta
#     x64\models\cdimages\skate3.rpf
#       skate3_skater.ymt, skate3_skater.yft (GTA's mp_m_freemode_01.yft)
#       skate3_skater\  every .ydd and .ytd
#   mods\update\x64\dlcpacks\skate3boards\dlc.rpf
#     setup2.xml, content.xml
#     x64\models\cdimages\skate3boards.rpf   skate3_board_*.ytd
#   mods\update\update.rpf\common\data\dlclist.xml  + both packs
#   SkateGTA-skater.json beside SkateGTA.asi
#
#   powershell -ExecutionPolicy Bypass -File .\pack-skate-skater.ps1 -Gta <GTA V folder> -CodeWalker <CodeWalker folder>
#
# Two smaller jobs with the same library:
#   -Mode skeleton -Out <file>  GTA's own mp_m_freemode_01.yft as CodeWalker
#                               XML (the skeleton the skater is built on)
#   -Mode dlclist               only the packs' lines in dlclist.xml
#   -Mode boards                only the board textures' pack
#   -Mode weapon -Weapon <dir>  the board as GTA weapon WEAPON_SKATEBOARD (a melee
#                               weapon cloned from the bat, its model and textures
#                               from skate-ped --weapon in <dir>), in its own pack
#                               skate3weapon
#   -Mode reference -Out <dir>  GTA's own ped variation files (Michael's,
#                               the freemode ped's) as XML, beside your
#                               skater's, to compare what GTA loads
#
# Close GTA V, OpenIV and CodeWalker first. A dlc.rpf already there is moved
# to _old beside this script; dlclist.xml's old text is kept there too.
param(
    [Parameter(Mandatory)] [string]$Gta,
    [Parameter(Mandatory)] [string]$CodeWalker,
    [string]$Skater = (Join-Path (Split-Path -Parent $PSScriptRoot) 'build\skate3-skater'),
    # Test packs for finding what in a pack GTA cannot load: 'meta' has only
    # the data files (no clothes archive), 'small' only two pieces of
    # clothing per slot, 'bare' the archive with only the variations file
    # and the body (no clothes), 'body' only the body, 'empty' an empty
    # archive.
    [ValidateSet('full', 'meta', 'small', 'bare', 'body', 'empty')] [string]$Variant = 'full',
    [ValidateSet('pack', 'skeleton', 'dlclist', 'boards', 'reference', 'weapon')] [string]$Mode = 'pack',
    [string]$Out,
    [string]$Weapon = (Join-Path (Split-Path -Parent $PSScriptRoot) 'build\skate3-weapon'),
    # Only the board part props, no WEAPON_SKATEBOARD of our own: with SK8V
    # as the core its skatev pack has the weapon (decision A28).
    [switch]$PropsOnly
)
$ErrorActionPreference = 'Stop'
$pack = 'skate3'
$boardPack = 'skate3boards'
$weaponPack = if ($PropsOnly) { 'skate3props' } else { 'skate3weapon' }
$ped = 'skate3_skater'

function Say([string]$text, [string]$color = 'Gray') { Write-Host $text -ForegroundColor $color }
function Fail([string]$text) { throw $text }

# --- What is needed --------------------------------------------------------
# GTA V Enhanced (GTA5_Enhanced.exe) reads the newer "gen9" resources;
# Legacy (GTA5.exe) the older ones. CodeWalker writes either.
$enhanced = Test-Path -LiteralPath (Join-Path $Gta 'GTA5_Enhanced.exe')
if (-not $enhanced -and -not (Test-Path -LiteralPath (Join-Path $Gta 'GTA5.exe'))) { Fail "No GTA5.exe or GTA5_Enhanced.exe in $Gta." }
$dlcSource = Join-Path $Skater 'dlc'
$rpfSource = Join-Path $Skater "x64\models\cdimages\$pack"
$parts = @()
$boardDicts = @(Get-ChildItem -LiteralPath $rpfSource -Filter 'skate3_board_*.ytd.xml' -File -ErrorAction SilentlyContinue)
if ($Mode -eq 'boards' -and $boardDicts.Count -eq 0) { Fail "No board textures in $rpfSource. Make the skater first (menu 5)." }
if ($Mode -eq 'pack') {
foreach ($need in @(
        (Join-Path $dlcSource 'setup2.xml'), (Join-Path $dlcSource 'content.xml'),
        (Join-Path $dlcSource 'common\data\peds.meta'), (Join-Path $rpfSource "$ped.ymt.xml"))) {
    if (-not (Test-Path -LiteralPath $need)) { Fail "Missing $need. Make the skater first (menu 5)." }
}
$parts = @(Get-ChildItem -LiteralPath (Join-Path $rpfSource $ped) -File |
    Where-Object { $_.Name -like '*.ydd.xml' -or $_.Name -like '*.ytd.xml' })
if ($parts.Count -eq 0) { Fail "No .ydd.xml or .ytd.xml in $(Join-Path $rpfSource $ped). Make the skater first (menu 5)." }
# GTA keeps a slot's texture count in one byte: a skater made before that
# was capped has to be made again.
[xml]$variations = [IO.File]::ReadAllText((Join-Path $rpfSource "$ped.ymt.xml"))
$overfull = @($variations.CPedVariationInfo.aComponentData3.Item | Where-Object {
    @($_.aDrawblData3.Item | ForEach-Object { @($_.aTexData.Item).Count } | Measure-Object -Sum).Sum -gt 255
})
if ($overfull.Count -gt 0) {
    Fail "Your skater has a clothing slot with more than 255 textures, which GTA cannot hold. Make it again: menu 5, answer Y."
}
}
$core = Join-Path $CodeWalker 'CodeWalker.Core.dll'
if (-not (Test-Path -LiteralPath $core)) { Fail "No CodeWalker.Core.dll in $CodeWalker." }
$busy = @(Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -match '^(GTA5|GTA5_Enhanced|PlayGTAV|OpenIV|CodeWalker.*)$' })
if ($busy.Count -gt 0) {
    Fail "Close these first, they hold the archives: $(($busy | ForEach-Object ProcessName | Sort-Object -Unique) -join ', ')"
}
$updateRpf = Join-Path $Gta 'mods\update\update.rpf'
if (-not (Test-Path -LiteralPath $updateRpf)) {
    $original = Join-Path $Gta 'update\update.rpf'
    if (-not (Test-Path -LiteralPath $original)) { Fail "No update\update.rpf in $Gta." }
    Say 'Copying update\update.rpf into the mods folder (once)...'
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $updateRpf) | Out-Null
    Copy-Item -LiteralPath $original -Destination $updateRpf
}
$old = Join-Path (Split-Path -Parent $PSScriptRoot) '_old'
New-Item -ItemType Directory -Force -Path $old | Out-Null
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
# Only the newest kept pack of each kind stays (each is up to 240 MB; the
# folder had grown to 2 GB, B94).
Get-ChildItem -LiteralPath $old -Filter '*-dlc-*.rpf' -File | Group-Object { $_.Name -replace '-dlc-.*$', '' } |
    ForEach-Object { $_.Group | Sort-Object LastWriteTime -Descending | Select-Object -Skip 1 | Remove-Item -Force }

# --- CodeWalker's library ----------------------------------------------------
# Windows marks DLLs from a downloaded ZIP as from the internet, and .NET
# then refuses them ("Operation is not supported", 0x80131515): unmarked
# first, and loaded the way that ignores the mark if that is not enough.
try {
    Get-ChildItem -LiteralPath $CodeWalker -Filter '*.dll' -File -ErrorAction SilentlyContinue |
        Unblock-File -ErrorAction SilentlyContinue
} catch { }
foreach ($dll in 'SharpDX.dll', 'SharpDX.Mathematics.dll', 'CodeWalker.Core.dll') {
    $path = Join-Path $CodeWalker $dll
    if (-not (Test-Path -LiteralPath $path)) { continue }
    try {
        [void][Reflection.Assembly]::LoadFrom($path)
    } catch {
        [void][Reflection.Assembly]::UnsafeLoadFrom($path)
    }
}
$RpfFile = [CodeWalker.GameFiles.RpfFile]
$OPEN = [CodeWalker.GameFiles.RpfEncryption]::OPEN
$status = [Action[string]] { param($m) }
$errors = New-Object System.Collections.Generic.List[string]
$errorLog = [Action[string]] { param($m) $errors.Add($m) }.GetNewClosure()

# GTA's keys, read from GTA5.exe as CodeWalker does (its versions differ in
# the parameters: they are filled by name).
if ($enhanced) {
    $gen9 = [CodeWalker.GameFiles.RpfManager].GetProperty('IsGen9')
    if (-not $gen9) { Fail 'This CodeWalker cannot write GTA V Enhanced files: get the newest CodeWalker.' }
    $gen9.SetValue($null, $true)
    Say 'GTA V Enhanced: writing gen9 files.' 'DarkGray'
}
Say "Reading GTA V's keys from the game (up to a minute)..."
$load = [CodeWalker.GameFiles.GTA5Keys].GetMethod('LoadFromPath')
$arguments = New-Object 'System.Collections.Generic.List[object]'
foreach ($p in $load.GetParameters()) {
    # Plain .NET values: PowerShell's wrapped ones do not convert.
    switch ($p.Name) {
        'path' { $arguments.Add([string]$Gta.TrimEnd('\')) }
        'gen9' { $arguments.Add([bool]$enhanced) }
        'key' { $arguments.Add($null) }
        default { $arguments.Add($p.DefaultValue) }
    }
}
[void]$load.Invoke($null, $arguments.ToArray())

function Open-Rpf([string]$path, [string]$relative) {
    $rpf = New-Object CodeWalker.GameFiles.RpfFile($path, $relative)
    $errors.Clear()
    $rpf.ScanStructure($status, $errorLog)
    if ($errors.Count -gt 0) { Fail "Cannot read $path`: $($errors[0])" }
    $rpf
}

# All archives in `rpf`, it first.
function All-Rpfs($rpf) {
    $rpf
    if ($rpf.Children) { foreach ($child in $rpf.Children) { All-Rpfs $child } }
}

# A file inside an archive, as CodeWalker's "Export Raw" gives it.
function Raw-Of($entry) {
    $data = $entry.File.ExtractFile($entry)
    if ($null -eq $data) { Fail "Cannot read $($entry.Path)." }
    if ($entry -is [CodeWalker.GameFiles.RpfResourceFileEntry]) {
        $data = [CodeWalker.GameFiles.ResourceBuilder]::Compress($data)
        $data = [CodeWalker.GameFiles.ResourceBuilder]::AddResourceHeader($entry, $data)
    }
    , $data
}

# A CodeWalker XML file made binary, as its "Import XML" does: the name
# without .xml, and for drawables and texture dictionaries the folder of
# textures beside the file.
function Import-Xml($dir, [string]$path) {
    $name = [IO.Path]::GetFileName($path)
    $trim = 4
    $format = [CodeWalker.GameFiles.XmlMeta]::GetXMLFormat($name.ToLowerInvariant(), [ref]$trim)
    $name = $name.Substring(0, $name.Length - $trim)
    $inside = $path.Substring(0, $path.Length - $trim)
    $inside = Join-Path ([IO.Path]::GetDirectoryName($inside)) ([IO.Path]::GetFileNameWithoutExtension($inside))
    $doc = New-Object System.Xml.XmlDocument
    $text = [IO.File]::ReadAllText($path)
    if ($text) { $doc.LoadXml($text) }
    $data = [CodeWalker.GameFiles.XmlMeta]::GetData($doc, $format, $inside)
    if ($null -eq $data) { Fail "CodeWalker could not import $path." }
    [void]$RpfFile::CreateFile($dir, $name, $data, $true)
}

function Add-File($dir, [string]$path) {
    [void]$RpfFile::CreateFile($dir, [IO.Path]::GetFileName($path), [IO.File]::ReadAllBytes($path), $true)
}

# The pack in mods\update\update.rpf's dlclist.xml (once).
function Add-DlcList([string[]]$names = @($pack, $boardPack)) {
    Say 'Adding the packs to dlclist.xml...'
    $update = Open-Rpf $updateRpf 'mods\update\update.rpf'
    $list = $null
    foreach ($rpf in All-Rpfs $update) {
        $list = $rpf.AllEntries | Where-Object {
            $_.NameLower -eq 'dlclist.xml' -and $_.Path -match '\\common\\data\\dlclist\.xml$'
        } | Select-Object -First 1
        if ($list) { break }
    }
    if (-not $list) { Fail 'No common\data\dlclist.xml in mods\update\update.rpf.' }
    $bytes = Raw-Of $list
    $text = [Text.Encoding]::UTF8.GetString($bytes).TrimStart([char]0xFEFF)
    $before = $text
    foreach ($name in $names) {
        $item = "dlcpacks:/$name/"
        if ($text -match [regex]::Escape($item)) {
            Say "  $item already there" 'DarkGray'
            continue
        }
        $at = $text.LastIndexOf('</Paths>')
        if ($at -lt 0) { Fail 'dlclist.xml has no </Paths>.' }
        $text = $text.Substring(0, $at) + "`t<Item>$item</Item>`r`n`t" + $text.Substring($at)
        Say "  added <Item>$item</Item>" 'Green'
    }
    if ($text -ne $before) {
        [IO.File]::WriteAllText((Join-Path $old "dlclist-$stamp.xml"), $before)
        [void]$RpfFile::CreateFile($list.Parent, 'dlclist.xml', [Text.Encoding]::UTF8.GetBytes($text), $true)
    }
}

# A new dlc.rpf at mods\update\x64\dlcpacks\<name>, the one there kept in _old.
function New-Dlc([string]$name) {
    $relative = "mods\update\x64\dlcpacks\$name\dlc.rpf"
    $path = Join-Path $Gta $relative
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    foreach ($leftover in "$path.crashes", "$path.off") {
        if (Test-Path -LiteralPath $leftover) { Remove-Item -LiteralPath $leftover -Force }
    }
    if (Test-Path -LiteralPath $path) {
        $kept = Join-Path $old "$name-dlc-$stamp.rpf"
        Move-Item -LiteralPath $path -Destination $kept
        Say "The old $name\dlc.rpf is kept as $kept" 'DarkGray'
    }
    Say "Making $path..."
    $RpfFile::CreateNew($Gta.TrimEnd('\'), $relative, $OPEN)
}

# The board textures' own pack: only texture dictionaries, which the
# plugin asks GTA for by name.
function Pack-Boards {
    if ($boardDicts.Count -eq 0) { Say 'No board textures made yet: the board is drawn in flat colours (menu 5 makes them).' 'Yellow'; return }
    $device = "dlc_$boardPack"
    $set = $device.ToUpperInvariant() + '_AUTOGEN'
    $file = "${device}:/%PLATFORM%/models/cdimages/$boardPack.rpf"
    $setup = @"
<?xml version="1.0" encoding="UTF-8"?>
<SSetupData>
  <deviceName>$device</deviceName>
  <datFile>content.xml</datFile>
  <timeStamp>10/08/2026 12:00:00</timeStamp>
  <nameHash>$boardPack</nameHash>
  <contentChangeSets />
  <contentChangeSetGroups>
    <Item>
      <NameHash>GROUP_STARTUP</NameHash>
      <ContentChangeSets>
        <Item>$set</Item>
      </ContentChangeSets>
    </Item>
  </contentChangeSetGroups>
  <startupScript />
  <scriptCallstackSize value="0" />
  <type>EXTRACONTENT_COMPAT_PACK</type>
  <order value="30" />
  <minorOrder value="0" />
  <isLevelPack value="false" />
  <dependencyPackHash />
  <requiredVersion />
  <subPackCount value="0" />
</SSetupData>
"@
    $content = @"
<?xml version="1.0" encoding="UTF-8"?>
<CDataFileMgr__ContentsOfDataFileXml>
  <disabledFiles />
  <includedXmlFiles />
  <includedDataFiles />
  <dataFiles>
    <Item>
      <filename>$file</filename>
      <fileType>RPF_FILE</fileType>
      <overlay value="false" />
      <disabled value="true" />
      <persistent value="true" />
    </Item>
  </dataFiles>
  <contentChangeSets>
    <Item>
      <changeSetName>$set</changeSetName>
      <filesToDisable />
      <filesToEnable>
        <Item>$file</Item>
      </filesToEnable>
    </Item>
  </contentChangeSets>
  <patchFiles />
</CDataFileMgr__ContentsOfDataFileXml>
"@
    $dlc = New-Dlc $boardPack
    [void]$RpfFile::CreateFile($dlc.Root, 'setup2.xml', [Text.Encoding]::UTF8.GetBytes($setup), $true)
    [void]$RpfFile::CreateFile($dlc.Root, 'content.xml', [Text.Encoding]::UTF8.GetBytes($content), $true)
    $cd = $RpfFile::CreateDirectory($RpfFile::CreateDirectory($RpfFile::CreateDirectory($dlc.Root, 'x64'), 'models'), 'cdimages')
    $images = $RpfFile::CreateNew($cd, "$boardPack.rpf", $OPEN)
    foreach ($dict in $boardDicts) {
        Import-Xml $images.Root $dict.FullName
        Say "  board textures: $($dict.Name -replace '\.ytd\.xml$')" 'DarkGray'
    }
    Say 'The board textures are packed.' 'Green'
}

# --- The board as a weapon -------------------------------------------------------
# GTA's text format for labels (.gxt2): "2TXG", the count, (hash, offset) for
# each sorted by hash, "2TXG" again, the size, then the strings.
function New-Gxt2([hashtable]$labels) {
    $joaat = {
        param([string]$s)
        [uint32]$h = 0
        foreach ($c in [Text.Encoding]::UTF8.GetBytes($s.ToLowerInvariant())) {
            $h = [uint32](([uint64]$h + $c) % 4294967296)
            $h = [uint32](([uint64]$h + ([uint64]$h * 1024)) % 4294967296)
            $h = [uint32]($h -bxor ($h -shr 6))
        }
        $h = [uint32](([uint64]$h + ([uint64]$h * 8)) % 4294967296)
        $h = [uint32]($h -bxor ($h -shr 11))
        [uint32](([uint64]$h + ([uint64]$h * 32768)) % 4294967296)
    }
    $items = @($labels.Keys | ForEach-Object { [pscustomobject]@{ Hash = (& $joaat $_); Text = [Text.Encoding]::UTF8.GetBytes($labels[$_] + [char]0) } } | Sort-Object Hash)
    $head = 8 + 8 * $items.Count + 8
    $ms = New-Object IO.MemoryStream
    $w = New-Object IO.BinaryWriter($ms)
    $w.Write([Text.Encoding]::ASCII.GetBytes('2TXG'))
    $w.Write([uint32]$items.Count)
    $at = $head
    foreach ($i in $items) { $w.Write([uint32]$i.Hash); $w.Write([uint32]$at); $at += $i.Text.Length }
    $w.Write([Text.Encoding]::ASCII.GetBytes('2TXG'))
    $w.Write([uint32]$at)
    foreach ($i in $items) { $w.Write([byte[]]$i.Text) }
    $w.Flush()
    , $ms.ToArray()
}

# GTA's own data file inside the mods copy of update.rpf (read only).
function Update-Text([string]$path) {
    $update = Open-Rpf $updateRpf 'mods\update\update.rpf'
    foreach ($rpf in All-Rpfs $update) {
        $e = $rpf.AllEntries | Where-Object { $_.Path -like "*\update.rpf\$path" } | Select-Object -First 1
        if ($e) { return [Text.Encoding]::UTF8.GetString((Raw-Of $e)).TrimStart([char]0xFEFF) }
    }
    Fail "No $path in update.rpf."
}

# The block from the <Item ...> opening before `marker` to its matching
# </Item>.
function Item-Block([string]$text, [string]$marker, [string]$open) {
    $at = $text.IndexOf($marker)
    if ($at -lt 0) { return $null }
    # (LastIndexOf only finds what ends by its start: from the marker's end.)
    $start = $text.LastIndexOf($open, [Math]::Min($text.Length - 1, $at + $marker.Length - 1))
    if ($start -lt 0) { return $null }
    $depth = 0
    $i = $start
    while ($true) {
        $nextOpen = $text.IndexOf('<Item', $i + 1)
        $nextClose = $text.IndexOf('</Item>', $i + 1)
        if ($nextClose -lt 0) { return $null }
        if ($nextOpen -ge 0 -and $nextOpen -lt $nextClose) {
            $tagEnd = $text.IndexOf('>', $nextOpen)
            # Self-closing items (<Item ... />) open nothing.
            if ($text[$tagEnd - 1] -ne '/') { $depth++ }
            $i = $nextOpen
        } else {
            if ($depth -eq 0) { return $text.Substring($start, $nextClose + 7 - $start) }
            $depth--
            $i = $nextClose
        }
    }
}

function Pack-Weapon {
    $model = 'w_me_skate3board'
    foreach ($need in "$model.ydr.xml", "$model.ytd.xml") {
        if (-not (Test-Path -LiteralPath (Join-Path $Weapon $need))) { Fail "Missing $need in $Weapon (skate-ped --weapon makes it)." }
    }
    Say 'Reading the bat from GTA''s weapon data...'
    $weapons = Update-Text 'common\data\ai\weapons.meta'
    $bat = Item-Block $weapons '<Name>WEAPON_BAT</Name>' '<Item type="CWeaponInfo">'
    if (-not $bat) { Fail 'No WEAPON_BAT in weapons.meta.' }
    $info = $bat.Replace('<Name>WEAPON_BAT</Name>', '<Name>WEAPON_SKATEBOARD</Name>').
        Replace('<Model>w_me_bat</Model>', "<Model>$model</Model>").
        Replace('<Slot>SLOT_BAT</Slot>', '<Slot>SLOT_SKATEBOARD</Slot>').
        Replace('<HumanNameHash>WT_BAT</HumanNameHash>', '<HumanNameHash>WT_SKATEBOARD</HumanNameHash>').
        Replace('<PickupHash>PICKUP_WEAPON_BAT</PickupHash>', '<PickupHash />')
    $info = [regex]::Replace($info, '(?s)<AttachPoints>.*?</AttachPoints>', '<AttachPoints />')
    $weaponMeta = @"
<?xml version="1.0" encoding="UTF-8"?>
<CWeaponInfoBlob>
  <SlotNavigateOrder>
    <Item>
      <WeaponSlots>
        <Item>
          <OrderNumber value="87" />
          <Entry>SLOT_SKATEBOARD</Entry>
        </Item>
      </WeaponSlots>
    </Item>
    <Item>
      <WeaponSlots>
        <Item>
          <OrderNumber value="87" />
          <Entry>SLOT_SKATEBOARD</Entry>
        </Item>
      </WeaponSlots>
    </Item>
  </SlotNavigateOrder>
  <SlotBestOrder>
    <WeaponSlots>
      <Item>
        <OrderNumber value="336" />
        <Entry>SLOT_SKATEBOARD</Entry>
      </Item>
    </WeaponSlots>
  </SlotBestOrder>
  <TintSpecValues />
  <FiringPatternAliases />
  <UpperBodyFixupExpressionData />
  <AimingInfos />
  <Infos>
    <Item>
      <Infos />
    </Item>
    <Item>
      <Infos>
        $info
      </Infos>
    </Item>
    <Item>
      <Infos />
    </Item>
    <Item>
      <Infos />
    </Item>
  </Infos>
  <VehicleWeaponInfos />
  <Name>Skate 3 - Skateboard</Name>
</CWeaponInfoBlob>
"@
    # The bat's animations, in every set that has them.
    $animations = Update-Text 'common\data\ai\weaponanimations.meta'
    $sets = New-Object Text.StringBuilder
    # A set reaches to the next one's key.
    $found = @([regex]::Matches($animations, '<Item key="([^"]+)">\s*(?:(?:<Fallback>[^<]*</Fallback>|<Fallback\s*/>)\s*)?<WeaponAnimations>'))
    for ($k = 0; $k -lt $found.Count; $k++) {
        $m = $found[$k]
        $setEnd = if ($k + 1 -lt $found.Count) { $found[$k + 1].Index } else { $animations.Length }
        $inside = $animations.Substring($m.Index, $setEnd - $m.Index)
        $entry = Item-Block $inside '<Item key="WEAPON_BAT">' '<Item key="WEAPON_BAT">'
        if ($entry) {
            [void]$sets.Append("    <Item key=""$($m.Groups[1].Value)"">`n      <WeaponAnimations>`n        ")
            [void]$sets.Append($entry.Replace('<Item key="WEAPON_BAT">', '<Item key="WEAPON_SKATEBOARD">'))
            [void]$sets.Append("`n      </WeaponAnimations>`n    </Item>`n")
        }
    }
    if ($sets.Length -eq 0) { Fail 'No WEAPON_BAT animations in weaponanimations.meta.' }
    $animationMeta = "<?xml version=""1.0"" encoding=""UTF-8""?>`n<CWeaponAnimationsSets>`n  <WeaponAnimationsSets>`n$sets  </WeaponAnimationsSets>`n</CWeaponAnimationsSets>`n"
    # Every board part in every finish as its own prop (skate-ped writes
    # them to parts\ with skate3parts.ytyp.xml and lists them in parts.txt):
    # the plugin builds any board from three of them. Props, not weapon
    # models: GTA holds only 115 of those (MaxWeaponModelInfos).
    $partsDir = Join-Path $Weapon 'parts'
    $partsList = Join-Path $partsDir 'parts.txt'
    $parts = @()
    if (Test-Path -LiteralPath $partsList) {
        $parts = @([IO.File]::ReadAllLines($partsList) | Where-Object { $_.Trim() })
    }
    # Extra sets of props, each in an archive and ityp of its own: with the
    # stickers in the parts' archive (1403 props) GTA loaded none of the
    # parts. Folder, archive/ityp name, list file.
    $extraSets = @()
    foreach ($set in @(@('stickers', 'skate3stickers', 'stickers.txt'), @('wheel', 'skate3wheel', 'wheel.txt'))) {
        $list = Join-Path (Join-Path $Weapon $set[0]) $set[2]
        if (Test-Path -LiteralPath $list) {
            $names = @([IO.File]::ReadAllLines($list) | Where-Object { $_.Trim() })
            if ($names.Count) { $extraSets += , @{ Dir = (Join-Path $Weapon $set[0]); Name = $set[1]; Models = $names } }
        }
    }
    $models = @($model)
    $archetypeItems = ($models | ForEach-Object { "    <Item>`n      <modelName>$_</modelName>`n      <txdName>$_</txdName>`n      <ptfxAssetName>NULL</ptfxAssetName>`n      <lodDist value=""60"" />`n    </Item>" }) -join "`n"
    $archetypes = "<?xml version=""1.0"" encoding=""UTF-8""?>`n<CWeaponModelInfo__InitDataList>`n  <InitDatas>`n$archetypeItems`n  </InitDatas>`n</CWeaponModelInfo__InitDataList>`n"

    $dlcText = @"
<?xml version="1.0" encoding="UTF-8"?>
<CExtraTextMetaFile>
  <hasGlobalTextFile value="true" />
  <hasAdditionalText value="false" />
  <isTitleUpdate value="false" />
</CExtraTextMetaFile>
"@
    $device = "dlc_$weaponPack"
    $set = $device.ToUpperInvariant() + '_AUTOGEN'
    $files = @(
        @("${device}:/common/data/dlctext.meta", 'TEXTFILE_METAFILE', 'true'),
        @("${device}:/common/data/weaponarchetypes.meta", 'WEAPON_METADATA_FILE', 'false'),
        @("${device}:/common/data/ai/weaponanimations.meta", 'WEAPON_ANIMATIONS_FILE', 'false'),
        @("${device}:/common/data/ai/weapon_skateboard.meta", 'WEAPONINFO_FILE', 'false'),
        @("${device}:/%PLATFORM%/models/cdimages/$weaponPack.rpf", 'RPF_FILE', 'true')
    )
    if ($PropsOnly) { $files = @() }
    if ($parts.Count) {
        $files += , @("${device}:/%PLATFORM%/levels/gta5/props/skate3parts.rpf", 'RPF_FILE', 'true')
        $files += , @("${device}:/%PLATFORM%/levels/gta5/props/skate3parts.ityp", 'DLC_ITYP_REQUEST', 'false')
    }
    foreach ($set in $extraSets) {
        $files += , @("${device}:/%PLATFORM%/levels/gta5/props/$($set.Name).rpf", 'RPF_FILE', 'true')
        $files += , @("${device}:/%PLATFORM%/levels/gta5/props/$($set.Name).ityp", 'DLC_ITYP_REQUEST', 'false')
    }
    $items = ($files | ForEach-Object { $contents = if ($_[1] -eq 'DLC_ITYP_REQUEST') { "`n      <contents>CONTENTS_PROPS</contents>" } else { '' }; "    <Item>`n      <filename>$($_[0])</filename>`n      <fileType>$($_[1])</fileType>`n      <overlay value=""false"" />`n      <disabled value=""true"" />`n      <persistent value=""$($_[2])"" />$contents`n    </Item>" }) -join "`n"
    $enable = ($files | ForEach-Object { "        <Item>$($_[0])</Item>" }) -join "`n"
    $content = "<?xml version=""1.0"" encoding=""UTF-8""?>`n<CDataFileMgr__ContentsOfDataFileXml>`n  <disabledFiles />`n  <includedXmlFiles />`n  <includedDataFiles />`n  <dataFiles>`n$items`n  </dataFiles>`n  <contentChangeSets>`n    <Item>`n      <changeSetName>$set</changeSetName>`n      <filesToDisable />`n      <filesToEnable>`n$enable`n      </filesToEnable>`n    </Item>`n  </contentChangeSets>`n  <patchFiles />`n</CDataFileMgr__ContentsOfDataFileXml>`n"
    $setup = @"
<?xml version="1.0" encoding="UTF-8"?>
<SSetupData>
  <deviceName>$device</deviceName>
  <datFile>content.xml</datFile>
  <timeStamp>10/08/2026 12:00:00</timeStamp>
  <nameHash>$weaponPack</nameHash>
  <contentChangeSets />
  <contentChangeSetGroups>
    <Item>
      <NameHash>GROUP_STARTUP</NameHash>
      <ContentChangeSets>
        <Item>$set</Item>
      </ContentChangeSets>
    </Item>
  </contentChangeSetGroups>
  <startupScript />
  <scriptCallstackSize value="0" />
  <type>EXTRACONTENT_COMPAT_PACK</type>
  <order value="31" />
  <minorOrder value="0" />
  <isLevelPack value="false" />
  <dependencyPackHash />
  <requiredVersion />
  <subPackCount value="0" />
</SSetupData>
"@
    $utf8 = { param([string]$s) , [Text.Encoding]::UTF8.GetBytes($s) }
    # What is packed, beside this script too (to read when GTA will not load it).
    $look = Join-Path $Weapon 'dlc'
    New-Item -ItemType Directory -Force -Path $look | Out-Null
    foreach ($pair in @(@('setup2.xml', $setup), @('content.xml', $content), @('weapon_skateboard.meta', $weaponMeta), @('weaponanimations.meta', $animationMeta), @('weaponarchetypes.meta', $archetypes))) {
        [IO.File]::WriteAllText((Join-Path $look $pair[0]), $pair[1])
    }
    $dlc = New-Dlc $weaponPack
    [void]$RpfFile::CreateFile($dlc.Root, 'setup2.xml', (& $utf8 $setup), $true)
    [void]$RpfFile::CreateFile($dlc.Root, 'content.xml', (& $utf8 $content), $true)
    $common = $RpfFile::CreateDirectory($RpfFile::CreateDirectory($dlc.Root, 'common'), 'data')
    [void]$RpfFile::CreateFile($common, 'dlctext.meta', (& $utf8 $dlcText), $true)
    [void]$RpfFile::CreateFile($common, 'weaponarchetypes.meta', (& $utf8 $archetypes), $true)
    $ai = $RpfFile::CreateDirectory($common, 'ai')
    [void]$RpfFile::CreateFile($ai, 'weaponanimations.meta', (& $utf8 $animationMeta), $true)
    [void]$RpfFile::CreateFile($ai, 'weapon_skateboard.meta', (& $utf8 $weaponMeta), $true)
    $x64 = $RpfFile::CreateDirectory($dlc.Root, 'x64')
    $lang = $RpfFile::CreateDirectory($RpfFile::CreateDirectory($x64, 'data'), 'lang')
    $text = New-Gxt2 @{ 'WT_SKATEBOARD' = 'Skateboard' }
    foreach ($language in 'american', 'german', 'french', 'italian', 'spanish', 'portuguese', 'polish', 'russian', 'mexican', 'chinese', 'chinesesimp', 'japanese', 'korean') {
        $archive = $RpfFile::CreateNew($lang, "${language}dlc.rpf", $OPEN)
        [void]$RpfFile::CreateFile($archive.Root, 'global.gxt2', $text, $true)
    }
    $cd = $RpfFile::CreateDirectory($RpfFile::CreateDirectory($x64, 'models'), 'cdimages')
    $images = $RpfFile::CreateNew($cd, "$weaponPack.rpf", $OPEN)
    Import-Xml $images.Root (Join-Path $Weapon "$model.ydr.xml")
    Import-Xml $images.Root (Join-Path $Weapon "$model.ytd.xml")
    $props = $null
    if ($parts.Count -or $extraSets.Count) {
        $props = $RpfFile::CreateDirectory($RpfFile::CreateDirectory($RpfFile::CreateDirectory($x64, 'levels'), 'gta5'), 'props')
    }
    foreach ($set in $extraSets) {
        $setRpf = $RpfFile::CreateNew($props, "$($set.Name).rpf", $OPEN)
        Import-Xml $setRpf.Root (Join-Path $set.Dir "$($set.Name).ytyp.xml")
        foreach ($m in $set.Models) {
            Import-Xml $setRpf.Root (Join-Path $set.Dir "$m.ydr.xml")
            Import-Xml $setRpf.Root (Join-Path $set.Dir "$m.ytd.xml")
        }
        Say "Packed $($set.Name): $($set.Models.Count) props of their own." 'Green'
    }
    if ($parts.Count) {
        $partsRpf = $RpfFile::CreateNew($props, 'skate3parts.rpf', $OPEN)
        Import-Xml $partsRpf.Root (Join-Path $partsDir 'skate3parts.ytyp.xml')
        $n = 0
        foreach ($part in $parts) {
            $n++
            if ($n % 50 -eq 0 -or $n -eq $parts.Count) { Say "  board parts $n/$($parts.Count)" 'DarkGray' }
            Import-Xml $partsRpf.Root (Join-Path $partsDir "$part.ydr.xml")
            Import-Xml $partsRpf.Root (Join-Path $partsDir "$part.ytd.xml")
        }
        Say "Every board part is packed too: $($parts.Count) HD props." 'Green'
    }
    if ($PropsOnly) { Say "The board parts are packed as $weaponPack (the weapon is SK8V's)." 'Green' }
    else { Say "The board is packed as WEAPON_SKATEBOARD ($model)." 'Green' }
}

# --- The smaller jobs ----------------------------------------------------------
if ($Mode -eq 'dlclist') { Add-DlcList; return }
if ($Mode -eq 'boards') { Pack-Boards; Add-DlcList @($boardPack); return }
if ($Mode -eq 'weapon') { Pack-Weapon; Add-DlcList @($weaponPack); return }
if ($Mode -eq 'reference') {
    if (-not $Out) { Fail 'No -Out folder for the reference files.' }
    New-Item -ItemType Directory -Force -Path $Out | Out-Null
    $x64v = Open-Rpf (Join-Path $Gta 'x64v.rpf') 'x64v.rpf'
    $wanted = @('player_zero.ymt', 'mp_m_freemode_01.ymt', 'a_m_y_skater_01.ymt')
    foreach ($rpf in All-Rpfs $x64v) {
        foreach ($entry in @($rpf.AllEntries | Where-Object { $wanted -contains $_.NameLower })) {
            $name = ''
            $xml = [CodeWalker.GameFiles.MetaXml]::GetXml($entry, $entry.File.ExtractFile($entry), [ref]$name, '')
            if ($xml) { [IO.File]::WriteAllText((Join-Path $Out "$($entry.NameLower).xml"), $xml); Say "  $($entry.Path)" 'DarkGray' }
        }
    }
    # Your skater's, as CodeWalker reads it back after making it binary.
    $ymtXml = Join-Path $rpfSource "$ped.ymt.xml"
    if (Test-Path -LiteralPath $ymtXml) {
        Copy-Item -LiteralPath $ymtXml -Destination (Join-Path $Out "yours-$ped.ymt.xml") -Force
        $doc = New-Object System.Xml.XmlDocument
        $doc.LoadXml([IO.File]::ReadAllText($ymtXml))
        $data = [CodeWalker.GameFiles.XmlMeta]::GetData($doc, [CodeWalker.GameFiles.MetaFormat]::RSC, '')
        if ($data) {
            $entry = [CodeWalker.GameFiles.RpfFile]::CreateResourceFileEntry([ref]$data, 2)
            $entry.Name = "$ped.ymt"
            $entry.NameLower = "$ped.ymt"
            $name = ''
            $back = [CodeWalker.GameFiles.MetaXml]::GetXml($entry, $data, [ref]$name, '')
            if ($back) { [IO.File]::WriteAllText((Join-Path $Out "yours-$ped.ymt.roundtrip.xml"), $back) }
        }
    }
    Say "Reference files written to $Out" 'Green'
    return
}
if ($Mode -eq 'skeleton') {
    if (-not $Out) { Fail 'No -Out file for the skeleton.' }
    Say 'Reading mp_m_freemode_01.yft from x64v.rpf...'
    $x64v = Open-Rpf (Join-Path $Gta 'x64v.rpf') 'x64v.rpf'
    $entry = $null
    foreach ($rpf in All-Rpfs $x64v) {
        if ($rpf.NameLower -ne 'streamedpeds_mp.rpf') { continue }
        $entry = $rpf.AllEntries | Where-Object { $_.NameLower -eq 'mp_m_freemode_01.yft' } | Select-Object -First 1
        if ($entry) { break }
    }
    if (-not $entry) { Fail 'mp_m_freemode_01.yft is not in x64v.rpf\models\cdimages\streamedpeds_mp.rpf.' }
    $yft = New-Object CodeWalker.GameFiles.YftFile
    $yft.Load($entry.File.ExtractFile($entry), $entry)
    $folder = Split-Path -Parent $Out
    New-Item -ItemType Directory -Force -Path $folder | Out-Null
    $xml = [CodeWalker.GameFiles.YftXml]::GetXml($yft, $folder)
    if (-not $xml) { Fail 'CodeWalker could not write the skeleton as XML.' }
    [IO.File]::WriteAllText($Out, $xml)
    Say "Skeleton written: $Out" 'Green'
    return
}

# --- The test packs -----------------------------------------------------------
$ymtPath = Join-Path $rpfSource "$ped.ymt.xml"
if ($Variant -eq 'small') {
    # The first two drawables of each slot, and their textures.
    $parts = @($parts | Where-Object { $_.Name -match '_(\d{3})_' -and [int]$Matches[1] -lt 2 })
    [xml]$small = [IO.File]::ReadAllText($ymtPath)
    foreach ($component in @($small.CPedVariationInfo.aComponentData3.Item)) {
        $drawables = @($component.aDrawblData3.Item)
        for ($i = $drawables.Count - 1; $i -ge 2; $i--) { [void]$component.aDrawblData3.RemoveChild($drawables[$i]) }
        $count = (@($component.aDrawblData3.Item) | ForEach-Object { @($_.aTexData.Item).Count } | Measure-Object -Sum).Sum
        $component.numAvailTex.value = [string]$count
    }
    foreach ($info in @($small.CPedVariationInfo.compInfos.Item)) {
        if ([int]$info.pedXml_drawblIdx.value -ge 2) { [void]$small.CPedVariationInfo.compInfos.RemoveChild($info) }
    }
    $smallDir = Join-Path $old "small-$stamp"
    New-Item -ItemType Directory -Force -Path $smallDir | Out-Null
    $ymtPath = Join-Path $smallDir "$ped.ymt.xml"
    $small.Save($ymtPath)
}

# --- GTA's ped skeleton (needs nothing written yet) -----------------------------
if ($Variant -in 'bare', 'body', 'empty') { $parts = @() }
if ($Variant -notin 'meta', 'empty') {
Say 'Finding mp_m_freemode_01.yft in x64v.rpf...'
$x64v = Open-Rpf (Join-Path $Gta 'x64v.rpf') 'x64v.rpf'
$skeleton = $null
foreach ($rpf in All-Rpfs $x64v) {
    if ($rpf.NameLower -ne 'streamedpeds_mp.rpf') { continue }
    $skeleton = $rpf.AllEntries | Where-Object { $_.NameLower -eq 'mp_m_freemode_01.yft' } | Select-Object -First 1
    if ($skeleton) { break }
}
if (-not $skeleton) { Fail 'mp_m_freemode_01.yft is not in x64v.rpf\models\cdimages\streamedpeds_mp.rpf.' }
$skeletonData = Raw-Of $skeleton
}

# --- dlc.rpf ------------------------------------------------------------------
$dlc = New-Dlc $pack
$root = $dlc.Root
Add-File $root (Join-Path $dlcSource 'setup2.xml')
# content.xml from skaters made before the fix names peds.meta's type wrong
# (GTA then takes it for an archive and stops loading story mode).
$content = [IO.File]::ReadAllText((Join-Path $dlcSource 'content.xml')).Replace('PEDS_META_FILE', 'PED_METADATA_FILE')
if ($Variant -eq 'meta') {
    # The clothes archive left out of the pack's file list too.
    $content = [regex]::Replace($content, '(?s)\s*<Item>\s*<filename>[^<]*\.rpf</filename>.*?</Item>', '')
    $content = [regex]::Replace($content, '\s*<Item>[^<]*\.rpf</Item>', '')
}
[void]$RpfFile::CreateFile($root, 'content.xml', [Text.Encoding]::UTF8.GetBytes($content), $true)
$data = $RpfFile::CreateDirectory($RpfFile::CreateDirectory($root, 'common'), 'data')
# A ped whose parts are a folder is a streamed ped; skaters made before the
# fix said otherwise, and GTA looked for parts that are not there.
$meta = [IO.File]::ReadAllText((Join-Path $dlcSource 'common\data\peds.meta')).Replace('<IsStreamedGfx value="false" />', '<IsStreamedGfx value="true" />')
# ...and named an expression dictionary that does not exist, and declared
# the base game's resident textures again; as Rockstar's own DLC peds.meta.
$meta = $meta.Replace('<ExpressionDictionaryName>default</ExpressionDictionaryName>', '<ExpressionDictionaryName>null</ExpressionDictionaryName>')
$meta = $meta.Replace('<ExpressionName>default</ExpressionName>', '<ExpressionName>null</ExpressionName>')
$meta = [regex]::Replace($meta, '\s*<residentTxd>[^<]*</residentTxd>|\s*<residentAnims\s*/>|\s*<MovementClipSets\s*/>', '')
[void]$RpfFile::CreateFile($data, 'peds.meta', [Text.Encoding]::UTF8.GetBytes($meta), $true)
if ($Variant -eq 'meta') { $parts = @() } else {
$cdimages = $RpfFile::CreateDirectory($RpfFile::CreateDirectory($RpfFile::CreateDirectory($root, 'x64'), 'models'), 'cdimages')
$images = $RpfFile::CreateNew($cdimages, "$pack.rpf", $OPEN)
if ($Variant -notin 'body', 'empty') { Import-Xml $images.Root $ymtPath }
if ($Variant -ne 'empty') { [void]$RpfFile::CreateFile($images.Root, "$ped.yft", $skeletonData, $true) }
if ($parts.Count -gt 0) { $folder = $RpfFile::CreateDirectory($images.Root, $ped) }
$done = 0
foreach ($part in $parts) {
    Import-Xml $folder $part.FullName
    $done++
    if ($done % 10 -eq 0 -or $done -eq $parts.Count) { Say "  $done of $($parts.Count) parts" }
}
}

# --- The board textures, and dlclist.xml ------------------------------------------
if ($Variant -eq 'full') { Pack-Boards }
Add-DlcList

# --- Beside the plugin ----------------------------------------------------------
$json = Join-Path $Skater 'SkateGTA-skater.json'
if (Test-Path -LiteralPath $json) {
    Copy-Item -LiteralPath $json -Destination (Join-Path $Gta 'SkateGTA-skater.json') -Force
}
$loader = if ($enhanced) { 'OpenRPF.asi' } else { 'OpenIV.asi' }
if (-not (Test-Path -LiteralPath (Join-Path $Gta $loader))) {
    Say "No $loader in your GTA V folder: GTA only reads the mods folder with it (Skate Mods menu A says where to get it)." 'Yellow'
}
if ($Variant -ne 'full') { Say "Test pack '$Variant' made ($($parts.Count) parts)." 'Green'; return }
Say "Done: the skater is packed ($($parts.Count) parts). In GTA V: F8 becomes your skater, F11 opens the skater editor." 'Green'
