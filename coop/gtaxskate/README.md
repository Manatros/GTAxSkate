# GTAxSkate cooperative pose contract — development groundwork

This folder is outside both pinned bases. It contains a standalone C# numeric
pose codec and bounded session state, plus a real RAGECOOP diagnostic resource
pair under `ragecoop/`. The resource pair performs a bounded capability-presence
challenge/ACK using RAGECOOP's existing Custom Events; it does **not** capture,
relay or render Skate poses. No multiplayer gameplay compatibility is established.
No game installation, SK8V source change, ASI loader, remote skeleton hook or
installer is included. No game archives, converted models, animation clips or
audio are transported here. This is not a legal assessment of derived data.

## RAGECOOP diagnostic resource pair, 2026-10-15

`powershell -NoProfile -ExecutionPolicy Bypass -File coop/gtaxskate/build.ps1`
builds the client/server resources under ignored `build/coop-adapter/Resources`.
It requires the exact staged official Client1.5.12.15, Server1.5.12.13 and
SHVDN3.7.0.193 binaries. Compiler references are hash checked. The server's
real managed assemblies are extracted statically from its pinned single-file
executable; the server is never executed by this build. .NET SDK6.0.428 with
ref pack6.0.36 is used locally from `build/coop-lab/tools/dotnet` by default.
The build does not install the SDK, change PATH or start/install a game/server.

The normal RAGECOOP loader accepts these generated directories under a server's
`Resources/Client/GTAxSkate.Coop.Client` and
`Resources/Server/GTAxSkate.Coop.Server`. Review and stage them with the separate
co-op installer workflow; do not copy an API DLL over the server's own files.
The client resource is sent by RAGECOOP's existing resource distribution.

Server diagnostic acceptance uses the authenticated `e.Client` object and its
current dictionary identity, a client challenge, fresh server nonce and one-use
ACK. No source ID is trusted from event arguments. **RAGECOOP IDs are signed
nonzero Int32**, so negative values are valid. The older pose contract below
requires positive IDs: a future pose adapter needs an explicit server mapping,
not `Abs`, unchecked casts or direct use of a raw RAGECOOP ID.

The client performs all main-thread work from OnTick, uses bounded callback
mailboxes, attempts at most three initial Hello messages five seconds apart,
and unsubscribes/invalidates callbacks on stop. Server state is limited to64
peers with15-second pending expiry and one Hello reply per second per peer.
Receiving a response cannot enable `RemotePoseEnabled`, which remains false.
The public client callback omits transport origin; the reviewed receive path
does not guard CustomEvent by SenderConnection. Accordingly the client only
reports a diagnostic response, never a trusted pose/policy authorization.

The server logs `diagnostic handshake accepted source=...` only after its
authenticated ACK checks. The client logs `roundtrip response received
(diagnostic only)`. A live handshake remains to be checked. Restarting only
the server diagnostic resource after a client has received its session does
not automatically restart that client's bounded initial handshake; reconnect
or restart the client resource for this diagnostic candidate.

`powershell -NoProfile -ExecutionPolicy Bypass -File coop/gtaxskate/tests/run-adapter.ps1`
executes25 event checks against the actual adapter source with API/transport
fixtures. It covers negative IDs, authenticated ownership, malformed protocol,
challenge/session mismatch, callback copying, cross-source ACK, replay,
disconnect, stale callbacks/queued startup, server lifetime and version guards.
It is separate from the actual-binary compile check and does not load real
RAGECOOP assemblies into a test process or establish gameplay acceptance.

## What exists and what does not

The inspected RAGECOOP source supports `byte[]` CustomEvent arguments
(`RageCoop.Core/Packets/CustomEvent.cs`, type `0x15`). Its server callback binds
`CustomEventReceivedArgs.Client` to the transport sender
(`RageCoop.Server/Scripting/API.cs`, `InvokeCustomEventReceived`). CustomEvents
use `ReliableOrdered` in `RageCoop.Server/Client.cs` and the scripting API.

The shared code has no direct dependency on those APIs. The future adapter
must take a **positive source ID from the authenticated connection object**,
not from event arguments. There is no claimed source ID in the pose payload.
It must accept exactly one `byte[]` argument of bounded length, check the
event name/version, and call `ServerPoseSessions.TryAccept`. Never forward raw
unvalidated bytes or use these observations to grant arbitrary native calls.

`BeginAuthenticatedSession` requires a separately approved `SkeletonLayout`.
It issues a fresh server nonce. The layout includes the model hash, ordered
bone tags and ordered parent indices; a SHA-256 fingerprint identifies that
layout. Unknown layouts, duplicate tags, changed model/parents and old epochs
fail closed. **Producing and approving this layout, authenticating its server
announcement, and validating the remote native target are not implemented.**
Creating a layout from any client claim does not make it trusted. The host must
independently verify the actual current ped's skeleton and capability/version
handshake before registration or rendering.

## Version 1 binary schema

Little-endian, IEEE-754 single-precision floats, row-vector GTA matrices. A rigid
matrix contains X/Y/Z axes then translation, 12 floats (48 bytes); implicit
homogeneous components are `0,0,0,1`. Values are not quantized or approximated.
The nonce is the 16-byte result of .NET `Guid.ToByteArray`, treated as opaque
bytes; cross-language code must preserve that byte order.

