# Repeatable, isolated integration checks for scripts\install.ps1.
# Runs the real installer in a disposable fixture with all expensive stages
# receipted. Never points at the user's GTA folder or music profile.
param(
    [Parameter(Mandatory)] [string] $GtaExeSource,
    # Exercise the real verified-cache route without requiring a download.
    [string] $BinArchive = '',
    # Exact test-build manifest for a new release; omission retains v0.6.0.
    [string] $ExpectedManifest = '',
    # Test the actual staged Setup installer with its candidate-specific bin pin.
    [string] $InstallerSource = ''
)
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$qaRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot 'build\qa-2026-10-13\installer'))
$qaPrefix = $qaRoot.TrimEnd('\') + '\'
function Assert([bool] $condition, [string] $message) { if (-not $condition) { throw $message } }
function Assert-UnderQa([string] $path) {
    $full = [IO.Path]::GetFullPath($path)
    if (-not [string]::Equals($full, $qaRoot, [StringComparison]::OrdinalIgnoreCase) -and
        -not $full.StartsWith($qaPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing a path outside the QA fixture root: $full"
    }
    return $full
}
function Assert-NoReparse([string] $path) {
    $full = Assert-UnderQa $path
    if (Test-Path -LiteralPath $full) {
        $item = Get-Item -LiteralPath $full -Force
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Refusing reparse point: $full" }
    }
}
Assert-NoReparse $qaRoot
for ($ancestor = $qaRoot; ; $ancestor = Split-Path -Parent $ancestor) {
    if (Test-Path -LiteralPath $ancestor) {
        $item = Get-Item -LiteralPath $ancestor -Force
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Refusing a reparse point in the QA path: $ancestor" }
    }
    if ([string]::Equals($ancestor, $repoRoot, [StringComparison]::OrdinalIgnoreCase)) { break }
    if (-not $ancestor.StartsWith($repoRoot.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)) { throw "QA path escaped the repository: $ancestor" }
}
New-Item -ItemType Directory -Force -Path $qaRoot | Out-Null
$runRoot = Join-Path $qaRoot ((Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
Assert-UnderQa $runRoot | Out-Null
New-Item -ItemType Directory -Path $runRoot | Out-Null

$source = [IO.Path]::GetFullPath($GtaExeSource)
Assert ([string]::Equals((Split-Path -Leaf $source), 'GTA5.exe', [StringComparison]::OrdinalIgnoreCase)) 'GTA fixture source must be GTA5.exe.'
Assert (Test-Path -LiteralPath $source -PathType Leaf) "Read-only GTA5.exe fixture is missing: $source"
$sourceExe = Get-Item -LiteralPath $source
Assert (($sourceExe.Attributes -band [IO.FileAttributes]::ReparsePoint) -eq 0) 'Refusing a reparse-point GTA5.exe source.'
Assert ($sourceExe.VersionInfo.FileVersion -eq '1.0.3889.0') "Expected GTA V 1.0.3889.0, found '$($sourceExe.VersionInfo.FileVersion)'."
$sourceHash = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash

$caseRoot = Join-Path $runRoot 'fixture'
$fixtureRepo = Join-Path $caseRoot 'repo'
$fixtureScripts = Join-Path $fixtureRepo 'scripts'
$fixtureBuild = Join-Path $fixtureRepo 'build'
$receiptRoot = Join-Path $fixtureBuild 'install'
$fixtureBin = Join-Path $fixtureBuild 'bin'
$fixtureGta = Join-Path $caseRoot 'GTA-test-only'
$fixtureSkate3 = Join-Path $caseRoot 'Skate3-empty-test-only'
# Follow the installer's pinned SK8V release so the fixture never goes stale.
$pinnedSk8v = [regex]::Match([IO.File]::ReadAllText((Join-Path $PSScriptRoot 'install.ps1')), '(?m)^\s*\$Sk8vTag\s*=\s*''([^'']+)''').Groups[1].Value
Assert ([bool]$pinnedSk8v) 'Could not read $Sk8vTag from install.ps1.'
$release = Join-Path $fixtureRepo "external\sk8v-release\SK8V-$pinnedSk8v"
$fixtureInstaller = Join-Path $fixtureScripts 'install.ps1'
$liveInstaller = Join-Path $PSScriptRoot 'install.ps1'
if ($InstallerSource) {
    $liveInstaller = [IO.Path]::GetFullPath($InstallerSource)
    $candidatePrefix = [IO.Path]::GetFullPath((Join-Path $repoRoot 'build')).TrimEnd('\') + '\'
    Assert ($liveInstaller.StartsWith($candidatePrefix, [StringComparison]::OrdinalIgnoreCase)) 'Candidate installer must be staged under the workspace build directory.'
    Assert ((Split-Path -Leaf $liveInstaller) -eq 'install.ps1') 'Candidate installer filename must be install.ps1.'
    Assert (Test-Path -LiteralPath $liveInstaller -PathType Leaf) 'Candidate installer is missing.'
}
$powershell51 = Join-Path $env:WINDIR 'System32\WindowsPowerShell\v1.0\powershell.exe'
Assert (Test-Path -LiteralPath $powershell51 -PathType Leaf) "Windows PowerShell 5.1 not found: $powershell51"
Assert (-not (Get-Process GTA5 -ErrorAction SilentlyContinue)) 'Refusing to run while GTA5.exe is running.'

# Establish and verify the only writable fixture tree. The GTA executable is
# copied byte-for-byte from E:, and the source remains read-only to this harness.
New-Item -ItemType Directory -Force -Path $fixtureScripts, $fixtureBin, $receiptRoot,
    (Join-Path $fixtureRepo 'tools'), (Join-Path $release 'python'),
    (Join-Path $release 'build\tools'), $fixtureGta, $fixtureSkate3 | Out-Null
Copy-Item -LiteralPath $liveInstaller -Destination $fixtureInstaller
$fixtureCollectibles = Join-Path $fixtureScripts 'collectibles'
New-Item -ItemType Directory -Force -Path $fixtureCollectibles | Out-Null
$sourceSongsBack = Join-Path (Split-Path -Parent $liveInstaller) 'collectibles\songs-back.ps1'
Copy-Item -LiteralPath $sourceSongsBack -Destination (Join-Path $fixtureCollectibles 'songs-back.ps1')
Assert ((Get-FileHash -LiteralPath $sourceSongsBack).Hash -eq (Get-FileHash -LiteralPath (Join-Path $fixtureCollectibles 'songs-back.ps1')).Hash) 'Fixture song helper is not an exact copy.'
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'install-asset-transaction.ps1') -Destination $fixtureScripts
Assert ((Get-FileHash -LiteralPath $liveInstaller -Algorithm SHA256).Hash -eq (Get-FileHash -LiteralPath $fixtureInstaller -Algorithm SHA256).Hash) 'Fixture installer is not an exact copy of the selected installer.'
Copy-Item -LiteralPath $source -Destination (Join-Path $fixtureGta 'GTA5.exe')
Assert ((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash -eq (Get-FileHash -LiteralPath (Join-Path $fixtureGta 'GTA5.exe') -Algorithm SHA256).Hash) 'GTA5.exe fixture copy failed its hash check.'
[IO.File]::WriteAllText((Join-Path $fixtureBin 'SkateVLegacy.asi'), 'fixture plugin binary')
[IO.File]::WriteAllText((Join-Path $fixtureBin 'SkateVRuntime.dll'), 'fixture runtime binary')
[IO.File]::WriteAllText((Join-Path $release 'python\python.exe'), 'receipted placeholder; never executed')
[IO.File]::WriteAllText((Join-Path $release 'build\tools\rage.exe'), 'receipted placeholder; never executed')

# A deliberate read-only music mock: it records invocation inside the fixture,
# reports missing Skate 3 audio files, and can never resolve/write Documents.
$musicMock = @'
param([string]$Game, [string]$Exe)
$repo = Split-Path -Parent $PSScriptRoot
$marker = Join-Path $repo 'build\install\mock-music-called.txt'
$pythonHome = [Environment]::GetEnvironmentVariable('PYTHONHOME', 'Process')
$pythonPath = [Environment]::GetEnvironmentVariable('PYTHONPATH', 'Process')
$pythonNoUserSite = [Environment]::GetEnvironmentVariable('PYTHONNOUSERSITE', 'Process')
if ($null -eq $pythonHome) { $pythonHome = '<unset>' }
if ($null -eq $pythonPath) { $pythonPath = '<unset>' }
if ($null -eq $pythonNoUserSite) { $pythonNoUserSite = '<unset>' }
[IO.File]::WriteAllText($marker, "Game=$Game`r`nExe=$Exe`r`nPYTHONHOME=$pythonHome`r`nPYTHONPATH=$pythonPath`r`nPYTHONNOUSERSITE=$pythonNoUserSite")
foreach ($name in 'data\audio\music\ipod.mpf', 'data\audio\music\Ipod_Stream.mus') {
    if (-not (Test-Path -LiteralPath (Join-Path $Game $name))) { throw "fixture missing disc file: $name" }
}
if (Test-Path -LiteralPath (Join-Path $repo 'build\install\mock-music-success.txt')) { return }
throw 'fixture expected the absent soundtrack converter; no profile output was touched'
'@
[IO.File]::WriteAllText((Join-Path $fixtureScripts 'get-skate-music.ps1'), $musicMock)

# Receipts skip tools/downloads, SK8V setup, binary acquisition, and board
# export. Step 8 still executes the real manifest, binary, and INI code.
foreach ($step in 'tools', 'sk8v', 'bin', 'boards', 'world') {
    [IO.File]::WriteAllText((Join-Path $receiptRoot "$step.done"), 'cached integration prerequisite')
}
$settings = [pscustomobject]@{ Gta = $fixtureGta; Skate3 = $fixtureSkate3; Stance = 'Regular'; Parts = 'keys,camera' }
[IO.File]::WriteAllText((Join-Path $receiptRoot 'folders.json'), ($settings | ConvertTo-Json), (New-Object Text.UTF8Encoding $false))
$probe = Join-Path $fixtureGta '.qa-write-probe'
[IO.File]::WriteAllBytes($probe, [byte[]]@())
Remove-Item -LiteralPath $probe -Force

function Write-Manifest([string[]] $entries, [string] $path) {
    $files = [pscustomobject]@{}
    foreach ($entry in $entries) { $files | Add-Member -NotePropertyName $entry -NotePropertyValue 'fixture-hash' }
    $manifest = [pscustomobject]@{ format = 'sk8v-install'; version = 1; gta = '1.0.3889.0'; files = $files }
    [IO.File]::WriteAllText($path, ($manifest | ConvertTo-Json -Depth 5), (New-Object Text.UTF8Encoding $false))
}
function Reset-Target([bool] $withManifest = $false) {
    foreach ($name in 'SkateVLegacy.asi', 'SkateVRuntime.dll', 'SkateVLegacy.ini', 'SkateVLegacy.installed.json',
        'SkateVLegacy.components.txt', 'SkateVLegacy.installed.json.bak', 'SkateVLegacy.installed.json.new') {
        $path = Join-Path $fixtureGta $name
        Assert-UnderQa $path | Out-Null
        if (Test-Path -LiteralPath $path -PathType Leaf) { Remove-Item -LiteralPath $path -Force }
    }
    if ($withManifest) { Write-Manifest @() (Join-Path $fixtureGta 'SkateVLegacy.installed.json') }
    Remove-Item -LiteralPath (Join-Path $receiptRoot 'install.done'), (Join-Path $receiptRoot 'music.done'),
        (Join-Path $receiptRoot 'mock-music-called.txt') -Force -ErrorAction SilentlyContinue
    foreach ($name in 'SkateVLegacy.asi', 'SkateVRuntime.dll') {
        [IO.File]::WriteAllText((Join-Path $fixtureBin $name), "fixture new $name")
    }
}
function Invoke-Installer([string] $label, [string] $parts, [switch] $NoMusic, [switch] $PoisonPythonEnvironment, [string] $Skate3Path = '', [string] $Redo = '', [string] $Components = '', [string] $Remove = '', [switch] $WithTutorial, [switch] $WithCoop) {
    if (-not $Skate3Path) { $Skate3Path = $fixtureSkate3 }
    $logPath = Join-Path $runRoot ("logs\$label")
    New-Item -ItemType Directory -Force -Path $logPath | Out-Null
    $console = Join-Path $logPath 'console.txt'
    $psArgs = @('-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', $fixtureInstaller,
        '-Gta', $fixtureGta, '-Skate3', $Skate3Path, '-Parts', $parts)
    if ($Components) { $psArgs += @('-Components', $Components) }
    if ($Remove) { $psArgs += @('-Remove', $Remove) }
    if ($WithTutorial) { $psArgs += '-WithTutorial' }
    if ($WithCoop) { $psArgs += '-WithCoop' }
    if ($NoMusic) { $psArgs += '-NoMusic' }
    if ($Redo) { $psArgs += @('-Redo', $Redo) }
    $oldPythonHome = [Environment]::GetEnvironmentVariable('PYTHONHOME', 'Process')
    $oldPythonPath = [Environment]::GetEnvironmentVariable('PYTHONPATH', 'Process')
    $oldPythonNoUserSite = [Environment]::GetEnvironmentVariable('PYTHONNOUSERSITE', 'Process')
    try {
        if ($PoisonPythonEnvironment) {
            [Environment]::SetEnvironmentVariable('PYTHONHOME', 'C:\QA-poisoned-python-home', 'Process')
            [Environment]::SetEnvironmentVariable('PYTHONPATH', 'C:\QA-poisoned-python-path', 'Process')
            [Environment]::SetEnvironmentVariable('PYTHONNOUSERSITE', '0', 'Process')
        }
        $output = @(& $powershell51 @psArgs 2>&1 | ForEach-Object { $_.ToString() })
        $exitCode = $LASTEXITCODE
    } finally {
        [Environment]::SetEnvironmentVariable('PYTHONHOME', $oldPythonHome, 'Process')
        [Environment]::SetEnvironmentVariable('PYTHONPATH', $oldPythonPath, 'Process')
        [Environment]::SetEnvironmentVariable('PYTHONNOUSERSITE', $oldPythonNoUserSite, 'Process')
    }
    [IO.File]::WriteAllText($console, ($output -join [Environment]::NewLine), (New-Object Text.UTF8Encoding $false))
    $transcript = Join-Path $receiptRoot 'install.log'
    if (Test-Path -LiteralPath $transcript -PathType Leaf) { Copy-Item -LiteralPath $transcript -Destination (Join-Path $logPath 'install-transcript.log') -Force }
    return [pscustomobject]@{ ExitCode = $exitCode; Output = ($output -join [Environment]::NewLine); Log = $console }
}
$results = New-Object System.Collections.Generic.List[object]
function Run-Scenario([string] $name, [scriptblock] $test) {
    try { & $test; $results.Add([pscustomobject]@{ scenario = $name; result = 'PASS'; coverage = 'installer real; external stages cached or mocked'; detail = 'assertions satisfied' }) }
    catch { $results.Add([pscustomobject]@{ scenario = $name; result = 'FAIL'; coverage = 'installer real; external stages cached or mocked'; detail = $_.Exception.Message }); Write-Host "FAIL $name -- $($_.Exception.Message)" -ForegroundColor Red }
}

Run-Scenario 'fresh-install-cached-steps' {
    Reset-Target
    $run = Invoke-Installer '01-fresh' 'keys,camera' -NoMusic
    Assert ($run.ExitCode -eq 0) "Installer exit $($run.ExitCode): $($run.Output)"
    foreach ($name in 'SkateVLegacy.asi', 'SkateVRuntime.dll') {
        Assert ((Get-FileHash (Join-Path $fixtureBin $name) -Algorithm SHA256).Hash -eq (Get-FileHash (Join-Path $fixtureGta $name) -Algorithm SHA256).Hash) "$name was not copied byte-for-byte."
    }
    $record = Get-Content (Join-Path $fixtureGta 'SkateVLegacy.installed.json') -Raw | ConvertFrom-Json
    Assert ($record.format -eq 'sk8v-install' -and $record.gta -eq '1.0.3889.0') 'Fresh manifest schema/build is wrong.'
    foreach ($name in 'SkateVLegacy.asi', 'SkateVRuntime.dll') {
        $expected = (Get-FileHash (Join-Path $fixtureGta $name) -Algorithm SHA256).Hash.ToLowerInvariant()
        Assert ($record.files.$name -eq $expected) "Manifest hash missing or wrong for $name."
    }
    $ini = [IO.File]::ReadAllText((Join-Path $fixtureGta 'SkateVLegacy.ini'))
    Assert ($ini -match '(?m)^Keyboard=1\r?$') 'Selected keyboard part was not written to INI.'
    Assert (Test-Path (Join-Path $receiptRoot 'install.done')) 'Install receipt was not written.'
}

Run-Scenario 'prebaked-map-failure-is-optional' {
    # No prepared world receipt and a map builder that fails: the installer warns,
    # writes no world receipt and still completes the install step.
    Reset-Target
    Remove-Item -LiteralPath (Join-Path $receiptRoot 'world.done') -Force -ErrorAction SilentlyContinue
    $fakeKeys = Join-Path $fixtureGta 'SK8V\gta\gta-keys'
    New-Item -ItemType Directory -Force -Path $fakeKeys | Out-Null
    [IO.File]::WriteAllText((Join-Path $fakeKeys 'gtav_aes_key.dat'), 'fixture key')
    [IO.File]::WriteAllText((Join-Path $fixtureScripts 'build-prebaked-map.ps1'), "param([string]`$Gta, [string]`$Out, [string]`$Rpf, [string]`$Keys, [string]`$Baker, [string]`$World)`r`nthrow 'fixture map builder failure'")
    [IO.File]::WriteAllText((Join-Path $fixtureBin 'skatev-world-cache.exe'), 'fixture baker')
    $run = Invoke-Installer '01b-map-fails' 'keys,camera' -NoMusic
    Assert ($run.ExitCode -eq 0) "Map failure stopped the installer (exit $($run.ExitCode)): $($run.Output)"
    Assert ($run.Output -match 'the prebaked map was skipped: fixture map builder failure') 'The map failure was not reported as an optional skip.'
    Assert (-not (Test-Path (Join-Path $receiptRoot 'world.done'))) 'A failed map was receipted as done.'
    Assert (Test-Path (Join-Path $receiptRoot 'install.done')) 'Installer did not continue through the install step after a map failure.'
    [IO.File]::WriteAllText((Join-Path $receiptRoot 'world.done'), 'cached integration prerequisite')
    $redoRun = Invoke-Installer '01c-map-redo-fails' 'keys,camera' -NoMusic -Redo 'world'
    Assert ($redoRun.ExitCode -eq 0) "Map redo failure stopped the installer (exit $($redoRun.ExitCode)): $($redoRun.Output)"
    Assert (-not (Test-Path (Join-Path $receiptRoot 'world.done'))) 'Failed -Redo world left stale world.done receipt behind.'
    [IO.File]::WriteAllText((Join-Path $receiptRoot 'world.done'), 'cached integration prerequisite')
}

Run-Scenario 'resume-with-cached-stages' {
    $run = Invoke-Installer '02-resume' 'keys,camera' -NoMusic
    Assert ($run.ExitCode -eq 0) "Resume exit $($run.ExitCode): $($run.Output)"
    Assert ($run.Output -notmatch 'Running SK8V''s setup|Exporting the board parts|downloading') 'Resume reran an expensive uncached stage.'
    Assert ((Get-Content (Join-Path $fixtureGta 'SkateVLegacy.installed.json') -Raw | ConvertFrom-Json).format -eq 'sk8v-install') 'Resume damaged the manifest.'
}

Run-Scenario 'NoMusic-with-existing-music-receipt' {
    [IO.File]::WriteAllText((Join-Path $receiptRoot 'music.done'), 'pretend soundtrack is already installed')
    [IO.File]::WriteAllText((Join-Path $fixtureGta 'SkateVLegacy.ini'), "[SkateV]`r`n")
    Remove-Item -LiteralPath (Join-Path $receiptRoot 'mock-music-called.txt') -Force -ErrorAction SilentlyContinue
    $run = Invoke-Installer '03-no-music' 'keys,music' -NoMusic
    Assert ($run.ExitCode -eq 0) "NoMusic exit $($run.ExitCode): $($run.Output)"
    $ini = [IO.File]::ReadAllText((Join-Path $fixtureGta 'SkateVLegacy.ini'))
    Assert ($ini -notmatch '(?m)^SkateRadio=') 'NoMusic wrote SkateRadio to the INI.'
    Assert ($run.Output -notmatch 'Perform Full Scan|Self Radio Mode: Radio') 'NoMusic printed the Self Radio scan instruction.'
    Assert (-not (Test-Path (Join-Path $receiptRoot 'mock-music-called.txt'))) 'NoMusic invoked the music script.'
}

Run-Scenario 'safe-nested-manifest-paths' {
    Reset-Target -withManifest $true
    Write-Manifest @('nested\owned.dll', 'nested/second-owned.dll') (Join-Path $fixtureGta 'SkateVLegacy.installed.json')
    $run = Invoke-Installer '04-safe-nested' 'keys,camera' -NoMusic
    Assert ($run.ExitCode -eq 0) "Safe nested manifest paths were rejected: $($run.Output)"
    $record = Get-Content (Join-Path $fixtureGta 'SkateVLegacy.installed.json') -Raw | ConvertFrom-Json
    Assert ($record.files.'nested\owned.dll' -eq 'fixture-hash' -and $record.files.'nested/second-owned.dll' -eq 'fixture-hash') 'Safe nested entries were not preserved.'
}

Run-Scenario 'invalid-rooted-and-traversal-manifest-paths' {
    $sentinel = Join-Path $caseRoot 'outside-gta-sentinel.txt'
    [IO.File]::WriteAllText($sentinel, 'must remain unchanged')
    $badIndex = 0
    foreach ($bad in '..\outside-gta-sentinel.txt', 'nested\..\outside-gta-sentinel.txt', 'C:\outside.txt', 'C:drive-relative.txt', '\rooted.txt', '.') {
        $badIndex++
        Reset-Target
        [IO.File]::WriteAllText((Join-Path $fixtureGta 'SkateVLegacy.asi'), 'old plugin')
        [IO.File]::WriteAllText((Join-Path $fixtureGta 'SkateVRuntime.dll'), 'old runtime')
        Write-Manifest @($bad) (Join-Path $fixtureGta 'SkateVLegacy.installed.json')
        $manifestBefore = [IO.File]::ReadAllText((Join-Path $fixtureGta 'SkateVLegacy.installed.json'))
        $run = Invoke-Installer ('05-reject-' + ('{0:D2}' -f $badIndex)) 'keys,camera' -NoMusic
        Assert ($run.ExitCode -ne 0) "Unsafe manifest entry was accepted: $bad"
        Assert ([IO.File]::ReadAllText((Join-Path $fixtureGta 'SkateVLegacy.asi')) -eq 'old plugin') "ASI changed before rejecting: $bad"
        Assert ([IO.File]::ReadAllText((Join-Path $fixtureGta 'SkateVRuntime.dll')) -eq 'old runtime') "Runtime changed before rejecting: $bad"
        Assert ([IO.File]::ReadAllText((Join-Path $fixtureGta 'SkateVLegacy.installed.json')) -eq $manifestBefore) "Manifest changed while rejecting: $bad"
        Assert ([IO.File]::ReadAllText($sentinel) -eq 'must remain unchanged') "Traversal touched the sentinel for: $bad"
    }
}

Run-Scenario 'cached-bin-receipt-missing-runtime-forces-rollback' {
    Reset-Target
    [IO.File]::WriteAllText((Join-Path $fixtureGta 'SkateVLegacy.asi'), 'old plugin')
    [IO.File]::WriteAllText((Join-Path $fixtureGta 'SkateVRuntime.dll'), 'old runtime')
    Write-Manifest @('prior.dll') (Join-Path $fixtureGta 'SkateVLegacy.installed.json')
    $before = [IO.File]::ReadAllText((Join-Path $fixtureGta 'SkateVLegacy.installed.json'))
    $heldDll = Join-Path $fixtureBin 'SkateVRuntime.dll.held-for-failure'
    Move-Item -LiteralPath (Join-Path $fixtureBin 'SkateVRuntime.dll') -Destination $heldDll
    try {
        $run = Invoke-Installer '06-forced-rollback' 'keys,camera' -NoMusic
        Assert ($run.ExitCode -ne 0) 'Missing fixture runtime unexpectedly succeeded.'
        Assert ($run.Output -match '(?m)^\[3/8\] GTAxSkate builds\r?\n  ready') 'The prepared bin.done receipt did not skip binary acquisition as expected.'
        Assert ([IO.File]::ReadAllText((Join-Path $fixtureGta 'SkateVLegacy.asi')) -eq 'old plugin') 'ASI was not restored after forced failure.'
        Assert ([IO.File]::ReadAllText((Join-Path $fixtureGta 'SkateVRuntime.dll')) -eq 'old runtime') 'Runtime was not restored after forced failure.'
        Assert ([IO.File]::ReadAllText((Join-Path $fixtureGta 'SkateVLegacy.installed.json')) -eq $before) 'Manifest changed after forced failure.'
        Assert (-not (Get-ChildItem -LiteralPath $fixtureGta -Force -Filter '*.install-backup-*' | Select-Object -First 1)) 'Successful rollback left a file backup behind.'
    } finally { Move-Item -LiteralPath $heldDll -Destination (Join-Path $fixtureBin 'SkateVRuntime.dll') -Force }
}

Run-Scenario 'optional-music-missing-disc-path' {
    Reset-Target
    $missingDisc = Join-Path $caseRoot 'disc-intentionally-absent'
    Assert (-not (Test-Path -LiteralPath $missingDisc)) 'Missing-disc fixture unexpectedly exists.'
    Assert (Test-Path (Join-Path $receiptRoot 'boards.done')) 'boards.done was not prepared for the missing-disc test.'
    Assert (-not (Test-Path (Join-Path $receiptRoot 'music.done'))) 'music.done must be absent for the missing-disc test.'
    $run = Invoke-Installer '07-missing-disc-path' 'keys,music' -Skate3Path $missingDisc
    $didSkip = $run.Output -match 'soundtrack was skipped'
    $didReachInstall = $run.Output -match '(?m)^\[8/8\] Installing into GTA V'
    $installDone = Test-Path (Join-Path $receiptRoot 'install.done')
    $musicDone = Test-Path (Join-Path $receiptRoot 'music.done')
    $mockCalled = Test-Path (Join-Path $receiptRoot 'mock-music-called.txt')
    $tail = (($run.Output -split "`r?`n" | Select-Object -Last 9) -join ' | ')
    Assert ($run.ExitCode -eq 0 -and $didSkip -and $didReachInstall -and $installDone -and -not $musicDone -and -not $mockCalled) `
        "Expected exit 0, optional skip, completed step 8, no music receipt, and no mock call; observed exit=$($run.ExitCode), skipped=$didSkip, reached_step8=$didReachInstall, install.done=$installDone, music.done=$musicDone, mock_called=$mockCalled; output tail: $tail"
}

Run-Scenario 'optional-music-missing-disc-assets-mock' {
    Reset-Target
    $run = Invoke-Installer '07-missing-music-assets' 'keys,music'
    Assert ($run.ExitCode -eq 0) "Optional soundtrack failure stopped installation: $($run.Output)"
    Assert (Test-Path (Join-Path $receiptRoot 'mock-music-called.txt')) 'Safe music mock was not invoked.'
    Assert (-not (Test-Path (Join-Path $receiptRoot 'music.done'))) 'Failed soundtrack was incorrectly receipted as done.'
    Assert ($run.Output -match 'soundtrack was skipped') 'Installer did not report the optional soundtrack failure.'
    Assert ($run.Output -notmatch 'Perform Full Scan') 'Failed soundtrack printed a misleading scan instruction.'
    Assert (Test-Path (Join-Path $receiptRoot 'install.done')) 'Installer did not continue through the install step.'
}

Run-Scenario 'poisoned-python-environment-is-pinned-for-music-child' {
    Reset-Target
    $run = Invoke-Installer '08-python-environment' 'keys,music' -PoisonPythonEnvironment
    Assert ($run.ExitCode -eq 0) "Poisoned-environment fixture exit $($run.ExitCode): $($run.Output)"
    $markerPath = Join-Path $receiptRoot 'mock-music-called.txt'
    Assert (Test-Path -LiteralPath $markerPath) 'Fixture music mock did not record the child process environment.'
    $marker = [IO.File]::ReadAllText($markerPath)
    $environmentLines = @{}
    foreach ($line in ($marker -split "`r?`n")) {
        $pair = $line -split '=', 2
        if ($pair.Count -eq 2) { $environmentLines[$pair[0]] = $pair[1] }
    }
    $expectedPythonHome = [IO.Path]::GetFullPath((Join-Path $release 'python'))
    Assert ($environmentLines['PYTHONHOME'] -ceq $expectedPythonHome) "Music child did not inherit pinned fixture PYTHONHOME=$expectedPythonHome; marker: $marker"
    Assert ($environmentLines['PYTHONPATH'] -ceq '<unset>') "Music child inherited PYTHONPATH instead of a cleared value; marker: $marker"
    Assert ($environmentLines['PYTHONNOUSERSITE'] -ceq '1') "Music child did not inherit PYTHONNOUSERSITE=1; marker: $marker"
    Assert ($run.Output -match 'soundtrack was skipped') 'Fixture converter failure was not caught as optional music.'
    Assert (Test-Path (Join-Path $receiptRoot 'install.done')) 'Installer did not continue through step 8.'
}

# A source checkout has no bin folder: step 3 fetches the pinned release builds.
# The first case really downloads them (network) into the fixture's build cache.
$installerText = [IO.File]::ReadAllText($liveInstaller)
$pinnedRelease = [regex]::Match($installerText, '(?m)^\s*\$GtaxSkateRelease\s*=\s*''([^'']+)''').Groups[1].Value
Assert ([bool]$pinnedRelease) 'Could not read $GtaxSkateRelease from install.ps1.'
$binCache = Join-Path $fixtureBuild "GTAxSkate-bin-$pinnedRelease.zip"
$acceptedBinaries = @{
    'SkateVLegacy.asi' = '3FF0D9883DF5E557D3351A71CB9F5643C78338A2A79D3E5B29D2050D3E852F13'
    'SkateVRuntime.dll' = 'E2108B27281F7B845CC672B1D8F2C04B250D430E31087CD14C3BCB2CFB92D1B2'
}
if ($ExpectedManifest) {
    $expectedBuild = Get-Content -LiteralPath $ExpectedManifest -Raw | ConvertFrom-Json
    Assert ($expectedBuild.format -eq 'gtaxskate-test' -and $expectedBuild.version -eq 1) 'Invalid expected test-build manifest.'
    $expectedNames = @('SkateVLegacy.asi', 'SkateVRuntime.dll')
    Assert (@($expectedBuild.files.PSObject.Properties).Count -eq $expectedNames.Count) 'Expected manifest must contain exactly the host/runtime pair.'
    foreach ($name in $expectedNames) {
        $property = $expectedBuild.files.PSObject.Properties[$name]
        Assert ($null -ne $property -and [string]$property.Value -cmatch '^[0-9a-fA-F]{64}$') "Invalid expected SHA256 for $name."
        $acceptedBinaries[$name] = ([string]$property.Value).ToUpperInvariant()
    }
}
function Clear-SourceBin {
    Remove-Item -LiteralPath (Join-Path $receiptRoot 'bin.done') -Force -ErrorAction SilentlyContinue
    Get-ChildItem -LiteralPath $fixtureBin -Force | ForEach-Object { Assert-UnderQa $_.FullName | Out-Null; Remove-Item -LiteralPath $_.FullName -Recurse -Force }
}
function Assert-AcceptedPair([string] $folder) {
    foreach ($name in $acceptedBinaries.Keys) {
        $path = Join-Path $folder $name
        Assert (Test-Path -LiteralPath $path -PathType Leaf) "$name missing in $folder."
        Assert ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -eq $acceptedBinaries[$name]) "$name in $folder is not the accepted build."
    }
}

Run-Scenario 'source-checkout-downloads-pinned-bin' {
    Reset-Target
    Clear-SourceBin
    Remove-Item -LiteralPath $binCache -Force -ErrorAction SilentlyContinue
    if ($BinArchive) {
        $expected = [regex]::Match($installerText, '(?m)^\$GtaxSkateBinSha256 = ''([0-9a-f]{64})''').Groups[1].Value
        Assert ((Get-FileHash -LiteralPath $BinArchive -Algorithm SHA256).Hash -eq $expected) 'Offline archive does not match the pinned release.'
        Copy-Item -LiteralPath $BinArchive -Destination $binCache
    }
    # This case checks acquisition and the exact binary pair, independently of
    # world exporters (a new state-safe baker correctly rejects mock receipts).
    $run = Invoke-Installer '09-source-download' 'keys,camera' -NoMusic -Components none
    Assert ($run.ExitCode -eq 0) "Source-checkout install exit $($run.ExitCode): $($run.Output)"
    if ($BinArchive) { Assert ($run.Output -notmatch 'downloading GTAxSkate-bin\.zip') 'Offline fixture unexpectedly downloaded.' }
    else { Assert ($run.Output -match 'downloading GTAxSkate-bin\.zip') 'The release builds were not downloaded.' }
    Assert (Test-Path -LiteralPath $binCache -PathType Leaf) 'The verified download was not cached.'
    Assert-AcceptedPair $fixtureBin
    Assert-AcceptedPair $fixtureGta
    Assert (Test-Path (Join-Path $receiptRoot 'bin.done')) 'bin.done was not written after a verified download.'
}

Run-Scenario 'source-checkout-reuses-verified-bin-cache' {
    Reset-Target
    Clear-SourceBin
    $run = Invoke-Installer '10-source-cache' 'keys,camera' -NoMusic -Components none
    Assert ($run.ExitCode -eq 0) "Cached source-checkout install exit $($run.ExitCode): $($run.Output)"
    Assert ($run.Output -notmatch 'downloading GTAxSkate-bin\.zip') 'A verified cache was downloaded again.'
    Assert-AcceptedPair $fixtureGta
}

Run-Scenario 'source-checkout-rejects-tampered-bin-cache' {
    Reset-Target
    Clear-SourceBin
    [IO.File]::WriteAllText($binCache, 'tampered cache')
    $run = Invoke-Installer '11-source-tampered' 'keys,camera' -NoMusic
    Assert ($run.ExitCode -ne 0) 'A tampered bin cache was accepted.'
    Assert ($run.Output -match 'Cached download checksum mismatch') 'The tampered cache was not reported.'
    Assert (-not (Test-Path -LiteralPath $binCache)) 'The tampered cache was not removed.'
    Assert (-not (Test-Path (Join-Path $fixtureBin 'SkateVLegacy.asi'))) 'Builds were extracted from a tampered cache.'
    Assert (-not (Test-Path (Join-Path $receiptRoot 'bin.done'))) 'bin.done was written after a rejected cache.'
}

Run-Scenario 'shipped-helper-only-upgrade-refreshes-bin' {
    Reset-Target
    $shipped = Join-Path $fixtureRepo 'bin'
    New-Item -ItemType Directory -Path $shipped -Force | Out-Null
    foreach ($name in 'SkateVLegacy.asi','SkateVRuntime.dll','skate-ped.exe','skate-music.exe','prepare-board-parts.py') {
        [IO.File]::WriteAllText((Join-Path $fixtureBin $name), "cached unchanged $name")
        Copy-Item -LiteralPath (Join-Path $fixtureBin $name) -Destination (Join-Path $shipped $name)
    }
    [IO.File]::WriteAllText((Join-Path $fixtureBin '_common.py'), 'old helper')
    [IO.File]::WriteAllText((Join-Path $shipped '_common.py'), 'new helper')
    [IO.File]::WriteAllText((Join-Path $receiptRoot 'bin.done'), 'previous successful bin')
    $run = Invoke-Installer '12-helper-upgrade' 'keys,camera' -NoMusic
    Assert ($run.ExitCode -eq 0) "Helper upgrade failed: $($run.Output)"
    Assert ([IO.File]::ReadAllText((Join-Path $fixtureBin '_common.py')) -eq 'new helper') 'Helper-only update left the cached payload stale.'
}


# Successful component mocks execute the real stage orchestration without GTA.
$shipped = Join-Path $fixtureRepo 'bin'
Assert-UnderQa $shipped | Out-Null
if (Test-Path -LiteralPath $shipped) { Remove-Item -LiteralPath $shipped -Recurse -Force }
$stubSource = @'
using System;
using System.IO;
public class ComponentExporter {
    public static int Main(string[] args) {
        string assets = null, output = null;
        for (int i = 0; i + 1 < args.Length; i++) {
            if (args[i] == "--assets") assets = args[i+1];
            if (args[i] == "--out") output = args[i+1];
        }
        if (assets != null) {
            string boards = Path.Combine(assets, "private", "boards");
            Directory.CreateDirectory(boards);
            File.WriteAllText(Path.Combine(boards, "library.json"), "{}");
        }
        if (output != null) {
            Directory.CreateDirectory(output);
            File.WriteAllText(Path.Combine(output, "parts.json"), "{}");
        }
        return 0;
    }
}
'@
$exporter = Join-Path $runRoot 'component-exporter.exe'
Add-Type -TypeDefinition $stubSource -OutputAssembly $exporter -OutputType ConsoleApplication
Copy-Item -LiteralPath $exporter -Destination (Join-Path $release 'python\python.exe') -Force
Copy-Item -LiteralPath $exporter -Destination (Join-Path $fixtureBin 'skate-ped.exe') -Force
Copy-Item -LiteralPath $exporter -Destination (Join-Path $fixtureBin 'skatev-world-cache.exe') -Force
[IO.File]::WriteAllText((Join-Path $fixtureScripts 'build-prebaked-map.ps1'), @'
param([string]$Gta, [string]$Out, [string]$Rpf, [string]$Keys, [string]$Baker, [string]$World)
New-Item -ItemType Directory -Force -Path $World | Out-Null
[IO.File]::WriteAllText((Join-Path $World 'sk8v-map.svwc'), 'fixture world')
'@)
function Reset-Components {
    Reset-Target
    foreach ($step in 'boards', 'glow', 'music', 'world', 'skinfix', 'tutorial', 'coop') {
        Remove-Item -LiteralPath (Join-Path $receiptRoot "$step.done") -Force -ErrorAction SilentlyContinue
    }
    $coopBackups = Join-Path $fixtureGta 'GTAxSkate-coop-backups'
    Assert-UnderQa $coopBackups | Out-Null
    if (Test-Path -LiteralPath $coopBackups) { Remove-Item -LiteralPath $coopBackups -Recurse -Force }
    $dataRoot = Join-Path $fixtureGta 'SK8V'
    Assert-UnderQa $dataRoot | Out-Null
    if (Test-Path -LiteralPath $dataRoot) { Remove-Item -LiteralPath $dataRoot -Recurse -Force }
    $native = Join-Path $dataRoot 'skate-data\assets\private\board-native'
    $keys = Join-Path $dataRoot 'gta\gta-keys'
    $musicDir = Join-Path $fixtureSkate3 'data\audio\music'
    New-Item -ItemType Directory -Force -Path $native, $keys, $musicDir | Out-Null
    [IO.File]::WriteAllText((Join-Path $native 'skatev_board.ydr.xml'), 'fixture native')
    [IO.File]::WriteAllText((Join-Path $keys 'gtav_aes_key.dat'), 'fixture keys')
    foreach ($file in 'ipod.mpf', 'Ipod_Stream.mus') { [IO.File]::WriteAllText((Join-Path $musicDir $file), 'fixture audio') }
    [IO.File]::WriteAllText((Join-Path $receiptRoot 'mock-music-success.txt'), 'allow conversion')
}
function Assert-Components([string[]]$expected) {
    $path = Join-Path $fixtureGta 'SkateVLegacy.components.txt'
    Assert (Test-Path -LiteralPath $path) 'Components file was not written.'
    $actual = @(Get-Content -LiteralPath $path)
    Assert (($actual -join ',') -eq ($expected -join ',')) "Wrong components file: $($actual -join ',')"
    Assert (@([IO.File]::ReadAllBytes($path) | Where-Object { $_ -gt 127 }).Count -eq 0) 'Components file is not ASCII.'
}
foreach ($case in @(
    @{ Name = 'core-only'; Selection = 'none'; Expected = @('boards=0','glow=0','music=0','world=0','tutorial=0','coop=0') },
    @{ Name = 'boards-only'; Selection = 'boards'; Expected = @('boards=1','glow=0','music=0','world=0','tutorial=0','coop=0') },
    @{ Name = 'world-only'; Selection = 'world'; Expected = @('boards=0','glow=0','music=0','world=1','tutorial=0','coop=0') },
    @{ Name = 'music-and-boards'; Selection = 'music,boards'; Expected = @('boards=1','glow=0','music=1','world=0','tutorial=0','coop=0') }
)) {
    Run-Scenario $case.Name {
        Reset-Components
        $run = Invoke-Installer $case.Name 'keys,camera,shop,music' -Components $case.Selection
        Assert ($run.ExitCode -eq 0) "Modular install failed: $($run.Output)"
        Assert-Components $case.Expected
        foreach ($step in 'boards', 'glow', 'music', 'world') {
            $selected = ($case.Selection -split ',') -contains $step
            Assert ((Test-Path -LiteralPath (Join-Path $receiptRoot "$step.done")) -eq $selected) "Unexpected receipt for $step"
        }
        Assert (Test-Path -LiteralPath (Join-Path $receiptRoot 'install.done')) 'Step 8 did not finish.'
    }
}
Run-Scenario 'enable-later-preserve-and-hide' {
    Reset-Components
    $run = Invoke-Installer 'core-before-boards' 'keys' -Components none
    Assert ($run.ExitCode -eq 0) 'Core failed.'
    $run = Invoke-Installer 'enable-boards' 'keys' -Components boards
    Assert ($run.ExitCode -eq 0) "Enabling boards failed: $($run.Output)"
    Assert-Components @('boards=1','glow=0','music=0','world=0','tutorial=0','coop=0')
    $stamp = [IO.File]::ReadAllText((Join-Path $receiptRoot 'boards.done'))
    $run = Invoke-Installer 'deselect-boards' 'keys' -Components none -Redo music
    Assert ($run.ExitCode -eq 0) 'Deselection failed.'
    Assert-Components @('boards=1','glow=0','music=0','world=0','tutorial=0','coop=0')
    Assert ([IO.File]::ReadAllText((Join-Path $receiptRoot 'boards.done')) -eq $stamp) 'Board receipt changed.'
    $run = Invoke-Installer 'hide-boards' 'keys' -Components none -Remove boards
    Assert ($run.ExitCode -eq 0) 'Metadata removal failed.'
    Assert-Components @('boards=0','glow=0','music=0','world=0','tutorial=0','coop=0')
    $run = Invoke-Installer 'keep-hidden' 'keys' -Components none
    Assert ($run.ExitCode -eq 0) 'Hidden resume failed.'
    Assert-Components @('boards=0','glow=0','music=0','world=0','tutorial=0','coop=0')
    $run = Invoke-Installer 'restore-boards' 'keys' -Components boards
    Assert ($run.ExitCode -eq 0) 'Restore failed.'
    Assert-Components @('boards=1','glow=0','music=0','world=0','tutorial=0','coop=0')
}

# B151: tutorial and coop are opt-in extras. All staged payloads are fixture
# dummies; the coop pack is a mock that only writes its receipt.
function Component-Line([string]$name) {
    @(Get-Content -LiteralPath (Join-Path $fixtureGta 'SkateVLegacy.components.txt') | Where-Object { $_ -like "$name=*" }) -join ','
}
Run-Scenario 'optional-default-keeps-tutorial-and-coop-off' {
    Reset-Components
    foreach ($step in 'boards', 'world') { [IO.File]::WriteAllText((Join-Path $receiptRoot "$step.done"), 'cached integration prerequisite') }
    $run = Invoke-Installer 'optional-default' 'keys,camera' -NoMusic
    Assert ($run.ExitCode -eq 0) "Default install failed: $($run.Output)"
    Assert ((Component-Line 'tutorial') -eq 'tutorial=0') "Default run changed tutorial: $(Component-Line 'tutorial')"
    Assert ((Component-Line 'coop') -eq 'coop=0') "Default run changed coop: $(Component-Line 'coop')"
    foreach ($step in 'tutorial', 'coop') { Assert (-not (Test-Path -LiteralPath (Join-Path $receiptRoot "$step.done"))) "Default run wrote $step.done." }
}
Run-Scenario 'optional-unstaged-is-not-fatal' {
    Reset-Components
    $run = Invoke-Installer 'optional-unstaged' 'keys' -Components none -WithTutorial -WithCoop
    Assert ($run.ExitCode -eq 0) "Unstaged optional install failed: $($run.Output)"
    Assert ($run.Output -match 'Tutorial pack not staged locally; skipping optional copy') 'Missing tutorial note.'
    Assert ($run.Output -match 'Coop pack not staged locally') 'Missing coop note.'
    Assert-Components @('boards=0','glow=0','music=0','world=0','tutorial=0','coop=0')
}
Run-Scenario 'optional-tutorial-and-coop-install-and-hide' {
    Reset-Components
    $school = Join-Path $fixtureRepo 'local\m0-tutorial'
    New-Item -ItemType Directory -Force -Path (Join-Path $school 'addons\skate_school\collision'), (Join-Path $school 'addons\skate_school\splines') | Out-Null
    [IO.File]::WriteAllText((Join-Path $school 'm0-tutorial-manifest.json'), '{"fixture":true}')
    [IO.File]::WriteAllText((Join-Path $school 'addons\skate_school\collision\DIST_SkateSchool.svwc'), 'fixture school collision')
    [IO.File]::WriteAllText((Join-Path $school 'addons\skate_school\splines\DIST_SkateSchool_rails.json'), '[]')
    [IO.File]::WriteAllText((Join-Path $school 'addons\skate_school\splines\notes.txt'), 'must not be copied')
    $coopSource = Join-Path $fixtureRepo 'coop\gtaxskate'
    $coopPack = Join-Path $fixtureBuild 'coop-test-ready-20261015\optional-coop'
    New-Item -ItemType Directory -Force -Path $coopSource, $coopPack | Out-Null
    [IO.File]::WriteAllText((Join-Path $coopSource 'Setup-FriendsCoop.ps1'), "param([string]`$Mode = 'VerifySecurity', [string]`$TargetDir = '')`r`nWrite-Host 'fixture coop helper'")
    [IO.File]::WriteAllText((Join-Path $coopPack 'Install-OptionalCoop.ps1'), @'
param([string]$Mode = 'Install', [string]$GtaDir = '')
if (-not (Test-Path -LiteralPath (Join-Path $GtaDir 'GTA5.exe'))) { throw 'fixture: no GTA5.exe' }
$backup = Join-Path $GtaDir 'GTAxSkate-coop-backups'
New-Item -ItemType Directory -Force -Path $backup | Out-Null
[IO.File]::WriteAllText((Join-Path $backup 'optional-coop-receipt.json'), '{"Schema":2,"Entries":[]}')
Write-Host "fixture coop pack installed ($Mode)"
'@)
    $run = Invoke-Installer 'optional-with-both' 'keys' -Components none -WithTutorial -WithCoop
    Assert ($run.ExitCode -eq 0) "Optional install failed: $($run.Output)"
    Assert-Components @('boards=0','glow=0','music=0','world=0','tutorial=1','coop=1')
    $schoolData = Join-Path $fixtureGta 'SK8V\addons\skate_school'
    $svwc = Join-Path $schoolData 'collision\DIST_SkateSchool.svwc'
    Assert (Test-Path -LiteralPath $svwc -PathType Leaf) 'DIST_SkateSchool.svwc was not copied.'
    Assert ([IO.File]::ReadAllText($svwc) -eq 'fixture school collision') 'Copied collision differs.'
    Assert (Test-Path -LiteralPath (Join-Path $schoolData 'splines\DIST_SkateSchool_rails.json') -PathType Leaf) 'Rail splines were not copied.'
    Assert (-not (Test-Path -LiteralPath (Join-Path $schoolData 'splines\notes.txt'))) 'An unrelated staged file was copied.'
    $coopReceipt = Join-Path $fixtureGta 'GTAxSkate-coop-backups\optional-coop-receipt.json'
    Assert (Test-Path -LiteralPath $coopReceipt -PathType Leaf) 'Coop pack was not run.'
    foreach ($step in 'tutorial', 'coop') { Assert (Test-Path -LiteralPath (Join-Path $receiptRoot "$step.done")) "$step.done is missing." }
    Assert (-not (Test-Path -LiteralPath (Join-Path $fixtureGta 'update'))) 'The optional stages touched the update folder.'
    $run = Invoke-Installer 'optional-hide-both' 'keys' -Components none -Remove 'tutorial,coop'
    Assert ($run.ExitCode -eq 0) "Hiding the extras failed: $($run.Output)"
    Assert-Components @('boards=0','glow=0','music=0','world=0','tutorial=0','coop=0')
    Assert (Test-Path -LiteralPath $svwc -PathType Leaf) 'Removal deleted tutorial files.'
    Assert (Test-Path -LiteralPath $coopReceipt -PathType Leaf) 'Removal deleted the coop receipt.'
    $run = Invoke-Installer 'optional-stay-hidden' 'keys' -Components none
    Assert ($run.ExitCode -eq 0) 'Hidden resume failed.'
    Assert-Components @('boards=0','glow=0','music=0','world=0','tutorial=0','coop=0')
    $run = Invoke-Installer 'optional-restore-tutorial' 'keys' -Components none -WithTutorial
    Assert ($run.ExitCode -eq 0) "Restoring the tutorial failed: $($run.Output)"
    Assert-Components @('boards=0','glow=0','music=0','world=0','tutorial=1','coop=0')
    Remove-Item -LiteralPath $school, (Join-Path $fixtureRepo 'coop'), (Join-Path $fixtureBuild 'coop-test-ready-20261015') -Recurse -Force
}

Assert ((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash -eq $sourceHash) 'Read-only source GTA5.exe changed during testing.'
$summaryPath = Join-Path $runRoot 'results.json'
[IO.File]::WriteAllText($summaryPath, ($results | ConvertTo-Json -Depth 4), (New-Object Text.UTF8Encoding $false))
Write-Host "`nInstaller integration results ($($results.Count) scenarios): installer steps execute from the real script; tools/SK8V/build data are cached; modular board/world/music exporters are fixture mocks; the missing-disc case exercises real Disc-With and continues through step 8; converter behavior and child environment are observed through a fixture-only mock." -ForegroundColor Cyan
$results | Format-Table scenario, result, detail -AutoSize
Write-Host "Logs and fixtures: $runRoot"
if ($BinArchive) { Write-Host 'Offline archive supplied: network download behavior was not exercised.' }
if (@($results | Where-Object result -ne 'PASS').Count -gt 0) { exit 1 }
exit 0
