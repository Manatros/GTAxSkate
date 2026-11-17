# Optional Friends-Only Coop helper (configure hardened host/client, verify security, or cleanly uninstall).
# Keeps GTAxSkate Story Coop strictly opt-in: blocks random public server browsers and unpinned IPs.
[CmdletBinding()]
param(
    [ValidateSet('GenerateHostConfig', 'ConfigureClient', 'VerifySecurity', 'PrepareOptionalPack', 'Uninstall')]
    [string]$Mode = 'VerifySecurity',
    [string]$TargetDir = '',
    [string]$HostEndpoint = '',
    [string]$FriendKey = '',
    [string]$HostLabel = 'Friends-VPN',
    [int]$Port = 4499,
    [switch]$AllowDevPack
)

$ErrorActionPreference = 'Stop'

function New-FriendKey([int]$Bytes = 18) {
    $rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
    try {
        $raw = New-Object byte[] $Bytes
        $rng.GetBytes($raw)
        return [Convert]::ToBase64String($raw).Replace('+', 'A').Replace('/', 'B').TrimEnd('=')
    } finally {
        $rng.Dispose()
    }
}

# Undo of RageCoopClientSettings.Apply (shared\RageCoopClientSettings.cs): MenuKey, MasterServer, LastServerAddress and
# Username go back to the values saved in <settings>.gtaxskate-backup.xml; keys that did not exist are removed; a file
# that did not exist is deleted again. No sidecar = nothing of ours to undo.
function Restore-RageCoopClientSettings([string]$SettingsFile) {
    $backupFile = "$SettingsFile.gtaxskate-backup.xml"
    if (-not (Test-Path -LiteralPath $backupFile -PathType Leaf)) { return }
    $backup = New-Object System.Xml.XmlDocument
    $backup.Load($backupFile)
    $fileExisted = ($backup.DocumentElement.GetAttribute('fileExisted') -eq '1')
    if (Test-Path -LiteralPath $SettingsFile -PathType Leaf) {
        $doc = New-Object System.Xml.XmlDocument
        $doc.Load($SettingsFile)
        $root = $doc.DocumentElement
        foreach ($key in 'MenuKey', 'MasterServer', 'LastServerAddress', 'Username') {
            $entry = $backup.DocumentElement.SelectSingleNode($key)
            $current = $root.SelectSingleNode($key)
            if ($entry -and $entry.GetAttribute('present') -eq '1') {
                if (-not $current) { $current = $doc.CreateElement($key); [void]$root.AppendChild($current) }
                $current.InnerText = $entry.InnerText
            } elseif ($current) {
                [void]$root.RemoveChild($current)
            }
        }
        if (-not $fileExisted -and $root.ChildNodes.Count -eq 0) {
            Remove-Item -LiteralPath $SettingsFile -Force
        } else {
            $doc.Save($SettingsFile)
        }
    }
    Remove-Item -LiteralPath $backupFile -Force
}

function Test-TrustedEndpoint([string]$HostText) {
    if ([string]::IsNullOrWhiteSpace($HostText)) { return $false }
    $h = $HostText.Trim().ToLowerInvariant()
    if ($h -eq 'localhost' -or $h -eq '127.0.0.1' -or $h -eq '::1') { return $true }
    [System.Net.IPAddress]$ip = $null
    if (-not [System.Net.IPAddress]::TryParse($h, [ref]$ip)) { return $false }
    $b = $ip.GetAddressBytes()
    if ($ip.AddressFamily -eq [System.Net.Sockets.AddressFamily]::InterNetwork -and $b.Length -eq 4) {
        if ($b[0] -eq 10) { return $true }
        if ($b[0] -eq 172 -and $b[1] -ge 16 -and $b[1] -le 31) { return $true }
        if ($b[0] -eq 192 -and $b[1] -eq 168) { return $true }
        if ($b[0] -eq 100 -and $b[1] -ge 64 -and $b[1] -le 127) { return $true } # Tailscale / CGNAT WireGuard
        return $false
    }
    if ($ip.AddressFamily -eq [System.Net.Sockets.AddressFamily]::InterNetworkV6 -and $b.Length -ge 1) {
        return (($b[0] -band 0xFE) -eq 0xFC) # Unique Local IPv6
    }
    return $false
}

