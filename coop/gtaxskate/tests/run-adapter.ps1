$ErrorActionPreference='Stop'
$root=Split-Path (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent) -Parent
$out=Join-Path $root 'build/coop-adapter-tests'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$csc=Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
$sources=@(Get-ChildItem (Join-Path (Split-Path $PSScriptRoot -Parent) 'ragecoop') -Filter '*.cs' | ForEach-Object FullName)
$sources+=@(Get-ChildItem (Join-Path (Split-Path $PSScriptRoot -Parent) 'shared') -Filter '*.cs' | ForEach-Object FullName)
$exe=Join-Path $out 'RageCoopAdapterTests.exe'
& $csc /nologo /warn:4 /warnaserror /define:COOP_FIXTURE /reference:System.Xml.dll /target:exe "/out:$exe" $sources (Join-Path $PSScriptRoot 'RageCoopAdapterTests.cs')
if($LASTEXITCODE){throw 'Adapter fixture compilation failed.'}
# The identity test recompiles a second copy of the controller from source; scratch files stay under build/.
$env:GTAXSKATE_COOP_SRC=(Split-Path $PSScriptRoot -Parent)
$env:GTAXSKATE_COOP_TEST_TMP=Join-Path $out 'tmp'
New-Item -ItemType Directory -Force -Path $env:GTAXSKATE_COOP_TEST_TMP | Out-Null
& $exe
if($LASTEXITCODE){throw 'Adapter event integration tests failed.'}
