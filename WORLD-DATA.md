# Local world data (v0.7.1)

World collision is generated on your PC from your own GTA V Legacy Story Mode
archives. Do not redistribute extracted collision, archive keys, meshes, rails or
baked tiles.

The world step requires **PowerShell 7 (`pwsh`)**. Release v0.7.1 uses world rules
`4217eedcd28b7deb`. Older tiles are incompatible and skipped; rerun Setup with
`-Redo world` after installing the matching host/runtime. The binary-only drop-in
does not rebuild world data. This build does not add fallback floors, generic
prop boxes or rail boxes.

## What the installer builds (step 7, `-Redo world` to repeat)

| Data | Purpose |
|---|---|
| `world/tiles/*.svwt` + `rules.txt`, `tile-size.txt`, `live-cells.txt` | The whole Story map (open world, interiors, metro, tunnels) as 64 m tiles of collision and grind rails, all processed by the same rules |
| `world/sk8v.visrails` (+ `.policy`) | Rails from GTA's visible models: handrails and railings GTA's collision leaves out |
| `world/sk8v.visparks` (+ `.policy`) | High-res skatepark meshes from the parks' visible models (F10 > World > World collision > *High-res skateparks*); GTA collision remains the base, refined on bowl curves |
| `world/collision-build.json` | Receipt: rules hash, region, cell counts, tool hashes |
| `world/sk8v.tracks/trains1..12.dat` | GTA's train path nodes for track grind rails (input) |

Places whose collision depends on the game state (some interiors and map
variants) are listed in `live-cells.txt` and always come from GTA's current
collision while you play.

## Guarantees

- Tiles, the baker and the runtime come from one build: every tile carries the
  same rules hash, and the runtime ignores (and logs) tiles from any other bake.
- Nothing in the game folder changes until the whole map is baked and passed
  native verification; then the complete set replaces the old one. The previous
  data stays in a `world-recovery-*` folder.
- Preparation takes long (whole map) and needs several GB of free space on the
  GTA drive; the game must be closed for the final install.

## For developers

```text
pwsh -NoProfile -File scripts\build-world-collision.ps1 -Gta <GTA> -Keys <GTA>\GTAxSkate\gta\gta-keys `
    -Baker skatev-world-cache.exe -Rails visual-rails.exe -World <GTA>\GTAxSkate\world -StageOnly
pwsh -NoProfile -File scripts\build-world-collision.ps1 -World <GTA>\GTAxSkate\world -Baker skatev-world-cache.exe -InstallFrom <work folder>
skatev-world-cache --rules          # rules hash of this build
```

`-StageOnly` bakes and verifies while the game may run; `-InstallFrom` installs a
staged result with the game closed. `SKATEV_BAKE_THREADS` sets the baker's
threads (default up to 12); `SKATEV_BAKE_OBJ=1` also writes `tiles/region.obj` for
inspection (large). Architecture:
`docs/ARCHITECTURE.md` in the development repository.
