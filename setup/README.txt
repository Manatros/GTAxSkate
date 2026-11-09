GTAxSkate v0.7.0: Skate 3 in GTA V (Legacy)
============================================

SK8V (by becaring) with GTAxSkate's additions: keyboard and mouse,
the Skate 3 board shop, third person and cinematic cameras, and skateable cars.

You need
  - GTA V Legacy 1.0.3889.0 (story mode)
  - PowerShell 7 (pwsh.exe), required for world collision generation
  - Your Skate 3 (Xbox 360) disc image (.iso) or its extracted folder
  - Script Hook V (http://www.dev-c.com/gtav/scripthookv/): copy
    ScriptHookV.dll and dinput8.dll from its bin folder into the GTA V
    folder first

Install
  1. Close GTA V, OpenIV and CodeWalker.
  2. Double-click "Install GTAxSkate.cmd".
  3. Pick your Skate 3 disc or folder when asked (GTA V is found by itself).
  4. Untick the parts you don't want, click Install, wait (20-40 minutes
     the first time: it converts Skate 3's data from your disc).

It gets SK8V and its tools by itself and keeps going where it stopped if
anything interrupts it: just run it again.
Music is optional. If the disc is missing when resuming, the mod installation
finishes and reports that the soundtrack was skipped. Run again with your disc
to retry the music.

Release status and optional data
  Authorized by Manatros on 26 October 2026. Exact installed candidate:
  test-20261026l-rc-b27eec6; world rules 4217eedcd28b7deb.
  Earlier gameplay tests confirmed the object dropper working. Final bowl,
  rail and shop changes remain untested in game; see TEST-RUN.md.
  Keep world selected: installer step 7 builds collision tiles, visible rails
  and high-res bowl meshes from your own GTA files using PowerShell 7.
  It needs time and several GB of space. It verifies the complete result
  before replacing world data and preserves the previous set for recovery.
  Older tiles are incompatible: after updating, use -Redo world to rebuild.
  There are no synthetic fallback floors, generic prop boxes or rail boxes.
  Glowing wheel models need a separately supplied CodeWalker installation;
  no CodeWalker is bundled. Keep the composition helper disabled and use
  separate board parts. Retry failed optional stages with -Redo.

In GTA V (story mode)
  - Pick the skateboard in the weapon wheel (or use SK8V's menu action).
  - WASD steer, Shift push, Space ollie (hold, let go), arrow keys tricks,
    E on/off the board, hold E/Y to leave, V camera, R cinematic camera.
  - F10: settings (Keyboard keys, Camera, and more).
  - Insert: object dropper (ramps, rails, benches and more).
  - G in Binco, the Discount Store or Suburban: the board shop.
  - E at the marked Vespucci Beach Skate Shop: daily stock and parts.
  - F10 > World > World collision: status/reset, reversible plain GTA
    comparison and Mark this spot for reports.

Problems: https://github.com/Manatros/GTAxSkate/issues
(attach relevant logs from %LOCALAPPDATA%\SkateV after redacting names,
personal paths, IP addresses and credentials; keep original logs locally).

Change what is installed later: run "Install GTAxSkate.cmd" again with
"-Choose" (scripts\install.ps1 -Choose), or use the F10 menu.
