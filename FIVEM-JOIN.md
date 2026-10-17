# GTAxSkate: joining the multiplayer test

**Paused / known blocker:** The published test cannot find its board model
(F6: No skateboard model found). No successful friends test is confirmed.
Shops and F10 are missing. Singleplayer validation currently takes priority.

This is an experimental FiveM test using SK8M's Lua skating. It does not yet
include the full Skate 3 engine or Story Mode F10 menu. A successful two-player
test remains outstanding.

## Join a friend

1. Install GTA V **Legacy** and [download FiveM](https://fivem.net/).
2. Start FiveM and select your GTA Legacy installation during setup.
3. Ask the server operator for the current connection address. The server must be running.
4. Open **F8** in FiveM and enter `connect ADDRESS:30120`, replacing ADDRESS
   with the current server address. Alternatively, select the server and Connect.
5. FiveM downloads the skating resources when you join. You do not need the
   server ZIP, Story installer or your own Skate 3 files for this Lua prototype.

| Key | Action |
|---|---|
| F6 | Mount / dismount |
| W / S | Push / brake |
| A / D | Steer |
| Space | Ollie |

F8: `skatedev` shows test diagnostics; `skaterules` shows server rules.
The Developer policy supports PushSpeed, PushPower, Braking and Steering.
The server controls permissions and limits. There is no Story Developer F10 preset here.

## Short two-player test

1. Both players mount: can each see the other player and board?
2. Ride, brake, jump, bail and mount again for five minutes.
3. One player leaves and reconnects: check for stale boards or frozen characters.
4. Report the time, description and relevant F8 message. Remove credentials,
   account names, identifying paths and private connection addresses first.

## Connection problems

- **Timeout / CURL 28:** The game endpoint is unreachable. The host checks the
  running server and TCP/UDP 30120 in the firewall and router.
- **On the server PC:** F8 → `connect 127.0.0.1:30120`.
  Connecting to your own public address may fail because of router behavior.
- **Private / browser Connect unavailable:** An empty `sv_master1 ""` disables
  browser Connect. The operator should review the official server-command
  documentation before changing server visibility and restarting the server.

As of 14 October 2026, all three resources in the published package were detected
on the local server. Internet reachability and shared gameplay remain unconfirmed;
the last reported public-address connection attempt timed out.

## Run your own server

The separate `GTAxSkate-FiveM-Friends-Developer-Test.zip` is a **server package**.
The published package contains three resources, licenses, checksums and a Developer
policy. Newer source preparation uses four resources, including a replacement
board model; that source change is not a tested published package.
The example configuration is only for an isolated local test; an Internet server
needs its own endpoints, registration and access restrictions. See the included
FRIENDS-TEST.md. No server keys are included.

Official guidance: [install FiveM](https://docs.fivem.net/docs/getting-started/installing-fivem/),
[resource downloads](https://docs.fivem.net/docs/scripting-reference/resource-manifest/),
[server commands / sv_master1](https://docs.fivem.net/docs/server-manual/server-commands/).
