param([string]$DotNetSdk = '', [string]$Output = '', [switch]$ClientOnly, [switch]$UseCoopTestReady)
$ErrorActionPreference = 'Stop'
$root = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
if (!$DotNetSdk) { $DotNetSdk = Join-Path $root 'build/coop-lab/tools/dotnet' }
if (!$Output) { $Output = Join-Path $root 'build/coop-adapter' }
$resolvedOutput = [IO.Path]::GetFullPath($Output)
$buildRoot = [IO.Path]::GetFullPath((Join-Path $root 'build')) + [IO.Path]::DirectorySeparatorChar
if (!$resolvedOutput.StartsWith($buildRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Build output must stay inside the workspace build folder.' }
$lab = Join-Path $root 'build/coop-lab'
$server = Join-Path $lab 'ragecoop-server/RageCoop.Server.exe'
$client = Join-Path $lab 'ragecoop-client/RageCoop'
$shvdn = Join-Path $lab 'shvdn-193/ScriptHookVDotNet3.dll'
if ($UseCoopTestReady) {
    $ready = Join-Path $root 'build/coop-test-ready-20261015/payload'
    if (Test-Path -LiteralPath $ready) {
        $client = Join-Path $ready 'scripts/RageCoop'
        $shvdn = Join-Path $ready 'ScriptHookVDotNet3.dll'
        $server = Join-Path (Split-Path $ready -Parent) 'server/RageCoop.Server.exe'
    }
}
$pinned = @{
    (Join-Path $client 'RageCoop.Client.dll') = 'b46f8f7689fd13579c5126fc8d088b94802731b84d8ce7cb66ce862e2cd69d62'
    (Join-Path $client 'RageCoop.Core.dll') = 'f06d0136b9759cf8adb7893ba488df166c95789330ddd33d31cd777eb3afe1c6'
    $shvdn = '13cacd649eff48a7c85eff33fdbdbea180cf1056b35dd6795f5ebc5648b3308e'
}
foreach ($file in $pinned.Keys) {
    if (!(Test-Path -LiteralPath $file) -or (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash -ne $pinned[$file]) { throw "Pinned reference mismatch: $([IO.Path]::GetFileName($file))" }
}
$clientOut = Join-Path $resolvedOutput 'Resources/Client/GTAxSkate.Coop.Client'
$serverOut = Join-Path $resolvedOutput 'Resources/Server/GTAxSkate.Coop.Server'
New-Item -ItemType Directory -Force -Path $clientOut,$serverOut | Out-Null
$shared = @(Get-ChildItem (Join-Path $PSScriptRoot 'shared') -Filter '*.cs' | ForEach-Object FullName)
$adapterCommon = @((Join-Path $PSScriptRoot 'ragecoop/DiagnosticProtocol.cs'), (Join-Path $PSScriptRoot 'ragecoop/SessionProtocol.cs')) + $shared
$csc = Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
$clientDll = Join-Path $clientOut 'GTAxSkate.Coop.Client.dll'
$clientSources = $adapterCommon + @(
    (Join-Path $PSScriptRoot 'ragecoop/ClientAdapter.cs'),
    (Join-Path $PSScriptRoot 'ragecoop/IGtaWorldBridge.cs'),
    (Join-Path $PSScriptRoot 'ragecoop/CoopRuntimeControl.cs'),
    (Join-Path $PSScriptRoot 'ragecoop/CoopBootstrapLogic.cs'),
    (Join-Path $PSScriptRoot 'client/ShvdnWorldBridge.cs'),
    (Join-Path $PSScriptRoot 'client/NatUpnpGateway.cs')
)
& $csc /nologo /warn:4 /warnaserror /target:library "/out:$clientDll" "/reference:$(Join-Path $client 'RageCoop.Client.dll')" "/reference:$(Join-Path $client 'RageCoop.Core.dll')" "/reference:$shvdn" "/reference:$(Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/netstandard.dll')" /reference:System.Windows.Forms.dll /reference:System.Drawing.dll $clientSources
if ($LASTEXITCODE) { throw 'Client adapter compilation failed.' }
$bootstrapSource = Join-Path $PSScriptRoot 'client/CoopBootstrapScript.cs'
# The only shared file the bootstrap compiles itself: the INTERNAL canonical-path helper it needs
# before the client DLL can be resolved. Runtime consent/state is NOT duplicated here: it stays in the client DLL.
$bootstrapPaths = Join-Path $PSScriptRoot 'shared/CoopInstallPaths.cs'
$bootstrapDll = Join-Path $resolvedOutput 'GTAxSkate.Coop.Bootstrap.dll'
& $csc /nologo /warn:4 /warnaserror /target:library "/out:$bootstrapDll" "/reference:$clientDll" "/reference:$(Join-Path $client 'RageCoop.Client.dll')" "/reference:$(Join-Path $client 'RageCoop.Core.dll')" "/reference:$shvdn" "/reference:$(Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/netstandard.dll')" /reference:System.Windows.Forms.dll /reference:System.Drawing.dll $bootstrapSource $bootstrapPaths
if ($LASTEXITCODE) { throw 'Coop bootstrap compilation failed.' }
$outputsBuilt = @($clientDll, $bootstrapDll)
if (-not $ClientOnly) {
    $dotnet = Join-Path $DotNetSdk 'dotnet.exe'
    $compiler = Join-Path $DotNetSdk 'sdk/6.0.428/Roslyn/bincore/csc.dll'
    $refs = Join-Path $DotNetSdk 'packs/Microsoft.NETCore.App.Ref/6.0.36/ref/net6.0'
    if (!(Test-Path -LiteralPath $compiler) -or !(Test-Path -LiteralPath $refs)) { throw 'Isolated .NET SDK 6.0.428 / reference pack 6.0.36 is required (or pass -ClientOnly).' }
    & python (Join-Path $PSScriptRoot 'extract_server_references.py') --server $server
    if ($LASTEXITCODE) { throw 'Server compiler reference extraction failed.' }
    $serverDll = Join-Path $serverOut 'GTAxSkate.Coop.Server.dll'
    $arguments = @($compiler, '/nologo', '/warn:4', '/warnaserror', '/noconfig', '/nostdlib+', '/target:library', "/out:$serverDll")
    $arguments += @(Get-ChildItem -LiteralPath $refs -Filter '*.dll' | ForEach-Object { "/reference:$($_.FullName)" })
    $arguments += "/reference:$(Join-Path $lab 'server-references/RageCoop.Server.dll')"
    $arguments += "/reference:$(Join-Path $lab 'server-references/RageCoop.Core.dll')"
    $arguments += $adapterCommon
    $arguments += (Join-Path $PSScriptRoot 'ragecoop/ServerAdapter.cs')
    & $dotnet @arguments
    if ($LASTEXITCODE) { throw 'Server adapter compilation failed.' }
    $outputsBuilt += $serverDll
}
$compiledSources = $clientSources + @($bootstrapSource, $bootstrapPaths)
if (-not $ClientOnly) { $compiledSources += (Join-Path $PSScriptRoot 'ragecoop/ServerAdapter.cs') }
$report = [ordered]@{
    Schema = 1
    Target = 'RAGECOOP Friends-Only Coop adapter development build'
    ClientApi = '1.5.12.15'
    ServerApi = '1.5.12.13'
    ClientOnly = [bool]$ClientOnly
    LiveTransportVerified = $false
    Outputs = @($outputsBuilt | ForEach-Object { [ordered]@{ File = $_.Substring($resolvedOutput.Length + 1); Sha256 = (Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant() } })
    Sources = @($compiledSources | ForEach-Object { [ordered]@{ File = $_.Substring($PSScriptRoot.Length + 1); Sha256 = (Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant() } })
}
$report | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $resolvedOutput 'adapter-build.json') -Encoding UTF8
Write-Host "Built RAGECOOP Coop resources: $resolvedOutput"
Write-Host 'No game/server installation or startup performed.'
