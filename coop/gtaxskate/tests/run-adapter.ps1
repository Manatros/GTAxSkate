$ErrorActionPreference='Stop'
$root=Split-Path (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent) -Parent
$out=Join-Path $root 'build/coop-adapter-tests'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$csc=Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
$sources=@(Get-ChildItem (Join-Path (Split-Path $PSScriptRoot -Parent) 'ragecoop') -Filter '*.cs' | ForEach-Object FullName)
$exe=Join-Path $out 'RageCoopAdapterTests.exe'
& $csc /nologo /warn:4 /warnaserror /target:exe "/out:$exe" $sources (Join-Path $PSScriptRoot 'RageCoopAdapterTests.cs')
if($LASTEXITCODE){throw 'Adapter fixture compilation failed.'}
& $exe
if($LASTEXITCODE){throw 'Adapter event integration tests failed.'}