if ($Mode -eq 'GenerateHostConfig') {
    if ([string]::IsNullOrWhiteSpace($TargetDir)) { throw 'Pass -TargetDir <coop-server-folder>.' }
    $key = if ([string]::IsNullOrWhiteSpace($FriendKey)) { New-FriendKey } else { $FriendKey.Trim() }
    if ($key.Length -lt 12) { throw 'FriendKey must be at least 12 characters.' }
    if (-not (Test-Path -LiteralPath $TargetDir)) { New-Item -ItemType Directory -Force -Path $TargetDir | Out-Null }

    $cfg = [ordered]@{
        Schema = 1
        PrivateFriendsOnly = $true
        AnnounceSelf = $false
        UPnP = $false
        Port = $Port
        RequireMutualHmacProof = $true
        RequirePinnedOrPrivateNetwork = $true
        FriendKey = $key
        AllowedClientEndpointHint = 'Use LAN IP, localhost, or Tailscale/WireGuard (100.64.x.x)'
    }
    $outFile = Join-Path $TargetDir 'FriendsCoopHost.json'
    [IO.File]::WriteAllText($outFile, ($cfg | ConvertTo-Json -Depth 4), [Text.UTF8Encoding]::new($false))
    Write-Host "Wrote a local friend key record: $outFile"
    Write-Host 'The development server reads GTAXSKATE_COOP_FRIEND_KEY and Settings.xml. It does not load this JSON file.'
    return
}

if ($Mode -eq 'ConfigureClient') {
    if ([string]::IsNullOrWhiteSpace($TargetDir)) { throw 'Pass -TargetDir <GTA-V-folder>.' }
    if (-not (Test-TrustedEndpoint $HostEndpoint)) {
        throw "Blocked insecure endpoint '$HostEndpoint'. Use localhost, private LAN, or Tailscale/WireGuard IP (100.64/10)."
    }
    if ([string]::IsNullOrWhiteSpace($FriendKey) -or $FriendKey.Trim().Length -lt 12) {
        throw 'FriendKey must be at least 12 characters.'
    }
    $scriptsCoop = Join-Path $TargetDir 'scripts\RageCoop'
    New-Item -ItemType Directory -Force -Path $scriptsCoop | Out-Null
    $cfg = [ordered]@{
        Schema = 1
        BlockPublicServerBrowser = $true
        RequireMutualHmacProof = $true
        ShowSessionStatusAlways = $true
        TrustedHostLabel = $HostLabel
        TrustedHostIp = $HostEndpoint.Trim()
        Port = $Port
        FriendKey = $FriendKey.Trim()
    }
    $outFile = Join-Path $scriptsCoop 'FriendsCoopClient.json'
    [IO.File]::WriteAllText($outFile, ($cfg | ConvertTo-Json -Depth 4), [Text.UTF8Encoding]::new($false))
    Write-Host "Configured optional Friends-Only Coop client: $outFile"
    return
}

if ($Mode -eq 'Uninstall') {
    if ([string]::IsNullOrWhiteSpace($TargetDir)) { throw 'Pass -TargetDir <GTA-V-folder>.' }
    $backupDir = Join-Path $TargetDir 'GTAxSkate-coop-backups'
    $receiptPath = Join-Path $backupDir 'optional-coop-receipt.json'
    if (!(Test-Path -LiteralPath $receiptPath)) {
        throw 'No optional-coop-receipt.json found. Use the original installer rollback to preserve shared ScriptHookVDotNet files.'
    }
    $rec = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
    $preExistingDir = Join-Path $backupDir 'optional-coop-prev'
    foreach ($entry in $rec.Entries) {
        $full = Join-Path $TargetDir $entry.RelativePath
        if ($entry.Existed) {
            $bak = Join-Path $preExistingDir $entry.RelativePath
            if (Test-Path -LiteralPath $bak -PathType Leaf) {
                New-Item -ItemType Directory -Force -Path (Split-Path -Parent $full) | Out-Null
                Copy-Item -LiteralPath $bak -Destination $full -Force
            }
        } elseif (Test-Path -LiteralPath $full -PathType Leaf) {
            Remove-Item -LiteralPath $full -Force
        }
    }
    if (Test-Path -LiteralPath $preExistingDir) { Remove-Item -LiteralPath $preExistingDir -Recurse -Force }
    $rageDir = Join-Path $TargetDir 'scripts\RageCoop'
    # Streamer-safe coop (design 2026-11-17-streamer-safe-coop): while multiplayer is on, RageCoop's own client UI is locked in
    # scripts\RageCoop\Data\RageCoop.Client.Settings.xml (menu key F24, no server browser, no address, name Skater). The user's
    # own values are in a sidecar; put them back before the folder is cleaned (same rules as RageCoopClientSettings.Restore).
    Restore-RageCoopClientSettings (Join-Path $rageDir 'Data\RageCoop.Client.Settings.xml')
    # Our own opt-in preference file (never part of the receipt); removing it keeps uninstall clean.
    $prefsFile = Join-Path $rageDir 'FriendsCoopPrefs.json'
    if (Test-Path -LiteralPath $prefsFile -PathType Leaf) { Remove-Item -LiteralPath $prefsFile -Force }
    if ((Test-Path -LiteralPath $rageDir) -and (@(Get-ChildItem -LiteralPath $rageDir -Recurse -File).Count -eq 0)) {
        Remove-Item -LiteralPath $rageDir -Recurse -Force
    }
    Remove-Item -LiteralPath $receiptPath -Force
    Write-Host "Uninstalled optional Friends-Only Coop from $TargetDir cleanly (shared ScriptHookVDotNet and prior files preserved)."
    return
}

