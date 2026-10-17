"""Execute the shipped Lua bridge with deterministic FiveM event/wait stubs.

Optional test dependency: lupa==2.6 (Lua 5.4), installed locally, not into GTA.
Run: python fivem/tests/bridge_runtime_test.py
Or: python -m pip install --target build/qa-lua-deps lupa==2.6
"""
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "build/qa-lua-deps"))
from lupa.lua54 import LuaRuntime

STUBS = """
net, events, commands = {}, {}, {}
clock, stopped, resets = 100, 0, 0
resources = { gtaxskate_rules = 'started', gtaxskate_sk8m = 'started' }
Config = { PushMax = 10, PushImpulse = 2, BrakeDecel = 5, TurnRate = 20 }
Tune = { reset = function() resets = resets + 1 end }
function GetGameTimer() return clock end
function GetResourceState(name) return resources[name] or 'missing' end
function GetCurrentResourceName() return 'skateboard' end
function RegisterNetEvent(name, fn) net[name] = fn end
function AddEventHandler(name, fn) events[name] = fn end
function RegisterCommand(name, fn) commands[name] = fn end
function CreateThread(fn) lifecycleThread = fn end
function TriggerServerEvent(name, ...) lastEvent, lastArgs = name, {...} end
function Wait(ms) clock = clock + 1; if waiting then waiting() end end
function policy(multiplier, revision, generation)
    return { version = 1, generation = generation or 'epoch-a', revision = revision or 1,
        settings = { PushSpeed = multiplier or 1, PushPower = 1, Braking = 1, Steering = 1 } }
end
function deliver(state) source = 65535; net['gtaxskate:sk8m:state'](state) end
function approve()
    source = 65535
    net['skateboard:boardReply'](true, lastArgs[1], lastArgs[2], lastArgs[3])
end
"""


class BridgeRuntimeTests(unittest.TestCase):
    def setUp(self):
        self.lua = LuaRuntime(unpack_returned_tuples=True)
        self.lua.execute(STUBS)
        self.lua.execute((ROOT / "fivem/gtaxskate_sk8m/bridge.lua").read_text(encoding="utf-8"))
        self.lua.execute("Friends.stop = function() stopped = stopped + 1 end")

    def run_lua(self, script):
        self.lua.execute(script)

    def test_duplicate_snapshot_preserves_waiting_request_and_grant(self):
        self.run_lua("""
        deliver(policy())
        waiting = function() deliver(policy()); approve() end
        assert(Friends.authorize(), 'same-state sync cancelled an in-flight mount')
        assert(Friends.granted())
        deliver(policy())
        assert(Friends.granted(), 'same-state diagnostics revoked a valid grant')
        assert(stopped == 1 and resets == 1, 'duplicate state restarted gameplay/tuning')
        """)

    def test_same_revision_player_edit_cancels_old_mount_and_rebuilds_baseline(self):
        self.run_lua("""
        deliver(policy())
        waiting = function() deliver(policy(2)); approve() end
        assert(not Friends.authorize(), 'old mount survived changed effective settings')
        assert(not Friends.granted())
        assert(Config.PushMax == 20 and stopped == 2)
        deliver(policy(3)); assert(Config.PushMax == 30)
        deliver(policy(1)); assert(Config.PushMax == 10, 'cumulative scaling')
        """)

    def test_policy_reload_and_resource_loss_revoke_grants(self):
        self.run_lua("""
        deliver(policy()); waiting = approve
        assert(Friends.authorize())
        deliver(policy(1, 2)); assert(not Friends.granted())
        assert(Friends.authorize())
        events.onClientResourceStop('gtaxskate_rules')
        assert(not Friends.canRide() and not Friends.granted())
        approve(); assert(not Friends.granted(), 'late reply resurrected grant')
        """)

    def test_non_server_events_cannot_change_state_or_authorize(self):
        self.run_lua("""
        source = 0; net['gtaxskate:sk8m:state'](policy())
        assert(not Friends.canRide())
        deliver(policy())
        waiting = function()
            source = 0
            net['skateboard:boardReply'](true, lastArgs[1], lastArgs[2], lastArgs[3])
        end
        assert(not Friends.authorize(), 'local event granted a mount')
        """)

    def test_invalid_policy_fails_closed(self):
        self.run_lua("""
        for _, revision in ipairs({0, -1, 0.5, math.huge, 0/0}) do
            deliver(policy()); deliver(policy(1, revision))
            assert(not Friends.canRide(), 'invalid revision accepted')
        end
        deliver(policy()); deliver(policy(1, 1, ''))
        assert(not Friends.canRide(), 'empty generation accepted')
        deliver(policy()); local bad = policy(); bad.settings.Braking = math.huge
        deliver(bad); assert(not Friends.canRide())
        """)

    def test_existing_canride_gate_remains_composed(self):
        # The previous callback is captured when the bridge loads.
        lua = LuaRuntime(unpack_returned_tuples=True)
        lua.execute(STUBS + "\nConfig.CanRide = function() return false end")
        lua.execute((ROOT / "fivem/gtaxskate_sk8m/bridge.lua").read_text(encoding="utf-8"))
        lua.execute("deliver(policy()); assert(not Config.CanRide(123))")


if __name__ == "__main__":
    unittest.main(verbosity=2)
