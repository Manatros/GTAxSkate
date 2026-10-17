param([string]$Fork = '')
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
if (-not $Fork) { $Fork = Join-Path $root 'external\sk8v' }
$qa = Join-Path $root 'build\board-pose-diagnostics-tests'
New-Item -ItemType Directory -Path $qa -Force | Out-Null
$checks = @'
#include "board_pose_diagnostics.h"
#include "board_native_detail.h"
#include <cassert>
#include <iostream>
#include <cmath>
using posemath::M;
namespace d=boardnative::diagnostics;
M turn(float angle){M m=posemath::Identity();m.r[0][0]=m.r[1][1]=std::cos(angle);m.r[0][1]=std::sin(angle);m.r[1][0]=-std::sin(angle);return m;}
void identity(M (&m)[7]){for(auto& x:m)x=posemath::Identity();}
void checkZero(const float (&v)[6]){for(float x:v)assert(x<0.04f);}
int main(){
 M initial[7],before[7],after[7],object[7],objectAfter[7];identity(initial);identity(before);identity(after);identity(object);identity(objectAfter);
 d::State s;
 assert(!d::ShouldSample(s,false,0,0));assert(!s.active);
 assert(d::ShouldSample(s,true,10,1000));d::Sample(s,10,1000,initial,before,after,object,objectAfter);d::Written(s,initial);
 assert(!d::ShouldSample(s,true,10,2000)); // same runtime tick, even after long wait
 assert(!d::ShouldSample(s,true,11,1249));assert(d::ShouldSample(s,true,11,1250));
 d::Sample(s,11,1250,initial,before,after,object,objectAfter);checkZero(s.motion);checkZero(s.beforeWritten);assert(s.localError==0&&s.objectError==0);
 // World/root motion does not animate any parent-relative wheel or truck.
 M world[7],local[7],objs[7];M root=turn(0.8f);root.r[3][0]=17;root.r[3][1]=-22;
 for(auto& m:world)m=root;
 assert(boardnative::detail::Convert(world,posemath::Identity(),objs,local));
 d::Sample(s,12,1500,local,before,local,objs,objs);checkZero(s.motion);
 d::ClearWindow(s);d::Written(s,initial);
 // Independent truck and wheel motion is observable without false sibling motion.
 M moving[7];identity(moving);moving[1]=turn(0.3f);moving[3]=turn(-0.6f);
 d::Sample(s,13,1750,moving,before,moving,object,objectAfter);
 assert(s.motion[0]>17&&s.motion[0]<18);assert(s.motion[2]>34&&s.motion[2]<35);
 assert(s.motion[1]<0.04f&&s.motion[3]<0.04f&&s.motion[4]<0.04f&&s.motion[5]<0.04f);
 assert(s.localError==0&&s.objectError==0);checkZero(s.beforeWritten);
 // Writes between samples must be compared with the actual last write, not
 // the older sampled pose (which would mistake ordinary animation for overwrite).
 d::Written(s,moving);M intermediate[7];for(int i=0;i<7;i++)intermediate[i]=moving[i];intermediate[3]=turn(-0.9f);
 d::Written(s,intermediate);d::ClearWindow(s);
 d::Sample(s,14,2000,intermediate,intermediate,intermediate,object,objectAfter);
 assert(s.motion[2]>17);checkZero(s.beforeWritten);
 // A changed pre-write skeleton is reported separately from exact readback.
 M changed[7];for(int i=0;i<7;i++)changed[i]=intermediate[i];changed[3]=turn(0.2f);
 d::Sample(s,15,2250,intermediate,changed,intermediate,object,objectAfter);
 assert(s.beforeWritten[2]>62&&s.beforeWritten[2]<64);assert(s.localError==0&&s.objectError==0);
 d::ClearWindow(s);M badAfter[7];for(int i=0;i<7;i++)badAfter[i]=intermediate[i];badAfter[3]=turn(0.0f);objectAfter[3]=turn(0.2f);
 d::Sample(s,16,2500,intermediate,intermediate,badAfter,object,objectAfter);
 assert(s.localError>0.5f&&s.objectError>0.1f);
 // Disabling/re-enabling starts fresh: old menu/ped epochs are not mixed.
 assert(!d::ShouldSample(s,false,17,2600));assert(d::ShouldSample(s,true,18,2601));assert(!s.previous&&!s.priorWrite&&s.samples==0);
 d::Sample(s,18,2601,moving,moving,moving,object,object);checkZero(s.motion);checkZero(s.beforeWritten);
 std::cout<<"PASS: static/root-only motion ignored; truck and wheel relative motion identified; actual prior writes distinguished; readback errors separated; duplicate ticks/rate/gating/reset checked\n";
}
'@
[IO.File]::WriteAllText((Join-Path $qa 'board-diagnostics-test.cpp'), $checks)
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vs = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | Select-Object -First 1)
if (-not $vs) { throw 'Visual Studio C++ tools not found.' }
$vcvars = Join-Path $vs 'VC\Auxiliary\Build\vcvars64.bat'
$include = Join-Path $Fork 'host\src'
$batch = "@echo off`r`ncall `"$vcvars`" >nul`r`nif errorlevel 1 exit /b 1`r`ncl /nologo /std:c++17 /EHsc /W4 /I`"$include`" `"$qa\board-diagnostics-test.cpp`" /Fe:`"$qa\board-diagnostics-test.exe`" /Fo:`"$qa\board-diagnostics-test.obj`"`r`nif errorlevel 1 exit /b 1`r`n`"$qa\board-diagnostics-test.exe`"`r`n"
[IO.File]::WriteAllText((Join-Path $qa 'run.cmd'), $batch)
& $env:ComSpec /d /c (Join-Path $qa 'run.cmd')
if ($LASTEXITCODE) { throw 'Board pose diagnostics fixture failed.' }
