# Original GTAxSkate test board

This resource supplies `gtaxskate_test_board`, a simple visible skateboard for
the experimental FiveM test. It contains no Skate 3/GTA model, image, sound,
animation, extracted game file or downloaded artwork. All mesh positions come
from the included mathematical generator: a capsule deck, box trucks and
cylindrical wheels. Five solid RGB colors are generated as small DDS textures.

Source and generated assets follow this repository's Apache-2.0 license.
CodeWalker's locally installed compiler serializes the public XML schema; its
DLLs are build tools and are not included in the resource.

Build from the repository with `scripts/build-fivem-board.ps1`. The builder
writes `build/fivem-board/gtaxskate_board`, including YDR/YTD/YTYP, manifest,
provenance and binary roundtrip checks. Source XML/DDS and roundtrip XML remain
in `build/fivem-board/source`. No game archives or running resources are edited.

Dimensions: approximately 0.80 m long, 0.215 m wide, 0.105 m high. Local long
axis is X, width Y, up Z. The standing middle deck top is z=0; wheel bottoms
are z=-0.08 m. The existing fallback's BoardYaw=90, FlipAxis=x, BoardZ=0.08
are the intended initial integration. Kick tips rise 0.025 m. Collision is a
conservative box, not a realistic physical simulation of individual wheels.

Server integration: start `gtaxskate_board` before the skateboard resource and
use only `gtaxskate_test_board` as its model (or declare the resource dependency).
The resource streams this original model/texture/archetype to clients.

Compiler/readback success proves serialization and counts, not in-game model
availability, lighting, placement, collision or remote visibility. Validate
those with two clients, F6 mount/dismount, push/ollie/bail, reconnect and restart.
