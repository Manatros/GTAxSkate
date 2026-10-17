param([string]$Fork='')
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
if(-not $Fork){$Fork=Join-Path $root 'external/sk8v'}
$qa=Join-Path $root 'build/play-profile-tests'
New-Item -ItemType Directory -Force -Path $qa|Out-Null
# Extract production declarations and constructor calls, retaining actual ranges,
# defaults and profile overrides. Only GTA callbacks/formatting are substituted.
$generator=@'
import re,sys
from pathlib import Path
fork,qa=map(Path,sys.argv[1:])
src=(fork/'host/src/main.cpp').read_text(encoding='utf-8-sig')
def balanced(text,start,op='(',cl=')'):
    n=0; quote=None; escape=False; line=False; block=False; i=start
    while i<len(text):
        c=text[i]; two=text[i:i+2]
        if line:
            if c=='\n': line=False
        elif block:
            if two=='*/': block=False;i+=1
        elif quote:
            if escape:escape=False
            elif c=='\\':escape=True
            elif c==quote:quote=None
        elif two=='//':line=True;i+=1
        elif two=='/*':block=True;i+=1
        elif c=='"':quote=c
        elif c==op:n+=1
        elif c==cl:
            n-=1
            if n==0:return i+1
        i+=1
    raise RuntimeError('Unbalanced production expression')
def strip_calls(text,name,replacement):
    while m:=re.search(r'\b'+name+r'\s*\(',text):
        end=balanced(text,text.index('(',m.start()))
        text=text[:m.start()]+replacement+text[end:]
    return text
def declaration(prefix):
    start=src.index(prefix);op=src.index('{',start)
    return src[start:balanced(src,op,'{','}')+1]
config=declaration('struct Config {')
constants=src[src.index('const char* const kDifficulties'):src.index('const Key kKeys[]')]
menu=src[src.index('std::vector<menu::Page> MenuPages'):src.index('\nvoid ScriptMain()')]
rows=[]
for m in re.finditer(r'\b(Flag|Number|Choice|Chord|Emote)\s*\(',menu):
    end=balanced(menu,menu.index('(',m.start()))
    expr=menu[m.start():end]
    expr=strip_calls(expr,'Bound','{}')
    expr=strip_calls(expr,'Fixed','{}')
    # Pure label formatting is irrelevant to validation; inline callbacks cannot
    # be linked outside GTA. Keep all arguments defining keys/defaults/ranges.
    expr=re.sub(r'\[&s\]\(const std::string& v\)\s*\{.*\}\s*\)$','{}\n)',expr,flags=re.S)
    expr=re.sub(r'\[\]\(const std::string& v\)\s*\{.*\}', '{}',expr,flags=re.S)
    expr=re.sub(r'\[\]\(float v\)\s*\{.*\}', '{}',expr,flags=re.S)
    if 'LimitText' in expr:expr=expr.replace('LimitText','{}')
    rows.append('pages[0].items.push_back('+expr+');')
profiles=[]
for m in re.finditer(r'\bApplyPreset\("(SK8V|RP|Developer)"\s*,',src):
    end=balanced(src,src.index('(',m.start()))
    profiles.append(src[m.start():end].replace('ApplyPreset(', 'CapturedPreset(',1)+';')
