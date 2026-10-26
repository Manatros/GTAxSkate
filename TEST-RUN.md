# v0.7.0 gameplay checklist — 26 October 2026

Manatros authorized release on 26 October 2026. The final candidate below is
installed but gameplay untested. Earlier tests confirmed the object dropper
working; that does not establish a pass for the complete final checklist.
Automated results are recorded separately in the release report.

**Candidate:** `test-20261026l-rc-b27eec6`, SK8V 1.1.10.

| Identity | Value |
|---|---|
| Host SHA-256 | `07ebf870abdc3388cb401fed437631c55290fe7fc377cde0051b3ac5c23d9097` |
| Runtime SHA-256 | `06a1e60ebd9619cd188a2d69aa166f2a4130098c98f63ddad36e6bde2c1ea242` |
| World rules | `4217eedcd28b7deb` |
| W01–W12 final gameplay results | **NOT RUN** |

## Before testing (about 5 minutes)

1. Use PowerShell 7 (`pwsh`) for the world step. Close GTA V, OpenIV and CodeWalker. Install the candidate with the world component.
   Do not start GTA while step 7 (world) is still replacing files: a test during the install
   is invalid.
2. Start GTA, open F10 > World > World collision and activate the
   **World collision: best/changed** status row to restore recommended settings. Every test below runs
   on the best settings unless a step says otherwise.
3. Ride once. `%LOCALAPPDATA%\SkateV\SkateVRuntime.log` must show `baked world: N tiles` with
   N > 0 and **no** line `installed tiles are from another bake`. If that warning appears,
   stop: the test would run on GTA's raw collision.
4. Record with the microphone if possible (say where you are). At every problem use F10 >
   World > World collision > **Mark this spot**; the log gets `MARK n at x y z`.
   Enable F10 > Advanced > Developer options, then collision view (F10 > Developer) where noted.
   MARK-INFO in the runtime log includes nearby surface origins and named live areas.

## Required tests (about 30 minutes)

| ID | Where / action | Expected |
|---|---|---|
| W01 | **Vespucci park stairs** (around -1374, -1396): ride down and up every flight, including the short ones and the pool stairs | Steps, not a ramp; no hole at the foot of the stairs; no slope over the last steps; no grind line on each step |
| W02 | **Vespucci bowl transitions**: ride from the big pool into the bean bowl and back several times (around -1379, -1385); ride both bean bowls | Smooth transition, no invisible grind line or bump on the crease, the board follows the visible bowl. Copings still grindable |
| W03 | W02 again with F10 > World > World collision > *High-res skateparks* off, then activate **World collision: best/changed** | Off should feel coarser (GTA's own collision). Note which feels right. The reset brings back the best settings |
| W04 | **No holes in the five skateparks**: Vespucci Beach (-1374, -1396), Hawick (280, -206), Little Seoul (-950, -786), Strawberry (-266, -1571), La Mesa rails (718, -1231). Ride every floor, bowl and ramp edge | No falling through, no invisible edges or walls; mark anything odd |
| W05 | **Object dropper**: press Insert (or F10 > World > *Build a park*); drop a rail, a half pipe, a quarter pipe and a bench; ride, grind, then *remove all objects* | Placed objects are solid; rail grinds from end to end with no lip at either end; the rail's legs are not solid; the half pipe's base has no step; no duplicate rails; removal clears everything |
| W06 | **Train track grinds** (metro tunnel between Del Perro and Portola Drive; a freight track outside) | Two grind lines per steel rail (one per edge); the board sits on the steel, not inside it; the line does not change midway; no added rail boxes or invisible obstacles |
| W07 | **Metro stations** (Portola Drive, around -806, -103 and -804, -129): stairs, handrails, platform edges, turnstiles | Stairs ride as steps; handrails grindable and as straight as the rail; no invisible barrier; turnstiles push |
| W08 | **Menu reset and warning**: on F10 > World > World collision, turn one switch off (for example *Grind tubes*); then the **World collision: best/changed** status row; then *Reset all settings to GTAxSkate defaults* | A warning appears when the switch leaves its best value; each reset shows its notice and the switches return to their recommended values; keys, boards and purchases stay |
| W09 | **Performance**: ride 10 minutes through the city, the metro and a park; also change two settings while riding | Note FPS while skating (an overlay or the verbose log's frame timings), stutter with time stamps, and whether settings change while riding. No new crash |
| W10 | **Known issues, only confirm** (do not spend long): railings with a solid wall under them, metro handrail zigzag, palm trunks, scaffolding and power poles, benches and protruding prop boxes, the road step near Clinton Ave (around 41, 301) | Mark each spot and record whether it remains; release authorization does not prove these fixed |
| W11 | **Earlier features still fine**: enter with the weapon wheel, hold Y (E) to leave; V cycles cameras; board shop, wheel animation, glowing wheels; cars collide and parked cars grind; save and reload once | As in v0.6.0 |
| W12 | **Board shops:** visit Suburban and the marked Vespucci Beach Skate Shop; browse daily stock and parts, buy a board, leave and return, advance one game day | Displays have sensible placement, E opens the Skate Shop, purchases persist and stock rotates; final display changes currently untested |

## Record the result

| Field | Entry |
|---|---|
| Date and time, recording file | |
| Build ID, host and runtime hashes, tiles rules hash | from `manifest.json` and the log; do not guess |
| W01-W12 | PASS / FAIL / NOT RUN per ID, with MARK numbers |
| FPS while skating | city / metro / park |
| Release decision | the maintainer's explicit word only |

A passing automated test or bake check is not a game test. "Fixed" only after the
maintainer's test.
