# Skate 3 for GTA V

Skate 3's own physics, flick-it tricks, grinds, wallrides, bails and
animations on your GTA V character, from the Skate 3 Rust engine, as a
Script Hook V plugin (`SkateGTA.asi`). GTA V Legacy, single player.

## Needs

- GTA V (Legacy) with Script Hook V installed.
- The Skate 3 data the 2010 Rust Rewrite Mashup converts from your own
  Skate 3 copy: its `skate-data\assets` folder, with `rig.json` in it.
- An Xbox / XInput controller, or keyboard and mouse.

## Install

1. Copy `SkateGTA.asi` into the GTA V folder (beside `ScriptHookV.dll`).
2. Start GTA V once; the plugin writes `SkateGTA.ini` beside itself.
3. Set `SkateAssets` in `SkateGTA.ini` to the mashup's `skate-data\assets`.

## Play

F9 or clicking both sticks in switches skate mode on and off. F10 opens
the Skate options: Skate 3's difficulty, trucks, wheels and tuning, the
camera, the board, the keyboard and mouse feel and every key, changed on
the spot. Everything is also in `SkateGTA.ini` and
`SkateGTA-controls.txt`.

## Keyboard and mouse

Skate's tricks are right-stick motions, written here as a fighting game
writes them (numpad notation: 2 is down, 8 up, 7 up-left): an ollie is 2
then 8, a kickflip 2 then 7, a shove-it 2-1-4.

- Classic: the arrow keys or the numpad hold the trick stick. Motions are
  typed as motions; two keys pressed or let go a moment apart count as a
  diagonal, like SF6 on a keyboard.
- Modern: one key plays a whole trick: I ollie, K nollie, U kickflip,
  O heelflip, H shove-it, L front shove-it, P 360 flip.
- Mouse: hold Mouse5 (or set it to always) and flick the mouse.
- WASD leans and steers, Space pushes, Z and C grab, Q gets off the board.

## Skateboards in clothing stores

Walk into a clothing store and press E (right on the d-pad) for Skate 3's
own decks, trucks and wheels, paid with the character's cash:

- Binco and the Discount Store: parts, cheap.
- Suburban: parts at street prices.
- Ponsonbys: complete boards only, at boutique prices.

Your character stands in Skate 3's on-foot pose holding the board with the
part you look at, the camera in front of them; Tab (Y) switches to the
board alone, turning in 3D beside the menu (`ShopView`). Up/down picks,
left/right changes the section, Enter buys (or rides what you own),
Space goes back to the default board, Backspace leaves. What you own and
ride is kept in `SkateGTA-boards.txt`.

Every part is sold new and wears as you ride it: the deck from slides,
landings and bails, the trucks from grinds, the wheels from the ground
they cover. Worn through, it shows Skate 3's worn finish. Buying the part
you ride again gets a new one (`SkateGTA-wear.txt` keeps the wear).

The catalog comes from the mashup: when it starts, it offers to convert
Skate 3's board catalog once (select `default.xex` again). GTA V cannot
load new models or textures from a script, so the plugin draws your board
itself from Skate 3's meshes, coloured from their textures. `BoardDetail`
and `BoardTriangles` in `SkateGTA.ini` trade detail for speed; set
`BoardLook = prop` to ride a GTA model instead. If the store prompt never
appears, the store's interior name differs in your game: see
`ShopInteriors`.

## The Skate 3 soundtrack on the radio

`get-skate-music.ps1` turns the soundtrack on your extracted Skate 3 into
MP3s in GTA V's `User Music` folder (it needs vgmstream and ffmpeg, which it
fetches). In GTA V, run Settings > Audio > Self Radio > Full Scan once.
While you skate, the radio then plays Self Radio on foot (`Radio = self`;
any station name works, or `off`), `.` skips a song (`RadioSkipKey`), and
the station you had comes back when you get off.

## Session markers and the checkpoint

As in Skate 3: hold LB and press down to set a marker, hold LB and up to go
back to it (N and M on the keyboard). A bail takes you back by itself a
moment later: to the marker, or without one to where the failed line
started (`Checkpoint = auto`); LB + left (comma) goes back at any time.

## Object dropper

LB + right (Y on the keyboard) drops GTA's own skate park props anywhere:
kickers, ramps, a quarter pipe, a half pipe, a spine, funboxes, rails,
benches, tables and crates. The left stick (WASD) moves the ghost, the
right stick (arrows) turns it, the triggers (Z/C) lower and raise it,
LB/RB (X/B) change the object, A (Space) drops it, X (LShift) picks up the
one under it, Y (Q) picks up all, B (LCtrl) closes. The world scan finds
them like any ground, so they ride, grind and launch as they look.

## Board sounds

The board makes its own sounds, played by the plugin: rolling on each
surface, the cracks in a pavement, pops, landings, truck grinds, board
slides, powerslides and bails (`BoardSound`, 0 to 100).

## Co-op (FiveM or a LAN)

Two copies of the plugin can see each other skate. Set `CoopPeer` to the
other computer's address and port (`192.168.1.20:47800`) on both, and
allow UDP port `CoopPort` (47800) through Windows' firewall. Each sends
its skater: on the other's screen they ride with their pose, board, board
sounds and dropped objects. Over FiveM (a private server with
`set sv_scriptHookAllowed 1`, the plugin in FiveM's `plugins` folder) the
other player's own character is posed; in two story-mode games a skater
stands in for them. Story missions stay each player's own.

## Your own skater: the fourth character

`make-skate-skater.ps1` builds your own Skate 3 skater as a GTA V add-on
ped: every head, hair style, hat, top, pair of pants and shoes of Skate 3's
Create-a-Skater, rebuilt on GTA's own ped skeleton. You import the result
with CodeWalker and OpenIV once (the guide walks through it). In game, F8
(GTA's fourth-character key) turns you into your skater and back, and F11
opens a Skate-style editor: every part and finish, a random skater, the
default one. Your outfit is kept in `SkateGTA-skater.txt`. While you are the
skater, the character wheel waits: F8 takes you back to the story
character you left.

## How it works

GTA gives scripts no collision meshes, so the ground around you is probed
with rays and rebuilt as Skate collision: steps, curbs and ledges are
located to about a centimetre, and their edges become grind rails, as do
handrails the probes happen to hit. Each probe also reads the surface's
material, so the board rolls as Skate 3's does on that ground: concrete,
tiles and wood fast, tarmac, brick and cobbles a little rougher, grass and
dirt slow, sand, mud and snow barely, ice sliding (`Surfaces` in the ini or
the options menu turns this off). The skater's pose is retargeted onto
your skeleton in game memory, found as ScriptHookVDotNet finds it; if this
game version hides it, you skate with GTA's own animation instead.

`SkateGTA.log` beside the plugin records what happened.