assert len(profiles)==3,profiles
pre=r'''
#include "menu_presets.h"
#include <cassert>
#include <cstdint>
#include <iostream>
namespace lifecycle {struct Settings{std::uint16_t menuButton=0x0120;bool inMissions=false,backgroundPrepare=true;};}
struct Session{};
'''
helpers=r'''
menu::Item Emote(const char* label,int direction,const Config& D){
 std::vector<std::string> values{"None"};values.insert(values.end(),std::begin(kGestures),std::end(kGestures));
 auto i=menu::Choice(label,kGestureKeys[direction],values,values);i.def=D.gestures[direction]<0?"None":kGestures[D.gestures[direction]];return i;
}
std::map<std::string,menu::presets::Settings> profiles;
std::map<std::string,std::vector<std::pair<std::string,std::string>>> overridesByProfile;
std::vector<menu::Page> pages;
void CapturedPreset(const std::string& name,const std::vector<std::pair<std::string,std::string>>& overrides){
 std::string error;menu::presets::Settings result;
 if(!menu::presets::Build(pages,{},overrides,result,error)){std::cerr<<name<<": "<<error<<"\n";std::exit(1);}
 profiles[name]=std::move(result);
 overridesByProfile[name]=overrides;
}
int main(){using namespace menu;const Config D{};pages.push_back({"All production settings",{}});
'''
checks=r'''
 const auto& base=profiles.at("SK8V");const auto& rp=profiles.at("RP");const auto& dev=profiles.at("Developer");
 assert(base.size()>100&&rp.size()==base.size()&&dev.size()==base.size());
 assert(base.at("Difficulty")=="Easy"&&base.at("Camera")=="High");
 assert(base.at("PedLaunch")=="1"&&base.at("BackwardsMan")=="1");
 for(const auto* key:{"SkateHud","Hud","ShowInputs","ShowDebug","KeyboardHints","TrickAnalyzer","HallOfMeat","HallOfMeatXray","BackwardsMan","PedLaunch","VehicleDamage"})assert(rp.at(key)=="0");
 assert(rp.at("Stamina")=="1"&&rp.at("Difficulty")=="Hardcore");
 assert(std::stof(rp.at("Pop"))<std::stof(base.at("Pop")));
 for(const auto* key:{"Runtime","PosePed","DrawBoard","BoardNative"})assert(rp.at(key)=="1"&&dev.at(key)=="1");
 assert(dev.at("ThreadCensus")=="0"&&dev.at("StallSampler")=="0"&&dev.at("LiveClip")=="0");
 // Complete results replace a previous profile rather than inherit untouched keys.
 auto active=dev;std::string error;active["ObsoleteValue"]="1";
 assert(menu::presets::Build(pages,{},overridesByProfile.at("RP"),active,error));assert(active==rp);
 assert(menu::presets::Build(pages,{},overridesByProfile.at("SK8V"),active,error));assert(active==base);
 auto corrupt=base;corrupt["Pop"]="nan";assert(!menu::presets::Validate(pages,corrupt,error));
 corrupt=base;corrupt.erase("Hud");assert(!menu::presets::Validate(pages,corrupt,error));
 corrupt=base;corrupt["NotASetting"]="1";assert(!menu::presets::Validate(pages,corrupt,error));
 std::cout<<"PASS: production schema "<<base.size()<<" keys, all defaults valid, SK8V/RP/Developer complete, grounded RP and conservative diagnostics\n";
}
'''
(qa/'test.cpp').write_text(pre+config+'\n'+constants+'\n'+helpers+'\n'.join(rows)+'\n'+'\n'.join(profiles)+checks,encoding='utf-8')
'@
[IO.File]::WriteAllText((Join-Path $qa 'generate.py'),$generator)
& python (Join-Path $qa 'generate.py') $Fork $qa
if($LASTEXITCODE){throw 'Could not extract production profiles.'}
$vswhere=Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vs=(& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath|Select-Object -First 1)
if(-not $vs){throw 'Visual Studio C++ tools missing.'}
$vcvars=Join-Path $vs 'VC/Auxiliary/Build/vcvars64.bat'
$batch="@echo off`r`ncall `"$vcvars`" >nul`r`nif errorlevel 1 exit /b 1`r`ncl /nologo /std:c++20 /EHsc /W3 /I`"$Fork/host/src`" `"$qa/test.cpp`" /Fe:`"$qa/test.exe`" /Fo:`"$qa/test.obj`"`r`nif errorlevel 1 exit /b 1`r`n`"$qa/test.exe`"`r`n"
[IO.File]::WriteAllText((Join-Path $qa 'run.cmd'),$batch)
& $env:ComSpec /d /c (Join-Path $qa 'run.cmd')
if($LASTEXITCODE){throw 'Production play profile validation failed.'}
$iconChecks=@'
import ast,runpy,sys,zlib,types,shutil,subprocess
from pathlib import Path
fork,qa=map(Path,sys.argv[1:]);sys.path.insert(0,str(fork/'tools'))
icon=runpy.run_path(str(fork/'tools/build-board-icon.py'))
plain=b'GFX'+bytes([10])+b'\x20\x00\x00\x00'+b'test payload'
compressed=b'CFX'+plain[3:8]+zlib.compress(plain[8:])
assert icon['uncompressed'](plain)==plain
assert icon['uncompressed'](compressed)==plain
try:icon['uncompressed'](b'CFX'+plain[3:8]+b'bad')
except zlib.error:pass
else:raise AssertionError('Corrupt CFX accepted')
# Run the real command entry point; a preexisting icon must bypass another edit.
already=plain+f"INT{icon['HASH']}".encode()
for suffix,data in [('gfx',already),('cfx',b'CFX'+already[3:8]+zlib.compress(already[8:]))]:
    source=qa/('source.'+suffix);out=qa/('copied.'+suffix);source.write_bytes(data)
    subprocess.run([sys.executable,'-B',str(fork/'tools/build-board-icon.py'),'--source',str(source),'--out',str(out)],check=True)
    assert out.read_bytes()==already
# Execute the actual production preparation method against controlled extraction
# and builder services. It must use the modified HUD if valid, else vanilla.
tree=ast.parse((fork/'tools/prepare.py').read_text(encoding='utf-8-sig'))
cls=next(x for x in tree.body if isinstance(x,ast.ClassDef) and x.name=='Prepare')
method=next(x for x in cls.body if isinstance(x,ast.FunctionDef) and x.name=='stage_icon')
for fail in [False,True]:
    case=qa/('icon-fallback-'+str(fail));case.mkdir(exist_ok=True)
    calls=[]
    def extract(tool,keys,archive,patterns,out):
        out.mkdir(parents=True,exist_ok=True);(out/'hud.gfx').write_bytes(b'movie');calls.append(('extract',str(archive)))
    def py(script,*args):
        source=Path(args[1]);calls.append(('build',str(source)))
        if fail and 'hud-mods' in source.parts:raise subprocess.CalledProcessError(1,['builder'])
    env={'shutil':shutil,'subprocess':subprocess,'gta_extract':types.SimpleNamespace(extract=extract,ExtractError=RuntimeError),'binary':lambda x:x,'RPF':'reader','py':py}
    exec(compile(ast.fix_missing_locations(ast.Module(body=[method],type_ignores=[])),'production-stage-icon','exec'),env)
    self=types.SimpleNamespace(work=case/'work',gta=case/'gta',clip=case/'clip',keys='test',hud_mod=lambda:case/'mods/update.rpf')
    env['stage_icon'](self)
    assert sum(c[0]=='build' for c in calls)==(2 if fail else 1)
    assert sum(c[0]=='extract' for c in calls)==(2 if fail else 1)
    if fail:assert calls[-1][1].endswith('hud/hud.gfx') or calls[-1][1].endswith('hud\\hud.gfx')
print('PASS: official 1.1.10 icon GFX/CFX, corrupt compressed input, existing-icon CLI idempotence, modified-HUD success and vanilla fallback')
'@
[IO.File]::WriteAllText((Join-Path $qa 'icon-checks.py'),$iconChecks)
& python -B (Join-Path $qa 'icon-checks.py') $Fork $qa
if($LASTEXITCODE){throw 'Official upstream icon checks failed.'}
