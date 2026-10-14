$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$source = [IO.File]::ReadAllText((Join-Path $root 'external\sk8v\host\src\main.cpp'))
$start = $source.IndexOf('void ConstrainCameraEye(')
$end = $source.IndexOf('// Third person while skating', $start)
if ($start -lt 0 -or $end -lt 0) { throw 'Camera helper not found.' }
$helper = $source.Substring($start, $end - $start)
$floor = [regex]::Match($source, 'if \(ray\(above, below, end, normal\) && (?<condition>[^\r\n]+)\) \{')
if (-not $floor.Success) { throw 'Actual cinematic floor predicate not found.' }
$qa = Join-Path $root 'build\camera-clearance-tests'
New-Item -ItemType Directory -Path $qa -Force | Out-Null
$stub = @'
#include <algorithm>
#include <cmath>
#include <cassert>
#include <iostream>
using BOOL = int;
using Entity = int;
struct Vector3 { float x=0,y=0,z=0; };
struct Session { int ped=73; };
bool hitResult=false;
Vector3 hitPoint{};
int seenFlags=0,seenPed=0,seenOption=0;
float seenAnchor[3]{},seenEye[3]{};
namespace gta {
constexpr int START_EXPENSIVE_SYNCHRONOUS_SHAPE_TEST_LOS_PROBE=1;
constexpr int GET_SHAPE_TEST_RESULT=2;
template<class T> T Call(int, float ax,float ay,float az,float ex,float ey,float ez,int flags,int ped,int option) {
    seenAnchor[0]=ax;seenAnchor[1]=ay;seenAnchor[2]=az;
    seenEye[0]=ex;seenEye[1]=ey;seenEye[2]=ez;
    seenFlags=flags;seenPed=ped;seenOption=option;
    return 41;
}
template<class T> T Call(int,int,BOOL* hit,Vector3* end,Vector3*,Entity*) {
    *hit=hitResult;*end=hitPoint;return 2;
}
}
using gta::Call;
'@
$checks = @'
int main() {
    Session s;
    const float anchor[3]={0,0,1.25f};
    float eye[3]={0,-3,1.5f};
    ConstrainCameraEye(s,anchor,eye,17);
    assert(eye[0]==0 && eye[1]==-3 && eye[2]==1.5f);
    assert(seenFlags==17 && seenPed==73 && seenOption==7);
    assert(seenAnchor[2]==1.25f && seenEye[1]==-3);
    hitResult=true;hitPoint={0,-0.8f,1.3f};
    ConstrainCameraEye(s,anchor,eye,1);
    // Wall closer than the old 1.6 m preferred distance: stay on our side.
    assert(eye[1]>hitPoint.y && eye[1]<=anchor[1]);
    const float distance=std::hypot(eye[1]-hitPoint.y,eye[2]-hitPoint.z);
    assert(std::fabs(distance-0.3f)<0.0001f);
    assert(seenFlags==1);
    hitPoint={0,0,1.25f};
    ConstrainCameraEye(s,anchor,eye,1);
    assert(std::isfinite(eye[0]) && std::isfinite(eye[1]) && std::isfinite(eye[2]));
    assert(eye[0]==0 && eye[1]==0 && eye[2]==1.25f);
    hitPoint={0,-0.1f,1.25f};eye[1]=-3;
    ConstrainCameraEye(s,anchor,eye,1);
    assert(eye[1]==0); // clearance never overshoots the anchor
    auto needsGroundLift=[](float eyeZ,float surfaceZ,float normalZ) {
        float eye[3]={0,0,eyeZ};Vector3 end{0,0,surfaceZ},normal{0,0,normalZ};
        return FLOOR_PREDICATE;
    };
    assert(!needsGroundLift(1.0f,1.8f,-1.0f)); // underside of ceiling
    assert(!needsGroundLift(1.0f,1.8f,1.0f)); // roof top above the safe eye
    assert(needsGroundLift(1.0f,0.9f,1.0f)); // grounded clearance actually needed
    assert(!needsGroundLift(1.5f,0.9f,1.0f)); // clear of the ground already
    std::cout << "PASS: no hit, forwarded arguments, close wall clearance, coincident hit, short segment\n";
    std::cout << "PASS: actual floor predicate rejects ceiling/roof and raises only near ground\n";
}
'@
$checks = $checks.Replace('FLOOR_PREDICATE', $floor.Groups['condition'].Value)
[IO.File]::WriteAllText((Join-Path $qa 'camera-test.cpp'), $stub + "`n" + $helper + "`n" + $checks)
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vs = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | Select-Object -First 1)
if (-not $vs) { throw 'Visual Studio C++ tools not found.' }
$vcvars = Join-Path $vs 'VC\Auxiliary\Build\vcvars64.bat'
$batch = "@echo off`r`ncall `"$vcvars`" >nul`r`nif errorlevel 1 exit /b 1`r`ncl /nologo /std:c++17 /EHsc /W4 `"$qa\camera-test.cpp`" /Fe:`"$qa\camera-test.exe`" /Fo:`"$qa\camera-test.obj`"`r`nif errorlevel 1 exit /b 1`r`n`"$qa\camera-test.exe`"`r`n"
[IO.File]::WriteAllText((Join-Path $qa 'run.cmd'), $batch)
& $env:ComSpec /d /c (Join-Path $qa 'run.cmd')
if ($LASTEXITCODE) { throw 'Camera clearance fixture failed.' }
