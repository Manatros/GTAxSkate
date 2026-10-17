param([string]$Fork='')
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
if(-not $Fork){$Fork=Join-Path $root 'external\sk8v'}
$qa=Join-Path $root 'build\board-composition-key-tests'
New-Item -ItemType Directory -Path $qa -Force|Out-Null
$header=(Join-Path $Fork 'host\src\board_composition_key.h').Replace('\','/')
# Expected values are independent Python-worker outputs from the real resource
# roundtrips in build/board-composition-real-20261016, not recalculated by C++.
$checks=@'
#include <algorithm>
#include <cstdlib>
#include <iostream>
#include <string>
#include <vector>
using Models=std::vector<std::string>;
int cases=0;
void require(bool ok,const char* label){++cases;if(!ok){std::cerr<<"FAIL: "<<label<<'\n';std::exit(1);}}
void valid(const Models& models,const char* key){
 const auto payload=boardcomposition::Payload(models);
 require(!payload.empty(),"valid model vector");
 std::string expected;for(const auto& model:models)expected+=model+'\n';
 require(payload==expected,"exact newline-framed request payload");
 require(boardcomposition::Key(payload)==key,"Python worker key compatibility");
 if(models.size()>1){auto reversed=models;std::reverse(reversed.begin(),reversed.end());
  require(boardcomposition::Key(boardcomposition::Payload(reversed))!=key,"order-sensitive key");}
}
void invalid(const Models& models){require(boardcomposition::Payload(models).empty(),"invalid vector rejected");}
int main(){
 valid({"skatev_w_2c7f381700171ab4"},"48d98ad8dee25859");
 valid({"skatev_d_2c7f381700081a5b","skatev_t_2c7f3817001716c1","skatev_w_2c7f381700171ab4"},"c2d9ece5586344af");
 valid({"skatev_d_2c7f381700081a5b","skatev_tb_blackg","skatev_th_blackm","skatev_w_2c7f381700171ab4","skatev_g_a841_black"},"9dda49593743e032");
 valid({"skatev_d_2c7f381700081a5b","skatev_g_a841_black"},"e95923d24b5612be");
 invalid({});
 Models nine;for(int i=0;i<9;++i)nine.push_back("skatev_w_"+std::to_string(i));invalid(nine);
 Models eight(nine.begin(),nine.begin()+8);require(!boardcomposition::Payload(eight).empty(),"eight models accepted");
 invalid({"skatev_d_default","skatev_d_default"});
 for(const auto& bad:Models{"../escape","skatev_d_../escape","skatev_d_..\\escape","skatev_d_a/b","skatev_d_a\\b","skatev_d_a\nskatev_w_default","skatev_d_a\r","skatev_x_default","skatev_d_","skatev_D_default","skatev_d_White","skatev_d_a:stream"})invalid({bad});
 invalid({"skatev_d_"+std::string(129,'a')});
 invalid({"skatev_th_"+std::string(129,'a')});
 require(!boardcomposition::Payload({"skatev_th_"+std::string(128,'a')}).empty(),"128-character suffix accepted");
 std::cout<<"PASS: "<<cases<<" production header checks; Python real-resource keys, ordered payloads, bounds and injection rejection\n";
}
'@
$cpp='#include "'+$header+'"'+"`n"+$checks
[IO.File]::WriteAllText((Join-Path $qa 'test.cpp'),$cpp)
$vswhere=Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vs=(& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath|Select-Object -First 1)
if(-not $vs){throw 'Visual Studio C++ tools missing.'}
$vcvars=Join-Path $vs 'VC\Auxiliary\Build\vcvars64.bat'
$batch="@echo off`r`ncall `"$vcvars`" >nul`r`nif errorlevel 1 exit /b 1`r`ncl /nologo /std:c++17 /EHsc /W4 `"$qa\test.cpp`" /Fe:`"$qa\test.exe`" /Fo:`"$qa\test.obj`"`r`nif errorlevel 1 exit /b 1`r`n`"$qa\test.exe`"`r`n"
[IO.File]::WriteAllText((Join-Path $qa 'run.cmd'),$batch)
& $env:ComSpec /d /c (Join-Path $qa 'run.cmd')
if($LASTEXITCODE){throw 'Board composition production key fixture failed.'}
