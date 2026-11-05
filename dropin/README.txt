GTAxSkate drop-in for SK8V 1.1.10
================================

Skate 3 in GTA V (SK8V by becaring) with GTAxSkate's optional additions.
See VERSION.txt and GTAxSkate-build.json for this package's exact source/build.
Development builds are test candidates; source fixes do not establish a gameplay pass.
The community AI skaters, skitch assist/towing and shoulder cat are excluded.

Install (SK8V 1.1.10 already installed and working):
  1. Close GTA V, OpenIV and CodeWalker.
  2. Back up SkateVLegacy.asi and SkateVRuntime.dll from your GTA V folder.
     Also back up GTAxSkate/collectibles.tsv if it already exists.
  3. Copy SkateVLegacy.asi and SkateVRuntime.dll from this ZIP into the GTA V
     folder (beside GTA5.exe), replacing them. Always both together.
  4. Copy the ZIP's GTAxSkate folder into that same GTA V folder, merging it
     and replacing GTAxSkate/collectibles.tsv with the matching shipped data.
     Keep the binaries and this data together. The ZIP contains no world tiles,
     INI settings or saved collectible progress; preserve your existing files.

What you get:
  - Keyboard and mouse: WASD, Shift push, Space ollie (hold, let go), arrow
    keys tricks, E on/off the board, hold E to leave skate mode, B held behind
    a car skitches, Tab gun (hold: weapon wheel), mouse aims/shoots.
    F10 opens SK8V's menu; page "Keyboard keys" rebinds every key.
  - Cameras: V cycles near / middle / far (third person that follows the
    skater), the skater's eyes, Skate 3's camera; R cinematic camera and
    skate filmer. Menu page "Camera".
  - Skateable cars and a higher hippie jump (menu: Hippie jump height).
  - Insert: build your own park with ramps, rails, benches and more.
  - F10 > World > World collision: status/reset, reversible comparison
    and Mark this spot for collision reports.
  - GPS route while skating, skate parks on the map, stamina, radio,
    people knocked over survive (all in the menu).

The Skate 3 board shop (every deck, truck, wheel, griptape, paint) needs
the board models made from your Skate 3 disc: use the full installer
("Install GTAxSkate.cmd" from https://github.com/Manatros/GTAxSkate).

Uninstall: put the two backed-up binaries back. Restore the previous
GTAxSkate/collectibles.tsv, or remove only that file if it did not exist before.
Keep your settings, progress and other GTAxSkate files.

Optional composed custom boards require separate helper setup; this binary
drop-in alone does not configure it. Instructions and separate helper rollback:
https://github.com/Manatros/GTAxSkate/blob/main/BOARD-COMPOSITION.md
The host/runtime pair is source-identified in GTAxSkate-build.json.
Enter through the weapon wheel or SK8V menu; F is not an extra mode switch.
This drop-in does not generate world data. Older world tiles
are incompatible and skipped. Use the matching Setup ZIP with world selected
and -Redo world to build tiles, rails and high-res bowl meshes from your GTA
files (requires PowerShell 7/pwsh, time and several GB of space). Without
matching data, skating uses GTA's live collision. See WORLD-DATA.md.
Keep the composition helper disabled; use separate board parts.
The checklist for future regression tests is at:
https://github.com/Manatros/GTAxSkate/blob/main/TEST-RUN.md
Redact names, personal paths and credentials before sharing logs.

Source: https://github.com/Manatros/SK8V (branch/commit in VERSION.txt; GTAXSKATE.md
explains every part). SK8V: https://github.com/becaring/SK8V. Apache-2.0,
see LICENSE-SK8V.txt and NOTICE.txt.
