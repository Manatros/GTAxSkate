# GTAxSkate

Skate 3 in GTA V Legacy (story mode), on top of **[SK8V](https://github.com/becaring/SK8V)**,
with a board store and full board customization.

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

## What it adds to SK8V

| | |
|---|---|
| Board store | G in GTA's clothing stores: complete boards, and every deck, truck and wheel by brand |
| HD board parts | every Skate 3 deck, truck and wheel as its own GTA model with Skate's normal maps and materials, on SK8V's board and in the hand |
| Customization | griptapes (colours, checkers, prints), painted decks, wheels and truck parts (matte, gloss, metallic; wheel hardness from the paint), stickers from Skate 3's graphics editor |
| Wear | boards scuff with riding and with blows |
| Boards on show | in stores, the same per store and game day |
| Keyboard and mouse | SK8V reads only a controller; this mod hands it the keyboard and mouse (Shift push, Space pop, Q hippie jump, E on/off the board, F back to GTA) |
| Cameras | R: GTA's cinematic camera, then a fisheye "filmer" camera |
| Other | GPS route while skating, a garage per character, skate park blips, stamina |

## Planned: the skate. add-on

A separate, optional add-on for players who also own EA's *skate.*: its decks, griptapes, trucks,
wheels and the rest as extra parts in the same store sections, with their materials (roughness,
gloss, metallic, normal maps) and sounds, so they look and sound as in the game. Not part of
this repository yet.

## What you need

Only your own copies of the games, and the usual GTA modding tools:

- **GTA V Legacy** 1.0.3889.0 (story mode, BattlEye off) with **Script Hook V** and **OpenIV.asi** (OpenIV > Tools > ASI Manager)
- **Skate 3** (Xbox 360), extracted to a folder
- **SK8V 1.0.9** set up with its own `SK8V Setup.cmd` (keep its release folder)
- **[CodeWalker](https://github.com/dexyfex/CodeWalker/releases)** (used to pack the board models)
- Git, **Rust** (rustup), and the **Visual Studio C++ build tools** with CMake (to build SK8V with the two patches and this mod)

No game data is in this repository; everything is made from your copies on your PC.

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

1. builds **SK8V** with the two patches in [`sk8v-patches/`](sk8v-patches) and installs its ASI and runtime
2. builds the **board library** from your Skate 3 data with the Skate 3 Rust Engine's own tools (from SK8V's release folder)
3. builds and installs **SkateGTA.asi**
4. makes the **HD board parts**, griptapes, stickers and single wheels and packs them as the `skate3props` DLC (in the mods folder)
5. hides SK8V's own board (`DrawBoard=0`, `DrawSkater=0` in `SkateVLegacy.ini`) so the HD parts take its place

Then, in GTA V: pick the skateboard in the weapon wheel to skate; **G** in a clothing store opens the board store.

## The SK8V patches

1. `SkateModsRuntime`: SK8V's ASI exports its runtime handle, so this mod reads the skater's state and the board's pose (`sv_get_output`, `sv_lifecycle_get_state`, `sv_get_board_pose`).
2. `ReadPadRaw`: while GTA has the keyboard, SK8V takes the pad this mod translates from the keyboard and mouse.

## Layout

| Path | What |
|---|---|
| `gta/skate-gta` | the Script Hook V plugin (Rust) |
| `gta/skate-ped` | builds the board library, the board part models, griptapes and stickers from your Skate 3 data |
| `gta/crates` | the board catalog, input, rules, park and sound crates the two use |
| `install-gta.ps1` | the installer above |
| `pack-skate-skater.ps1` | packs the models into the DLC (called by the installer) |

## License

Apache License 2.0 (see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE)), the same as SK8V.
