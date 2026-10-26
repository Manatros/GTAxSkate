# Shared by optional installer asset stages. Caller owns staging cleanup.
function Install-AssetTransaction {
    [CmdletBinding()]
    param([Parameter(Mandatory)][string]$TargetRoot,
          [Parameter(Mandatory)][string]$StageRoot,
          [Parameter(Mandatory)][AllowEmptyCollection()][object[]]$Files,
          [Parameter(Mandatory)][string]$BackupRoot)
    function Assert-PlainPath([string]$Path) {
        $full = [IO.Path]::GetFullPath($Path)
        for ($p = $full; $p; $p = Split-Path -Parent $p) {
            if ((Test-Path -LiteralPath $p) -and ((Get-Item -LiteralPath $p -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Reparse point refused: $p" }
        }
        $full
    }
    function Resolve-AssetPath([string]$Root, [string]$Name) {
        if (-not $Name -or [IO.Path]::IsPathRooted($Name) -or $Name -match '[\x00-\x1f:<>"|*?]' -or @($Name -split '[\\/]' | Where-Object { $_ -eq '..' -or $_ -eq '.' -or $_ -eq '' -or $_ -match '[ .]$' -or $_ -match '^(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)' }).Count) { throw "Invalid asset name: $Name" }
        $full = Assert-PlainPath (Join-Path $Root $Name)
        if (-not $full.StartsWith($Root.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)) { throw "Asset escapes root: $Name" }
        $full
    }
    $target = Assert-PlainPath $TargetRoot
    $stage = Assert-PlainPath (Join-Path $StageRoot 'payload')
    $backup = Assert-PlainPath $BackupRoot
    foreach ($pair in @(@($target,$stage), @($target,$backup), @($stage,$backup))) {
        if ($pair[0].Equals($pair[1], [StringComparison]::OrdinalIgnoreCase) -or $pair[0].StartsWith($pair[1].TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase) -or $pair[1].StartsWith($pair[0].TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Transaction roots must be separate.' }
    }
    if (Test-Path -LiteralPath $backup) { throw "Recovery folder already exists: $backup" }
    $seen = @{}
    $plan = @()
    foreach ($f in $Files) {
        $name = [string]$f.name
        $dest = Resolve-AssetPath $target $name
        $src = Resolve-AssetPath $stage $name
        $save = Resolve-AssetPath $backup $name
        if ($seen.ContainsKey($dest)) { throw "Duplicate asset: $name" }; $seen[$dest] = $true
        if ($f.sha256 -notmatch '^[0-9a-fA-F]{64}$' -or -not (Test-Path -LiteralPath $src -PathType Leaf) -or (Get-FileHash -LiteralPath $src -Algorithm SHA256).Hash -ne $f.sha256) { throw "Invalid staged asset: $name" }
        $exists = Test-Path -LiteralPath $dest
        if ($exists -and -not (Test-Path -LiteralPath $dest -PathType Leaf)) { throw "Asset destination is not a file: $name" }
        if ($f.originalSha256 -and ($f.originalSha256 -notmatch '^[0-9a-fA-F]{64}$' -or -not $exists -or (Get-FileHash -LiteralPath $dest -Algorithm SHA256).Hash -ne $f.originalSha256)) { throw "Original asset changed: $name" }
        $plan += [pscustomobject]@{ name=$name; source=$src; destination=$dest; backup=$save; existed=$exists }
    }
    # No destination changes until every payload and backup has been checked.
    foreach ($p in $plan) {
        if ($p.existed) {
            New-Item -ItemType Directory -Force -Path (Split-Path -Parent $p.backup) | Out-Null
            Copy-Item -LiteralPath $p.destination -Destination $p.backup -Force -ErrorAction Stop
            if ((Get-FileHash -LiteralPath $p.destination).Hash -ne (Get-FileHash -LiteralPath $p.backup).Hash) { throw "Backup verification failed: $($p.name)" }
        }
    }
    $changed = @()
    try {
        foreach ($p in $plan) {
            $null = Assert-PlainPath $p.destination
            New-Item -ItemType Directory -Force -Path (Split-Path -Parent $p.destination) | Out-Null
            $changed += $p # A failed copy may already have truncated its destination.
            Copy-Item -LiteralPath $p.source -Destination $p.destination -Force -ErrorAction Stop
            if ((Get-FileHash -LiteralPath $p.source).Hash -ne (Get-FileHash -LiteralPath $p.destination).Hash) { throw "Installed asset verification failed: $($p.name)" }
        }
    } catch {
        $cause = $_.Exception.Message
        $failures = @()
        for ($i = $changed.Count - 1; $i -ge 0; $i--) {
            $p = $changed[$i]
            try {
                $null = Assert-PlainPath $p.destination
                if ($p.existed) {
                    Copy-Item -LiteralPath $p.backup -Destination $p.destination -Force -ErrorAction Stop
                    if ((Get-FileHash -LiteralPath $p.backup).Hash -ne (Get-FileHash -LiteralPath $p.destination).Hash) { throw 'Restored bytes differ' }
                } else { Remove-Item -LiteralPath $p.destination -Force -ErrorAction SilentlyContinue; if (Test-Path -LiteralPath $p.destination) { throw 'New asset could not be removed' } }
            } catch { $failures += "$($p.name): $($_.Exception.Message)" }
        }
        if ($failures.Count) { throw "Asset installation failed: $cause. ROLLBACK INCOMPLETE; retain staging and recovery folder $backup. $($failures -join '; ')" }
        throw "Asset installation failed: $cause. Rollback completed; recovery folder: $backup"
    }
    [pscustomobject]@{ installed=$plan.Count; rollback='not needed'; backupRoot=$backup }
}
