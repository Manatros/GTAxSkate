# Puts every Skate 3 song locked by the collectibles back on Self Radio
# (renames "Skate 3 Track NN.mp3.locked" to ".mp3"). For use when GTAxSkate
# is removed or turned off; in game, Collectibles=0 does the same.
# A song present in both forms keeps the larger copy.
$ErrorActionPreference = 'Stop'
$dir = Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'Rockstar Games\GTA V\User Music\Skate 3'
if (-not (Test-Path -LiteralPath $dir)) { "No songs folder: $dir"; return }
$n = 0
foreach ($locked in Get-ChildItem -LiteralPath $dir -Filter '*.mp3.locked' -File) {
    $mp3 = Join-Path $dir ($locked.Name -replace '\.locked$', '')
    if (Test-Path -LiteralPath $mp3) {
        if ((Get-Item -LiteralPath $mp3).Length -ge $locked.Length) { Remove-Item -LiteralPath $locked.FullName; continue }
        Remove-Item -LiteralPath $mp3
    }
    Rename-Item -LiteralPath $locked.FullName -NewName (Split-Path -Leaf $mp3)
    $n++
}
"$n song(s) back on Self Radio. In GTA: Settings > Audio > Perform Quick Scan."
