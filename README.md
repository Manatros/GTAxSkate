# GTAxSkate: Skate 3 skateboarding in GTA V (Legacy), with a board shop and full board customization

**GTAxSkate** is a GTA V mod for **skateboarding in GTA 5 story mode** with the real **Skate 3**
physics, tricks and animations. It runs on top of **[SK8V](https://github.com/becaring/SK8V)** and adds
a **skateboard shop** in GTA's clothing stores, every Skate 3 deck, truck and wheel as a **HD model**,
griptapes, paints, stickers, board wear, **keyboard and mouse controls**, cinematic cameras and more.

For **GTA V Legacy** (1.0.3889.0, story mode) on PC. Built from your own copies of the games: no game
files are in this repository.

**Status:** work in progress, a personal mod. Nothing here is a release.

## How this came about

This started as **my own mod**: Skate 3's simulation running in GTA V with my own pose writing,
ground probes, park collision, board sounds and Hall of Meat, and a lot of board customization
around it (store, HD parts, paints, griptapes, stickers, wear, cameras). While I was working on
it, I found **SK8V** by becaring, built on the same Skate base at the same time.

SK8V did several core things better than mine (the pose hook, the transitions between GTA and
Skate, Skate's own audio engine, collision, the HUD), so I took those and kept what mine does
better or what SK8V does not have. **SK8V is the foundation now, and this mod runs beside it as
the extras.** Thanks to becaring for SK8V and for releasing it under Apache-2.0.

## Features

| | |
|---|---|
| Skateboard shop | **G** in GTA's clothing stores: complete boards, and every deck, truck and wheel, sorted by brand |
| HD board parts | every Skate 3 deck (303), truck (40) and wheel (142) as its own GTA model with Skate's normal maps and materials, under your feet and in your hand |
| Customization | griptapes (colours, checkers, prints), painted decks, wheels and truck parts (matte, gloss, metallic; wheel hardness from the paint), stickers from Skate 3's graphics editor |
| Your own decks | drop images into `custom-assets/decks/`, see [below](#your-own-decks) |
| Board wear | boards scuff with riding and with blows |
| Boards on show | in stores, the same per store and game day |
| Keyboard and mouse | SK8V reads only a controller; this mod hands it the keyboard and mouse |
| Options menu | difficulty, truck tightness, wheel hardness, pop, grind pop, push, braking, steering and more, plus every key, live in the game |
| Cameras | **R**: GTA's cinematic camera, then a fisheye "filmer" camera like the Skate videos |
| Other | GPS route while skating, a garage per character (Michael, Franklin, Trevor), skate park blips, stamina |

## Controls (keyboard and mouse)

| Key | Does |
|---|---|
| Weapon wheel: skateboard, or **E** with the board in hand | get on the board |
| **E** | get on / off the board |
| **F** | back to GTA (board in hand) |
| **Shift** | push |
| **Space** | pop (hold, then let go) |
| **Q** | hippie jump |
| **G** | board shop (in a clothing store) |
| **R** | cinematic cameras |

Every key can be changed in the options menu (F10 by default).

## What you need

Only your own copies of the games, and the usual GTA modding tools:

- **GTA V Legacy** 1.0.3889.0 (story mode, BattlEye off) with **Script Hook V** and **OpenIV.asi** (OpenIV > Tools > ASI Manager)
- **Skate 3** (Xbox 360), extracted to a folder
- **SK8V 1.0.9** set up with its own `SK8V Setup.cmd` (keep its release folder)
- **[CodeWalker](https://github.com/dexyfex/CodeWalker/releases)** (used to pack the board models)
- Git, **Rust** (rustup), and the **Visual Studio C++ build tools** with CMake (to build SK8V with the patches and this mod)

## Install

With GTA V closed, in PowerShell:

```powershell
git clone https://github.com/Manatros/GTAxSkate.git
cd GTAxSkate
powershell -ExecutionPolicy Bypass -File install-gta.ps1 `
    -Gta "C:\Games\Grand Theft Auto V" `
    -Skate3 "C:\Games\Skate 3" `
    -Sk8v "C:\Downloads\SK8V-1.0.9" `
    -CodeWalker "C:\Tools\CodeWalker"
```

It does, in order (each step is skipped when already done, so it can be run again):

1. builds **SK8V** with the patches in [`sk8v-patches/`](sk8v-patches) and installs its ASI and runtime
2. builds the **board library** from your Skate 3 data with the Skate 3 Rust Engine's own tools (from SK8V's release folder), and adds [your own decks](#your-own-decks)
3. builds and installs **SkateGTA.asi**
4. makes the **HD board parts**, griptapes, stickers and single wheels and packs them as the `skate3props` DLC (in the mods folder; GTA's own files are not changed)
5. hides SK8V's own board (`DrawBoard=0`, `DrawSkater=0` in `SkateVLegacy.ini`) so the HD parts take its place

## Your own decks

Put images into `custom-assets/decks/` and run the installer again: each one becomes a deck in the
shop, beside Skate 3's own. A folder or file named after a brand puts it under that brand
(`custom-assets/decks/Baker/my deck.png`), anything else goes under "Custom". Details:
[`custom-assets/README.md`](custom-assets/README.md).

## The SK8V patches

1. SK8V's ASI exports its runtime handle, so this mod reads the skater's state and the board's pose.
2. While GTA has the keyboard, SK8V takes the pad this mod translates from the keyboard and mouse.
3. **E** with the board in hand gets on; a take-out GTA refuses (a car door, a cutscene) puts the board away and says why.
4. `sv_set_player_setup`: the options menu's trucks, wheels, tuning and hold-fakie go to SK8V's Skate session.

## Layout

| Path | What |
|---|---|
| `gta/skate-gta` | the Script Hook V plugin (Rust) |
| `gta/skate-ped` | builds the board library, the board part models, griptapes and stickers from your Skate 3 data |
| `gta/crates` | the board catalog, input, rules, park and sound crates the two use |
| `custom-assets` | your own decks |
| `install-gta.ps1` | the installer above |
| `pack-skate-skater.ps1` | packs the models into the DLC (called by the installer) |

## Legal

GTAxSkate is a free, non-commercial fan mod for single-player use. It is **not affiliated with,
endorsed or sponsored by** Electronic Arts, EA Black Box, Full Circle, Rockstar Games or
Take-Two Interactive. *Skate*, *Skate 3* and *skate.* are trademarks of Electronic Arts Inc.;
*Grand Theft Auto* and *GTA V* are trademarks of Take-Two Interactive Software, Inc. Brand names
of real skate companies appear only as the names Skate 3 itself gives its parts.

**No game files are distributed.** This repository holds only source code and scripts. The
installer reads the data from your own, legally obtained copies of Skate 3 and GTA V on your PC,
and everything it makes stays there. It does not modify GTA V's own archives (it adds a DLC pack
in the `mods` folder) and works in **story mode only**: do not use it online.

## License

Apache License 2.0 (see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE)), the same as SK8V.

<sub>Keywords: GTA 5 skateboard mod, GTA V skateboarding, Skate 3 in GTA 5, Skate 3 PC, skateboard
mod GTA V Legacy, SK8V, Script Hook V skateboard, GTA 5 skate park, board shop, griptape, Skate 3 boards.</sub>
