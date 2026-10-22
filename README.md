# GTAxSkate

[![CI](https://github.com/Manatros/GTAxSkate/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Manatros/GTAxSkate/actions/workflows/ci.yml)

**Skate 3 skateboarding inside GTA V story mode, with keyboard and mouse, a board shop and more.**

GTAxSkate is a free fan mod that builds on **[SK8V](https://github.com/becaring/SK8V)** by becaring.
SK8V runs the real *Skate 3* engine inside *GTA V* (Skate's own physics, tricks, animations, audio
and Hall of Meat). GTAxSkate adds what SK8V doesn't have: **keyboard and mouse controls**, a
**skateboard shop** with every Skate 3 part, **GTA-style and cinematic cameras**, trainer tuning,
stamina, GPS while skating, skateable cars and injuries after hard bails. One installer, every
part optional.

> **Current release: [v0.6.0](https://github.com/Manatros/GTAxSkate/releases/tag/v0.6.0)** (beta), for
> **GTA V Legacy 1.0.3889.0** story mode on PC, using SK8V 1.1.10. Accepted in a Story Mode gameplay
> test on 17 October 2026. No game files are in this repository; the mod is built from your own copies
> of the games.

## Which repository is which?

GTAxSkate is split across a few places. Pick the one that matches what you want to do:

| Repository | What it is | Go here if you want to... |
|---|---|---|
| **[Manatros/GTAxSkate](https://github.com/Manatros/GTAxSkate)** (this one) | The player-facing package: installer, setup scripts, downloads, docs and the issue tracker | install the mod, report a bug, change the installer or docs |
| **[Manatros/SK8V](https://github.com/Manatros/SK8V)** | Our fork of SK8V: the modified mod source (branch `skatemods`) and one `contrib/...` branch per feature, offered back to SK8V | change how skating, the shop, cameras or controls work in code |
| **[becaring/SK8V](https://github.com/becaring/SK8V)** | The original SK8V by becaring, which everything builds on | use plain SK8V without our additions |

## Branches: stable vs. latest

| Branch | What's on it | Who it's for |
|---|---|---|
| **`main`** | Released, tested versions only. Changes reach `main` after they pass CI and an in-game test. | Players. This is what the installer and releases use. |
| **[`dev`](https://github.com/Manatros/GTAxSkate/tree/dev)** | The **absolute latest** development state, pushed continuously as work happens. | Testers and contributors who want to try new things early. |

> [!WARNING]
> **`dev` can be completely broken at any time.** It may not install, may crash GTA, or may change
> settings. Back up your GTA folder and `SkateVLegacy.ini` before trying it, and don't use it if you
> just want to skate. Bugs found on `dev` are very welcome: open an issue and say you were on `dev`
> (include the commit).

Tagged test builds also appear as [pre-releases](https://github.com/Manatros/GTAxSkate/releases)
before they reach `main`.

## Get it

You need **GTA V Legacy** 1.0.3889.0 (story mode, BattlEye off), your own **Skate 3** (Xbox 360)
disc image or folder, and **[Script Hook V](http://www.dev-c.com/gtav/scripthookv/)** (copy
`ScriptHookV.dll` and `dinput8.dll` into the GTA V folder). The installer fetches everything else.

1. Download **[GTAxSkate-Setup.zip](https://github.com/Manatros/GTAxSkate/releases/latest/download/GTAxSkate-Setup.zip)** and unzip it anywhere.
2. Close GTA V.
3. Double-click **`Install GTAxSkate.cmd`**, pick your Skate 3 disc and tick the parts you want.

In game, press **F10** for the settings menu. Full guide, FiveM options and uninstall steps:
**[docs/INSTALL.md](docs/INSTALL.md)**.

## Basic controls

| Key | Does |
|---|---|
| **F** / **E** | board out and skate / get on or off |
| **W A S D** | steer, lean, walk |
| **Shift** / **Space** | push / ollie (hold, then let go) |
| **Arrow keys** | trick stick (kickflip: down, then up-left) |
| **V** / **R** | camera views / cinematic camera |
| **G** | board shop (in Binco, Discount Store, Suburban) |
| **F10** | settings menu, key rebinding and presets |

Controllers work as in SK8V. All keys, keyboard presets and play profiles:
**[docs/FEATURES-AND-CONTROLS.md](docs/FEATURES-AND-CONTROLS.md)**.

## Known issues

- Board lighting indoors and in tunnels can look wrong.
- **M** while skating does not open GTA's interaction menu yet (works on foot).
- Printed griptapes show a seam at the deck's edge.
- Riding sideways along an edge the board does not always stick; a board hit while bailing can drag the body.
- The board shop can register at most 240 new part models per session (GTA's limit); restart GTA to see more.
- The FiveM multiplayer prototype is **paused** (missing board model); singleplayer comes first.

More in [SUPPORT.md](SUPPORT.md) and the [changelog](CHANGELOG.md).

## Reporting problems

Open an [issue](https://github.com/Manatros/GTAxSkate/issues/new/choose) with small, **redacted**
log excerpts from `%LOCALAPPDATA%\SkateV`. Remove personal paths, names and keys first; read
[PRIVACY.md](PRIVACY.md). Security problems go through [SECURITY.md](SECURITY.md).

## Contributing

1. Branch off **`dev`** and open your pull request against **`dev`**.
2. Keep one reviewable change per pull request and say which checks you actually ran.
3. Once a batch on `dev` passes CI and an in-game test, it's merged into `main` and released.

Code that changes SK8V itself belongs in [Manatros/SK8V](https://github.com/Manatros/SK8V). Details:
[CONTRIBUTING.md](CONTRIBUTING.md). How the pieces fit together:
[docs/TECHNICAL-NOTES.md](docs/TECHNICAL-NOTES.md).

## Documentation

| Document | Contents |
|---|---|
| [docs/INSTALL.md](docs/INSTALL.md) | requirements, installer steps, FiveM targets, uninstall |
| [docs/FEATURES-AND-CONTROLS.md](docs/FEATURES-AND-CONTROLS.md) | every feature, full key list, keyboard presets, play profiles |
| [docs/TECHNICAL-NOTES.md](docs/TECHNICAL-NOTES.md) | how it builds on SK8V, repo layout, release evidence, fix notes, FiveM prototype |
| [BOARD-COMPOSITION.md](BOARD-COMPOSITION.md) | optional board composition helper |
| [custom-assets/README.md](custom-assets/README.md) | adding your own deck images |
| [CHANGELOG.md](CHANGELOG.md) · [SUPPORT.md](SUPPORT.md) · [PRIVACY.md](PRIVACY.md) | history, support boundaries, safe log sharing |

## Credits and legal

Thanks to becaring for [SK8V](https://github.com/becaring/SK8V) and for releasing it under
Apache-2.0, and to the Skate 3 rewrite it builds on.

GTAxSkate is a free, non-commercial fan mod for single-player use. It is **not affiliated with,
endorsed or sponsored by** Electronic Arts, EA Black Box, Full Circle, Rockstar Games or
Take-Two Interactive. *Skate*, *Skate 3* and *skate.* are trademarks of Electronic Arts Inc.;
*Grand Theft Auto* and *GTA V* are trademarks of Take-Two Interactive Software, Inc. Brand names
of real skate companies appear only as the names Skate 3 itself gives its parts.

**No game files are distributed.** This repository holds only source code and scripts. Everything
is made from your own, legally obtained copies of Skate 3 and GTA V on your PC and stays there. The
mod does not change GTA V's own archives and works in **story mode only**: do not use it online.

Historical privacy-affected downloads remain withdrawn; see [PRIVACY.md](PRIVACY.md).

## License

Apache License 2.0 (see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE)).
