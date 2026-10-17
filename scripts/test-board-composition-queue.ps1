param([string]$Fork='')
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
if(-not $Fork){$Fork=Join-Path $root 'external\sk8v'}
$production=[IO.File]::ReadAllText((Join-Path $Fork 'host\src\board_parts.cpp'))
$start=$production.IndexOf('std::string Composed(Set& set) {')
$end=$production.IndexOf('// Shows `set` posed by `pose`; true when every part is out and posed.',$start)
if($start -lt 0 -or $end -lt 0){throw 'Production composition queue function not found.'}
$function=$production.Substring($start,$end-$start)
$qa=Join-Path $root 'build\board-composition-queue-tests'
New-Item -ItemType Directory -Path $qa -Force|Out-Null
$scratch=Join-Path $qa ([IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Path $scratch|Out-Null
$header=(Join-Path $Fork 'host\src\board_composition_key.h').Replace('\','/')
$stub=@'
#include <chrono>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <iterator>
#include <map>
#include <string>
#include <vector>
#include <cstdlib>
using ULONGLONG=unsigned long long;
ULONGLONG tick=1000;
ULONGLONG GetTickCount64(){return tick;}
std::filesystem::path g_root,g_compositionRoot;
std::string g_compositionSession="1234";
bool g_compose=true;
std::map<std::filesystem::path,std::pair<std::uintmax_t,std::filesystem::file_time_type>> g_compositionInputs;
struct Set {
 std::vector<std::string> models;
 std::string composition;
 ULONGLONG compositionChecked=0;
 bool compositionPublished=false,compositionErrorLogged=false;
};
std::vector<std::string> logs;
int launches=0,cases=0;
void Log(const char* text){logs.emplace_back(text);}
void Logf(const char* fmt,const char* a,const char* b=""){char line[1024];std::snprintf(line,sizeof(line),fmt,a,b);Log(line);}
void EnsureCompositionWorker(){++launches;}
void require(bool ok,const char* label){++cases;if(!ok){std::cerr<<"FAIL: "<<label<<'\n';std::exit(1);}}
void write(const std::filesystem::path& path,const std::string& value){std::ofstream out(path,std::ios::binary|std::ios::trunc);out<<value;out.close();require(bool(out),"fixture file write");}
std::string read(const std::filesystem::path& path){std::ifstream in(path,std::ios::binary);return std::string(std::istreambuf_iterator<char>(in),{});}
int countLog(const std::string& text){int n=0;for(const auto& log:logs)if(log.find(text)!=std::string::npos)++n;return n;}
'@
$checks=@'
int main(int argc,char** argv){
 require(argc==2,"scratch directory argument");
 const std::filesystem::path scratch(argv[1]);
 g_root=scratch/"private"/"board-parts";
 g_compositionRoot=scratch/"private"/"board-composed";
 const auto native=scratch/"private"/"board-native"/"skatev_board.ydr";
 std::filesystem::create_directories(g_root);std::filesystem::create_directories(g_compositionRoot);std::filesystem::create_directories(native.parent_path());
 write(native,"native-original");
 const std::vector<std::string> a{"skatev_d_default","skatev_t_default","skatev_w_default"};
 const std::vector<std::string> b{"skatev_w_blue"};
 for(const auto& m:a)write(g_root/(m+".ydr"),"original-"+m);
 write(g_root/(b[0]+".ydr"),"original-blue");
 Set set;set.models=a;
 require(Composed(set).empty(),"pending preparation returns no composed model");
 const auto nameA=set.composition;
 const auto requestA=g_compositionRoot/(nameA+".request");
 const auto expected=boardcomposition::Payload(a);
 require(read(requestA)==expected,"exact selected vector request");
 require(set.models==a,"pending keeps selected part vector for Show fallback");
 require(launches==1,"pending ensures worker");
 const auto requestTime=std::filesystem::last_write_time(requestA);
 write(g_compositionRoot/(nameA+".ready"),"skatev_d_wrong\n");
 write(g_compositionRoot/(nameA+".ydr"),"verified-drawable");
 write(g_compositionRoot/(nameA+".ytyp"),"verified-archetype");
 tick+=500;
 require(Composed(set).empty(),"ready vector mismatch rejected");
 require(!set.compositionPublished,"mismatch never marks publication");
 require(std::filesystem::last_write_time(requestA)==requestTime,"existing request never rewritten");
 write(g_compositionRoot/(nameA+".ready"),expected);
 const int beforeThrottle=launches;
 ++tick;
 require(Composed(set).empty()&&launches==beforeThrottle,"pending lookup throttled for 500ms");
 tick+=500;
 require(Composed(set)==nameA&&set.compositionPublished,"matching ready and both resources accepted");
 ++tick;
 require(Composed(set)==nameA,"published result survives throttle");
 set.models=b;
 require(Composed(set).empty(),"new selection resets published state immediately");
 const auto nameB=set.composition;
 require(nameB!=nameA&&!set.compositionPublished,"new vector has distinct unpublished identity");
 require(read(g_compositionRoot/(nameB+".request"))==boardcomposition::Payload(b),"single-part switch exact request");
 write(g_compositionRoot/(nameB+".error"),"export failed");
 tick+=500;require(Composed(set).empty(),"failed preparation retains fallback");
 const int failedLaunches=launches;
 tick+=500;require(Composed(set).empty(),"repeated failure retains fallback");
 require(countLog("preparation failed for")==1&&launches==failedLaunches,"failure logged once and no worker restart for failed selection");
 set.models=a;
 require(Composed(set)==nameA,"return to previous selection reuses matching published resources");
 require(!set.compositionErrorLogged,"selection change resets error log gate");
 require(read(native)=="native-original"&&read(g_root/(a[0]+".ydr"))=="original-"+a[0],"queue never changes source assets");
 write(g_root/(a[0]+".ydr"),"changed source with a new size");
 tick+=500;
 require(Composed(set).empty()&&!g_compose,"source size change disables composition");
 require(countLog("source assets changed")==1,"source change logged");
 // Independent same-size timestamp-only change also invalidates a session.
 g_compose=true;g_compositionInputs.clear();set=Set{};set.models=b;tick+=500;
 require(Composed(set).empty(),"new timestamp test session starts pending");
 const auto sourceB=g_root/(b[0]+".ydr");
 const auto size=std::filesystem::file_size(sourceB);
 std::filesystem::last_write_time(sourceB,std::filesystem::last_write_time(sourceB)+std::chrono::seconds(5));
 tick+=500;
 require(Composed(set).empty()&&!g_compose&&std::filesystem::file_size(sourceB)==size,"mtime-only source change disables composition");
 std::cout<<"PASS: "<<cases<<" production queue checks; exact requests, throttling, fallback, readiness, selection return, errors and source changes\n";
}
'@
$cpp='#include "'+$header+'"'+"`n"+$stub+"`n"+$function+"`n"+$checks
[IO.File]::WriteAllText((Join-Path $qa 'test.cpp'),$cpp)
$vswhere=Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vs=(& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath|Select-Object -First 1)
if(-not $vs){throw 'Visual Studio C++ tools missing.'}
$vcvars=Join-Path $vs 'VC\Auxiliary\Build\vcvars64.bat'
$batch="@echo off`r`ncall `"$vcvars`" >nul`r`nif errorlevel 1 exit /b 1`r`ncl /nologo /std:c++17 /EHsc /W4 `"$qa\test.cpp`" /Fe:`"$qa\test.exe`" /Fo:`"$qa\test.obj`"`r`nif errorlevel 1 exit /b 1`r`n`"$qa\test.exe`" `"$scratch`"`r`n"
[IO.File]::WriteAllText((Join-Path $qa 'run.cmd'),$batch)
& $env:ComSpec /d /c (Join-Path $qa 'run.cmd')
if($LASTEXITCODE){throw 'Board composition production queue fixture failed.'}
