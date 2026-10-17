"""Execute the actual pinned/overlaid Lua mount function; local lupa dependency.

python -m pip install --target build/lua-test-deps lupa
python scripts/test-fivem-mount.py
No FiveM server/game assets or running server required.
"""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'build/lua-test-deps'))
from lupa.lua54 import LuaRuntime

original = subprocess.check_output([
    'git', '-C', str(ROOT / 'external/sk8m'), 'show',
    '50b923097691d7c73d6e8eb719cffc5cab40d138:fivem/skateboard/client.lua'
], text=True)
overlaid = subprocess.check_output(['node', '-e',
    "const p=require('./fivem/gtaxskate_sk8m/prepare.js');"
    "process.stdout.write(p.overlayClient(require('fs').readFileSync(0,'utf8')))",
], input=original, cwd=ROOT, text=True)

def mount_body(source):
    start = source.index('local function mount(from)')
    end = source.index('local function getOn()', start)
    return source[start:end] + '\nreturn mount'

def run(source, model, entity, authorized=True):
    lua = LuaRuntime()
    lua.execute('''
      Config={CallBoard=true,NetworkBoard=true,BoardLodDist=120}
      sk={}; Friends={authorize=function() return auth end}; SK={fit=function() return true end}
      notices={}; released=0
      function PlayerPedId() return 1 end
      function GetEntityCoords() return {x=1,y=2,z=3} end
      function ownsBoard() return true end
      function pickBoardModel() return chosen_model,false end
      function CreateObject() return chosen_entity end
      function SetModelAsNoLongerNeeded() released=released+1 end
      function SetEntityLodDist() end
      function addBoardTarget() end
      function DetachEntity() end
      function SetEntityCollision() end
      function DoesEntityExist() return true end
      function vector3() error('ride path reached') end
      function notify(s) table.insert(notices,s) end
    ''')
    lua.globals().chosen_model = model
    lua.globals().chosen_entity = entity
    lua.globals().auth = authorized
    fn = lua.execute(mount_body(source))
    try:
        fn()
        return True, list(lua.globals().notices.values()), lua.globals().released
    except Exception as exc:
        if 'ride path reached' not in str(exc):
            raise
        return False, [], lua.globals().released

# Reproduce the former invisible ride path before checking the correction.
assert run(original, None, None)[0] is False
assert run(original, 123, 0)[0] is False
for model, entity in [(None, None), (123, 0), (123, None)]:
    stopped, notices, released = run(overlaid, model, entity)
    assert stopped and len(notices) == 1, (model, entity, notices)
    assert released == (0 if model is None else 1)
assert run(overlaid, 123, 456)[0] is False  # valid board still enters ride path
assert run(overlaid, 123, 456, authorized=False)[0] is True
print('PASS: actual Lua reproduces old invisible ride; missing/zero/nil board aborts; valid board proceeds; authorization retained')