| Offset | Field |
|---|---|
| 0 | `GXS1` magic, four bytes |
| 4 | uint16 protocol version `1` |
| 6 | uint16 flags: bit 0 = riding; all other bits rejected |
| 8 | 16-byte server-issued session nonce |
| 24 | uint64 increasing sequence, starts at 1; no wrap within an epoch |
| 32 | uint32 ped model hash |
| 36 | uint16 bone count, 1–512 |
| 38 | uint16 reserved, must be zero |
| 40 | Ped entity world rigid matrix |
| 88 | Board entity world rigid matrix |
| 136 | Seven parent-local board bone matrices, 48 bytes each |
| 472 | Character bones, 52 bytes each: uint16 tag, int16 parent index, local matrix |

Board tags are fixed `0..6`: root, front truck, back truck, left-front wheel,
left-back wheel, right-front wheel, right-back wheel. Parent indices are
`-1,0,0,1,2,1,2`. Board bone 0 is entity-relative; every other board bone and
every character bone is parent-relative. Character root uses parent `-1` and
is ped-entity-relative. Exactly one root, unique tags, no cycles, depth ≤64.

Maximum packet: **27,096 bytes**. Length must match exactly; no trailing bytes.
All matrix entries must be finite, axes approximately unit/perpendicular with
positive determinant. Local translations are bounded to ±5 m per axis; entity
world positions to ±32,768 m per axis. These are explicit protocol limits,
not a claim that every possible add-on skeleton or scaled model fits them.

## Experimental raw probe: not approved for live integration

`NativeCapture.cs` is an experimental ABI probe with synthetic checks. Its
lack of explicit setters does **not** make it side-effect-free: runtime
`sv_get_output` (`rust/skatev-runtime/src/lib.rs:556`) replaces `rt.frame`, the
shared presentation latch consumed by `sv_get_character_pose` and
`sv_get_board_pose`. The probe calls that getter twice. Running it beside the
local SK8V host could replace the host-selected presentation frame. Matching
ticks and executing on a main script thread do not establish exclusive frame
ownership, runtime lifetime or safe save/reload scheduling.

The official SK8V 1.1.9 host also has no `SkateModsRuntime` handle export;
that export belongs to our fork. `GetModuleHandle` does not pin either module
or keep the borrowed runtime object alive. Do not activate this probe from
the RAGECOOP adapter or present it as compatible with the official binary.

**Proposal only:** SK8V remains the sole local presentation owner and exposes
a copy of its already-produced frame, with epoch and verified layout/model
identity. The adapter would copy that snapshot without calling runtime Output,
Step or setters. First validate one-way telemetry to the authenticated server;
remote pose writes remain blocked by transport-origin/layout/lifetime checks.
This snapshot-copy interface is not implemented; no SK8V source was changed
for this proposal.

## Requirements for a future host-owned snapshot

Use an explicitly versioned host-owned snapshot copy after verifying its
capabilities and lifetime contract. Capture source tick, current ped
model, verified skeleton tag/parent arrays, ped entity world transform and the
character pose from the same accepted frame. `sv_get_character_pose` alone
does not provide verified bone IDs/parents. Convert world/object matrices into
the parent-local schema, not a matrix layout guessed from the array size.

Include the host's already-produced board pose from that same frame; preserve its entity matrix and
all seven bone transforms. Convert the board world matrices to parent locals
using the fixed hierarchy. Do not substitute engine root/world coordinates
for the character entity matrix, or send only a rigid board when wheel/truck
animation is required. No approved live snapshot/render bridge exists here.

## Bounds and lifecycle

- Server: at most 64 registered sources, 20 accepted frames/s sustained with
  burst 4. Rejected packets also consume a rate token. Decode and structural
  validation happen after the rate gate. Host code must use monotonic elapsed
  milliseconds (e.g. `Stopwatch`), not untrusted client clocks.
- Per-source session: authenticated disconnect removes it; 30 s without a
  valid accepted frame expires it. A new session needs a new nonce and layout
  handshake. Call `Expire` regularly and `Clear` on resource/server teardown.
- Client: at most 64 registered remote sessions, one immutable latest frame
  each, no application FIFO. Only trusted server announcements create/change
  sessions. Call `TryApplyTrustedServerFrame` only after validating server
  origin; call `TryLatest` on the game/script thread. No frame older than
  500 ms since local receipt is returned. Call `Disconnect`/`Clear` on teardown
  and release any native remote pose hooks in the future adapter.
- Shared state is locked for cross-thread use; frame matrices/bones are copied
  on construction and exposed by value. Helpers do not call game natives.

**ReliableOrdered transport can still backlog.** Latest-only application slots
do not erase Lidgren/connection queues, and receipt-time expiry does not prove
that a newly arrived packet was recently captured. Transport backpressure,
fragmentation/loss measurements and possibly a supported unreliable channel
require separate investigation before gameplay deployment.

At 512 bones and 20 Hz the raw owner stream is about **542 kB/s** before
transport overhead, per receiving peer. At 317 bones it is about **339 kB/s**.
Server fan-out multiplies that cost. Quantization, lower rates and transport
changes are future versioned work, not silently applied pose approximations.
Initial real testing must be limited to two peers and measure actual cost.

## Offline verification

`powershell -ExecutionPolicy Bypass -File coop/gtaxskate/tests/run.ps1`

This compiles the actual shared sources with the installed .NET Framework C#
compiler. Tests cover exact roundtrip, 512-bone bounds, truncation/extra bytes,
NaN/infinity/improper rotations, duplicate IDs/cycles/depth, independent root
and wheel matrices, skeleton changes, authenticated-source binding, replay,
nonce reuse, expiry, rate limits and latest-only state. They do not establish
RAGECOOP compatibility or a successful two-player/game-render test.
