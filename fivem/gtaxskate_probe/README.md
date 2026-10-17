# Optional FiveM developer capability probe

This separate resource tests the same minimal WebAssembly `answer() == 42`
module in three surfaces: server JavaScript, client scripting JavaScript and
NUI. **Success does not prove the Skate engine runs**, WASI support, sufficient
memory, threading, asset loading, full animation poses or multiplayer parity.
Client reports are untrusted diagnostic observations, never authorization.

For a new isolated developer server, copy this folder into its resources and
add `ensure gtaxskate_probe`. Do not overwrite an existing resource. This work
has not installed or restarted any server.

1. Join with FiveM and open F8. Enter `skateprobe` once.
2. Save the client and server console lines. Status is `ok`, `unsupported`,
   `error` (instantiate/export failed) or `timeout`.
3. Server-console `skateprobe` tests only the server runtime. It cannot test a
   disconnected player's JavaScript or NUI.
4. Record client build, server build/runtime and resource version printed in
   metadata. An unavailable or rejected metadata value is `unknown`.
5. Stop/remove this optional resource after diagnosis.

`unsupported` means the surface did not expose the required WebAssembly API;
`timeout` can also mean the hidden page was not ready or its callback failed.
Wait at least 10 seconds and retry after the resource has finished starting.
Neither status identifies the platform cause by itself. No exception stacks,
user identifiers, local paths or game data are included in reports.

No game assets, controls, pose/movement changes, NUI focus, external requests,
browser flags or security bypasses. The hidden page posts only to its own local
FiveM NUI callback. Requests are rate-limited to one per player per 10 seconds,
reports must match a fresh server-generated token and expire after 10 seconds.
NUI waits at most 5 seconds; oversized/extra fields and unsolicited reports are
discarded. The network event transport's own allocation limits still apply.

Run offline tests: `node --test fivem/gtaxskate_probe/probe.test.cjs` from the
repository root. These VM tests are not a real FiveM capability result.
