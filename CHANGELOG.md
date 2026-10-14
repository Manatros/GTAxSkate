# Changelog

All notable changes, newest first. Versions follow [semantic versioning](https://semver.org/):
a new minor version for each state confirmed in a game test, a patch version for fixes.
The v0.5.4 GitHub release exists, but its asset contents and identity have not been independently
verified. The build described below has not been confirmed in an in-game test; publication does
not imply that confirmation.

## Unreleased - FiveM rules groundwork, 2026-10-14

- Source-identified Story test snapshots now include SHA256 checks, non-destructive configuration handling and a verified standalone rollback; separate rules-only FiveM test ZIP. Eight isolated Story installer/rollback cases are in CI. Game confirmation remains pending.
- Separate experimental FiveM rules resource and installer; no skating adapter yet. Server modes: all, selected, explicit ranges, fully locked. Server validation covers individual edits and atomic preset requests.
- FiveM client/txAdmin/server setup guide, administrator examples, existing SK8M reuse and separate co-op target. Future bases use versioned adapters; arbitrary future compatibility is not promised.
- Rule/server integration tests and isolated installer checks, added to CI. Existing Story Mode installer and gameplay binaries unchanged.
- F10 categories/Advanced and GTA (WIP), Skate and RP presets from the preceding local round remain awaiting an in-game test.

## Unreleased — installer QA, 2026-10-13

- Optional soundtrack setup now continues when the disc is absent on a resumed install.
- The installer pins its bundled Python environment to avoid interference from another Python/Conda installation.
- Soundtrack migration protects MP3-named directories and manifest entries replaced by directories or links.
- Added a repeatable Windows PowerShell 5.1 integration harness: 9 scenarios passed. Music tests: 8 passed; actual isolated conversion: 46 tracks. A real sandbox reinstall passed. These checks do not establish in-game behavior.

## v0.5.4 (published; in-game status unconfirmed)

GTAxSkate builds on **SK8V 1.1.8**: SK8V does the skating, GTAxSkate adds what it does not
have yet, each part also offered to SK8V as a pull request. This version has not been confirmed
in an in-game test.

- **Skate 3's soundtrack**: the in-game iPod's 46 songs (`ipod.mpf`, each song put back together from its 30 parts) become MP3s for Self Radio; no ffmpeg or winget required (LAME is built in, vgmstream is used). The old step put ambience recordings there; they are replaced.
- From a disc **ISO**, the files SK8V does not copy (the music) are read from the ISO.
- Installer: asks your stance (Regular/Goofy) like SK8V's setup; checks free space; starts again as administrator when the GTA folder needs it; dialogs open in front; a plain message instead of a PowerShell error page; everything shown goes to `build\install\install.log`; works on older Windows 10 (TLS 1.2).
- Works with SK8V instead of beside it: data in SK8V's own `SK8V` folder (an SK8V already set up is reused, nothing twice on disk); our two files are recorded in SK8V's install record, so SK8V's uninstall removes them and its updates replace them; setup leftovers are removed as SK8V's `--clean` does.
- Script Hook V: another ASI loader than `dinput8.dll` is accepted (a note instead of a stop), an old Script Hook V gets a note.
- Controller: Y with the board in hand near a vehicle stays GTA's "enter vehicle".
- v0.5.3: the installer checks GTA's version, Script Hook V (needed, not downloaded: see What you need) and write access before SK8V's setup, and repeats SK8V's own error message when its setup fails (log in `build\install\sk8v-setup.log`).
- v0.5.2: calmer third person camera (only walls pull it in, never under the ground, never closer than 1.6 m, slower swing); the board lit by the tunnel or room it is in while skating.
- v0.5.1: fixed the script stopping after a long store visit (GTA's model registration is guarded); gentler cars (push and dent settings on F10 > Peds and vehicles); the deck's own grip as a griptape choice; smooth highlights on shiny paints.
- Street prices by part (deck 45-95 $ … griptape 10-20 $); worn look from half wear; blows with the board wear it.
- The board picked in the weapon wheel stays in hand: swing it, E or F gets on (`CarryBoard`).
- A limp after hard bails (`Injuries`). M's GTA interaction menu while skating remains unresolved (B104).
- Keyboard layouts: standard or Skate's (I J K L trick stick). First person looks the right way; the third person camera aims at chest height.
- Griptape over the deck's rounded edge; matte, gloss and metallic paints clearly apart; Skate 3's own spec maps on the parts.
- **Installer: choose the parts** (tick boxes, remembered; `-Choose`, `-Parts all`).
- Third person camera follows the skater (or the board: `CameraFocus`), first person is the skater's eyes.
- Keyboard: Tab and the mouse buttons for guns on the board; B held behind a car skitches; F on foot leaves at once.
- Shop: GTA's help box at the boards on show (E), boards hang level behind the till; WASD/wheel turn and zoom.
- Dev: `ShowInputs` shows the keys and pad on screen.
- Built on SK8V 1.1.8 (Trick Analyzer, wheel tracks in sand and snow, no snow on the board).
- Keyboard and mouse work with a controller connected too (Steam Input, DS4Windows, a wheel):
  whichever was used last plays.
- **Keyboard and mouse** for SK8V: GTA-like keys (WASD, Shift push, Space ollie, arrow keys for the
  trick stick, E board, Q hippie jump, F back to GTA), rebindable in `SkateVKeys.txt`, your keys on
  screen while skating.
- **Board shop** in SK8V: every Skate 3 deck, truck, wheel, griptape and paint, a garage per
  character, the bought board ridden with Skate 3's own models (trucks turn, wheels spin), five boards
  on show in every store, painted wheels as hard as their paint. The part models load from loose
  files: no OpenIV or CodeWalker needed any more.
- **Cameras**: GTA's own camera while skating (mouse), **V** cycles views, **R** a cinematic camera
  and a fisheye skate filmer.
- **Tuning** in SK8V's menu (pop, push, braking, steering, grip, ...) and **stamina**.
- GPS route while skating, GTA's skate parks on the map.
- Skating tunes the radio to Self Radio (Skate 3's soundtrack); `,` / `.` change station, M stays GTA's
  interaction menu.
- The ridden board wears until it shows Skate 3's worn finish.
- The store preview shows (it never did: its matrix was a mirror); **Tab** shows the part alone.
- **F** on foot (no vehicle near) gets on the board; `/` (SK8V's menu) is listed with the keys.
- People the board knocks over get up again (`PedsSurvive`, "Knocked peds survive" in the menu).
- **Cars** SK8V only knows as boxes are measured once (roof, hood, trunk to ride; gap to hippie jump);
  the **hippie jump** height is adjustable (1.6 clears a car).
- **F10** opens the menu; every key can be rebound in it (*Keyboard keys*). **E** at the boards on show
  opens the shop. A *Camera* menu page.
- **Installer** rebuilt: SK8V's own setup, then GTAxSkate's ready builds and the board parts; no Rust,
  Visual Studio, OpenIV or CodeWalker needed. Your SK8V settings are kept.

## 0.3.0 (2026-10-10)

Tested in the game: skating, tricks, the board shop and bought boards work; installed by the installer
from Skate 3 directly. Known: the board sounds are not right yet (Skate 3's own audio engine is next).

- **Animation**: Skate's pose on your character with SK8V's retarget (GTA keeps its own proportions,
  feet and grabbing hands follow Skate through two-bone IK, roll bones twist correctly).
- **Pose hook**: the pose is written inside GTA's own animation pass instead of against it (no flicker,
  smooth 0.5 s hand-over between GTA and Skate). The old way stays as a fallback.
- Board sounds came back (the sound output started too late and gave up).
- People knocked over by the skater no longer die.
- Bought painted boards are kept when the game loads them again.
- Shop prompt no longer flickers while wanted.
- America griptape removed (the USA print looks better).
- Weapon-wheel icon for the skateboard; stickers grouped by maker in the shop.
- D-pad gestures (Raise the roof, Dunno, Shaka, Point) no longer stop the Skate engine.
- Our own Skate core is the base again; SK8V is a code source, not needed to play.
- **Merged Skate engine**: our engine changes on top of SK8V 1.1.0's 47 engine fixes (skitching, moving
  geometry, stability), prepared automatically from the public sources: no 2010 Rust Rewrite Mashup install needed.
- **Installer**: `Install GTAxSkate.cmd` makes the mod from your own Skate 3 and GTA V and installs it.

## 0.2.0-experimental (2026-10-10)

The extras on top of SK8V (board shop, HD parts, keyboard and mouse for SK8V). Never confirmed in a
test; replaced by the own-core line.

## 0.1.0 (2026-10-10)

The first working version, with the plugin's own Skate core (built on the 2010 Rust Rewrite Mashup):
skating at 80–100 fps, the board shop in clothing stores, every Skate 3 deck, truck and wheel as a HD
model, griptapes, painted wheels and trucks, boards on show, skate parks, Skate 3's board sounds,
Hall of Meat, cinematic camera.
