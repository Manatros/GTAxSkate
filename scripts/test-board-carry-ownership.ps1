param([string]$Fork='')
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
if(-not $Fork){$Fork=Join-Path $root 'external\sk8v'}
$shop=[IO.File]::ReadAllText((Join-Path $Fork 'host\src\board_shop.cpp'))
$main=[IO.File]::ReadAllText((Join-Path $Fork 'host\src\main.cpp'))
$gateStart=$shop.IndexOf('    if (!busy && carryBoard) Carry(ped);')
$gateEnd=$shop.IndexOf('    const int tier = TierHere(ped);',$gateStart)
if($gateStart -lt 0 -or $gateEnd -lt 0){throw 'Production carry/clear ownership branch not found.'}
$gate=$shop.Substring($gateStart,$gateEnd-$gateStart)
$ownStart=$main.IndexOf('bool SkateOwnsPlayer(const Session& s)')
$ownEnd=$main.IndexOf('void Notify(', $ownStart)
if($ownStart -lt 0 -or $ownEnd -lt 0){throw 'Production lifecycle ownership helper not found.'}
$owns=$main.Substring($ownStart,$ownEnd-$ownStart)
$callStart=$main.IndexOf('boardshop::Tick(SkateOwnsPlayer(s)')
$callEnd=$main.IndexOf(');',$callStart)
if($callStart -lt 0 -or $callEnd -lt 0){throw 'Production shop/carry lifecycle call not found.'}
$call=$main.Substring($callStart,$callEnd-$callStart+2)
$stub=@'
#include <cassert>
#include <iostream>
#include <string_view>
enum class LifeState { Unprepared, Preparing, Ready, Waiting, Entering, Skating, Leaving, Delegated };
struct Session { LifeState state; struct {const char* state_utf8;} last; };
struct Config {bool carryBoard;};
struct SvBoardPose {};
bool visible=false,lastBusy=false;
int carries=0,clears=0;
void Carry(int ped){assert(ped==1);visible=true;++carries;}
namespace boardparts {void Hand(std::initializer_list<int> empty,SvBoardPose){assert(empty.size()==0);visible=false;++clears;}}
namespace boardshop {
void Tick(bool busy,bool carryBoard){lastBusy=busy;int ped=1;
'@
$checks=@'
int main(){
 const LifeState states[]={LifeState::Unprepared,LifeState::Preparing,LifeState::Ready,LifeState::Waiting,LifeState::Entering,LifeState::Skating,LifeState::Leaving,LifeState::Delegated};
 int cases=0;
 for(auto state:states)for(bool carry:{false,true})for(bool foot:{false,true}){
  Session s{state,{foot?"BipedGrounded":"BoardRiding"}};Config cfg{carry};
  // Begin with a previously shown GTA weapon board: every forbidden state
  // must actually clear it, including Skate's Biped state and transitions.
  visible=true;carries=clears=0;MainFrame(s,cfg);
  const bool expectedCarry=state==LifeState::Ready&&carry;
  assert(visible==expectedCarry&&carries==int(expectedCarry)&&clears==int(!expectedCarry));
  const bool expectedBusy=state==LifeState::Entering||state==LifeState::Leaving||(state==LifeState::Skating&&!foot);
  assert(lastBusy==expectedBusy); // Biped browsing stays available independently.
  ++cases;
 }
 std::cout<<"PASS: "<<cases<<" lifecycle/option/board-state cases; CarryBoard=0 clears held parts; only Ready permits optional GTA weapon board; Skate Biped browsing remains available\n";
}
'@
$qa=Join-Path $root 'build\board-carry-ownership-tests'
New-Item -ItemType Directory -Path $qa -Force|Out-Null
$cpp=$stub+"`n"+$gate+"`n}}`n"+$owns+"`nvoid MainFrame(const Session& s,const Config& cfg){"+$call+"}`n"+$checks
[IO.File]::WriteAllText((Join-Path $qa 'test.cpp'),$cpp)
$vswhere=Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vs=(& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath|Select-Object -First 1)
if(-not $vs){throw 'Visual Studio C++ tools missing.'}
$vcvars=Join-Path $vs 'VC\Auxiliary\Build\vcvars64.bat'
$batch="@echo off`r`ncall `"$vcvars`" >nul`r`nif errorlevel 1 exit /b 1`r`ncl /nologo /std:c++17 /EHsc /W4 `"$qa\test.cpp`" /Fe:`"$qa\test.exe`" /Fo:`"$qa\test.obj`"`r`nif errorlevel 1 exit /b 1`r`n`"$qa\test.exe`"`r`n"
[IO.File]::WriteAllText((Join-Path $qa 'run.cmd'),$batch)
& $env:ComSpec /d /c (Join-Path $qa 'run.cmd')
if($LASTEXITCODE){throw 'Board carry ownership fixture failed.'}
