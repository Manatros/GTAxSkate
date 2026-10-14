# Reload lifecycle design and regression — 2026-10-14

Scope: host-side runtime reuse and initial HUD viewport publication after
ScriptHookV restarts ScriptMain. This follows the demonstrated callback fix
and the captured 13:27 log audit. No runtime binary or ABI
change is needed. Actual production edits are owned by the parent task;
this delegated task owns the regression harness and design record.

## Ownership and evidence

| Lifetime | Existing owners | Required behavior |
|---|---|---|
| GTA process / loaded ASI | Global RuntimeBridge, loaded DLL, audio feeder, HUD renderer and Present callbacks | Preserve their shared runtime identity across save reload; no duplicate callback or engine allocation. |
| ScriptMain invocation | Local Session, per-run lifecycle timings and menu bindings | Start clean and track the newly loaded player/world through the existing preparation path. |
| Runtime worker | Loaded Skate session, job queue, lifecycle and HUD/audio publication | Suspend the previous ride before the new host session activates; retain expensive loaded data. |

The 13:19 test showed three runtime creations across two reloads but only one
audio configuration. `gtaaudio::Start` returns on its `g_started` flag before
assigning its runtime pointer. Creating another runtime therefore separates
the script from the still-running feeder's handle and loses ownership of the
earlier allocation. Rebinding or destroying while the feeder/render consumers
can access the handle would need broader synchronization.

## Minimal selected design

In `RuntimeBridge::Load`, if `runtime_` already exists:

1. Call `Deactivate()` and require success; this means the worker accepted a
   Suspend job. Report failure through Load's error/result and preserve the
   handle if enqueue fails. This is not proof the entire host becomes inert:
   caller behavior on Load failure is a separate responsibility.
2. Clear the successful call's error and return the existing handle, avoiding
   DLL/export lookup and `sv_create`.
3. Let the existing ScriptMain initialization reapply supported current
   options, reset physics locator reporting and track the new player.

`sv_deactivate` sends `Job::Suspend` through the runtime's job channel. It does
**not** wait for completion. Worker handling suspends input, stops a playing
line, marks active false, clears accumulated stepping time, deactivates audio,
suspends HUD and sets lifecycle phase Inactive/status Ready. Subsequent jobs
from the script pass through the same queue. Do not describe the API return
as a synchronous lifecycle acknowledgement or assume shared snapshots change
before Load returns.

The loaded session and world data remain available. Session preparation can
recenter collision around the new tracked/spawned location using the existing
worker flow. No extra per-frame/network work is introduced. Full activation
and world/entity lifetime behavior still need actual reload tests.

**Configuration boundary:** DataRoot, WorldCache, creation-only presentation
flags and triangle budget are captured by the first `sv_create`; changing
these requires a full GTA restart. Reuse does not claim to live-reload those
values. Existing runtime setter options continue to be applied by ScriptMain.
The normal explicit unload path already stops the audio feeder before Release;
this change does not introduce a new destroy/recreate path during save loading.

## HUD initialization

On successful valid HUD binding in `hudoverlay::Start`, clear `g.sent` even
when the runtime pointer is unchanged. This makes the next Tick send the
current valid viewport although resolution/safe zone did not change. Preserve
renderer registration, textures and their generation caches; resetting all
GPU resources is unnecessary for same-instance reuse.

Runtime HUD suspend increments its serial and hides its frame; activation
restarts its movie. Reuse preserves monotonic serial/texture generation
relationships, avoiding the newly created runtime's reset counters against
retained host publication caches. The harness checks invalidation, not actual
D3D rendering or the complete movie lifecycle.

## Reproducible regression harness

Run `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-runtime-reload.ps1`.
Optional `-Fork` selects a separate source fixture. Builds and copied source
evidence are isolated under unique `build/qa-runtime-reload-*` directories.
It uses installed Visual Studio 2022/MSVC x64 and Windows SDK; no real game or
runtime DLL is loaded.

* Runtime case compiles the entire actual `runtime_bridge.cpp` with its real
  header/ABI. Mock DLL lookup supplies typed create/deactivate/destroy/API
  functions; unused exports are resolved but never invoked. It tests 100
  reloads retaining an external consumer's identity, exactly one creation,
  no destruction during reload, no module/export churn, failed suspend error
  propagation with handle retained, recovery, repeated explicit Release,
  creation after Release, and initial missing-export/ABI/create/DLL failures.
* HUD case extracts the actual balanced Start function and compiles it with
  real ABI types plus mocked platform/renderer setup. It tests 100 same-handle
  restarts invalidating the sent viewport, fresh logger, one renderer/callback
  registration and unchanged Hud=0/missing-export inert behavior.

| Source | Runtime case | HUD case | Evidence |
|---|---|---|---|
| Original fork HEAD copied into fixture | Expected FAIL: consumer runtime identity changes | Expected FAIL: sent viewport remains cached | `build/qa-runtime-reload-f9a9cf2ee3ef498a8e88a579b902cb73/` |
| Current guarded production source | PASS | PASS | `build/qa-runtime-reload-1ecd0abbb60649f2b8951e2686e078cf/` |

These tests establish the host ownership/cache contract with deterministic
stubs. They do not execute the actual Rust worker, audio thread, engine
physics or D3D renderer. Full host build and game acceptance remain separate.

## In-game acceptance for the next build

1. Start fresh, ride with normal trick HUD and audible rolling/push/landing
   sounds; reload the same save twice and a different location once.
2. Confirm HUD returns without changing presets, sound follows the new ride,
   and boards/camera/input/pose/collision recover. Include active and inactive
   skating at the load trigger, and cinematic → normal camera transitions.
3. New logs should show script restarts, runtime suspension/reactivation and
   initial viewport messages without a fresh runtime ABI/create/preload
   segment for each save. Existing queue snapshots may briefly be stale;
   a successful enqueue alone does not count as completed recovery.
4. Measure process private bytes before and after several completed reloads
   and subsequent riding. Previous diagnostics stopped before reloads, so no
   before/after memory reduction claim is currently supported.

Stale process-lived ped/entity state and audio editor fallback behavior remain
watchpoints. Keep any newly observed failure separate from the original
duplicate-callback fatal instead of claiming this small fix covers all reloads.
