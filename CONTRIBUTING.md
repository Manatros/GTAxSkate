# Contributing

Start with the [README](README.md), [support boundaries](SUPPORT.md) and existing [GTAxSkate issues](https://github.com/Manatros/GTAxSkate/issues). Explain the problem and reproduction before making a large redesign. Keep one independently reviewable behavior change per pull request.

## Choose the correct code boundary

| Change | Location / expectation |
|---|---|
| Installer, package settings, adapter, documentation | [Manatros/GTAxSkate](https://github.com/Manatros/GTAxSkate). Preserve player configuration and existing installation/rollback behavior. |
| SK8V host or basis contribution | [Manatros/SK8V](https://github.com/Manatros/SK8V). Identify the upstream base and contribution branch. Keep the change separable from unrelated package features. |
| SK8M integration | Pin the reviewed SK8M revision; keep overlays and capability checks outside the original source checkout. Preserve Apache notices. Do not silently accept a new incompatible revision. |

Do not replace functioning Story code with experimental multiplayer code. Story support currently targets Windows GTA V Legacy 1.0.3889.0. FiveM rules and the SK8M Lua friends test have separate acceptance criteria; neither establishes compatibility with GTA Online, Enhanced or an unverified future LS Skate API.

## Before editing

1. Check relevant issue history, recent commits and existing adapters/patches. Mention earlier attempts that affect the proposal and what new evidence justifies revisiting them.
2. Identify who owns the behavior: package, basis mod, client, server or rendering/audio worker. Avoid duplicate menus, physics loops or network transports.
3. State supported versions/capabilities and a failure behavior for missing or incompatible dependencies. Retain server-side policy validation and existing inventory checks; do not treat a client-side menu lock as anti-cheat.
4. Keep game assets, local credentials, private paths and generated build outputs out of source changes. Apache-2.0 code attribution belongs in the applicable NOTICE/LICENSE files.

## Validation and pull request evidence

Use the narrow relevant checks and state which ones actually ran. Examples in the development tree include `node --test fivem/tests/*.test.js`, installer fixture tests and host regression harnesses for callback registration, runtime reload and camera clearance. Some checks need a pinned source checkout, Windows/MSVC or locally converted game data; record an unavailable prerequisite instead of claiming a pass.

Include:

- The concrete trigger and resulting behavior, linked issue and exact base/source revision.
- A regression that fails before and passes after when the change warrants a test; avoid tests that only repeat an implementation constant.
- Build/test commands, outcome and relevant log evidence. Distinguish static/source review, mocked tests, full host build and in-game testing.
- For game tests, exact package/build, target and scenario. A local test cannot establish two-client synchronization or server performance.
- Configuration/migration effects, preservation/rollback behavior and known limitations. Creation-only runtime settings can require a full GTA restart.

Use the pull request template. Keep screenshots/logs redacted and never submit Cfx keys or private game assets. If AI tools helped, describe their scope and independently verified findings; do not claim another reviewer approved work when its request failed or its output was not checked.

## Upstream handoff

For a contribution suitable for upstream SK8V, explain why plain upstream needs it, the smallest code change, optional interfaces and older-runtime behavior. Supply reproducible evidence and adoption steps without forcing unrelated package features. Document a merge-only integration fix as such. A successful mock test is not a confirmed game test; preserve that distinction in descriptions and release notes.