if ($Mode -eq 'PrepareOptionalPack') {
    $repoRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
    $packRoot = if ([string]::IsNullOrWhiteSpace($TargetDir)) { Join-Path $repoRoot 'build\coop-optional-pack' } else { $TargetDir }
    $readyDir = Join-Path $repoRoot 'build\coop-test-ready-20261015'
    $adapterRoot = Join-Path $repoRoot 'build\coop-adapter'
    $receipt = Join-Path $adapterRoot 'adapter-build.json'
    if (!(Test-Path -LiteralPath $receipt)) { throw 'Missing adapter-build.json. Run coop/gtaxskate/build.ps1 first.' }
    $liveVerified = ((Get-Content -LiteralPath $receipt -Raw | ConvertFrom-Json).LiveTransportVerified -eq $true)
    if (-not $liveVerified -and -not $AllowDevPack) {
        throw 'The Friends-Only Coop adapter is a development build. Live transport verification is required before staging an installable pack (or pass -AllowDevPack for local testing).'
    }
    if (-not $liveVerified) {
        Write-Warning 'Staging development Friends-Only Coop pack (-AllowDevPack): live transport verification in GTA V is still pending.'
    }
    $clientResource = 'Resources\Client\GTAxSkate.Coop.Client\GTAxSkate.Coop.Client.dll'
    $serverResource = 'Resources\Server\GTAxSkate.Coop.Server\GTAxSkate.Coop.Server.dll'
    $adapterDll = Join-Path $adapterRoot $clientResource
    $bootstrapDll = Join-Path $adapterRoot 'GTAxSkate.Coop.Bootstrap.dll'
    $serverAdapterDll = Join-Path $adapterRoot $serverResource
    if (!(Test-Path -LiteralPath $adapterDll) -or !(Test-Path -LiteralPath $bootstrapDll) -or !(Test-Path -LiteralPath $serverAdapterDll)) { throw 'The matching client, bootstrap, and server adapters are required.' }
    if (!(Test-Path -LiteralPath $readyDir)) { throw "Pinned coop-test-ready package missing at $readyDir" }

    $clientStage = Join-Path $packRoot 'client-payload'
    $serverStage = Join-Path $packRoot 'server'
    New-Item -ItemType Directory -Force -Path $clientStage, $serverStage | Out-Null
    Copy-Item -Path (Join-Path $readyDir 'payload\*') -Destination $clientStage -Recurse -Force
    $clientRageDir = Join-Path $clientStage 'scripts\RageCoop'
    New-Item -ItemType Directory -Force -Path $clientRageDir | Out-Null
    Copy-Item -LiteralPath $bootstrapDll -Destination (Join-Path $clientRageDir 'GTAxSkate.Coop.Bootstrap.dll') -Force
    $lidgrenDll = Join-Path $repoRoot 'external\ragecoop\libs\Lidgren.Network.dll'
    if (Test-Path -LiteralPath $lidgrenDll) {
        Copy-Item -LiteralPath $lidgrenDll -Destination (Join-Path $clientRageDir 'Lidgren.Network.dll') -Force
    }
    $clientResDir = Join-Path $clientRageDir 'Resources\GTAxSkate.Coop.Client'
    New-Item -ItemType Directory -Force -Path $clientResDir | Out-Null
    Copy-Item -LiteralPath $adapterDll -Destination (Join-Path $clientResDir 'GTAxSkate.Coop.Client.dll') -Force

    Copy-Item -Path (Join-Path $readyDir 'server\*') -Destination $serverStage -Recurse -Force
    Remove-Item -LiteralPath (Join-Path $serverStage 'Resources\Client\GTAxSkate.Coop.Client') -Recurse -Force -ErrorAction SilentlyContinue  # stale from older staging (Copilot review)
    # No client resource on the server (2026-11-16 test 21:57): every player already has the client adapter
    # (bootstrap), and RageCoop loading a second, downloaded copy right after connect broke the friend check.
    Copy-Item -LiteralPath $serverAdapterDll -Destination (Join-Path $serverStage $serverResource) -Force

    # Stage an embedded HostServer inside client-payload\scripts\RageCoop\HostServer so players can host directly in-game via F11.
    $inGameHostDir = Join-Path $clientStage 'scripts\RageCoop\HostServer'
    New-Item -ItemType Directory -Force -Path (Join-Path $inGameHostDir 'Resources\Server\GTAxSkate.Coop.Server') | Out-Null
    Copy-Item -Path (Join-Path $readyDir 'server\*') -Destination $inGameHostDir -Recurse -Force
    Remove-Item -LiteralPath (Join-Path $inGameHostDir 'Resources\Client\GTAxSkate.Coop.Client') -Recurse -Force -ErrorAction SilentlyContinue
    Copy-Item -LiteralPath $serverAdapterDll -Destination (Join-Path $inGameHostDir $serverResource) -Force
    $genKey = New-FriendKey
    $hostCfg = [ordered]@{
        Schema = 1
        PrivateFriendsOnly = $true
        AnnounceSelf = $false
        UPnP = $false
        Port = $Port
        MaxPlayers = 10
        RequireMutualHmacProof = $true
        RequirePinnedOrPrivateNetwork = $true
        FriendKey = $genKey
        AllowedClientEndpointHint = 'Use LAN IP, localhost, or Tailscale/WireGuard (100.64.x.x)'
    }
    [IO.File]::WriteAllText((Join-Path $serverStage 'FriendsCoopHost.json'), ($hostCfg | ConvertTo-Json -Depth 4), [Text.UTF8Encoding]::new($false))
    $serverLauncher = @'
$ErrorActionPreference = 'Stop'
$cfg = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'FriendsCoopHost.json') -Raw | ConvertFrom-Json
if ([string]::IsNullOrWhiteSpace($cfg.FriendKey) -or $cfg.FriendKey.Length -lt 12) {
    throw 'Invalid FriendKey in FriendsCoopHost.json.'
}
$env:GTAXSKATE_COOP_FRIEND_KEY = $cfg.FriendKey
$settingsPath = Join-Path $PSScriptRoot 'Settings.xml'
if (!(Test-Path -LiteralPath $settingsPath)) {
    $port = if ($cfg.Port) { [int]$cfg.Port } else { 4499 }
    $maxPlayers = if ($cfg.MaxPlayers) { [int]$cfg.MaxPlayers } else { 10 }
    $xml = @"
<?xml version="1.0" encoding="utf-8"?>
<Settings xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <Port>$port</Port>
  <MaxPlayers>$maxPlayers</MaxPlayers>
  <AnnounceSelf>false</AnnounceSelf>
  <UseUPnP>false</UseUPnP>
  <WelcomeMessage>GTAxSkate Friends-Only Coop (Private Lobby)</WelcomeMessage>
</Settings>
"@
    [IO.File]::WriteAllText($settingsPath, $xml, [Text.UTF8Encoding]::new($false))
}
Write-Host "Starting GTAxSkate Friends-Only Coop Server on port $($cfg.Port) (MaxPlayers: 10, PrivateFriendsOnly: true)..."
& (Join-Path $PSScriptRoot 'RageCoop.Server.exe')
'@
    [IO.File]::WriteAllText((Join-Path $serverStage 'Start-FriendsCoopServer.ps1'), $serverLauncher, [Text.UTF8Encoding]::new($false))
    Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $packRoot 'Setup-FriendsCoop.ps1') -Force

    $installer = @'
