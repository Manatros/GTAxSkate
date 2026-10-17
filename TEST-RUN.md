# Acceptance and future regression checklist

**17 October 2026:** the maintainer completed a recording of the installed `test-presets-20261016-a17823f` build, reports that everything seems to work, and approves it as the new main version (v0.6.0). Board lighting indoors and in tunnels is a known deferred issue. The checklist below remains useful for future regression tests; this acceptance does not assert every row was independently logged or exercised.

Update, 16 October 2026: the tester reports plausible wheel speed and confirms
the corrected rear-wheel direction. Stationary behavior was not separately
confirmed. This limited feedback does not replace the controller/reload/audio/
camera/shop tests below or approve main. Story development uses SK8V 1.1.10;
[BOARD-COMPOSITION.md](BOARD-COMPOSITION.md) explains the optional helper's setup
and separate rollback.

Plan dated 14 October 2026: validate the next dev candidate before promoting its
state to main or a confirmed release. A passing workflow does not replace a game
test. A statement that testing finished is insufficient: results, logs and any
video must identify the tested build.

## Before testing (about 3 minutes)

1. Close GTA/FiveM, OpenIV and CodeWalker. Install through the test package's
   installer and retain the backup/rollback path. Preserve INI, boards, bindings,
   saves and archives.
2. Record the build ID from `manifest.json` and F10 > Developer (host build/
   receipt). Source tags and receipts are distinct from checked binary hashes.
   The candidate must contain the GitHub #1 controller fix; v0.5.4 and earlier
   local packages do not contain it.
3. Connect the controller; record its model and Steam Input/DS4Windows setup.
   If recording, check the microphone. Use GTA Legacy 1.0.3889.0 Story with
   BattlEye off. Do not use GTA Online.
4. Select F10 > Presets > Developer. Input/debug displays and VerboseLog must
   appear; Developer now resets all exposed settings to its documented baseline. Save Custom first if you want to return to the current setup.

## Required Story tests (about 20–25 minutes)

| ID | Action | Expected evidence |
|---|---|---|
| S01 | Equip/put away the board; mount/dismount with E/F; open/close F10 twice on foot and riding | Exactly one menu; no hangs or duplicate boards. |
| S02 — GitHub #1 | Stand still and ride straight; circle/flick the controller right stick and perform several tricks | No free camera turn triggered by the trick stick. Automatic travel-direction following remains allowed. Record time and input display. |
| S03 — GitHub #1 | Repeat S02 in V views near/middle/far, skater first person and Skate view | Trick stick stays separate from looking in every view. |
| S04 — GitHub #1 | Aim/open weapon wheel with controller; switch to mouse/keyboard and back | Intentional aiming/look works; mouse look returns; idle/virtual controller does not block keyboard. |
| S05 | Switch R cinematic/filmer and V repeatedly; ride near a wall, ramp, curb and low ceiling | No camera trapping or falling under the map; cinematic hides Skate HUD, normal view restores it. Individual GTA text messages are not Skate HUD. |
| S06 | Load an existing save three times: inactive, skating and another location; do not overwrite it | No fatal callback; skating, boards, collision and inputs still work. |
| S07 | After EVERY load, ride, push, jump/land; inspect HUD without changing presets | Trick HUD returns automatically; Skate sounds follow current movement. A live audio-stream log alone is insufficient. |
| S08 | Save Custom; change a setting manually; select SK8V, RP, Developer, then User and Custom; restart GTA | Full profile values replace prior tuning. User restores manual values; Custom restores its saved snapshot. RP hides every Skate overlay, including after reload, bails and radio changes. LS Skate shows unavailable and changes nothing. Boards and personal bindings survive. |
| S09 | Disable Developer diagnostics and continue riding | Extra inputs/debug/VerboseLog disabled; normal HUD remains; no sustained flicker. |
| S10 | Open shop with G/E; check preview/purchase/riding; return through V/R; check normal GTA controls and Self Radio | Existing flows work. Do not claim exact display-board purchasing, which remains backlog. |
| S11 | Ride at least 10 more minutes; repeat two reloads if possible | No new crash, substantial persistent performance loss or lost state. Record FPS/frame times and log timestamps; do not infer a memory leak without measurement. |

## FiveM: separate approval block (about 15 minutes after setup)

Prerequisites: new isolated server, Cfx key kept local and deliberately configured
access settings. See [FIVEM-FRIENDS-TEST.md](FIVEM-FRIENDS-TEST.md). This is an
experimental SK8M Lua fallback with four multipliers, not Skate 3/WASM physics.
The published missing-model test remains paused until a replacement is verified.

| ID | Action | Expected result |
|---|---|---|
| F01 | Connect locally; F6, W/S/A/D/Space; F8 skatedev and skaterules | All resources required by that package run; board/ped usable; valid rules with four supported fields. Older published package has three resources, newer source preparation four. |
| F02 | Server Developer policy; try `skateset PushSpeed 1.1`, then `skateset PushSpeed 99` and `skateset Pop 2` | 1.1 confirmed; 99/unsupported Pop refused. This does not establish physics anti-cheat. |
| F03 | Restore locked policy; server skaterules_reload; retry adjustment | Rider stops; adjustment blocked; only administrator changes policy. |
| F04 | Stop/restart rules, adapter and skateboard resources while riding | No persistently frozen character/camera or stale boards; mounting blocked without valid rules. |
| F05 | Friend joins from a different Internet connection | Mutual riders/boards/tricks visible; test spawn, bail, distance/streaming, late joining and disconnect/reconnect. Local hairpin routing is not an external network test. |

An unconfigured server or unperformed two-player test means NOT RUN, never PASS.
Do not advertise FiveM as confirmed multiplayer without these results. Define the
actual release scope before main; unverified features remain dev/experimental.

## Record the approval result

| Field | Entry |
|---|---|
| Tester, date/time | |
| Build ID, host/runtime hashes | Use manifest.json; do not guess. |
| Controller / input software | |
| S01–S11 | PASS / FAIL / NOT RUN per ID |
| F01–F05 | PASS / FAIL / NOT RUN per ID |
| Failures | ID, timestamp, expected/actual result, reproducible steps |
| Video/logs | Host/runtime/ScriptHookV or FiveM client/server; remove keys, tokens and personal data before sharing. |
| main approval | Explicit decision after review; never automatic. |

Block main on a crash, reproducible controller-camera issue #1, HUD/audio failure
after reload, installation/rollback data loss or new blocking gameplay defect.
Before promotion: passing CI for the exact tested commit, matching test binary
hashes, assessed findings, documented limits and checked release assets. Never
present an old untested ZIP as a new build. GitHub #1 stays open until an actual
controller test confirms success; a C++ fixture is not player confirmation.
