# Issue #1 — controller trick input moving the camera

## Report and prior work

[Manatros/GTAxSkate #1](https://github.com/Manatros/GTAxSkate/issues/1), open,
created 2026-10-13: controller tricks also move the camera wildly. Public issue
body read through GitHub REST on 2026-10-14. Read BACKLOG A5 (mouse-controlled
GTA-style camera, V views), existing mouse/controller selection, camera history,
and the newly integrated world-clearance changes before edits. Upstream source
tag is 1.1.8, current fork baseline `2c31840`; no history rewrite or alternate
camera implementation needed.

The earlier camera-clearance fix addresses world penetration, not input
ownership. It remains intact. ReadPad already arbitrates XInput/native
PlayStation/keyboard and publishes `g_controller`; reuse that existing owner.

## Source findings

1. HeadCamera read `GET_DISABLED_CONTROL_NORMAL(0,1/2)` unconditionally.
   Disabled control values still contain right-stick movement: disabling GTA
   controls does not make those values safe mouse-only deltas. Controller trick
   flicks therefore fed the script camera's look yaw/pitch.
2. Orbit and GTA-look exceptions relied only on
   IS_USING_KEYBOARD_AND_MOUSE, rather than the host's actual selected input
   source. A stale/mixed-device GTA flag could allow controller movement into
   mouse camera handling.
3. Generic input suppression is activity/idle optimized. Preserve its existing
   implementation and replay/pause exceptions, while explicitly protecting only
   look axes 1/2 each controller skating frame.

## Narrow fix

- `SkateMouseLook()` permits camera mouse input only when the host's active
  source is not controller and GTA reports keyboard/mouse.
- `ReadSkateCameraLook()` returns zero without reading look axes for controller
  input; both OrbitCamera and HeadCamera use it. The original right stick still
  reaches Skate through ReadPad unchanged.
- The generic look exception uses the same predicate. No default view, key,
  controller button, preset or saved setting changes.
- `DisableControllerCameraLook()` explicitly suppresses axes 1/2 while the
  controller owns skating. Aiming and GTA's weapon wheel retain their own
  right-stick behavior. An open settings menu retains existing menu control
  ownership. Non-skating GTA movement is untouched.
- Only two bounded extra native disable calls per controller skating frame;
  no new world probes, allocations, timers or network work.

The existing native wrapper exposes the real GTA hash. The public
[Cfx native declaration](https://raw.githubusercontent.com/citizenfx/natives/master/PAD/DisableControlAction.md)
names the third boolean `disable` and demonstrates true to suppress input.
The prior generic suppression's comments interpret that flag as related-action
walking; this change deliberately does not rewrite every existing control call
on that basis. It applies explicit suppression only to the two conflicting
look axes and still leaves weapon aim/wheel as exceptions.

Aim-path review: BoardControls sets `s.aiming = true` and immediately calls
GameplayCam. GameplayCam calls `RENDER_SCRIPT_CAMS(0, ...)`, handing rendering
to GTA's gameplay camera. UpdateCameraView will not restore a script camera
while `s.aiming`; the Developer A/B transition likewise restores only when not
aiming and not in the wheel. Orbit/Head still update their unrendered camera
objects, so zero controller deltas there do not suppress the visible GTA aim
camera. StopAim restores scripted rendering after aim ends. Weapon-wheel entry
also calls GameplayCam, with restoration guarded by `!s.aiming`. This is existing
call-graph evidence, not a fixture assertion about live GTA camera behavior.

## Tests and handoff

`scripts/test-controller-camera.ps1` extracts the actual helper and existing
DisableSkateControls implementation from main.cpp, compiles against small native
stubs using existing MSVC tools, then checks:

| Case | Expected / result |
|---|---|
| Active mouse in GTA/orbit view | Mouse values retained; look controls not suppressed — pass |
| Controller trick with GTA mouse flag still true | Script look zero; no raw axis read — pass |
| Controller look suppression | Correct axes/group and explicit true flag — pass |
| Controller after arbitrary idle interval | Both look controls still suppressed — pass |
| Aiming / weapon wheel | Controller look not suppressed by the new guard — pass |
| Open menu | Existing early return retained — pass |
| No active mouse source | No script mouse delta — pass |
| Switch back to mouse | Mouse delta immediately available again — pass |

World-clearance and package-menu reverse patch checks remain successful.
Separate `patches/sk8v-controller-camera.patch` contains only main.cpp changes
against baseline HEAD. Root owns host build, integration, commit, publication
and any issue comment. This worker made none of those mutations.

Pending gameplay: controller ollie/flip/grab in Skate/Near/Middle/Far/Head views;
alternate mouse/controller while both connected; aim and weapon-wheel right
stick; V view cycling; R cinematic; normal GTA look after leaving skate mode.
Confirm no lost trick input and compare camera response, not just absence of a
crash. A stub pass is not a gameplay pass; issue stays open until confirmed.
