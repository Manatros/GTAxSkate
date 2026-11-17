param([string]$Output = '')
# Builds the client resources and bootstrap into the repo's build folder (never the GTA folder) with the pinned
# references, then runs a metadata-only identity check on the built DLLs.
$ErrorActionPreference='Stop'
$root=Split-Path (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent) -Parent
if(!$Output){$Output=Join-Path $root 'build/coop-adapter-optin'}
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path (Split-Path $PSScriptRoot -Parent) 'build.ps1') -UseCoopTestReady -ClientOnly -Output $Output
if($LASTEXITCODE){throw 'Coop build failed.'}
$ready=Join-Path $root 'build/coop-test-ready-20261015/payload'
$out=Join-Path $root 'build/coop-identity-tests'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$csc=Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
$exe=Join-Path $out 'BuildIdentityCheck.exe'
& $csc /nologo /warn:4 /warnaserror /target:exe "/out:$exe" (Join-Path $PSScriptRoot 'BuildIdentityCheck.cs')
if($LASTEXITCODE){throw 'Identity check compilation failed.'}
$client=Join-Path $Output 'Resources/Client/GTAxSkate.Coop.Client/GTAxSkate.Coop.Client.dll'
$boot=Join-Path $Output 'GTAxSkate.Coop.Bootstrap.dll'
& $exe $client $boot (Join-Path $ready 'scripts/RageCoop') $ready (Join-Path $Output 'Resources/Client/GTAxSkate.Coop.Client')
if($LASTEXITCODE){throw 'Built-DLL identity check failed.'}
