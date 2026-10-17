GTAxSkate: Skate 3 skating in GTA V (Legacy)
============================================

SK8V (by becaring) with GTAxSkate's additions: keyboard and mouse,
the Skate 3 board shop, third person and cinematic cameras, and skateable cars.

You need
  - GTA V Legacy 1.0.3889.0 (story mode)
  - Your Skate 3 (Xbox 360) disc image (.iso) or its extracted folder
  - Script Hook V (http://www.dev-c.com/gtav/scripthookv/): copy
    ScriptHookV.dll and dinput8.dll from its bin folder into the GTA V
    folder first

Install
  1. Close GTA V.
  2. Double-click "Install GTAxSkate.cmd".
  3. Pick your Skate 3 disc or folder when asked (GTA V is found by itself).
  4. Untick the parts you don't want, click Install, wait (20-40 minutes
     the first time: it converts Skate 3's data from your disc).

It gets SK8V and its tools by itself and keeps going where it stopped if
anything interrupts it: just run it again.
Music is optional. If the disc is missing when resuming, the mod installation
finishes and reports that the soundtrack was skipped. Run again with your disc
to retry the music.

Accepted version and optional composed boards
  v0.6.0 uses SK8V 1.1.10 and the exact binary pair accepted in a Story Mode test
  on 17 October 2026. Known issue: board lighting indoors/in tunnels.
  The optional composition helper is not configured by this installer: it needs
  existing converted board data, Python >=3.10 and the pinned RAGE tool.
  https://github.com/Manatros/GTAxSkate/blob/main/BOARD-COMPOSITION.md
  Keep backup paths; close GTA and model tools before setup or rollback.

In GTA V (story mode)
  - Pick the skateboard in the weapon wheel, or press F on foot.
  - WASD steer, Shift push, Space ollie (hold, let go), arrow keys tricks,
    E on/off the board, F back to GTA, V camera, R cinematic camera.
  - F10: settings (Keyboard keys, Camera, and more).
  - G in Binco, the Discount Store or Suburban: the board shop.

Problems: https://github.com/Manatros/GTAxSkate/issues
(attach relevant logs from %LOCALAPPDATA%\SkateV after redacting names,
personal paths, IP addresses and credentials; keep original logs locally).

Change what is installed later: run "Install GTAxSkate.cmd" again with
"-Choose" (scripts\install.ps1 -Choose), or use the F10 menu.
