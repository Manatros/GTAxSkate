# Features and controls

Everything GTAxSkate adds on top of [SK8V](https://github.com/becaring/SK8V), plus the full key
reference. Back to the [README](../README.md).

## Features (on top of SK8V)

| | |
|---|---|
| Keyboard and mouse | play Skate without a controller: WASD, Shift push, Space ollie, arrow keys for the trick stick, every key rebindable in the menu (F10 > *Keyboard keys*) or `SkateVKeys.txt`, your keys on screen while skating |
| Skateboard shop | **G** in Binco, the Discount Store and Suburban: complete boards, and every deck (by maker), truck, wheel, griptape and paint on its own, paid with the character's cash |
| Your board | the board you buy is the one you ride, with Skate 3's own models: trucks turn and wheels spin; Michael, Franklin and Trevor each keep their own boards |
| Wear | the board wears with riding, grinds, slides and bails and shows Skate 3's worn finish from half worn; a new one starts fresh |
| Customization | griptapes (colours, checkers, prints, or none: the deck's own grip), painted decks, wheels and truck parts (matte, gloss, metallic; painted wheels ride as hard as their paint), at street prices (deck 55-80 $, trucks 45-70 $, wheels 30-45 $, griptape 10-15 $) |
| Boards on show | each store hangs four boards on a rack behind the till (or stands five along a wall), new ones every game day; look at them and press **E** |
| Your own decks | images in `custom-assets/decks/` become decks in the shop ([how](../custom-assets/README.md)) |
| Cars | cars SK8V only knows as boxes are measured once, so their roof, hood and trunk can be ridden and hippie jumped (adjustable height; 1.6 clears a car); how hard the skater pushes cars and how big the dents get are configurable (development menu: *World and roleplay* / *Advanced*) |
| People | optionally only knocked over, not killed |
| Injuries | after a hard bail your character limps on foot for 30 s to 2 min with GTA's injured walk |
| Board in hand | optional (`CarryBoard=1`, off by default): the board picked in the weapon wheel stays in your hand in GTA, swung with GTA's melee; E or F gets on |
| Cameras | **V**: near, middle, far (a calm third person camera that follows the skater, or the board: the development camera prioritizes obstacle clearance over follow distance; the mouse turns it. Geometry clearance still needs in-game regression checks), the skater's eyes (first person), Skate 3's camera. The view is kept between walking and skating; the standard view is set in the menu. **R**: a cinematic camera that cuts like GTA's in a car (wide bars, film look, shallow focus), then a skate filmer (4:3, handheld) |
| Tuning | pop, grind pop, push, braking, steering, wobble, grip, turn power, manual drag, hold fakie in SK8V's menu |
| Stamina | GTA's stamina skill sets push and pop; skating at speed trains it |
| Map | GTA's GPS route stays on while skating; GTA's skate parks are on the map |
| Radio | skating tunes the radio to a station you choose (`SkateRadio=`); `,` / `.` change station. Skate 3's soundtrack (the in-game iPod's 46 songs) on Self Radio: in GTA set Settings > Audio > Self Radio Mode to Radio and run Perform Full Scan once |

Prefer it more like GTA or more like Skate 3? `CameraView=Near` (the installer's default) is the
third person camera, `CameraView=Skate` **Original SK8V / Skate 3**, using SK8V's original camera output (High/Low); **V** switches while skating, and F10 > *Camera*
sets the standard view, what it follows and the cinematic camera's look.

The optional Python/RAGE board composition helper prepares exact shop selections as one skeletal drawable, including split truck paint and griptape. It requires separate setup and is not enabled by the stable installer. See [setup and rollback](../BOARD-COMPOSITION.md).

## Controls (keyboard and mouse)

| Key | Does |
|---|---|
| Weapon wheel: skateboard | skate mode (walking with the board) |
| **F** | board out and skate (on foot, no vehicle near) |
| **E** / hold **E** | get on or off the board / put it away |
| **F** (skating) | back to GTA in one press |
| **W A S D** | steer, lean, walk |
| **Shift** | push |
| **Space** | ollie: hold to crouch, let go |
| **Arrow keys** | trick stick (kickflip: down, then up-left) |
| **Q** | hippie jump |
| **T** / **Ctrl** | Skate's X / B |
| **V** | camera views |
| **R** | cinematic camera, skate filmer |
| **,** / **.** | radio: previous / next station |
| **N** / hold **J** | set a session marker / back to it |
| **M** | GTA's interaction menu on foot; opening it while skating remains unresolved (B104) |
| **G**, or **E** looking at the boards on show | board shop (in a clothing store); **Tab** shows the part alone, **W A S D** or the mouse turn it, the wheel zooms |
| **F10** (or **/** on a US keyboard) | One GTAxSkate settings menu with categories and *Advanced* for uncommon tuning; page *Keyboard keys* selects GTA, Twin Stick, Mouse Flick or Arcade; saves/restores personal bindings and rebinds Skate actions |
| **Tab** / mouse buttons | gun out or away (hold Tab: GTA's weapon wheel); with a gun out, right mouse aims and left shoots |
| **B** (hold, behind a moving car) | skitch |

### Keyboard presets

Choose a keyboard preset under **F10 > Keyboard keys**. The table above describes
**GTA**, the first-run default. Existing key files are preserved until you explicitly
choose a preset. Presets change input only, independently of the play profiles.

| Preset | Translation | Push | Tricks | Grabs | Manual / nose manual |
|---|---|---|---|---|---|
| **GTA** | familiar PC movement and a charged jump | Shift | hold/release Space; arrow stick (Space + Left/Right on release for flips) | Z / C | 1 / 2 |
| **Twin Stick** | two hands replace the two controller sticks | Space | WASD body, IJKL board; K then I ollie, K then I+J/I+L flips | U / O | H / P |
| **Mouse Flick** | mouse movement replaces the right stick | Shift | hold CapsLock, move down then flick up; diagonal flicks for flips | Z / C | 1 / 2 |
| **Arcade** | one key plays a complete stick gesture | Shift | Space ollie, K nollie, U/O flips, H/L shoves, P 360 flip | Z / C | 1 / 2 |

Twin Stick uses **Shift** for Skate's X and hold **5** to return to a marker.
The other presets use **T** and hold **J**. All use **E/F**, **Q**, **B**, **V/R**,
**Tab**, **M**, **G** and **F10** as above. Mouse movement normally looks around;
Mouse Flick borrows it only while CapsLock is physically held. CapsLock also toggles
Windows' capitalization state; you can rebind the clutch to Mouse4/Mouse5 if available.
Aiming uses the mouse normally, and weapon wheels/menus release trick input.
Original SK8V camera behavior is retained; mouse look applies to the existing
mouse-steerable camera options. Mouse sensitivity still needs in-game validation.

Use **Save my personal bindings** before experimenting, then **Restore my personal
bindings** to return. The first preset change also keeps `SkateVKeys.before-presets.txt`;
**Restore bindings from before presets** loads it. The personal slot is
`SkateVKeys.custom.txt`. Individual rebinding activates its required trick mode,
so customized hybrids work; reserved GTA/system keys are rejected. Install the
matching host/runtime pair; mixed versions are unsupported for these extensions.

Controller input follows SK8V; keyboard/mouse support is provided by this fork. The controller camera fix ([issue #1](https://github.com/Manatros/GTAxSkate/issues/1)) is confirmed in game: the right stick does tricks without swinging the camera.

Skate's own postures and riding styles are on F10 > *Skating*.

## Play presets (F10 > Presets)

- **SK8V:** restores every exposed setting to the current base defaults (Easy difficulty, original High Skate camera, standard multipliers), with documented package defaults for extension features.
- **LS Skate:** unavailable until a verified adapter exists. Displays a notice and changes nothing; it cannot supply LS Skate animations in this package.
- **RP:** resets all exposed settings, then uses Hardcore difficulty, lower pop/jump/push values, stamina/injuries, no glitch assist, artificial pedestrian launch, vehicle dents/push or shooting while riding. All Skate overlays are hidden immediately, including trick/x-ray, records, debug/input displays, hints and radio banners. F10 remains usable. These are conservative gameplay values, not a calibrated physical simulation.
- **User:** restores your last manually configured settings. Selecting another profile does not overwrite this snapshot; later manual edits update their corresponding saved User values.
- **Custom:** explicitly save your current settings, then restore that snapshot whenever wanted. Saving again replaces the slot.
- **Developer:** resets to a complete test baseline and enables HUD/input/debug/log diagnostics. Advanced exposes Motorized difficulty, BackwardsMan, trainer multipliers, stamina and other test controls separately. Expensive CPU/stack probes and alternative animation paths stay opt-in; no external cheat trainer is included.

Complete SK8V, RP, User, Custom and Developer settings profiles cover 120 menu settings. Profiles cover every keyed F10 setting, including audio and advanced tuning. They apply only on Enter. Changes marked for restart require a full GTA restart; the master HUD toggle applies live. Settings are validated and saved together before live callbacks. Missing or invalid snapshots leave settings unchanged. User and Custom snapshots persist in `SkateVLegacy.ini`.

Personal keyboard bindings remain independent (F10 > Keyboard keys has its own save/restore). Boards, purchases, wear, records, saves, installation paths and game assets are never replaced by a play profile. Existing settings are untouched until you explicitly apply one. This is a Story Mode profile system, not FiveM/Coop or LS Skate runtime support. LS Skate is explicitly unavailable until its own verified adapter exists. SK8V retains animation and transition ownership.

Developer test settings: Story F10 > Presets > Developer enables input/debug HUD and verbose logs. Developer > Disable diagnostics turns those extras off. The separate FiveM ZIP includes an admin Developer policy and skatedev; see its test guide.

Performance: verbose frame timings no longer run the expensive thread CPU scan
automatically. The separate **Thread CPU scan** diagnostic is off by default and
can itself cause stutter. Switching presets can still pause for about 2 seconds.
