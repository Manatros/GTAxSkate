# GTAxSkate: Skate 3 skateboarding in GTA V (Legacy), keyboard, board shop and more for SK8V

[![CI](https://github.com/Manatros/GTAxSkate/actions/workflows/ci.yml/badge.svg?branch=dev)](https://github.com/Manatros/GTAxSkate/actions/workflows/ci.yml)

**Download: [GTAxSkate-Setup.zip](https://github.com/Manatros/GTAxSkate/releases/latest/download/GTAxSkate-Setup.zip)**, unzip, close GTA V, double-click `Install GTAxSkate.cmd`. ([All downloads](https://github.com/Manatros/GTAxSkate/releases/latest), [how to install](#download))

**GTAxSkate** adds to **[SK8V](https://github.com/becaring/SK8V)** by becaring, the mod that runs
the real **Skate 3** engine in **GTA V story mode**. SK8V does the skating (Skate's own physics,
tricks, animations, audio, collision, Hall of Meat). GTAxSkate fills the gaps around it:
**keyboard and mouse controls**, a **skateboard shop** with every Skate 3 deck, truck, wheel,
griptape and paint, **GTA's own camera** and a **cinematic camera**, trainer tuning and stamina,
the GPS route while skating, skateable cars, the board in hand as a melee weapon and a limp after
hard bails. All of it in one install, each part optional.

For **GTA V Legacy** (1.0.3889.0, story mode) on PC. No game files are in this repository: the mod
is made from your own copies of the games.

> **Status:** beta. Tested by its author on one PC; please report problems as an
> [issue](https://github.com/Manatros/GTAxSkate/issues) with your logs from `%LOCALAPPDATA%\SkateV`.
> Every addition is also offered to SK8V itself (one branch per feature). `main` holds the released
> version, `dev` the work in progress. See the [changelog](CHANGELOG.md).

## Download

1. Download **[GTAxSkate-Setup.zip](https://github.com/Manatros/GTAxSkate/releases/latest/download/GTAxSkate-Setup.zip)** and unzip it.
2. Close GTA V.
3. Double-click **`Install GTAxSkate.cmd`**, pick your Skate 3 disc or folder, untick what you don't want, wait.

That is all: it installs SK8V, converts your Skate 3 data and adds everything below. Details: [Install](#install).

Already have SK8V 1.1.8 and only want the additions without the board shop?
[GTAxSkate-dropin-for-SK8V-1.1.8.zip](https://github.com/Manatros/GTAxSkate/releases/latest/download/GTAxSkate-dropin-for-SK8V-1.1.8.zip):
copy its two files into the GTA V folder (back up yours first).

## Features (on top of SK8V)

| | |
|---|---|
| Keyboard and mouse | play Skate without a controller: WASD, Shift push, Space ollie, arrow keys for the trick stick, every key rebindable in the menu (F10 > *Keyboard keys*) or `SkateVKeys.txt`, your keys on screen while skating |
| Skateboard shop | **G** in Binco, the Discount Store and Suburban: complete boards, and every deck (by maker), truck, wheel, griptape and paint on its own, paid with the character's cash |
| Your board | the board you buy is the one you ride, with Skate 3's own models: trucks turn and wheels spin; Michael, Franklin and Trevor each keep their own boards |
| Wear | the board wears with riding, grinds, slides and bails and shows Skate 3's worn finish from half worn; a new one starts fresh |
| Customization | griptapes (colours, checkers, prints), painted decks, wheels and truck parts (matte, gloss, metallic; painted wheels ride as hard as their paint), at street prices (deck 55-80 $, trucks 45-70 $, wheels 30-45 $, griptape 10-15 $) |
| Boards on show | each store hangs four boards on a rack behind the till (or stands five along a wall), new ones every game day; look at them and press **E** |
| Your own decks | images in `custom-assets/decks/` become decks in the shop ([how](custom-assets/README.md)) |
| Cars | cars SK8V only knows as boxes are measured once, so their roof, hood and trunk can be ridden and hippie jumped (adjustable height; 1.6 clears a car) |
| People | optionally only knocked over, not killed |
| Injuries | after a hard bail your character limps on foot for 30 s to 2 min with GTA's injured walk |
| Board in hand | the board picked in the weapon wheel stays in your hand in GTA: swing it (GTA's melee; blows wear it), E or F gets on (`CarryBoard=`; F10 > *Board in hand* places it) |
| Cameras | **V**: near, middle, far (a third person camera that follows the skater, or the board, and never swings into them; the mouse turns it), the skater's eyes (first person), Skate 3's camera. The view is kept between walking and skating; the standard view is set in the menu. **R**: a cinematic camera that cuts like GTA's in a car (wide bars, film look, shallow focus), then a skate filmer (4:3, handheld) |
| Tuning | pop, grind pop, push, braking, steering, wobble, grip, turn power, manual drag, hold fakie in SK8V's menu |
| Stamina | GTA's stamina skill sets push and pop; skating at speed trains it |
| Map | GTA's GPS route stays on while skating; GTA's skate parks are on the map |
| Radio | skating tunes the radio to a station you choose (`SkateRadio=`); `,` / `.` change station. Skate 3's soundtrack on Self Radio: the installer's music step does not find the songs yet (it is being fixed), so it may be skipped |

Prefer it more like GTA or more like Skate 3? `CameraView=Near` (the installer's default) is the
third person camera, `CameraView=Skate` Skate 3's; **V** switches while skating, and F10 > *Camera*
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
| **M** | GTA's interaction menu, also while skating (F10 > *Interaction*: GPS to the nearest clothing store or skate spot) |
| **G**, or **E** looking at the boards on show | board shop (in a clothing store); **Tab** shows the part alone, **W A S D** or the mouse turn it, the wheel zooms |
| **F10** (or **/** on a US keyboard) | SK8V's settings menu; page *Keyboard keys* rebinds every key or switches the layout (standard, or Skate's with I J K L as the trick stick) |
| **Tab** / mouse buttons | gun out or away (hold Tab: GTA's weapon wheel); with a gun out, right mouse aims and left shoots |
| **B** (hold, behind a moving car) | skitch |

A controller works as in SK8V; the keyboard is used whenever none is connected.

## What you need

- **GTA V Legacy** 1.0.3889.0 (story mode, BattlEye off)
- **Skate 3** (Xbox 360): your disc image (ISO) or its extracted folder

Everything else the installer gets by itself: SK8V and its tools (Python, the rage tool, Script Hook V),
and GTAxSkate's ready builds. No OpenIV, CodeWalker, Rust or Visual Studio needed.

## Install

1. Download [GTAxSkate-Setup.zip](https://github.com/Manatros/GTAxSkate/releases/latest/download/GTAxSkate-Setup.zip)
   and unzip it anywhere (it carries the installer and every GTAxSkate program).
2. Close GTA V.
3. Double-click **`Install GTAxSkate.cmd`**.

(From this repository's source instead: the same `Install GTAxSkate.cmd`; it downloads the programs.)

It finds GTA V by itself, asks for your Skate 3 disc or folder, and shows a list of the parts with
tick boxes: board shop, Skate 3 music, keyboard and mouse, GTA-style camera, skateable cars, GPS and
skate parks, stamina, people surviving, a limp after hard bails, and the board in hand.
Untick what you don't want (it is remembered; `-Choose` asks again, `-Parts all` skips the question).
Then it:

| Step | |
|---|---|
| 1 tools | gets the SK8V release (Python, the rage tool) and vgmstream |
| 2 sk8v | runs SK8V's own setup: Skate 3's data from your disc, installed into GTA V (10–30 minutes) |
| 3 bin | GTAxSkate's builds (SK8V with our additions, the board tool), from the package |
| 4 boards | the board library from your Skate 3 and every part as a GTA model |
| 5 music | Skate 3's soundtrack on Self Radio (not finding the songs yet; skipped if it fails) |
| 6 install | our SK8V build over SK8V's, our settings on (yours are kept) |

Every step is remembered: if anything stops it, run it again and it goes on where it stopped.
`scripts\install.ps1 -Redo <step>` runs a step again (for example `-Redo boards` after adding your own decks).

Afterwards, in GTA V: **F10** opens the settings menu (on a controller: Back + LB).

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

## License

Apache License 2.0 (see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE)).
