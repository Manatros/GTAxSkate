$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$source = [IO.File]::ReadAllText((Join-Path $root 'external\sk8v\host\src\main.cpp'))
$start = $source.IndexOf('bool SkateMouseLook()')
$end = $source.IndexOf('// Radio while skating', $start)
if ($start -lt 0 -or $end -lt 0) { throw 'Actual camera-input helpers not found.' }
$helpers = $source.Substring($start, $end - $start)
$qa = Join-Path $root 'build\controller-camera-tests'
New-Item -ItemType Directory -Path $qa -Force | Out-Null
$stub = @'
#include <cstdint>
#include <cassert>
#include <vector>
#include <algorithm>
#include <iostream>
using BOOL=int;
bool g_controller=false,g_weaponWheel=false,g_keyboard=true,g_gtaView=true,g_orbitOn=false;
bool nativeMouse=true,menuOpen=false;
float raw[360]{};
std::uint8_t g_controlIdle[360]{};
constexpr std::uint8_t kControlHoldFrames=10;
constexpr int kPartControlsRead=1;
void Lap(int){}
namespace menu { bool IsOpen(){return menuOpen;} }
struct Disable {int group,control,value;};
std::vector<Disable> disabled;
int axisReads=0;
namespace gta {
constexpr int IS_USING_KEYBOARD_AND_MOUSE=1,GET_DISABLED_CONTROL_NORMAL=2,DISABLE_CONTROL_ACTION=3;
template<class T>T Call(int native,int){assert(native==IS_USING_KEYBOARD_AND_MOUSE);return T(nativeMouse);}
template<class T>T Call(int native,int group,int c){assert(native==GET_DISABLED_CONTROL_NORMAL&&group==0);++axisReads;return T(raw[c]);}
template<class T>T Call(int native,int group,int c,int value){assert(native==DISABLE_CONTROL_ACTION);disabled.push_back({group,c,value});}
}
using gta::Call;
'@
$checks = @'
void reset(){disabled.clear();axisReads=0;std::fill(std::begin(raw),std::end(raw),0.0f);std::fill(std::begin(g_controlIdle),std::end(g_controlIdle),std::uint8_t{255});g_controller=false;g_weaponWheel=false;nativeMouse=true;menuOpen=false;}
int main(){
    float x=99,y=99;
    reset();raw[1]=0.6f;raw[2]=-0.8f;
    ReadSkateCameraLook(x,y);assert(x==0.6f&&y==-0.8f&&axisReads==2);
    disabled.clear();DisableSkateControls(0,false);
    assert(disabled.empty()); // active mouse look stays GTA's in GTA/orbit view
    reset();g_controller=true;raw[1]=0.6f;raw[2]=-0.8f;
    // Even a stale/mixed-device GTA mouse flag must not turn tricks into look.
    ReadSkateCameraLook(x,y);assert(x==0&&y==0&&axisReads==0);
    DisableControllerCameraLook(false);
    assert(disabled.size()==2&&disabled[0].control==1&&disabled[1].control==2);
    assert(disabled[0].group==0&&disabled[0].value==1&&disabled[1].value==1);
    reset();g_controller=true;DisableSkateControls(0,false);
    assert(disabled.size()==2); // look remains protected after arbitrary idle time
    reset();g_controller=true;DisableSkateControls(0,true);assert(disabled.empty()); // aim
    reset();g_controller=true;g_weaponWheel=true;DisableSkateControls(0,false);assert(disabled.empty());
    reset();g_controller=true;menuOpen=true;DisableSkateControls(0,false);assert(disabled.empty()&&axisReads==0);
    reset();nativeMouse=false;raw[1]=0.5f;ReadSkateCameraLook(x,y);assert(x==0&&y==0&&axisReads==0);
    reset();raw[1]=0.3f;ReadSkateCameraLook(x,y);assert(x==0.3f); // switching back restores mouse
    std::cout<<"PASS: mouse preserved; controller trick axes never consumed as look; stale device flag guarded; idle suppression; aim/wheel/menu retained; device switch\n";
}
'@
[IO.File]::WriteAllText((Join-Path $qa 'controller-test.cpp'),$stub+"`n"+$helpers+"`n"+$checks)
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vs = (& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | Select-Object -First 1)
if (-not $vs) { throw 'Visual Studio C++ tools not found.' }
$vcvars = Join-Path $vs 'VC\Auxiliary\Build\vcvars64.bat'
$batch = "@echo off`r`ncall `"$vcvars`" >nul`r`nif errorlevel 1 exit /b 1`r`ncl /nologo /std:c++17 /EHsc /W4 `"$qa\controller-test.cpp`" /Fe:`"$qa\controller-test.exe`" /Fo:`"$qa\controller-test.obj`"`r`nif errorlevel 1 exit /b 1`r`n`"$qa\controller-test.exe`"`r`n"
[IO.File]::WriteAllText((Join-Path $qa 'run.cmd'),$batch)
& $env:ComSpec /d /c (Join-Path $qa 'run.cmd')
if ($LASTEXITCODE) { throw 'Controller camera fixture failed.' }
