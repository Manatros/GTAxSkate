# Component licenses and corresponding source

GTAxSkate contains components with different licenses. The top-level Apache-2.0
license does not replace the licenses below or third-party copyright notices.

| Component | License | Source |
|---|---|---|
| GTAxSkate integration and local `gta/crates` | Apache-2.0 | GTAxSkate source distribution |
| `skate-ped.exe` (`gta/skate-ped`) | GPL-3.0-or-later | Corresponding source supplied with the binary release |
| `skate-music.exe` (`gta/skate-music`) | GPL-3.0-or-later | Corresponding source supplied with the binary release |
| SK8V host/runtime and modifications | Apache-2.0, plus dependency terms | Manatros/SK8V, exact revision recorded in the release |
| `skate-xma.exe`, if distributed | LGPL-2.1-or-later | `rust/skate-xma` at the exact SK8V release revision |
| Other linked dependencies | Individual licenses | `THIRD-PARTY-NOTICES.txt` and component source |

Read `LICENSE.txt`, `NOTICE.txt`, `NOTICE-SK8V.txt`,
`THIRD-PARTY-NOTICES.txt` and `licenses/GPL-3.0.txt` in binary packages.
If the decoder is included, its LGPL text must accompany it. Original
third-party authors and legal notices are retained.

## GPL tool source distribution

A binary release containing either GPL tool must also provide its complete
corresponding source. The package includes `GPL-CORRESPONDING-SOURCE.zip` and
`GPL-SOURCE-RECEIPT.json`. The receipt records the exact source revision and
SHA-256 values for that archive and both tool binaries. A link to a moving
branch is not evidence of the source for a particular executable.

The source archive must include both tools, their required local crates,
Cargo manifests and lockfiles, dependency source and license files, build
configuration, modifications, and instructions/scripts needed to build them.
Use the Rust toolchain version recorded by the build and the locked dependency
versions. Preserve all applicable notices. The source offer must remain
available to recipients under the applicable GPL terms.

The packaging check verifies revision and file hashes in the receipt; the
release preparer remains responsible for verifying that the archive is complete
and that both executables were actually rebuilt from that source. A receipt is
not a license-compliance certification.

## Rights and local game assets

No permission to redistribute proprietary game content is granted here.
Players prepare game assets from their own lawful copies. Custom graphics must
be original or used with permission. Check the applicable game's terms and
third-party tool terms before distributing additional material.
