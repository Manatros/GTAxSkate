# Camera clearance and Developer provenance — 2026-10-14

## Existing decisions and evidence

The tester's recording `2026-10-14 13-19-22`, 02:39–02:51 asks for stable cameras
without clipping into ground/walls/ceilings; 03:06 and 04:29–04:47 asks for
component versions in Developer. Read CLAUDE.md/BACKLOG A5, prior camera work
and git history, VIDEO-REVIEW-2026-10-14-1327.md and the fresh upstream check
before edits. Existing fixes from 11 October already provided mouse orbit,
slow height/follow smoothing, map-only obstruction checks and ground clearance.
They were reused rather than adding a new camera system. Upstream main remains
48c6128; no newly landed upstream solution exists in reviewed public metadata.

## Narrow causes and changes

| Observed source behavior | Change and reason |
|---|---|
| Orbit preferred minimum distance 1.6 m could exceed a near-wall hit distance. Easing could leave the eye behind a detected wall while converging. | A world obstruction overrides comfort distance; clamp inward immediately, retain slow outward easing. 0.3 m clearance, bounded by the segment length. |
| Smoothed orbit aim can lag below a rising ramp. | Final world trace from actual skater/board chest anchor to the chosen eye, after ground clearance. Existing direction, mouse controls, distances and aim remain. |
| Film camera cast originated at its look target, which can be ground-level. | Safety trace starts at actual skater chest; the film shot's visual look target and lens behavior remain unchanged. |
| Downward clearance ray accepted ceilings or an overhead roof's top. | Lift only for an upward normal (`z > 0.5`) within 0.3 m above the current eye; recheck film line of sight only after a lift. |

Changes affect existing scripted Orbit/Action camera paths. Native GTA camera,
default view, input mappings, presets, saved settings and existing filters are
unchanged. Orbit map-only mask remains 1 (passing cars/props do not yank it).
Film mask remains 1|16, preserving its previous prop obstruction behavior.

Budget: Orbit now three bounded synchronous rays per frame instead of two.
Film normally two, at most three when a ground lift requires another trace.
This is bounded but still requires in-game frame-cost comparison; no claim of
improved performance is made. Failed/incomplete native tests retain the prior
camera decision; no new asynchronous pool or streaming cache added.

Line traces are not a full swept camera volume. They cannot promise zero
near-plane clipping at all FOVs, inside invalid collision origins or missing GTA
collision. The scope is the concrete smoothing/minimum-distance/roof problems,
not a wholesale replacement of camera physics.

## Developer provenance, without invented version detection

Four information actions appear on a separate Developer page:

- `SK8V source basis: 1.1.8`: identifies the known source basis of this fork,
  explicitly not an automatic detector of arbitrary third-party installs.
- `GTAxSkate host build`: actual compiler `__DATE__` / `__TIME__` of the loaded
  host binary, shown when selected.
- `Test package receipt (metadata)`: installer-written ID from
  GTAxSkate-test-version.json, read once, at most 16 KiB, format/prefix and
  unescaped identifier characters checked. UI explicitly says metadata only,
  binaries not verified; missing/oversized/unreadable receipt reports unknown.
- `LS Skate: not integrated`: states that this package has no integration and
  its installed version is unavailable. It does not pretend to detect absence
  of an unrelated user's installation.

No compatibility decision or setting mutation depends on receipt data. Existing
Advanced diagnostics remain where players already find them. AppendDeveloperPage
is applied at ScriptMain's existing menu construction call, preserving the
separate menu overlay's original patch context.

## Ownership, patches and tests

Only `external/sk8v/host/src/main.cpp` modified in the fork by this task. Separate
patches are `patches/sk8v-camera-clearance.patch` and
`patches/sk8v-developer-versions.patch`. Root reviews/integrates/commits them;
no contribution-history rewrite or standalone commit by this worker.

`scripts/test-camera-clearance.ps1` extracts the actual ConstrainCameraEye
implementation and actual film floor predicate from main.cpp, compiles them
against small native stubs using existing MSVC tools, then exercises:

1. No hit leaves eye unchanged; correct flags, ped, coordinates and native options.
2. Wall at 0.8 m (closer than old 1.6 m minimum) keeps the camera on the
   skater side, with 0.3 m margin.
3. Coincident hit remains finite; a short segment never overshoots the anchor.
4. Ceiling underside and overhead roof top cannot trigger a ground lift;
   a genuinely near floor can, an already clear eye cannot.

All checks pass under Windows PowerShell 5.1 / MSVC. Fixture source/executable
stay under build/camera-clearance-tests. Both new patches and existing
sk8v-package-menu.patch pass reverse apply checks together. Shared host build
and other workers' runtime/HUD regressions are root-owned and reported there.
No live gameplay validation performed by this worker.

Next gameplay checks: same recorded cinematic spot, low/high ramps, close wall,
tunnel roof, near/middle/far, R cinematic/filmer, V and native GTA return;
Developer page before/after save reload, missing receipt fallback, compare log
camera phase cost and watch for overly close camera near a wall.


Source provenance follow-up: CMake obtains the nearest numeric1.x Git source
tag with a5-second bound and unknown fallback; the UI consumes this generated
macro rather than retaining a hard-coded1.1.8 on later upstream merges.
This is source ancestry metadata, never an automatic installed-release detector.
The combined host rebuild passed; observed source tag1.1.8. Actual game checks
still require tester confirmation. Compiler deprecation warnings were pre-existing.


Developer preset follow-up: F10 Presets now adds Developer input/debug/verbose
logging, keyboard hints/trick HUD and immediate guarded HUD reinitialization.
It does not change driving/physics settings. Developer page can disable the
extra input/debug overlays and verbose log; normal HUD/hints remain enabled.
FiveM has separate admin-only developer policy for four supportedmultipliers
and read-only skatedev diagnostics.29Node tests pass including Developer bounds.
Earlier uninstalled test-20261014-181943-01dcbe6 is superseded by the next package.
