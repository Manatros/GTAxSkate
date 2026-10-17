# FiveM: prepared friends test (SK8M Lua fallback)

As of 14 October 2026. This test uses SK8M's Lua skating; Skate 3 physics is not
connected. A two-player test remains outstanding. Movement and tricks run on
each player's client. Server-confirmed settings are **not server-authoritative
physics or anti-cheat**. The published friends test is paused because its board
model was missing; the newer source model fix still needs a game test.

## Prepared scope

| Component | Scope |
|---|---|
| Base | SK8M `50b923097691d7c73d6e8eb719cffc5cab40d138`, Apache-2.0; LICENSE/NOTICE included. |
| Rendering | Own procedural board model and GTA standing animation; no Skate 3 files or WASM engine. Sounds disabled. |
| Networking | Existing SK8M network objects and normal player ped; no additional multiplayer engine. |
| Adapter | PushSpeed, PushPower, Braking and Steering multipliers; original values rescaled on each update. |
| Rules | locked/selected/ranges/all; other catalog options remain blocked and hidden. |
| Limits | No F10 menu, complete SK8V port or equivalent difficulty/camera presets. Other settings commands in this test base are disabled. |

The Story installation and original SK8M checkout remain unchanged. Preparation
creates a new package under `build/fivem-friends`. `package-manifest.json` records
source revision and source/overlay/output hashes.

## 1. Prepare locally

1. Run `scripts/prepare-fivem-friends.ps1`, optionally with `-Name my-test`.
   An existing target name is refused.
2. `build/fivem-friends/<Name>` contains four resources and
   `server-local.example.cfg`. Nothing has been installed or started.
3. The example is **not a complete runnable server configuration**. It contains
   no key and uses loopback/`sv_lan 1` for an isolated test on the same PC.
   It does not open a port.

## 2. Set up FiveM and a server manually

1. Each player installs [FiveM](https://fivem.net/) for their own GTA copy.
   Story ASI/Script Hook V are not required.
2. Server files are not included. Follow the
   [official txAdmin guide](https://docs.fivem.net/docs/server-manual/setting-up-a-server-txadmin/).
   The server owner handles sign-in, license terms and administrator access.
3. Create a server key through the official Cfx portal linked in that guide;
   keep it local. Do not put credentials in this package, chats or Git.
4. Successfully start and stop an unchanged CFX default server first.
   Preparation neither downloads nor starts FXServer.
5. Copy the package's four `resources` directories only into a **new isolated
   test server**. If `skateboard`, `gtaxskate_board`, `gtaxskate_rules` or
   `gtaxskate_sk8m` already exists, use a new test server. Do not replace existing
   resources. The rules installer still installs rules only, not this friends test.
6. Add to the server owner's `server.cfg`:

   ```cfg
   sv_scriptHookAllowed 0
   ensure gtaxskate_board
   ensure gtaxskate_rules
   ensure gtaxskate_sk8m
   ensure skateboard
   ```

   For the isolated test, deliberately replace existing endpoints with the
   example's loopback endpoints; do not append duplicate bindings. Keep the
   key and remaining server configuration local.

## 3. Check settings

The default is `locked`: all four server values apply and players cannot change
them. Policy: `resources/gtaxskate_rules/policy.json`.

| Mode | Effect on the four supported settings |
|---|---|
| locked | Blocks every player change, including individually adjustable entries. |
| selected | Only entries explicitly marked `adjustable: true`. |
| ranges | Only explicitly adjustable entries; numeric entries need min and max. |
| all | All four within 0.5–3; individual locks/ranges still apply. |

For a limited test, set `mode` to `ranges` and replace `rules` with:

```json
{
  "PushSpeed": { "adjustable": true, "min": 0.8, "max": 1.1 },
  "PushPower": { "adjustable": true, "min": 0.8, "max": 1.1 },
  "Braking": { "adjustable": true, "min": 0.8, "max": 1.2 },
  "Steering": { "adjustable": false }
}
```

Defaults must fit their ranges. Run `skaterules_reload` in the server console;
using this administrator command in-game requires ACE
`command.skaterules_reload`. An invalid policy preserves the last valid state.
Policy changes stop riders and clear their adjustments; mounting again uses the
new state.

Players: F8 → `skaterules`; `skateset PushPower 1.1`. Accepted supported player
adjustments apply immediately. `skateset Pop 2` stays blocked because Pop is not
implemented. Mounting must fail without a valid policy/adapter. Losing either
while riding stops riding and frees the ped through SK8M's own cleanup.

## 4. Test locally, then with two players

1. Start the server manually. In local FiveM F8: `connect 127.0.0.1:30120`.
2. F6 mounts/dismounts (rebindable in FiveM). This is the SK8M test default,
   not the complete Story control layout.
3. Check pushing, braking, steering, jumping, bailing and mounting again.
4. Before Internet testing, the owner chooses hosting, reachable TCP/UDP game
   endpoints, firewall/router settings and access restrictions. Loopback and
   `sv_lan 1` only permit the isolated local test; friends on other PCs cannot
   connect with those settings. Scripts do not configure public access.
   txAdmin is not the game endpoint. Do not globally relax entity lockdown.
5. Both players check each other's boards/peds, tricks, bail boards,
   distance/streaming, late joining, owner disconnect and reconnect.
6. Stop/restart the rules, adapter and skateboard resources while riding.
   Check for frozen peds/cameras and stale boards. Mounting again must fail
   while valid rules are unavailable.
7. Check locked/selected/ranges/all, locked fields, boundaries and rule changes.
   Grants from an old generation/revision must not authorize a change.

Record client/server logs, ping and resource/frame costs with both players.
Remove keys, account information and identifying paths/addresses before sharing.
Only the tester confirms the real test; offline checks do not approve multiplayer.

## Developer policy and short test

Story: F10 > Presets > Developer enables ShowInputs, debug text, VerboseLog,
KeyboardHints, TrickAnalyzer and the original HUD. Diagnostics and VerboseLog can
be disabled in Developer. This does not change riding physics.

FiveM has a separate, smaller Developer test:

1. Back up the isolated server's existing `policy.json` separately.
2. Copy `policy-presets/developer.json` from the package to
   `resources/gtaxskate_rules/policy.json`. Only the administrator selects this
   policy; players cannot activate it.
3. In the SERVER console, run `skaterules_reload`. All four supported settings
   are adjustable within 0.5–3; defaults are 1.
4. In client F8, run `skatedev` and `skaterules`. Expect a valid generation/
   revision, four values and permissions; no unsupported options presented as
   available. `skatedev` only displays diagnostics.
5. Test F6, W push, S brake, A/D steer and Space ollie. Personal FiveM bindings
   may differ from these SK8M defaults.
6. F8: `skateset PushSpeed 1.1` must display the confirmed value;
   `skateset PushSpeed 99` and `skateset Pop 2` must be refused.
7. Restore the saved locked policy and run SERVER `skaterules_reload`.
   The rider must dismount; further player adjustments must be blocked.
8. With a second player, check mutual boards/tricks, resource stop/restart,
   disconnect and rejoining.

FXServer/txAdmin sign-in and a Cfx key are prerequisites, not ZIP contents. A
configured running server is required before joining. The owner handles platform
license acceptance through the official download flow.

Current source uses the own `gtaxskate_test_board` model (no retail assets),
verified by a CodeWalker binary roundtrip. Missing models or failed entity
creation abort mounting before movement/freeze logic. Visibility, position and
mutual rendering still require game tests. The older published friends test
remains marked failed; this is not confirmed engine/animation parity. Shops/F10
remain future work.