param(
    [ValidateSet('Install', 'Uninstall')]
    [string]$Mode = 'Install',
    [string]$GtaDir = '',
    [string]$HostIp = '127.0.0.1',
    [string]$FriendKey = '',
    [string]$HostLabel = 'Friends-VPN',
    [int]$Port = 4499
)
$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($GtaDir) -or !(Test-Path -LiteralPath (Join-Path $GtaDir 'GTA5.exe'))) {
    throw 'Pass -GtaDir <GTA V folder containing GTA5.exe>'
}
if (Get-Process GTA5, GTA5_Enhanced, FiveM, OpenIV, CodeWalker -ErrorAction SilentlyContinue) {
    throw 'Close GTA V, OpenIV, and CodeWalker first.'
}
if ($Mode -eq 'Uninstall') {
    & (Join-Path $PSScriptRoot 'Setup-FriendsCoop.ps1') -Mode Uninstall -TargetDir $GtaDir
    return
}
if (Test-Path -LiteralPath (Join-Path $GtaDir 'GTAxSkate-coop-backups\optional-coop-receipt.json')) {
    Write-Host "Removing prior Friends-Only Coop install before applying updated pack..."
    & (Join-Path $PSScriptRoot 'Setup-FriendsCoop.ps1') -Mode Uninstall -TargetDir $GtaDir
}
$key = if ([string]::IsNullOrWhiteSpace($FriendKey)) {
    (Get-Content -LiteralPath (Join-Path $PSScriptRoot 'server\FriendsCoopHost.json') -Raw | ConvertFrom-Json).FriendKey
} else { $FriendKey }

