# Changelog

All notable changes, newest first. Versions follow [semantic versioning](https://semver.org/):
a new minor version for each state confirmed in a game test, a patch version for fixes.
Each version is a tag on GitHub; `main` holds the newest confirmed one, `dev` the work in progress.

## Unreleased (`dev`)

- **Animation**: Skate's pose on your character with SK8V's retarget (GTA keeps its own proportions,
  feet and grabbing hands follow Skate through two-bone IK, roll bones twist correctly).
- **Pose hook**: the pose is written inside GTA's own animation pass instead of against it (no flicker,
  smooth 0.5 s hand-over between GTA and Skate). The old way stays as a fallback.
- Board sounds came back (the sound output started too late and gave up).
- People knocked over by the skater no longer die.
- Bought painted boards are kept when the game loads them again.
- Shop prompt no longer flickers while wanted.
- America griptape removed (the USA print looks better).
- Our own Skate core is the base again; SK8V is a code source, not needed to play.

## 0.2.0-experimental (2026-10-10)

The extras on top of SK8V (board shop, HD parts, keyboard and mouse for SK8V). Never confirmed in a
test; replaced by the own-core line.

## 0.1.0 (2026-10-10)

The first working version, with the plugin's own Skate core (built on the 2010 Rust Rewrite Mashup):
skating at 80–100 fps, the board shop in clothing stores, every Skate 3 deck, truck and wheel as a HD
model, griptapes, painted wheels and trucks, boards on show, skate parks, Skate 3's board sounds,
Hall of Meat, cinematic camera.
