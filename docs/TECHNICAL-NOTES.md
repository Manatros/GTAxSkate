# Technical notes

How GTAxSkate is put together, plus detailed notes on recent fixes. Back to the [README](../README.md).

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

The SK8V additions themselves live in the fork [Manatros/SK8V](https://github.com/Manatros/SK8V)
(branches `contrib/keyboard-mouse`, `contrib/board-shop`, `contrib/gta-camera`,
`contrib/trainer-tuning`, `contrib/map-helpers`, `contrib/skate-radio`, `contrib/gentle-peds`,
`contrib/vehicle-shapes`, `contrib/injuries`), each to be offered to SK8V as a pull request. Branch
`skatemods` is all of them on the newest SK8V: the build in the release.

## Repository layout

| Path | What |
|---|---|
| `Install GTAxSkate.cmd` | the installer (runs `scripts/install.ps1`) |
| `gta/skate-ped` | the board library and every part (decks, trucks, wheels, griptapes, paints) from your Skate 3 data |
| `gta/skate-music` | Skate 3's soundtrack as MP3s for Self Radio |
| `gta/crates` | board catalog, input and rule crates shared with the SK8V additions |
| `gta/skate-gta` | the earlier stand-alone plugin (kept for reference; not installed) |
| `scripts` | the installer and the steps it runs |
| `custom-assets` | your own decks |
| `fivem` | FiveM rules resource and the experimental friends adapter |
| `docs` | the detailed guides linked from the README |

## Release evidence (v0.6.0)

The current main version uses SK8V **1.1.10**. The maintainer completed a roughly 32-minute gameplay test and approved the exact installed build on 17 October 2026. The published host/runtime pair is byte-identical to that tested pair. This is acceptance on one PC, not a claim that every possible configuration or checklist case was exercised.

**Known issue:** board lighting indoors and in tunnels can look wrong. It is deferred and does not block this release. Clearer menu organization, native GTA styling and easier navigation are planned next; they are not changes in this accepted build.

## Save-reload test fix (2026-10-14)

The host guards duplicate graphics callback registration when loading a Story save. Cinematic HUD updates run once per frame. See [independent SK8V adoption notes](../SK8V-SAVE-RELOAD-INTEGRATION.md) and [test instructions](../test-build/TESTPLAN.txt). The original duplicate-registration fatal survived two recorded reloads after its fix. The cumulative Story build was accepted on 17 October 2026; the historical fix-specific notes retain their original evidence limits. Standalone FiveM rules and the separate experimental friends adapter are different packages.

## Reload, camera and Developer follow-up

The host reuses its process runtime after a save load and resends the
HUD viewport. This keeps the persistent sound feeder attached to the same runtime.
Scripted follow/cinematic cameras now respect nearby obstacle hits immediately;
normal GTA camera behavior is unchanged. F10 > Developer shows the SK8V source
basis, compiled host build, optional test receipt metadata and LS integration status.
Receipt metadata does not verify installed binaries; LS Skate remains unintegrated.
See [runtime lifecycle adoption](../SK8V-RELOAD-LIFECYCLE.md) and
[camera/version adoption](../SK8V-CAMERA-VERSIONS.md). The cumulative Story build was accepted on 17 October 2026; individual checklist coverage was not exhaustively recorded.

## Experimental FiveM friends slice

**Multiplayer prototype - currently paused (known missing-board model):** [FiveM test page and downloads](https://github.com/Manatros/GTAxSkate/releases/tag/fivem-friends-test-20261014).
Joining players need GTA V Legacy and [FiveM](https://fivem.net/); the server sends the skate resources automatically.
See the [short joining guide](../FIVEM-JOIN.md). The ZIP is for server hosts. The current prototype fails to load its board model; full animations, shops and F10 are not included. Singleplayer validation currently takes priority.

A separate pinned SK8M Lua-fallback package now connects four server-controlled
multipliers (push speed/power, braking, steering). It uses base-game props and
client-owned skating; Skate 3/WASM physics and a real two-player acceptance test
are still pending. [Preparation, server setup and test guide](../FIVEM-FRIENDS-TEST.md).
The existing standalone rules installer remains separate. No Story ASI belongs
in FiveM, and no account/key/firewall/server startup is automated here.
