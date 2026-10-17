# GTAxSkate: Skate 3 skateboarding in GTA V (Legacy), keyboard, board shop and more for SK8V

[![CI](https://github.com/Manatros/GTAxSkate/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Manatros/GTAxSkate/actions/workflows/ci.yml)

**Multiplayer prototype - currently paused (known missing-board model):** [FiveM test page and downloads](https://github.com/Manatros/GTAxSkate/releases/tag/fivem-friends-test-20261014).
Joining players need GTA V Legacy and [FiveM](https://fivem.net/); the server sends the skate resources automatically.
See the [short joining guide](FIVEM-JOIN.md). The ZIP is for server hosts. The current prototype fails to load its board model; full animations, shops and F10 are not included. Singleplayer validation currently takes priority.

**Current release: [v0.6.0](https://github.com/Manatros/GTAxSkate/releases/tag/v0.6.0)** — accepted in a Story Mode gameplay test on 17 October 2026, using SK8V 1.1.10. Known issue: board lighting indoors and in tunnels needs further work. Historical privacy-affected downloads remain withdrawn; see [privacy guidance](PRIVACY.md).

**GTAxSkate** adds to **[SK8V](https://github.com/becaring/SK8V)** by becaring, the mod that runs
the real **Skate 3** engine in **GTA V story mode**. SK8V does the skating (Skate's own physics,
tricks, animations, audio, collision, Hall of Meat). GTAxSkate fills the gaps around it:
**keyboard and mouse controls**, a **skateboard shop** with every Skate 3 deck, truck, wheel,
griptape and paint, **GTA's own camera** and a **cinematic camera**, trainer tuning and stamina,
the GPS route while skating, skateable cars and a limp after hard bails. All of it in one install, each part optional.

For **GTA V Legacy** (1.0.3889.0, story mode) on PC. No game files are in this repository: the mod
is made from your own copies of the games.

> **Status:** beta. Tested by its author on one PC; please report problems as an
> [issue](https://github.com/Manatros/GTAxSkate/issues) with minimal redacted log excerpts from `%LOCALAPPDATA%\SkateV` (see [PRIVACY.md](PRIVACY.md)).
> Every addition is also offered to SK8V itself (one branch per feature). `main` holds the released
> version; test builds are published separately as pre-releases. See the [changelog](CHANGELOG.md).

## Project status and repository roles

The current main version uses SK8V **1.1.10**. The maintainer completed a roughly 32-minute gameplay test and approved the exact installed build on 17 October 2026. The published host/runtime pair is byte-identical to that tested pair. This is acceptance on one PC, not a claim that every possible configuration or checklist case was exercised.

Complete SK8V, RP, User, Custom and Developer settings profiles cover 120 menu settings. Keyboard presets remain separate, preserving personal bindings. LS Skate is explicitly unavailable until its own verified adapter exists. SK8V retains animation and transition ownership.

**Known issue:** board lighting indoors and in tunnels can look wrong. It is deferred and does not block this release. Clearer menu organization, native GTA styling and easier navigation are planned next; they are not changes in this accepted build.

The optional Python/RAGE board composition helper prepares exact shop selections as one skeletal drawable, including split truck paint and griptape. It requires separate setup and is not enabled by the stable installer. See [setup and rollback](BOARD-COMPOSITION.md).

| Channel / repository | Purpose and current evidence |
|---|---|
| [GTAxSkate](https://github.com/Manatros/GTAxSkate) | Player installer, setup scripts, downloads, issue tracking and package documentation. |
| [SK8V development fork](https://github.com/Manatros/SK8V/tree/skatemods) | Modified SK8V source, contribution branches and independently adoptable package patches. Upstream attribution remains intact. |
| `main` / v0.6.0 | Accepted Story build on SK8V 1.1.10; exact tested host/runtime, known indoor/tunnel board-lighting issue. |
| Test builds / future work | Pre-releases with their own tests before they reach `main`; experimental multiplayer support remains separate. |

[Report a bug](https://github.com/Manatros/GTAxSkate/issues/new/choose) ·
[Known issues and support](SUPPORT.md) · [Contributing](CONTRIBUTING.md) ·
[Security reporting](SECURITY.md) · [Changelog](CHANGELOG.md)

## Download

Download [v0.6.0](https://github.com/Manatros/GTAxSkate/releases/tag/v0.6.0): **GTAxSkate-Setup.zip** for the full installer, or **GTAxSkate-dropin-for-SK8V-1.1.10.zip** for an existing matching SK8V installation. Always replace both host and runtime together. Release assets include notices and checksums; the full tool bundle includes corresponding source for GPL tools. The Setup ZIP is the complete package; installing from this repository's source downloads the same accepted programs and checks their SHA-256 digest.

The separate [FiveM test release](https://github.com/Manatros/GTAxSkate/releases/tag/fivem-friends-test-20261014) remains a paused historical prototype with a missing-board model.

## Features (on top of SK8V)

| | |
|---|---|
| Keyboard and mouse | play Skate without a controller: WASD, Shift push, Space ollie, arrow keys for the trick stick, every key rebindable in the menu (F10 > *Keyboard keys*) or `SkateVKeys.txt`, your keys on screen while skating |
| Skateboard shop | **G** in Binco, the Discount Store and Suburban: complete boards, and every deck (by maker), truck, wheel, griptape and paint on its own, paid with the character's cash |
| Your board | the board you buy is the one you ride, with Skate 3's own models: trucks turn and wheels spin; Michael, Franklin and Trevor each keep their own boards |
| Wear | the board wears with riding, grinds, slides and bails and shows Skate 3's worn finish from half worn; a new one starts fresh |
| Customization | griptapes (colours, checkers, prints, or none: the deck's own grip), painted decks, wheels and truck parts (matte, gloss, metallic; painted wheels ride as hard as their paint), at street prices (deck 55-80 $, trucks 45-70 $, wheels 30-45 $, griptape 10-15 $) |
| Boards on show | each store hangs four boards on a rack behind the till (or stands five along a wall), new ones every game day; look at them and press **E** |
| Your own decks | images in `custom-assets/decks/` become decks in the shop ([how](custom-assets/README.md)) |
| Cars | cars SK8V only knows as boxes are measured once, so their roof, hood and trunk can be ridden and hippie jumped (adjustable height; 1.6 clears a car); how hard the skater pushes cars and how big the dents get are configurable (development menu: *World and roleplay* / *Advanced*) |
| People | optionally only knocked over, not killed |
| Injuries | after a hard bail your character limps on foot for 30 s to 2 min with GTA's injured walk |
| Board in hand | optional (`CarryBoard=1`, off by default): the board picked in the weapon wheel stays in your hand in GTA, swung with GTA's melee; E or F gets on |
| Cameras | **V**: near, middle, far (a calm third person camera that follows the skater, or the board: the development camera prioritizes obstacle clearance over follow distance; the mouse turns it. Geometry clearance still needs in-game regression checks), the skater's eyes (first person), Skate 3's camera. The view is kept between walking and skating; the standard view is set in the menu. **R**: a cinematic camera that cuts like GTA's in a car (wide bars, film look, shallow focus), then a skate filmer (4:3, handheld) |
| Tuning | pop, grind pop, push, braking, steering, wobble, grip, turn power, manual drag, hold fakie in SK8V's menu |
| Stamina | GTA's stamina skill sets push and pop; skating at speed trains it |
| Map | GTA's GPS route stays on while skating; GTA's skate parks are on the map |
| Radio | skating tunes the radio to a station you choose (`SkateRadio=`); `,` / `.` change station. Skate 3's soundtrack (the in-game iPod's 46 songs) on Self Radio: in GTA set Settings > Audio > Self Radio Mode to Radio and run Perform Full Scan once |

Prefer it more like GTA or more like Skate 3? `CameraView=Near` (the installer's default) is the
third person camera, `CameraView=Skate` **Original SK8V / Skate 3**, using SK8V's original camera output (High/Low); **V** switches while skating, and F10 > *Camera*
sets the standard view, what it follows and the cinematic camera's look.

## Controls (keyboard and mouse)

| Key | Does |
|---|---|
| Weapon wheel: skateboard | skate mode (walking with the board) |
| **F** | board out and skate (on foot, no vehicle near) |
| **E** / hold **E** | get on or off the board / put it away |
| **F** (skating) | back to GTA in one press |
| **W A S D** | steer, lean, walk |
| **Shift** | push |
| **Space** | ollie: hold to crouch, let go |
| **Arrow keys** | trick stick (kickflip: down, then up-left) |
| **Q** | hippie jump |
| **T** / **Ctrl** | Skate's X / B |
| **V** | camera views |
| **R** | cinematic camera, skate filmer |
| **,** / **.** | radio: previous / next station |
| **N** / hold **J** | set a session marker / back to it |
| **M** | GTA's interaction menu on foot; opening it while skating remains unresolved (B104) |
| **G**, or **E** looking at the boards on show | board shop (in a clothing store); **Tab** shows the part alone, **W A S D** or the mouse turn it, the wheel zooms |
| **F10** (or **/** on a US keyboard) | One GTAxSkate settings menu with categories and *Advanced* for uncommon tuning; page *Keyboard keys* selects GTA, Twin Stick, Mouse Flick or Arcade; saves/restores personal bindings and rebinds Skate actions |
| **Tab** / mouse buttons | gun out or away (hold Tab: GTA's weapon wheel); with a gun out, right mouse aims and left shoots |
| **B** (hold, behind a moving car) | skitch |

Choose a keyboard preset under **F10 > Keyboard keys**. The table above describes
**GTA**, the first-run default. Existing key files are preserved until you explicitly
choose a preset. Presets change input only, independently of the play profiles.

| Preset | Translation | Push | Tricks | Grabs | Manual / nose manual |
|---|---|---|---|---|---|
| **GTA** | familiar PC movement and a charged jump | Shift | hold/release Space; arrow stick (Space + Left/Right on release for flips) | Z / C | 1 / 2 |
| **Twin Stick** | two hands replace the two controller sticks | Space | WASD body, IJKL board; K then I ollie, K then I+J/I+L flips | U / O | H / P |
| **Mouse Flick** | mouse movement replaces the right stick | Shift | hold CapsLock, move down then flick up; diagonal flicks for flips | Z / C | 1 / 2 |
| **Arcade** | one key plays a complete stick gesture | Shift | Space ollie, K nollie, U/O flips, H/L shoves, P 360 flip | Z / C | 1 / 2 |

Twin Stick uses **Shift** for Skate's X and hold **5** to return to a marker.
The other presets use **T** and hold **J**. All use **E/F**, **Q**, **B**, **V/R**,
**Tab**, **M**, **G** and **F10** as above. Mouse movement normally looks around;
Mouse Flick borrows it only while CapsLock is physically held. CapsLock also toggles
Windows' capitalization state; you can rebind the clutch to Mouse4/Mouse5 if available.
Aiming uses the mouse normally, and weapon wheels/menus release trick input.
Original SK8V camera behavior is retained; mouse look applies to the existing
mouse-steerable camera options. Mouse sensitivity still needs in-game validation.

Use **Save my personal bindings** before experimenting, then **Restore my personal
bindings** to return. The first preset change also keeps `SkateVKeys.before-presets.txt`;
**Restore bindings from before presets** loads it. The personal slot is
`SkateVKeys.custom.txt`. Individual rebinding activates its required trick mode,
so customized hybrids work; reserved GTA/system keys are rejected. Install the
matching host/runtime pair; mixed versions are unsupported for these extensions.

Controller input follows SK8V; keyboard/mouse support is provided by this fork. Controller camera/trick-stick separation is tracked in [issue #1](https://github.com/Manatros/GTAxSkate/issues/1); the v0.6.0 acceptance test did not separately cover a controller, so the issue stays open until a controller test passes.

## Known issues

- **M** while skating does not open GTA's interaction menu yet (B104; it works on foot).
- Printed griptapes still show a seam at the deck's edge; a thin seam between the deck's middle and its nose and tail.
- Riding sideways along an edge the board does not always stick to it; a board hit while bailing can drag the body.
- The board shop registers at most 240 new part models a session (GTA's limit); restart GTA to see more.

Skate's own postures and riding styles are on F10 > *Skating*. Problems: open an
[issue](https://github.com/Manatros/GTAxSkate/issues) with minimal redacted log excerpts from `%LOCALAPPDATA%\SkateV` (see [PRIVACY.md](PRIVACY.md)).

## What you need

- **GTA V Legacy** 1.0.3889.0 (story mode, BattlEye off)
- **Skate 3** (Xbox 360): your disc image (ISO) or its extracted folder
- **[Script Hook V](http://www.dev-c.com/gtav/scripthookv/)**: copy `ScriptHookV.dll` and `dinput8.dll`
  from its `bin` folder into the GTA V folder before you run the installer

Everything else the installer gets by itself: SK8V and its tools (Python, the rage tool),
and GTAxSkate's ready builds. No OpenIV, CodeWalker, Rust or Visual Studio needed.

## Install

### Choose your target

| Target | Installation and status |
|---|---|
| **Story Mode (Legacy)** | Use `Install GTAxSkate.cmd` as described below; published Story package; see status table and known issues |
| **FiveM rules only** | `Install GTAxSkate FiveM.cmd` installs the standalone rules resource; it does not install skating. |
| **FiveM friends prototype** | Separate pinned SK8M Lua fallback with network boards and four server-controlled settings; [setup/test guide](FIVEM-FRIENDS-TEST.md). Real two-client acceptance is pending. |
| **Existing co-op mod** | Separate compatibility target; no confirmed co-op release yet |

For FiveM client download, txAdmin/server setup, resource installation and
administrator settings, follow the [FiveM and co-op guide](fivem/README.md).
It documents unrestricted, selected, bounded and fully server-locked settings.
Do not install the Story Mode ASIs as a FiveM resource.

### Story Mode installer

1. Download [GTAxSkate-Setup.zip](https://github.com/Manatros/GTAxSkate/releases/latest/download/GTAxSkate-Setup.zip) (the latest release)
   and unzip it anywhere (it carries the installer and every GTAxSkate program).
2. Close GTA V.
3. Double-click **`Install GTAxSkate.cmd`**.

(From this repository's source instead: the same `Install GTAxSkate.cmd`; it downloads the accepted release's programs and checks their SHA-256 digest.)

It finds GTA V by itself, asks for your Skate 3 disc or folder, and shows a list of the parts with
tick boxes: board shop, Skate 3 music, keyboard and mouse, GTA-style camera, skateable cars, GPS and
skate parks, stamina, people surviving and a limp after hard bails (the board in hand as a melee weapon is optional).
Untick what you don't want and pick your stance (Regular or Goofy, as in SK8V's setup); it is remembered (`-Choose` asks again, `-Parts all` skips the question).
Before it starts it checks GTA's version, Script Hook V, free space (about 8 GB) and write access, and starts again as administrator if the GTA folder needs it.
Then it:

| Step | |
|---|---|
| 1 tools | gets the SK8V release (Python, the rage tool) and vgmstream |
| 2 sk8v | runs SK8V's own setup: Skate 3's data from your disc, installed into GTA V (10–30 minutes; an SK8V already set up is reused) |
| 3 bin | GTAxSkate's builds (SK8V with our additions, the board tool): from the Setup ZIP, or downloaded from the accepted release and checked against its pinned SHA-256 |
| 4 boards | the board library from your Skate 3 and every part as a GTA model |
| 5 music | Skate 3's soundtrack as MP3s in User Music\Skate 3 for Self Radio (about a minute; skipped with a note if it fails) |
| 6 install | our SK8V build over SK8V's (recorded in SK8V's install record, so SK8V's own uninstall and updates handle it), our settings on (yours are kept); then the setup's leftovers go |

Every step is remembered: if anything stops it, run it again and it goes on where it stopped. If it stops, the reason is in red at the bottom; `build\install\install.log` keeps local diagnostics. Share only relevant redacted excerpts.
Music is optional: a missing disc on a resumed install skips the soundtrack and
finishes the mod installation. To retry the music, run again with your Skate 3 disc or folder.
`scripts\install.ps1 -Redo <step>` runs a step again (for example `-Redo boards` after adding your own decks).

Afterwards, in GTA V: **F10** opens the settings menu (on a controller: Back + LB).

**Uninstall:** as SK8V's [uninstall](https://github.com/becaring/SK8V#uninstall) (GTA V closed): remove
`SkateVLegacy.asi`, `SkateVRuntime.dll`, `SkateVLegacy.ini`, `SkateVLegacy.installed.json`, the `SK8V\`
folder (`GTAxSkate\` for installs before v0.5.4) and `update\x64\dlcpacks\skatev\`; the soundtrack is in
`Documents\Rockstar Games\GTA V\User Music\Skate 3`.

## Layout

| Path | What |
|---|---|
| `Install GTAxSkate.cmd` | the installer (runs `scripts/install.ps1`) |
| `gta/skate-ped` | the board library and every part (decks, trucks, wheels, griptapes, paints) from your Skate 3 data |
| `gta/skate-music` | Skate 3's soundtrack as MP3s for Self Radio |
| `gta/crates` | board catalog, input and rule crates shared with the SK8V additions |
| `gta/skate-gta` | the earlier stand-alone plugin (kept for reference; not installed) |
| `scripts` | the installer and the steps it runs |
| `custom-assets` | your own decks |

The SK8V additions themselves live in the fork [Manatros/SK8V](https://github.com/Manatros/SK8V)
(branches `contrib/keyboard-mouse`, `contrib/board-shop`, `contrib/gta-camera`,
`contrib/trainer-tuning`, `contrib/map-helpers`, `contrib/skate-radio`, `contrib/gentle-peds`,
`contrib/vehicle-shapes`, `contrib/injuries`), each to be offered to SK8V as a pull request. Branch
`skatemods` is all of them on the newest SK8V: the build in the release.

## How it works and why

GTAxSkate began as its own Skate 3 mod with its own Skate core. SK8V does the
core better (Skate's own audio engine, the pose hook, live collision, Hall of
Meat), so GTAxSkate now builds on SK8V and only adds what SK8V does not have,
each part in SK8V's own code and style, in its own branch of the fork
[Manatros/SK8V](https://github.com/Manatros/SK8V), and offered to SK8V as a
pull request. Options that change how SK8V plays are off in SK8V itself; this
installer turns on what GTAxSkate prefers (GTA's camera, people surviving).
The fork's [GTAXSKATE.md](https://github.com/Manatros/SK8V/blob/skatemods/GTAXSKATE.md)
lists every branch, how it works and the reasons.

## Thanks

To becaring for [SK8V](https://github.com/becaring/SK8V) and for releasing it under Apache-2.0, and
to the Skate 3 rewrite it builds on.

## Legal

GTAxSkate is a free, non-commercial fan mod for single-player use. It is **not affiliated with,
endorsed or sponsored by** Electronic Arts, EA Black Box, Full Circle, Rockstar Games or
Take-Two Interactive. *Skate*, *Skate 3* and *skate.* are trademarks of Electronic Arts Inc.;
*Grand Theft Auto* and *GTA V* are trademarks of Take-Two Interactive Software, Inc. Brand names
of real skate companies appear only as the names Skate 3 itself gives its parts.

**No game files are distributed.** This repository holds only source code and scripts. Everything
is made from your own, legally obtained copies of Skate 3 and GTA V on your PC and stays there. The
mod does not change GTA V's own archives and works in **story mode only**: do not use it online.

### Play presets (F10 > Presets)

- **SK8V:** restores every exposed setting to the current base defaults (Easy difficulty, original High Skate camera, standard multipliers), with documented package defaults for extension features.
- **LS Skate:** unavailable until a verified adapter exists. Displays a notice and changes nothing; it cannot supply LS Skate animations in this package.
- **RP:** resets all exposed settings, then uses Hardcore difficulty, lower pop/jump/push values, stamina/injuries, no glitch assist, artificial pedestrian launch, vehicle dents/push or shooting while riding. All Skate overlays are hidden immediately, including trick/x-ray, records, debug/input displays, hints and radio banners. F10 remains usable. These are conservative gameplay values, not a calibrated physical simulation.
- **User:** restores your last manually configured settings. Selecting another profile does not overwrite this snapshot; later manual edits update their corresponding saved User values.
- **Custom:** explicitly save your current settings, then restore that snapshot whenever wanted. Saving again replaces the slot.
- **Developer:** resets to a complete test baseline and enables HUD/input/debug/log diagnostics. Advanced exposes Motorized difficulty, BackwardsMan, trainer multipliers, stamina and other test controls separately. Expensive CPU/stack probes and alternative animation paths stay opt-in; no external cheat trainer is included.

Profiles cover every keyed F10 setting, including audio and advanced tuning. They apply only on Enter. Changes marked for restart require a full GTA restart; the master HUD toggle applies live. Settings are validated and saved together before live callbacks. Missing or invalid snapshots leave settings unchanged. User and Custom snapshots persist in `SkateVLegacy.ini`.

Personal keyboard bindings remain independent (F10 > Keyboard keys has its own save/restore). Boards, purchases, wear, records, saves, installation paths and game assets are never replaced by a play profile. Existing settings are untouched until you explicitly apply one. This is a Story Mode profile system, not FiveM/Coop or LS Skate runtime support.

### Save-reload test fix (2026-10-14)

The host guards duplicate graphics callback registration when loading a Story save. Cinematic HUD updates run once per frame. See [independent SK8V adoption notes](SK8V-SAVE-RELOAD-INTEGRATION.md) and [test instructions](test-build/TESTPLAN.txt). The original duplicate-registration fatal survived two recorded reloads after its fix. The cumulative Story build was accepted on 17 October 2026; the historical fix-specific notes retain their original evidence limits. Standalone FiveM rules and the separate experimental friends adapter are different packages.

### Reload, camera and Developer follow-up

The host reuses its process runtime after a save load and resends the
HUD viewport. This keeps the persistent sound feeder attached to the same runtime.
Scripted follow/cinematic cameras now respect nearby obstacle hits immediately;
normal GTA camera behavior is unchanged. F10 > Developer shows the SK8V source
basis, compiled host build, optional test receipt metadata and LS integration status.
Receipt metadata does not verify installed binaries; LS Skate remains unintegrated.
See [runtime lifecycle adoption](SK8V-RELOAD-LIFECYCLE.md) and
[camera/version adoption](SK8V-CAMERA-VERSIONS.md). The cumulative Story build was accepted on 17 October 2026; individual checklist coverage was not exhaustively recorded.

### Experimental FiveM friends slice

A separate pinned SK8M Lua-fallback package now connects four server-controlled
multipliers (push speed/power, braking, steering). It uses base-game props and
client-owned skating; Skate 3/WASM physics and a real two-player acceptance test
are still pending. [Preparation, server setup and test guide](FIVEM-FRIENDS-TEST.md).
The existing standalone rules installer remains separate. No Story ASI belongs
in FiveM, and no account/key/firewall/server startup is automated here.

Developer test settings: Story F10 > Presets > Developer enables input/debug HUD and verbose logs. Developer > Disable diagnostics turns those extras off. The separate FiveM ZIP includes an admin Developer policy and skatedev; see its test guide.

Performance: verbose frame timings no longer run the expensive thread CPU scan
automatically. The separate **Thread CPU scan** diagnostic is off by default and
can itself cause stutter. Switching presets can still pause for about 2 seconds.

### Privacy and safe support

Read [PRIVACY.md](PRIVACY.md) before sharing diagnostics. Post only small, redacted log excerpts: remove personal paths, names, addresses, tokens and server keys. Public GitHub issues and release downloads are public. See [SECURITY.md](SECURITY.md) for sensitive reports.

## License

Apache License 2.0 (see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE)).
