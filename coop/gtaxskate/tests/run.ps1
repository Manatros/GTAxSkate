$ErrorActionPreference='Stop'
$root=Split-Path (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent) -Parent
$out=Join-Path $root 'build\coop-pose-protocol-tests'
New-Item -ItemType Directory -Path $out -Force | Out-Null
$csc=Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
if(!(Test-Path -LiteralPath $csc)){throw '.NET Framework C# compiler missing.'}
$sources=@(Get-ChildItem (Join-Path (Split-Path $PSScriptRoot -Parent) 'shared') -Filter '*.cs' | ForEach-Object FullName)
$sources+=Join-Path $PSScriptRoot 'PoseProtocolTests.cs'
$exe=Join-Path $out 'PoseProtocolTests.exe'
& $csc /nologo /warn:4 /target:exe "/out:$exe" $sources
if($LASTEXITCODE){throw 'Pose codec compilation failed.'}
& $exe
if($LASTEXITCODE){throw 'Pose codec tests failed.'}
