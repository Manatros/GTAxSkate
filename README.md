# GTAxSkate: Skate 3 skateboarding in GTA V (Legacy), keyboard, board shop and more for SK8V

[![CI](https://github.com/Manatros/GTAxSkate/actions/workflows/ci.yml/badge.svg?branch=dev)](https://github.com/Manatros/GTAxSkate/actions/workflows/ci.yml)

**GTAxSkate** adds to **[SK8V](https://github.com/becaring/SK8V)** by becaring, the mod that runs
the real **Skate 3** engine in **GTA V story mode**. SK8V does the skating (Skate's own physics,
tricks, animations, audio, collision, Hall of Meat). GTAxSkate fills the gaps around it:
**keyboard and mouse controls**, a **skateboard shop** with every Skate 3 deck, truck, wheel,
griptape and paint, **GTA's own camera** and a **cinematic camera**, trainer tuning and stamina,
and the GPS route while skating.

For **GTA V Legacy** (1.0.3889.0, story mode) on PC. No game files are in this repository: the mod
is made from your own copies of the games.

> **Status:** work in progress. Every addition is also offered to SK8V itself as a pull request
> (one per feature), so it can become part of SK8V. `main` holds the newest version confirmed in a
> game test, `dev` the work in progress. See the [changelog](CHANGELOG.md).

## Features (on top of SK8V)

| | |
|---|---|
| Keyboard and mouse | play Skate without a controller: WASD, Shift push, Space ollie, arrow keys for the trick stick, every key rebindable in `SkateVKeys.txt`, your keys on screen while skating |
| Skateboard shop | **G** in Binco, the Discount Store and Suburban: complete boards, and every deck (by maker), truck, wheel, griptape and paint on its own, paid with the character's cash |
| Your board | the board you buy is the one you ride, with Skate 3's own models: trucks turn and wheels spin; Michael, Franklin and Trevor each keep their own boards |
| Wear | the board wears with riding, grinds, slides and bails until it shows Skate 3's worn finish; a new one starts fresh |
| Customization | griptapes (colours, checkers, prints), painted decks, wheels and truck parts (matte, gloss, metallic; painted wheels ride as hard as their paint) |
| Boards on show | each store shows five boards behind the till or on a wall, new ones every game day |
| Your own decks | images in `custom-assets/decks/` become decks in the shop ([how](custom-assets/README.md)) |
| Cars | cars SK8V only knows as boxes are measured once, so their roof, hood and trunk can be ridden and hippie jumped (adjustable height; 1.6 clears a car) |
| People | optionally only knocked over, not killed |
| Cameras | **V**: near, middle, far (they follow the board and never swing into the skater; the mouse turns them), first person, the skater's eyes, Skate 3's camera. The view is kept between walking and skating; the standard view is set in the menu. **R**: a cinematic camera that cuts like GTA's in a car (wide bars, film look, shallow focus), then a skate filmer (4:3, handheld) |
| Tuning | pop, grind pop, push, braking, steering, wobble, grip, turn power, manual drag, hold fakie in SK8V's menu |
| Stamina | GTA's stamina skill sets push and pop; skating at speed trains it |
| Map | GTA's GPS route stays on while skating; GTA's skate parks are on the map |
| Music | Skate 3's soundtrack on GTA's Self Radio, tuned in whenever you start skating; `,` / `.` change station |

Prefer it more like GTA or more like Skate 3? `CameraView=Near` (the installer's default) uses
GTA's camera, `CameraView=Skate` Skate 3's; **V** switches while skating.

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
| **M** | GTA's interaction menu (also while skating) |
| **G**, or **E** looking at the boards on show | board shop (in a clothing store); **Tab** shows the part alone, **W A S D** or the mouse turn it, the wheel zooms |
| **F10** (or **/** on a US keyboard) | SK8V's settings menu; page *Keyboard keys* rebinds every key |
| **Tab** / mouse buttons | gun out or away (hold Tab: GTA's weapon wheel); with a gun out, right mouse aims and left shoots |
| **B** (hold, behind a moving car) | skitch |

A controller works as in SK8V; the keyboard is used whenever none is connected.

## What you need

- **GTA V Legacy** 1.0.3889.0 (story mode, BattlEye off)
- **Skate 3** (Xbox 360): your disc image (ISO) or its extracted folder

Everything else the installer gets by itself: SK8V and its tools (Python, the rage tool, Script Hook V),
and GTAxSkate's ready builds. No OpenIV, CodeWalker, Rust or Visual Studio needed.

## Install

1. Download this repository (green **Code** button > **Download ZIP**) and unzip it, or `git clone` it.
2. Close GTA V.
3. Double-click **`Install GTAxSkate.cmd`**.

It finds GTA V by itself and asks only for your Skate 3 disc or folder. Then it:

| Step | |
|---|---|
| 1 tools | gets the SK8V release (Python, the rage tool) and vgmstream |
| 2 sk8v | runs SK8V's own setup: Skate 3's data from your disc, installed into GTA V (10–30 minutes) |
| 3 bin | GTAxSkate's builds (SK8V with our additions, the board tool) |
| 4 boards | the board library from your Skate 3 and every part as a GTA model |
| 5 music | Skate 3's soundtrack on Self Radio (`-NoMusic` skips it) |
| 6 install | our SK8V build over SK8V's, our settings on (yours are kept) |

Every step is remembered: if anything stops it, run it again and it goes on where it stopped.
`scripts\install.ps1 -Redo <step>` runs a step again (for example `-Redo boards` after adding your own decks).

Afterwards, in GTA V: Settings > Audio > Self Radio > Full Scan once for the soundtrack.

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
`contrib/trainer-tuning`, `contrib/map-helpers`, `contrib/skate-radio`), each offered to SK8V as a pull request.

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

## License

Apache License 2.0 (see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE)).
