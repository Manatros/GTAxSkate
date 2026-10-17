# Getting help

Use [GTAxSkate issues](https://github.com/Manatros/GTAxSkate/issues) for installation problems, bugs and feature requests. Search existing issues first, then choose the closest issue form. There is no promised response time or dedicated support service.

| Target | Current support boundary |
|---|---|
| Story Mode | Windows, GTA V Legacy 1.0.3889.0. Report the exact GTAxSkate test/release ID and SK8V build. |
| FiveM rules | Experimental server settings resource; installing rules alone does not install skating. |
| FiveM friends test | Experimental pinned SK8M Lua fallback, client-owned physics and four supported adjustments. Requires manual server setup and actual two-client verification. |
| GTA Online, Enhanced, other co-op/LS Skate versions | Do not assume compatibility from Story Mode or FiveM results. Provide exact versions; unverified targets are not advertised as working. |

The [SK8V fork](https://github.com/Manatros/SK8V) contains basis/contribution source. File package problems centrally in GTAxSkate rather than opening duplicate reports in both repositories. If a report belongs to upstream SK8V, include the original issue link when discussing it there.

## Known issues in v0.6.0

- Board lighting indoors and in tunnels can look wrong; deferred after the accepted 17 October Story test.
- Menu organization, text readability, GTA-native styling and simpler navigation are planned improvements.
- LS Skate has no verified adapter. FiveM and Coop remain separate experimental targets.

## Useful evidence

1. State Story Mode or FiveM, GTA build, package version/test ID, install method and other loaded mods/resources.
2. Give the shortest reproducible steps, expected result and actual result. For save-load bugs, distinguish a full game restart from loading another save in the same process.
3. Attach a small relevant log excerpt with timestamps. Story logs normally include `%LOCALAPPDATA%\SkateV\SkateVLegacy.log`, `SkateVRuntime.log` and game-folder `ScriptHookV.log`. Preserve `.prev.log` before another launch replaces it. A `crash.log` may contain memory diagnostics rather than an exception; attach its relevant contents without interpreting the filename as proof.
4. For FiveM, include client F8 and server-console errors, artifact/resource versions, whether one or two clients were tested and which rule mode was active. Rules-only tests do not prove remote board visibility or physics behavior.
5. If a video helps, provide the relevant timestamp and short clip. State what you expected; do not require someone to guess from a long recording.

Redact account names, personal paths/IPs, tokens, server license keys and passwords. Do not post a full `server.cfg`, private game files, converted Skate assets or a memory dump by default. Keep your original logs locally; a redacted copy is enough for the initial report. Read [SECURITY.md](SECURITY.md) for sensitive security findings.

For the optional [board composer](BOARD-COMPOSITION.md), include the test build ID
and selected parts. Keep `worker.ini` and backup receipts local: they contain
machine-specific paths. Redact identifying paths in preparation errors before
posting them. A recording is optional; a precise written report is useful.

## Safe checks before reporting

Close the game/server before reinstalling. Keep the backup/rollback from the test package, confirm the target version and follow that package's test plan. Test a new isolated FiveM server before adding existing server resources. Do not remove your saves or change router/firewall settings to troubleshoot an unrelated Story Mode issue.

See the [README](README.md), [FiveM guide](fivem/README.md) and [friends-test guide](FIVEM-FRIENDS-TEST.md). If a guide and your package disagree, report the package ID and the exact step instead of guessing at compatibility.
