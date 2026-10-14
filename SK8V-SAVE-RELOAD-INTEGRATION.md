# SK8V maintainer handoff: save reload and camera HUD

Date: 2026-10-14. Two independently adoptable host-only changes. Neither changes
the Skate runtime, ABI, player settings, physics or multiplayer behavior.

## 1. Idempotent board stencil callback registration — upstream applicable

**Trigger:** load a Story save without exiting GTA. ScriptHookV restarts
`ScriptMain` while the ASI and its registered Present callbacks remain loaded.

**Observed failure:** GTA Legacy 1.0.3889.0, package
`test-20261014-094417-f3996f0`, ScriptHookV log at 11:29:43:
`FATAL: IDXGISwapChain::Present() callback is already registered`.
Script id changes from 3 to 133; host initialization reaches HUD setup and
immediately calls `boardstencil::Start` again. The callback address has not been
symbolized against the installed binary, but this call order and the explicit
duplicate-registration fatal match the unguarded source path.

**Applicability:** the same unguarded Start is present in locally inspected
upstream SK8V 1.1.8 (`9fbef44`) and `upstream/main` (`48c6128`). This is not caused
by our keyboard/menu/camera contributions. The Start registration dates to
upstream 1.1.4 (`1b292884`); 1.1.6 (`1f04e4fb`) added a registration flag to avoid
unregistering a callback that never registered, but Start does not check it.

**Minimal implementation:** in `host/src/board_stencil.cpp`, immediately after
`g_log = log;` in `Start`, add `if (g_registered) return;`. Keep the existing
registration and assignment afterward and keep `Stop` unchanged. Updating the
logger before returning preserves current initialization behavior. Do not clear
the flag on script restart: it represents the process-lived SHV registration.

**Why this boundary:** `hudoverlay::Start` already has equivalent registration
protection. Avoid renderer recreation, force-unregistering unknown callbacks,
or changing pose/collision ownership to fix this specific failure. The guard
has no extra per-frame work and preserves Start → Stop → Start reuse.

**Regression:** `scripts/test-present-registration.ps1` extracts the actual
production Start/Stop bodies and compiles them as C++17 with MSVC x64, replacing
only external SHV/D3D dependencies with stubs. The SHV mock rejects duplicate
register and unknown unregister calls. It checks Stop before Start, repeated
Start with logger refresh, repeated Stop, re-registration after Stop, cleanup
state and 100 repeated cycles.

| Source | Result | Local evidence |
|---|---|---|
| Unmodified fork HEAD source in isolated fixture | Expected failure: duplicate registration, exit 1 | `build/qa-present-290d4c1d716d4a93bfb82d4e5cffcf91/` |
| Guarded production bodies | Pass: 102 registrations, 102 unregisters | `build/qa-present-f2105e3454cc44f1abf50d62df5e15f0/` |

The harness does not compile or run D3D itself. Full host Release compilation passed; an
actual save-load reproduction remains a separate acceptance check. Source/log
details and captured artifact hashes are retained in the private project audit.

## 2. One HUD publication per frame — package camera integration only

Our merged `host/src/main.cpp` calls `hudoverlay::Tick` twice in the same frame:
first with `skating && !filming`, then with `skating`. The latter immediately
overrides cinematic/filmer HUD suppression and duplicates HUD preparation.

Remove only the second unconditional `hudoverlay::Tick` call. Retain the
camera contribution's `filming` calculation, GTA HUD/radar suppression and
single `Tick(skating && !filming)`. The camera contribution worktree already
contains that intended single call; this is a merged package integration fix,
not a reason to alter vanilla upstream HUD policy.

Adopt independently from change 1. If upstream does not adopt our camera
feature, no camera-specific HUD change is needed. Acceptance: normal skating
HUD remains visible, cinematic/filmer views suppress it, leaving those views
restores it, and only one Tick is present in the frame loop. Do not claim this
visual acceptance passed until checked in game.

## Release and remaining verification

Keep `SkateVRuntime.dll` byte-identical for this small host release; record both
host and runtime hashes in the test manifest and preserve INI/board files plus
rollback. Do not submit an upstream PR as game-tested before the maintainer confirms.

1. Reload the same save at least twice while inactive, then while skating;
   verify ScriptMain restart without SHV fatal, then continued movement,
   carried/ridden boards, menus, HUD, camera and sound.
2. Check sustained private bytes across reloads. RuntimeBridge currently
   creates another runtime on Load, while the persistent audio Start returns
   before replacing its runtime handle. This is a separate lifecycle concern;
   do not free the old handle while an audio/HUD worker may still use it.
3. Confirm physics locator reconnects and no stale pose/entity state survives
   active reload. The inactive crash session had already restored pose hooks;
   it cannot prove active reload safety.
4. Repeat cinematic/filmer → normal camera transitions and inspect the new
   host and ScriptHookV logs. Existing runtime memory diagnostic lines alone
   are not crash reports.

This handoff claims a reproduced stub regression and a source-level fix for a
specific logged fatal, not completion of every save/reload or audio issue.
