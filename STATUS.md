# Project status (v0.9.0)

GTAxSkate is on hold: the maintainer has stopped active development after v0.9.0.
The code, tools and design notes stay public so anyone can pick it up.
This page is the honest list of what works and what does not.

## Works (tested in game by the maintainer)

| Area | State |
|---|---|
| Skating in GTA V Legacy story mode (build 3889) | Skate 3 physics, tricks, grinds, manuals, bails; 80-100 fps on the test PC |
| Getting on and off | E/F to step on, board in hand, carry, put away; GTA camera carries over |
| Controller and keyboard/mouse | Controller (incl. the right-stick camera fix), keyboard layouts (Mouse Flick, Twin Stick, Arcade, Standard) |
| Boards and parts | Board shop (Suburban, Vespucci Skate Shop), decks, griptapes, trucks, wheels, paints, GTA-brand equipment |
| Skitching | Hold on to GTA cars |
| Music | Skate 3's 46 songs on GTA's Self Radio |
| Replay editor | Last 30 s, Skate 3 style timeline, clips, cameras, lenses, filters, sound, saved replays, photos, full keyboard/mouse shortcuts |
| Collectibles | Hidden boards and parts, Skate 3 centre popups, score milestones (v2: "works for now") |
| Baked map collision | Whole-map collision and grind lines baked from your own GTA files; 91 marked spots confirmed fixed in v0.8.0, skatepark bowls ride well |
| Live collision (SK8V style) | GTA's own collision as SK8V uses it; no bake needed (F10 > World > Collision and grind rails); switching in game works |
| NPC traffic | Peds and cars move normally, also while hosting coop (with Realism Enhanced Traffic off, see below) |
| Friends coop on one PC (localhost) | Hosting and connecting to your own local server |

## Built but not confirmed in game

These are in v0.9.0 and pass the automated tests, but the maintainer did not get to test them fully.

| Area | State |
|---|---|
| Park rules R1-R6 and road-surface alignment | Fewer duplicate grind lines and fence boxes in parks, arch openings, drawn-ground alignment (needs the v0.9.0 bake) |
| Replay Wide lens and lens strength | Host only |
| Tutorial Island (Coach Frank, 17 lessons) | Optional component (`-WithTutorial`); lessons and travel work offline; island fidelity is incomplete |
| Friends coop between two PCs (RageCoop based) | Optional component (`-WithCoop`); streamer-safe invites; never tested between two real PCs. "Host online" did not work on the maintainer's connection (DS-Lite: no public IPv4; an IPv6 hosting design exists in the workspace) |
| F11 coop menu key lock | While the F11 panel is open, arrows/Enter/Backspace/Esc no longer open the phone or pause |

## Known problems (not fixed)

| Area | Problem |
|---|---|
| Metro stairs (marks M14-M16) | Stairs collide wrong; no version got them right |
| Concrete wall under a handrail (M27) | Not treated as a railing |
| Some park spots (1115-3/4/5/7) | A pipe stub, a short ridge line, a cut slab on a wall top |
| A few false train tracks | Short parallel strips (Richman, Kortz, port) get grind lines |
| Dropped halfpipe | 0.217 m lip at the drop |
| Shop | Seller position and shelves are rough |
| Map mods | Trees of map mods (e.g. Forests of San Andreas) are not solid until a mod-aware bake exists |
| GTA Online / FiveM | Not supported. Never use the story-mode ASI online |
| Realism Enhanced Traffic and Driver A.I | Incompatible: its `pedpersonality.ymt` freezes every NPC in place. Leave it off |
| Whole-map bake time | The v0.9.0 rules bake slower than v0.8 in dense city tiles; parks install first, the rest can take several hours |

## For contributors

- Code: the SK8V fork (github.com/Manatros/SK8V, branch `dev`; v0.9.0 source on `release/v0.9.0`).
- Build: `tools/build-rust.ps1` (runtime, baker), `tools/build-host.ps1` (ASI), `tools/build-host-tests.ps1` (host test suites).
- Rust tests: `cd rust; cargo test --offline --locked --release -p skatev-runtime -p skatev-world-cache -p svwc`.
- World collision: one build = one rules hash; tiles and the runtime must come from the same commit (`skatev-world-cache --rules`).
- The design notes for every rule and feature are in the fork's history and in this repository's release notes.
