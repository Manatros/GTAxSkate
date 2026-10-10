# GTAxSkate: Skate 3 skateboarding in GTA V (Legacy)

[![CI](https://github.com/Manatros/GTAxSkate/actions/workflows/ci.yml/badge.svg?branch=dev)](https://github.com/Manatros/GTAxSkate/actions/workflows/ci.yml)

**GTAxSkate** is a GTA V mod for **skateboarding in GTA 5 story mode** with the real **Skate 3**
physics, tricks and animations: a **skateboard shop** in GTA's clothing stores, every Skate 3 deck,
truck and wheel as a **HD model**, griptapes, paints, stickers, board wear, Skate 3's own board
sounds, **keyboard and mouse controls**, cinematic cameras and more.

For **GTA V Legacy** (1.0.3889.0, story mode) on PC. No game files are in this repository: the mod
is made from your own copies of the games.

> **Status:** work in progress, a personal mod. `main` holds the newest version confirmed in a game
> test, `dev` the work in progress. See the [changelog](CHANGELOG.md).

## How this came about

This is **my own mod**: Skate 3's simulation running in GTA V with its own Skate core, ground
probes, park collision, board sounds and Hall of Meat, and a lot of board customization around it.
While working on it I found **[SK8V](https://github.com/becaring/SK8V)** by becaring, built on the
same Skate base at the same time. I tried running mine on top of SK8V (version 0.2), then went back
to my own mod as the base and **took over the parts SK8V does better**, ported into this code:
its retarget (how Skate's skater is put on a GTA character) and its pose hook (writing the pose
inside GTA's own animation pass). More will follow where SK8V or other mods do something better.
Thanks to becaring for SK8V and for releasing it under Apache-2.0.

## Features

| | |
|---|---|
| Skating | Skate 3's physics, flick-it tricks, grinds, manuals, bails; on foot and on the board |
| Animation | SK8V's retarget and pose hook: GTA's proportions, feet and grabbing hands on Skate's |
| Skateboard shop | **G** in GTA's clothing stores: complete boards, and every deck, truck and wheel by brand |
| HD board parts | every Skate 3 deck (303), truck (40) and wheel (142) as its own GTA model |
| Customization | griptapes (colours, checkers, prints), painted decks, wheels and truck parts (matte, gloss, metallic), stickers from Skate 3's graphics editor |
| Your own decks | images in `custom-assets/decks/` become decks in the shop ([how](custom-assets/README.md)) |
| Board wear | boards scuff with riding and with blows |
| Sounds | Skate 3's own rolling, landing, pop and grind recordings, by surface |
| Skate parks | GTA's skate spots with their exact collision, on the map |
| Cameras | **R**: GTA's cinematic camera, then a fisheye "filmer" camera like the Skate videos |
| Options menu | **F10**: difficulty, trucks, wheels, pop, push, braking, steering and every key, live in the game |
| Other | garage per character (Michael, Franklin, Trevor), GPS route while skating, Hall of Meat, stamina |

## Controls (keyboard and mouse)

| Key | Does |
|---|---|
| Weapon wheel: skateboard, then **F** | skate mode |
| **E** | get on / off the board |
| **Shift** | push |
| **Space** | pop (hold, then let go) |
| **Q** | hippie jump |
| **R** | cinematic cameras |
| **V** | camera views |
| **G** | board shop (in a clothing store) |
| **F10** | options |

## What you need

- **GTA V Legacy** 1.0.3889.0 (story mode, BattlEye off) with **Script Hook V** and **OpenIV.asi**
- **Skate 3** (Xbox 360), extracted to a folder
- **[CodeWalker](https://github.com/dexyfex/CodeWalker/releases)** (packs the board models)
- Git and **Rust** (rustup), to build the mod

## Install

A one-click installer (a batch file that builds the mod from your Skate 3 and GTA V and puts it in
place) is being written. Until then the mod is built from source:

1. `gta/skate-gta-own` builds `SkateGTA.asi` (the plugin with its Skate core; the core is the
   [2010 Rust Rewrite Mashup](https://github.com/chasmlol/2010-rust-rewrite-mashup)'s skate-host,
   checked out at `external/mashup-src`).
2. `gta/skate-ped` makes the board library and the HD board part models from your Skate 3 data;
   `scripts/pack-skate-skater.ps1` packs them as a DLC in GTA's `mods` folder.

## Layout

| Path | What |
|---|---|
| `gta/skate-gta` | the Script Hook V plugin's source (Rust); its tests run without the game |
| `gta/skate-gta-own` | builds the plugin with its Skate core |
| `gta/skate-ped` | the board library, board part models, griptapes and stickers from your Skate 3 data |
| `gta/crates` | board catalog, input, rules, park and sound crates |
| `scripts` | the DLC packer |
| `custom-assets` | your own decks |

## Legal

GTAxSkate is a free, non-commercial fan mod for single-player use. It is **not affiliated with,
endorsed or sponsored by** Electronic Arts, EA Black Box, Full Circle, Rockstar Games or
Take-Two Interactive. *Skate*, *Skate 3* and *skate.* are trademarks of Electronic Arts Inc.;
*Grand Theft Auto* and *GTA V* are trademarks of Take-Two Interactive Software, Inc. Brand names
of real skate companies appear only as the names Skate 3 itself gives its parts.

**No game files are distributed.** This repository holds only source code and scripts. Everything
is made from your own, legally obtained copies of Skate 3 and GTA V on your PC and stays there. The
mod does not change GTA V's own archives (it adds a DLC pack in the `mods` folder) and works in
**story mode only**: do not use it online.

## License

Apache License 2.0 (see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE)).

<sub>Keywords: GTA 5 skateboard mod, GTA V skateboarding, Skate 3 in GTA 5, Skate 3 PC, skateboard
mod GTA V Legacy, Script Hook V skateboard, GTA 5 skate park, board shop, griptape, Skate 3 boards, SK8V.</sub>
