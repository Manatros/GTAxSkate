# Installing GTAxSkate

The detailed install guide. The short version is in the [README](../README.md#get-it).

## What you need

- **GTA V Legacy** 1.0.3889.0 (story mode, BattlEye off)
- **Skate 3** (Xbox 360): your disc image (ISO) or its extracted folder
- **[Script Hook V](http://www.dev-c.com/gtav/scripthookv/)**: copy `ScriptHookV.dll` and `dinput8.dll`
  from its `bin` folder into the GTA V folder before you run the installer

Everything else the installer gets by itself: SK8V and its tools (Python, the rage tool),
and GTAxSkate's ready builds. No OpenIV, CodeWalker, Rust or Visual Studio needed.

## Download

Download [v0.6.0](https://github.com/Manatros/GTAxSkate/releases/tag/v0.6.0): **GTAxSkate-Setup.zip** for the full installer, or **GTAxSkate-dropin-for-SK8V-1.1.10.zip** for an existing matching SK8V installation. Always replace both host and runtime together. Release assets include notices and checksums; the full tool bundle includes corresponding source for GPL tools. The Setup ZIP is the complete package; installing from this repository's source downloads the same accepted programs and checks their SHA-256 digest.

The separate [FiveM test release](https://github.com/Manatros/GTAxSkate/releases/tag/fivem-friends-test-20261014) remains a paused historical prototype with a missing-board model.

## Choose your target

| Target | Installation and status |
|---|---|
| **Story Mode (Legacy)** | Use `Install GTAxSkate.cmd` as described below; published Story package; see status and known issues in the README |
| **FiveM rules only** | `Install GTAxSkate FiveM.cmd` installs the standalone rules resource; it does not install skating. |
| **FiveM friends prototype** | Separate pinned SK8M Lua fallback with network boards and four server-controlled settings; [setup/test guide](../FIVEM-FRIENDS-TEST.md). Real two-client acceptance is pending. |
| **Existing co-op mod** | Separate compatibility target; no confirmed co-op release yet |

For FiveM client download, txAdmin/server setup, resource installation and
administrator settings, follow the [FiveM and co-op guide](../fivem/README.md).
It documents unrestricted, selected, bounded and fully server-locked settings.
Do not install the Story Mode ASIs as a FiveM resource. Joining players only need the
[short joining guide](../FIVEM-JOIN.md).

## Story Mode installer

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

## Uninstall

As SK8V's [uninstall](https://github.com/becaring/SK8V#uninstall) (GTA V closed): remove
`SkateVLegacy.asi`, `SkateVRuntime.dll`, `SkateVLegacy.ini`, `SkateVLegacy.installed.json`, the `SK8V\`
folder (`GTAxSkate\` for installs before v0.5.4) and `update\x64\dlcpacks\skatev\`; the soundtrack is in
`Documents\Rockstar Games\GTA V\User Music\Skate 3`.