$backupDir = Join-Path $GtaDir 'GTAxSkate-coop-backups'
$preExistingDir = Join-Path $backupDir 'optional-coop-prev'
New-Item -ItemType Directory -Force -Path $backupDir | Out-Null

$payloadRoot = Join-Path $PSScriptRoot 'client-payload'
$entries = New-Object System.Collections.Generic.List[object]
$shvdnShared = @('ScriptHookVDotNet.asi', 'ScriptHookVDotNet2.dll', 'ScriptHookVDotNet3.dll')
foreach ($file in Get-ChildItem -LiteralPath $payloadRoot -Recurse -File) {
    $rel = $file.FullName.Substring($payloadRoot.Length).TrimStart('\')
    $dest = Join-Path $GtaDir $rel
    $existed = Test-Path -LiteralPath $dest -PathType Leaf
    if (($shvdnShared -contains $rel) -and $existed) {
        Write-Host "Preserving existing $rel in GTA folder."
        continue
    }
    if ($existed) {
        $bak = Join-Path $preExistingDir $rel
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $bak) | Out-Null
        Copy-Item -LiteralPath $dest -Destination $bak -Force
    }
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $dest) | Out-Null
    Copy-Item -LiteralPath $file.FullName -Destination $dest -Force
    $entries.Add([ordered]@{ RelativePath = $rel; Existed = $existed })
}

$cfgRel = 'scripts\RageCoop\FriendsCoopClient.json'
$cfgDest = Join-Path $GtaDir $cfgRel
$cfgExisted = Test-Path -LiteralPath $cfgDest -PathType Leaf
if ($cfgExisted) {
    $cfgBak = Join-Path $preExistingDir $cfgRel
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $cfgBak) | Out-Null
    Copy-Item -LiteralPath $cfgDest -Destination $cfgBak -Force
}
& (Join-Path $PSScriptRoot 'Setup-FriendsCoop.ps1') -Mode ConfigureClient -TargetDir $GtaDir -HostEndpoint $HostIp -FriendKey $key -HostLabel $HostLabel -Port $Port
$entries.Add([ordered]@{ RelativePath = $cfgRel; Existed = $cfgExisted })

$receiptObj = [ordered]@{
    Schema = 2
    InstalledUtc = [DateTime]::UtcNow.ToString('o')
    Entries = $entries.ToArray()
}
[IO.File]::WriteAllText((Join-Path $backupDir 'optional-coop-receipt.json'), ($receiptObj | ConvertTo-Json -Depth 4), [Text.UTF8Encoding]::new($false))
Write-Host "Installed optional Friends-Only Coop into $GtaDir cleanly (receipt saved for uninstall)."
'@
    [IO.File]::WriteAllText((Join-Path $packRoot 'Install-OptionalCoop.ps1'), $installer, [Text.UTF8Encoding]::new($false))
    Write-Host "Prepared optional Friends-Only Coop bundle at: $packRoot"
    return
}

# Mode -eq VerifySecurity
Write-Host 'Friends-Only Coop: offline verification only. Live transport security is pending.'
Write-Host 'Test suites available in coop/gtaxskate/tests/: run.ps1, run-session.ps1, run-adapter.ps1'
