# GTAxSkate v0.1.0 (2026-10-10): the first working version

This is an **old version**, kept for reference. The current one is on the
[`main` branch](https://github.com/Manatros/GTAxSkate).

Skate 3's physics, tricks and animations in GTA V Legacy (story mode), with the plugin's
own Skate core, before SK8V was tried. Tested in the game: skating works for the most part
(80–100 fps), the board shop in clothing stores, every Skate 3 deck, truck and wheel as a HD
model, griptapes, painted wheels and trucks, boards on show, skate parks, Skate 3's own board
sounds, Hall of Meat, cinematic camera.

## What this version needs

It builds on the [2010 Rust Rewrite Mashup](https://github.com/chasmlol/2010-rust-rewrite-mashup)
for the Skate engine and its converted Skate 3 data (later versions do not):

| | |
|---|---|
| `mashup-mods.patch` | the changes to the mashup at commit `f608f85e407ff1b7689d54a9aafdd16e95711ac4` (the Skate engine and the board catalog crates) |
| `build-skate-gta.ps1` | checks out that commit next to it as `mashup-src`, applies the patch and builds `SkateGTA.asi` |
| `gta/skate-gta` | the Script Hook V plugin |
| `gta/skate-ped` | the board part models and the skateboard weapon |
| `pack-skate-skater.ps1` | packs them as a DLC in the `mods` folder (needs CodeWalker) |
| `get-skate-sounds.ps1`, `get-skate-parks.ps1` | Skate 3's board sounds and GTA's skate park collision |

No game files are included: everything is made from your own copies of Skate 3 and GTA V.
Licensed under Apache-2.0 like the main branch.
