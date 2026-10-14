# FiveM and co-op — experimental groundwork

**Current status:** the Story Mode installer is playable; this separate FiveM
package installs **server settings management only**. It does not yet run or
synchronize Skate 3 skating. The Story Mode ASI is not a FiveM resource.

## Install the Story Mode mod

1. Download [GTAxSkate-Setup.zip](https://github.com/Manatros/GTAxSkate/releases/latest/download/GTAxSkate-Setup.zip).
2. Extract it, close GTA V and run `Install GTAxSkate.cmd`.
3. Follow the prompts for your own Skate 3 copy, GTA V Legacy, stance and optional parts.
4. Start Story Mode. F10 opens settings. See the main README for Script Hook V and supported builds.

This installs the released single-player mod. It does not prepare a FiveM server.

## Prepare FiveM and a test server

1. Install the client from [FiveM](https://fivem.net/) and follow the
   [official client installation guide](https://docs.fivem.net/docs/client-manual/installing-fivem/).
2. Follow [Cfx's txAdmin setup](https://docs.fivem.net/docs/server-manual/setting-up-a-server-txadmin/):
   download the recommended server artifact, extract it and launch FXServer.
3. Complete account linking, create the administrator login and deploy the
   **CFX Default FiveM** recipe with your own Cfx Portal license key.
4. Locate the profile's **server-data folder**, containing `server.cfg` and
   `resources`. Test the unmodified server before adding resources.
5. Stop FXServer. In a checkout of the current development source, run
   `Install GTAxSkate FiveM.cmd` and select that server-data folder. Or run:

   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/install-fivem.ps1 -ServerData "C:\FXServer\server-data"
   ```

   This command is in development source; the existing released setup ZIP may
   not contain it. It installs only `resources/gtaxskate_rules`, preserves an
   existing `policy.json` and custom files, and saves replaced code outside
   `resources` in `gtaxskate-backups`. Local edits to packaged JS/Lua code are
   preserved in that backup, not merged into the updated code.
6. Add these lines to your existing `server.cfg` using txAdmin's CFG editor:

   ```cfg
   sv_scriptHookAllowed 0
   ensure gtaxskate_rules
   # Optional: let your existing administrator group reload the settings.
   add_ace group.admin command.skaterules_reload allow
   ```

   Keep your existing endpoints, license key, permissions, framework and
   resources. The installer does not replace `server.cfg`, install Script Hook
   V into FiveM, change firewall/router settings or install a framework.
7. Start the server. Connect locally from FiveM's F8 console with
   `connect 127.0.0.1:30120` if you retained the default port. Remote hosting
   needs the host's normal TCP/UDP endpoint configuration; use the
   [official server guide](https://docs.fivem.net/docs/server-manual/setting-up-a-server/).
8. Enter `skaterules` in F8. The resource explicitly reports that no skating
   adapter is connected. This is a configuration test, not a skating test.

## Server administrators own the settings

Edit `resources/gtaxskate_rules/policy.json` **on the server**. JSON uses real
booleans (`true`, `false`) and numbers, not quoted substitutes. Run
`skaterules_reload` in the server console after saving; in-game execution
requires the ACE permission above. Invalid reloads retain the last good rules.

| `mode` | Player adjustments |
|---|---|
| `all` | All catalog settings, within package limits; explicit per-setting locks/ranges still apply |
| `selected` | Only settings with `adjustable: true`; optional ranges/choice lists apply |
| `ranges` | Only explicitly enabled settings; enabled numeric settings must have explicit `min` and `max` |
| `locked` | None, even if a rule says `adjustable: true`; server defaults control everything |

The shipped configuration is **locked**. To allow only a few bounded edits:

```json
{
  "version": 1,
  "mode": "ranges",
  "defaults": {
    "Difficulty": "Hardcore",
    "Pop": 1,
    "PushSpeed": 1,
    "BoardAim": false,
    "VehicleDamage": false
  },
  "rules": {
    "CameraView": { "adjustable": true },
    "Pop": { "adjustable": true, "min": 0.8, "max": 1.2 },
    "PushSpeed": { "adjustable": true, "min": 0.8, "max": 1.1 },
    "Difficulty": { "adjustable": true, "values": ["Normal", "Hardcore"] }
  }
}
```

Use `skaterules` to list the complete catalog and allowed values. Test a player
request with `skateset Pop 1.1`; a prohibited request such as `skateset Pop 5`
is rejected. Presets submit one atomic patch through the same server check;
one forbidden field rejects the entire patch. No client may reload the policy.
New/unknown setting names are denied; `all` never enables arbitrary runtime,
debug or administrator controls.

Server reload increments the revision, resets player overrides and notifies
connected players. Overrides are session-only and clear on disconnect. Traffic
is event-based with a small burst budget and a sustained limit of four
requests per second per player; no per-frame network polling is used.

**Enforcement boundary:** these rules govern this resource's stored settings.
They do not yet control another resource's gameplay or prevent modified clients
from changing unrelated local state. A server-side skating adapter must consume
the effective settings for physics, start permission and all mutation paths.
Simply starting this resource beside SK8M does not create that adapter.

## Reuse existing work; keep the bases separate

[Geckomacabre/SK8M](https://github.com/Geckomacabre/SK8M) already contains a
FiveM resource, server-side Skate engine/WASM work, asset conversion and map
collision preparation. We reviewed its manifest, Lua permissions and engine
start path before adding this missing generic rules layer. Its README still
leaves real multiplayer presentation, asset/audio and in-game validation open.
We have not copied its engine or claimed that this resource makes it playable.

The next adapter should reuse that work where verified and permitted, while
covering its engine start, Lua fallback, item checks, settings and presets.
Keep its source/credits and converted private game assets separate. There is
no need to build another multiplayer transport from our old UDP prototype.

Co-op through an existing Story Mode co-op mod is a **separate alternative**.
[RAGECOOP](https://github.com/RAGECOOP/RAGECOOP-V) exposes a resource/API system
and is a candidate for evaluation, not confirmed GTAxSkate compatibility.
FiveM setup and its `server.cfg` do not apply to RAGECOOP. Exact builds, pose,
board and sound synchronization need independent tests before instructions can
promise working co-op.

## Adapter contract and future versions

Our package code and server policy remain outside SK8V/LS Skate sources.
Server exports are `GetPolicy()` and `GetEffectiveSettings(playerId)`; a missing
policy/player returns `null`. A future adapter must deny skating on `null`,
check protocol version and supported capabilities, and use confirmed values on
every start/change. It must stop safely if the rules resource disappears.

Client exports `GetRules()` and `RequestSettings(patch)` are presentation and
request helpers, not trusted gameplay state. Local server event
`gtaxskate:rules:changed(playerId, settings, revision, generation)` informs adapters of
accepted edits/reloads. Client event `gtaxskate:rules:updated(state)` informs a
future menu. Client caches must be discarded after a resource restart. Each
instance has a fresh generation token; requests must match both this token and
the current revision, so old requests cannot become current again after restart.

Compatibility is by versioned interface and capabilities, not just a newer
version number. Unsupported options must be hidden/locked; no silent fallback
may bypass the server. No public LS Skate API has been verified. A stable
interface can make updates routine, but arbitrary future changes cannot be
guaranteed without adapter maintenance. The existing Legacy menu patch still
fails with a clear conflict if an upstream change requires adaptation.

## Checks and remaining work

```powershell
node --test fivem/tests/*.test.js
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-fivem-install.ps1
```

These cover policy and installer behavior without GTA/FiveM. Live FXServer,
reconnect/restart, malicious-event, concurrent-rider and gameplay-adapter tests
remain required before this becomes a supported multiplayer release.
